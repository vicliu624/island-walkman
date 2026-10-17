//! OMA tag serialization and observed OMGAUDIO device readers.
pub use crate::database::write_database;
use crate::metadata::{self, Track};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

fn u32b(v: &mut Vec<u8>, n: usize) {
    v.extend((n as u32).to_be_bytes());
}
fn be16(b: &[u8]) -> usize {
    u16::from_be_bytes([b[0], b[1]]) as usize
}
fn be32(b: &[u8]) -> usize {
    u32::from_be_bytes(b[..4].try_into().unwrap()) as usize
}
fn utf16(s: &str) -> Vec<u8> {
    s.encode_utf16().flat_map(u16::to_be_bytes).collect()
}
pub fn fixed_field(tag: &str, text: &str) -> Vec<u8> {
    let mut v = vec![0; 128];
    v[..4].copy_from_slice(tag.as_bytes());
    v[5] = 2;
    let mut pos = 6;
    // Keep a full UTF-16 NUL terminator, never split a surrogate pair.
    for c in text.chars().filter(|c| *c != '\0') {
        let x = utf16(&c.to_string());
        if pos + x.len() > 126 {
            break;
        }
        v[pos..pos + x.len()].copy_from_slice(&x);
        pos += x.len();
    }
    v
}
fn text_frame(out: &mut Vec<u8>, tag: &str, text: &str) {
    let mut p = vec![2];
    p.extend(utf16(text));
    frame(out, tag, &p);
}
fn frame(out: &mut Vec<u8>, tag: &str, payload: &[u8]) {
    out.extend(tag.as_bytes());
    u32b(out, payload.len());
    out.extend([0, 0]);
    out.extend(payload);
}
pub fn oma_name(id: u16) -> PathBuf {
    PathBuf::from(format!(
        "10F{:02X}/{:08X}.OMA",
        id as usize / 256,
        0x10000000u32 + id as u32
    ))
}
pub fn write_oma(track: &Track, dest: &Path) -> Result<()> {
    let mut frames = vec![];
    for (tag, s) in [
        ("TIT2", track.title.clone()),
        ("TPE1", track.artist.clone()),
        ("TALB", track.album.clone()),
        ("TPE2", track.album_artist.clone()),
        ("TCON", track.genre.clone()),
        ("TYER", track.year.to_string()),
        ("TRCK", format!("{}/{}", track.track, track.track_total)),
        ("TPOS", format!("{}/{}", track.disc, track.disc_total)),
        ("TLEN", track.duration_ms.to_string()),
    ] {
        text_frame(&mut frames, tag, &s);
    }
    text_frame(&mut frames, "TXXX", &format!("OMG_TRACK\0{}", track.track));
    if let Some(cover) = &track.cover {
        let data = fs::read(cover)?;
        let mut payload = b"\0image/jpeg\0\x03\0".to_vec();
        payload.extend(data);
        frame(&mut frames, "APIC", &payload);
    }
    let total = (frames.len() + 10).max(3072).div_ceil(128) * 128;
    ensure!(total < 1 << 28, "OMA tags are too large");
    let n = (total - 10) as u32;
    let mut out = File::create(dest)?;
    out.write_all(&[
        b'e',
        b'a',
        b'3',
        3,
        0,
        0,
        ((n >> 21) & 127) as u8,
        ((n >> 14) & 127) as u8,
        ((n >> 7) & 127) as u8,
        (n & 127) as u8,
    ])?;
    out.write_all(&frames)?;
    out.write_all(&vec![0; total - 10 - frames.len()])?;
    let mut ea = vec![0; 96];
    ea[..8].copy_from_slice(b"EA3\x02\0\x60\xff\xff");
    ea[32..36].copy_from_slice(&track.properties);
    ea[36..40].copy_from_slice(&track.duration_ms.to_be_bytes());
    ea[40..44].copy_from_slice(&track.frames.to_be_bytes());
    out.write_all(&ea)?;
    let mut source = File::open(&track.path)?;
    source.seek(SeekFrom::Start(track.audio_offset))?;
    std::io::copy(&mut source, &mut out)?;
    out.sync_all()?;
    Ok(())
}

impl Track {
    pub fn album_artist_or_artist(&self) -> &str {
        if self.album_artist.is_empty() {
            &self.artist
        } else {
            &self.album_artist
        }
    }
}
pub fn read_device(root: &Path) -> Result<Vec<Track>> {
    let dir = root.join("OMGAUDIO");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let track_covers = crate::sony_artwork::existing_track_covers(&dir)?;
    let mut album_covers = None;
    let b = fs::read(dir.join("04CNTINF.DAT")).context("Cannot read the device music database")?;
    ensure!(
        b.len() >= 48 && &b[..4] == b"CNIF",
        "Unrecognized device database"
    );
    let start = be32(&b[20..24]);
    ensure!(start + 16 <= b.len(), "Database header is out of bounds");
    let count = be16(&b[start + 4..]);
    let size = be16(&b[start + 6..]);
    ensure!(
        size >= 0x290 && count <= 60000,
        "Invalid database record size"
    );
    ensure!(
        start + 16 + size * count <= b.len(),
        "Database is truncated"
    );
    let mut out = vec![];
    for i in 0..count {
        let r = &b[start + 16 + i * size..start + 16 + (i + 1) * size];
        if r[..12].iter().all(|v| *v == 0) {
            continue;
        }
        let id = (i + 1) as u16;
        let p = dir.join(oma_name(id));
        ensure!(p.is_file(), "OMA file missing for track {id}");
        let text = |index: usize| -> String {
            let z = &r[16 + index * 128 + 6..16 + (index + 1) * 128];
            let v: Vec<u16> = z
                .chunks_exact(2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .take_while(|x| *x != 0)
                .collect();
            String::from_utf16_lossy(&v)
        };
        let mut t = Track {
            key: format!("device-{id}"),
            path: p.to_string_lossy().into(),
            title: text(0),
            artist: text(1),
            album: text(2),
            genre: text(3),
            duration_ms: be32(&r[8..12]) as u32,
            properties: r[4..8].try_into().unwrap(),
            bytes: p.metadata()?.len(),
            device_id: Some(id),
            tag_version: "OMA / EA3".into(),
            ..Default::default()
        };
        // Extract full text from the EA3 header rather than silently keeping truncated DB text.
        let mut f = File::open(&p)?;
        let mut h = [0; 10];
        f.read_exact(&mut h)?;
        if &h[..3] == b"ea3" {
            let n = metadata::synchsafe(&h[6..10]) as usize;
            ensure!(n < 16 * 1024 * 1024, "OMA tags exceed size limits");
            let mut tag = vec![0; n];
            f.read_exact(&mut tag)?;
            let mut pos = 0;
            while pos + 10 <= tag.len() && tag[pos] != 0 {
                let name = &tag[pos..pos + 4];
                let len = be32(&tag[pos + 4..pos + 8]);
                if pos + 10 + len > tag.len() {
                    break;
                }
                let payload = &tag[pos + 10..pos + 10 + len];
                if payload.first() == Some(&2) {
                    let units: Vec<u16> = payload[1..]
                        .chunks_exact(2)
                        .map(|x| u16::from_be_bytes([x[0], x[1]]))
                        .collect();
                    let s = String::from_utf16_lossy(&units);
                    match name {
                        b"TIT2" => t.title = s,
                        b"TPE1" => t.artist = s,
                        b"TALB" => t.album = s,
                        b"TPE2" => t.album_artist = s,
                        b"TYER" => t.year = s.parse().unwrap_or(0),
                        b"TRCK" => {
                            let mut parts = s.split('/');
                            t.track = parts.next().unwrap_or("").parse().unwrap_or(0);
                            t.track_total = parts.next().unwrap_or("").parse().unwrap_or(0);
                        }
                        b"TPOS" => {
                            let mut parts = s.split('/');
                            t.disc = parts.next().unwrap_or("").parse().unwrap_or(0);
                            t.disc_total = parts.next().unwrap_or("").parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
                pos += 10 + len;
            }
        }
        t.cover = track_covers.get(&id).cloned();
        if t.cover.is_none() {
            if album_covers.is_none() {
                album_covers = Some(crate::sony_artwork::existing_album_covers(&dir)?);
            }
            t.cover = album_covers
                .as_ref()
                .unwrap()
                .get(&(t.album_artist_or_artist().to_owned(), t.album.clone()))
                .cloned();
        }
        out.push(t);
    }
    Ok(out)
}
pub fn validate_device(root: &Path) -> Result<()> {
    ensure!(root.is_dir(), "Device directory does not exist");
    let canonical = root.canonicalize()?;
    #[cfg(windows)]
    {
        let system = std::env::var_os("SystemDrive")
            .and_then(|value| windows_drive(Path::new(&value)))
            .unwrap_or(b'C');
        validate_windows_drive(&canonical, system)?;
    }
    #[cfg(unix)]
    ensure!(
        canonical != Path::new("/"),
        "Refusing to write to the filesystem root"
    );
    ensure!(
        canonical.join("DCIM").is_dir()
            && canonical.join("VIDEO").is_dir()
            && canonical.join("PICTURE").is_dir(),
        "Not an expected A918 root directory"
    );
    Ok(())
}

#[cfg(windows)]
fn windows_drive(path: &Path) -> Option<u8> {
    use std::path::{Component, Prefix};
    match path.components().next()? {
        Component::Prefix(prefix) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => {
                Some(letter.to_ascii_uppercase())
            }
            _ => None,
        },
        _ => None,
    }
}
#[cfg(windows)]
fn validate_windows_drive(path: &Path, system: u8) -> Result<()> {
    let drive = windows_drive(path).context("Windows target must use a local drive letter")?;
    ensure!(
        drive != system.to_ascii_uppercase(),
        "Refusing to write to the system drive"
    );
    Ok(())
}

/// The current writer emits an empty playlist tree. Until playlist editing is
/// implemented, fail before touching the device instead of erasing user lists.
pub fn validate_rebuild_supported(root: &Path) -> Result<()> {
    for name in ["01TREE22.DAT", "03GINF22.DAT"] {
        let path = root.join("OMGAUDIO").join(name);
        if !path.exists() {
            continue;
        }
        let bytes = fs::read(&path)
            .with_context(|| format!("Cannot read playlists: {}", path.display()))?;
        ensure!(bytes.len() >= 16, "Truncated playlist header: {name}");
        let sections = bytes[8] as usize;
        ensure!(
            sections > 0 && 16 + sections * 16 <= bytes.len(),
            "Invalid playlist directory: {name}"
        );
        for i in 0..sections {
            let descriptor = &bytes[16 + i * 16..32 + i * 16];
            let start = be32(&descriptor[4..8]);
            let length = be32(&descriptor[8..12]);
            ensure!(
                start >= 16 + sections * 16
                    && length >= 16
                    && start
                        .checked_add(length)
                        .is_some_and(|end| end <= bytes.len()),
                "Playlist section out of bounds: {name}"
            );
            ensure!(
                bytes[start..start + 4] == descriptor[..4],
                "Playlist section identifier mismatch: {name}"
            );
            ensure!(
                be16(&bytes[start + 4..]) == 0,
                "Existing device playlists are not yet supported; transfer stopped without changing the library"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    #[cfg(windows)]
    #[test]
    fn windows_drive_policy_handles_verbatim_prefixes_and_alternate_system_drives() {
        for path in [r"C:\WALKMAN", r"\\?\C:\WALKMAN", r"c:\WALKMAN"] {
            assert!(validate_windows_drive(Path::new(path), b'C').is_err());
        }
        assert!(validate_windows_drive(Path::new(r"\\?\D:\WALKMAN"), b'D').is_err());
        for path in [r"F:\", r"\\?\F:\"] {
            validate_windows_drive(Path::new(path), b'C').unwrap();
        }
        for path in [
            r"\\server\share\WALKMAN",
            r"\\?\Volume{not-a-drive-letter}\",
        ] {
            assert!(validate_windows_drive(Path::new(path), b'C').is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    fn root_aliases_are_rejected_before_device_signature_checks() {
        for path in [Path::new("/"), Path::new("/./")] {
            assert!(
                validate_device(path)
                    .unwrap_err()
                    .to_string()
                    .contains("filesystem root")
            );
        }
        let temp = tempfile::tempdir().unwrap();
        let alias = temp.path().join("root-alias");
        std::os::unix::fs::symlink("/", &alias).unwrap();
        assert!(
            validate_device(&alias)
                .unwrap_err()
                .to_string()
                .contains("filesystem root")
        );
    }
    #[cfg(windows)]
    #[test]
    fn device_validation_keeps_system_drive_checks_after_canonicalization() {
        use std::path::{Component, Prefix};
        let temp = tempfile::tempdir().unwrap();
        for name in ["DCIM", "VIDEO", "PICTURE"] {
            fs::create_dir(temp.path().join(name)).unwrap();
        }
        let canonical = temp.path().canonicalize().unwrap();
        let drive = match canonical.components().next().unwrap() {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d.to_ascii_uppercase(),
                _ => panic!("Expected local temporary drive"),
            },
            _ => panic!("Expected Windows drive prefix"),
        };
        let system_drive = std::env::var("SystemDrive")
            .unwrap_or_else(|_| "C:".into())
            .as_bytes()[0]
            .to_ascii_uppercase();
        assert_eq!(validate_device(&canonical).is_err(), drive == system_drive);
    }
    #[test]
    fn rebuild_refuses_existing_playlists_and_malformed_sections() {
        let d = tempfile::tempdir().unwrap();
        let db = d.path().join("OMGAUDIO");
        fs::create_dir(&db).unwrap();
        write_database(&db, &[], false).unwrap();
        validate_rebuild_supported(d.path()).unwrap();
        let path = db.join("01TREE22.DAT");
        let original = fs::read(&path).unwrap();
        let mut populated = original.clone();
        populated[52..54].copy_from_slice(&1u16.to_be_bytes());
        fs::write(&path, &populated).unwrap();
        assert!(
            validate_rebuild_supported(d.path())
                .unwrap_err()
                .to_string()
                .contains("Existing device playlists")
        );
        assert_eq!(fs::read(&path).unwrap(), populated);
        let mut truncated = original;
        truncated[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
        fs::write(path, truncated).unwrap();
        assert!(validate_rebuild_supported(d.path()).is_err());
    }
    #[test]
    fn oma_preserves_full_unicode_and_disc_numbers_with_unmodified_audio() {
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("source.mp3");
        let audio: Vec<u8> = (0..=255).collect();
        let mut input = vec![1, 2, 3, 4];
        input.extend(&audio);
        fs::write(&source, &input).unwrap();
        let t = Track {
            path: source.to_string_lossy().into_owned(),
            audio_offset: 4,
            device_id: Some(1),
            title: format!("{}🎵", "超长曲名".repeat(25)),
            artist: "艺术家 🎸".into(),
            album: "Album".into(),
            album_artist: "合辑艺术家".into(),
            year: 2003,
            track: 9,
            track_total: 20,
            disc: 2,
            disc_total: 3,
            ..Default::default()
        };
        let db = d.path().join("OMGAUDIO");
        let oma = db.join(oma_name(1));
        fs::create_dir_all(oma.parent().unwrap()).unwrap();
        write_oma(&t, &oma).unwrap();
        write_database(&db, std::slice::from_ref(&t), false).unwrap();
        let result = read_device(d.path()).unwrap().remove(0);
        assert_eq!(result.title, t.title);
        assert_eq!(result.artist, t.artist);
        assert_eq!(result.album_artist, t.album_artist);
        assert_eq!(
            (
                result.track,
                result.track_total,
                result.disc,
                result.disc_total
            ),
            (9, 20, 2, 3)
        );
        let bytes = fs::read(oma).unwrap();
        let offset = 10 + metadata::synchsafe(&bytes[6..10]) as usize + 96;
        assert_eq!(&bytes[offset..], audio);
        assert_eq!(fs::read(source).unwrap(), input);
    }
    #[test]
    fn firmware_thumbnail_tree_resolves_each_album_without_crossing_artists() {
        let d = tempfile::tempdir().unwrap();
        let cover = d.path().join("cover.jpg");
        image::RgbImage::from_pixel(60, 40, image::Rgb([210, 80, 30]))
            .save(&cover)
            .unwrap();
        let db = d.path().join("OMGAUDIO");
        fs::create_dir(&db).unwrap();
        let tracks: Vec<Track> = [
            (1, "甲", false),
            (3, "甲", true),
            (8, "乙", true),
            (12, "丙", false),
        ]
        .into_iter()
        .map(|(id, artist, has_cover)| Track {
            device_id: Some(id),
            title: id.to_string(),
            artist: artist.into(),
            album: "同名专辑".into(),
            duration_ms: 1000,
            cover: has_cover.then(|| cover.to_string_lossy().into_owned()),
            ..Default::default()
        })
        .collect();
        write_database(&db, &tracks, true).unwrap();
        let info = fs::read(db.join("02TREINF.DAT")).unwrap();
        let marker = 48 + 44 * 144 + 16;
        assert_eq!(
            &info[marker..marker + 128],
            fixed_field("TIT2", "STD_TPE1_THUMBNAIL")
        );
        let groups = fs::read(db.join("03GINF2D.DAT")).unwrap();
        let stride = be16(&groups[38..]);
        assert_eq!(stride, 0x210);
        let tree = fs::read(db.join("01TREE2D.DAT")).unwrap();
        let count = be16(&tree[52..]);
        let mut leaves = vec![];
        for i in 0..count {
            let row = &tree[64 + i * 8..64 + (i + 1) * 8];
            if row[2] == 2 {
                leaves.push((be16(row), be16(&row[4..])));
            }
        }
        assert_eq!(leaves.len(), 3);
        let ids: Vec<usize> = (0..4).map(|i| be16(&tree[0x4050 + i * 2..])).collect();
        let mut found = BTreeMap::new();
        for (i, (group, start)) in leaves.iter().enumerate() {
            let end = leaves.get(i + 1).map(|x| x.1 - 1).unwrap_or(ids.len());
            let row = &groups[48 + (group - 1) * stride..48 + group * stride];
            assert_eq!(&row[16 + 128..16 + 132], b"PICP");
            let raw = &row[16 + 128 + 6..16 + 256];
            let units: Vec<u16> = raw
                .chunks_exact(2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .take_while(|v| *v != 0)
                .collect();
            let pic = String::from_utf16(&units).unwrap();
            if !pic.is_empty() {
                let path = pic.split_once(';').unwrap().1.replace('\\', "/");
                assert!(db.join(&path).is_file());
                assert_eq!(image::image_dimensions(db.join(&path)).unwrap(), (200, 200));
            }
            for id in &ids[start - 1..end] {
                assert!(found.insert(*id, pic.clone()).is_none());
            }
        }
        assert!(!found[&1].is_empty()); // second song supplies the first album's cover
        assert_eq!(found[&1], found[&3]);
        assert_ne!(found[&1], found[&8]); // identical album names retain separate owners
        assert!(found[&12].is_empty());
        // Read the generated library through the real append-transfer path,
        // then rebuild its indexes in a new stage. Existing covers must survive.
        for t in &tracks {
            let path = db.join(oma_name(t.device_id.unwrap()));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, [0; 10]).unwrap();
        }
        let existing = read_device(d.path()).unwrap();
        assert!(
            existing
                .iter()
                .find(|t| t.device_id == Some(1))
                .unwrap()
                .cover
                .is_some()
        );
        assert!(
            existing
                .iter()
                .find(|t| t.device_id == Some(12))
                .unwrap()
                .cover
                .is_none()
        );
        let next = d.path().join("next-stage");
        fs::create_dir(&next).unwrap();
        write_database(&next, &existing, true).unwrap();
        let covers = crate::sony_artwork::existing_album_covers(&next).unwrap();
        assert_eq!(covers.len(), 2);
    }
    #[test]
    fn unicode_record_boundaries() {
        let s = format!("{}🎵中", "A".repeat(59));
        let f = fixed_field("TIT2", &s);
        assert_eq!(f.len(), 128);
        assert_eq!(&f[124..], &[0, 0, 0, 0]);
    }
    #[test]
    fn multiple_albums_and_sparse_ids_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        fs::create_dir(root.join("OMGAUDIO")).unwrap();
        let mut ts = vec![];
        for (id, a) in [(1, "甲"), (3, "乙")] {
            let t = Track {
                device_id: Some(id),
                title: "中文标题".into(),
                artist: a.into(),
                album: "同名专辑".into(),
                duration_ms: 1000,
                properties: [3, 128, 217, 16],
                ..Default::default()
            };
            let p = root.join("OMGAUDIO").join(oma_name(id));
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, vec![0; 10]).unwrap();
            ts.push(t);
        }
        write_database(&root.join("OMGAUDIO"), &ts, false).unwrap();
        let read = read_device(root).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[1].device_id, Some(3));
        assert_eq!(read[0].title, "中文标题");
        let b = fs::read(root.join("OMGAUDIO/03GINF01.DAT")).unwrap();
        assert_eq!(be16(&b[36..]), 2);
        assert!(root.join("OMGAUDIO/01TREE2D.DAT").is_file());
    }
}
