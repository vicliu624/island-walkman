//! Shared host-side exclusion for media writes and firmware device operations.
//! Conservative across all devices because volume and USB identities cannot yet
//! be reliably correlated. Keep the guard through reconnect, readback and cleanup.
//! Future firmware executors MUST acquire this guard before touching hardware;
//! a pure offline firmware Session is not an executor and owns no hardware lock.
use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub struct Guard {
    _file: File,
}

pub fn acquire() -> Result<Guard> {
    let path = std::env::temp_dir().join("island-walkman-device-operation-v1.lock");
    // Isolate unrelated unit-test fixtures; contention itself is tested below
    // using independent handles to the very same lock file.
    #[cfg(test)]
    let path = path.with_extension(format!("{:?}.lock", std::thread::current().id()));
    acquire_at(&path)
}

fn acquire_at(path: &Path) -> Result<Guard> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .context("Cannot open device operation lock")?;
    fs2::FileExt::try_lock_exclusive(&file).context(
        "Device is busy: finish or pause the music transfer or firmware operation first",
    )?;
    // Never unlink: replacing a locked file could let another process bypass it.
    Ok(Guard { _file: file })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn firmware_entries_reject_an_existing_device_operation() {
        let operation = acquire().unwrap();
        let error = crate::firmware::probe().unwrap_err();
        assert!(error.to_string().contains("Device is busy"));
        #[cfg(windows)]
        assert!(
            crate::firmware::windows_readonly::probe("invalid", "invalid")
                .unwrap_err()
                .to_string()
                .contains("Device is busy")
        );
        drop(operation);
        assert!(acquire().is_ok());
    }

    #[test]
    fn firmware_and_music_exclude_each_other_and_release_on_error() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("operation.lock");
        for _ in 0..2 {
            let first = acquire_at(&path).unwrap();
            let other = path.clone();
            assert!(
                std::thread::spawn(move || acquire_at(&other).is_err())
                    .join()
                    .unwrap()
            );
            drop(first);
            let failure = || -> Result<()> {
                let _guard = acquire_at(&path)?;
                anyhow::bail!("injected I/O failure")
            };
            assert!(failure().is_err());
            assert!(acquire_at(&path).is_ok());
        }
    }
}
