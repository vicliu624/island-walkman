//! Conservative application-level pacing for experimental player writes.
//! This does not change USB packet sizes, host controllers or device firmware.
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    time::Duration,
};

pub const CHUNK_BYTES: usize = 32 * 1024;
const BYTES_PER_SECOND: u64 = 1024 * 1024;
const SETTLE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Compatible,
    Standard,
}

impl Profile {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value {
            None | Some("compatible") => Ok(Self::Compatible),
            Some("standard") => Ok(Self::Standard),
            _ => bail!("--device-io must be compatible or standard"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Standard => "standard",
        }
    }
}

fn read_block(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

fn delay(bytes: usize) -> Duration {
    Duration::from_nanos(bytes as u64 * 1_000_000_000 / BYTES_PER_SECOND)
}

fn copy_paced(
    reader: &mut impl Read,
    writer: &mut impl Write,
    mut wait: impl FnMut(Duration),
) -> io::Result<u64> {
    let mut buffer = [0; CHUNK_BYTES];
    let mut total = 0;
    loop {
        let count = read_block(reader, &mut buffer)?;
        if count == 0 {
            return Ok(total);
        }
        writer.write_all(&buffer[..count])?;
        total += count as u64;
        wait(delay(count));
    }
}

fn hash_paced(reader: &mut impl Read, mut wait: impl FnMut(Duration)) -> io::Result<String> {
    let mut hash = Sha256::new();
    let mut buffer = [0; CHUNK_BYTES];
    loop {
        let count = read_block(reader, &mut buffer)?;
        if count == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        hash.update(&buffer[..count]);
        wait(delay(count));
    }
}

pub fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    let expected = crate::metadata::hash_file(source)
        .with_context(|| format!("Cannot read file: {}", source.display()))?;
    let mut input =
        File::open(source).with_context(|| format!("Cannot read file: {}", source.display()))?;
    // Incoming staging paths must be new. Never truncate a file on retry.
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .with_context(|| format!("Cannot create file: {}", destination.display()))?;
    copy_paced(&mut input, &mut output, std::thread::sleep)
        .with_context(|| format!("Cannot copy file: {}", destination.display()))?;
    // Retain the existing durability requirement: one final flush, not a flush
    // after every chunk. An I/O failure is returned without media-error retries.
    output
        .sync_all()
        .with_context(|| format!("Cannot flush device file: {}", destination.display()))?;
    drop(output);
    std::thread::sleep(SETTLE);
    let mut verification = File::open(destination)
        .with_context(|| format!("Cannot verify device file: {}", destination.display()))?;
    let actual = hash_paced(&mut verification, std::thread::sleep)
        .with_context(|| format!("Cannot verify device file: {}", destination.display()))?;
    ensure!(
        actual == expected && crate::metadata::hash_file(source)? == expected,
        "Copy verification failed: {}",
        destination.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[derive(Default)]
    struct ShortWriter {
        bytes: Vec<u8>,
        largest_request: usize,
        flushes: usize,
    }
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.largest_request = self.largest_request.max(bytes.len());
            let count = bytes.len().min(997);
            self.bytes.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }
    #[test]
    fn bounded_chunks_handle_short_writes_and_pace_without_chunk_flushes() {
        let source = vec![0x5a; CHUNK_BYTES * 2 + 17];
        let mut output = ShortWriter::default();
        let mut waited = Duration::ZERO;
        let copied = copy_paced(&mut Cursor::new(&source), &mut output, |pause| {
            waited += pause
        })
        .unwrap();
        assert_eq!(copied, source.len() as u64);
        assert_eq!(output.bytes, source);
        assert_eq!(output.largest_request, CHUNK_BYTES);
        assert_eq!(output.flushes, 0);
        assert!(waited.as_secs_f64() >= source.len() as f64 / BYTES_PER_SECOND as f64 - 1e-9);
        let mut read_wait = Duration::ZERO;
        let hash = hash_paced(&mut Cursor::new(&output.bytes), |pause| read_wait += pause).unwrap();
        assert_eq!(hash, format!("{:x}", Sha256::digest(&source)));
        assert_eq!(waited, read_wait);
    }
    #[test]
    fn media_errors_stop_without_retrying_or_writing_later_chunks() {
        struct Failure {
            calls: usize,
        }
        impl Write for Failure {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.calls += 1;
                if self.calls == 2 {
                    Err(io::Error::from_raw_os_error(1117))
                } else {
                    Ok(bytes.len())
                }
            }
            fn flush(&mut self) -> io::Result<()> {
                panic!("Unexpected flush")
            }
        }
        let mut target = Failure { calls: 0 };
        let error = copy_paced(
            &mut Cursor::new(vec![0; CHUNK_BYTES * 4]),
            &mut target,
            |_| {},
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(1117));
        assert_eq!(target.calls, 2);
    }
    #[test]
    fn existing_destinations_are_never_truncated() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::write(&source, b"new candidate").unwrap();
        std::fs::write(&target, b"existing staged data").unwrap();
        assert!(copy_file(&source, &target).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"existing staged data");
    }
    #[test]
    fn default_is_conservative_and_unknown_profiles_fail() {
        assert_eq!(Profile::parse(None).unwrap(), Profile::Compatible);
        assert_eq!(Profile::parse(Some("standard")).unwrap(), Profile::Standard);
        assert!(Profile::parse(Some("invalid")).is_err());
    }
}
