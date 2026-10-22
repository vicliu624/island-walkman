//! Disk app catalog and file-only installer. Never formats, flashes or edits OMGAUDIO.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Serialize, Deserialize)]
pub struct AppFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AppPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api: u32,
    pub target: String,
    pub status: String,
    pub description: String,
    pub files: Vec<AppFile>,
}
#[derive(Serialize, Deserialize)]
pub struct Catalog {
    pub schema: u32,
    pub required_firmware: String,
    pub apps: Vec<AppPackage>,
}
pub fn catalog() -> Result<Catalog> {
    Ok(serde_json::from_str(include_str!(
        "../assets/apps/catalog.json"
    ))?)
}
fn embedded(id: &str, path: &str) -> Result<&'static [u8]> {
    Ok(match (id, path) {
        ("tetris", "app") => include_bytes!("../assets/apps/tetris/app"),
        ("tetris", "app.ini") => include_bytes!("../assets/apps/tetris/app.ini"),
        ("tamagotchi-id", "app") => include_bytes!("../assets/apps/tamagotchi-id/app"),
        ("tamagotchi-id", "app.ini") => include_bytes!("../assets/apps/tamagotchi-id/app.ini"),
        _ => anyhow::bail!("Unknown catalog file"),
    })
}
pub fn published(source: Option<&Path>) -> Result<Catalog> {
    let Some(source) = source else {
        return catalog();
    };
    ordinary(source)?;
    let manifest = source.join("catalog.json");
    ordinary(&manifest)?;
    ensure!(
        fs::metadata(&manifest)?.len() <= 256 * 1024,
        "Catalog is too large"
    );
    let c: Catalog = serde_json::from_slice(&fs::read(manifest)?)?;
    ensure!(c.schema == 1 && c.apps.len() <= 32, "Unsupported catalog");
    let mut ids = std::collections::HashSet::new();
    for a in &c.apps {
        ensure!(
            id_ok(&a.id) && ids.insert(&a.id) && a.api == 1 && a.target == "sony-a918-armv5-oabi",
            "Invalid catalog application"
        );
        ensure!(
            !a.files.is_empty() && a.files.len() <= 32,
            "Invalid application file list"
        );
        let mut paths = std::collections::HashSet::new();
        for f in &a.files {
            ensure!(
                plain_name(&f.path)
                    && !f.path.starts_with("state.")
                    && paths.insert(f.path.to_ascii_lowercase())
                    && f.bytes <= 8 * 1024 * 1024,
                "Unsafe app file"
            );
        }
        ensure!(
            a.files.iter().map(|f| f.bytes).sum::<u64>() <= 16 * 1024 * 1024,
            "Application is too large"
        );
        ensure!(
            a.files.iter().any(|f| f.path == "app") && a.files.iter().any(|f| f.path == "app.ini"),
            "Application needs app and app.ini"
        );
    }
    Ok(c)
}
fn payload(id: &str, f: &AppFile, source: Option<&Path>) -> Result<Vec<u8>> {
    if let Some(source) = source {
        let folder = source.join(id);
        ordinary(&folder)?;
        let path = folder.join(&f.path);
        ordinary(&path)?;
        ensure!(
            fs::metadata(&path)?.len() == f.bytes,
            "Unexpected app file size"
        );
        Ok(fs::read(path)?)
    } else {
        Ok(embedded(id, &f.path)?.to_vec())
    }
}
fn verify_runtime_manifest(pkg: &AppPackage, source: Option<&Path>) -> Result<()> {
    let executable = pkg
        .files
        .iter()
        .find(|f| f.path == "app")
        .context("Missing app executable")?;
    let manifest = pkg
        .files
        .iter()
        .find(|f| f.path == "app.ini")
        .context("Missing app.ini")?;
    let data = payload(&pkg.id, executable, source)?;
    ensure!(
        data.len() >= 52
            && data.len() <= 2 * 1024 * 1024
            && &data[..6] == b"\x7fELF\x01\x01"
            && data[16..20] == [2, 0, 40, 0],
        "Expected a static ARM32 executable"
    );
    let mut crc = 0xffffffffu32;
    for b in &data {
        crc ^= *b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320u32.wrapping_mul(crc & 1));
        }
    }
    crc ^= 0xffffffff;
    ensure!(
        !pkg.version.is_empty()
            && pkg.version.len() <= 24
            && pkg
                .version
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-'),
        "Unsupported app version syntax"
    );
    let text = payload(&pkg.id, manifest, source)?;
    ensure!(text.len() <= 512, "Manifest too large");
    let text = std::str::from_utf8(&text)?;
    let mut values = std::collections::HashMap::new();
    for line in text.lines() {
        let (k, v) = line.split_once('=').context("Invalid app.ini")?;
        ensure!(values.insert(k, v).is_none(), "Duplicate app.ini field");
    }
    ensure!(
        values.len() == 7
            && values.get("api") == Some(&"1")
            && values.get("id") == Some(&pkg.id.as_str())
            && values.get("exec") == Some(&"app")
            && values.get("version") == Some(&pkg.version.as_str()),
        "app.ini does not match catalog"
    );
    let name = values.get("name").context("Missing app name")?;
    ensure!(
        !name.is_empty()
            && name.len() <= 32
            && name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b' ' || b == b'-'),
        "App display name is unsupported"
    );
    ensure!(
        values
            .get("bytes")
            .context("Missing app size")?
            .parse::<usize>()?
            == data.len()
            && u32::from_str_radix(values.get("crc32").context("Missing app checksum")?, 16)?
                == crc,
        "Runtime manifest checksum mismatch"
    );
    Ok(())
}
fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn plain_name(name: &str) -> bool {
    let base = name.split('.').next().unwrap_or("").to_ascii_lowercase();
    let reserved = matches!(base.as_str(), "con" | "prn" | "aux" | "nul")
        || (base.len() == 4
            && (base.starts_with("com") || base.starts_with("lpt"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'));
    !reserved
        && !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && !name.ends_with('.')
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

fn ordinary(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    ensure!(
        !m.file_type().is_symlink(),
        "Symbolic links are not allowed"
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            m.file_attributes() & 0x400 == 0,
            "Reparse points are not allowed"
        );
    }
    Ok(())
}
fn dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir(path)?;
    }
    ordinary(path)?;
    ensure!(path.is_dir(), "Expected a directory");
    Ok(())
}
fn validated(root: &Path) -> Result<PathBuf> {
    crate::protocol::validate_device(root)?;
    Ok(root.canonicalize()?)
}
#[derive(Serialize)]
pub struct Installed {
    pub id: String,
    pub version: String,
    pub name: String,
}
pub fn installed(root: &Path) -> Result<Vec<Installed>> {
    let root = validated(root)?;
    let apps = root.join("APPS");
    if !apps.exists() {
        return Ok(vec![]);
    }
    ordinary(&apps)?;
    let mut out = vec![];
    for e in fs::read_dir(apps)? {
        let e = e?;
        let id = e.file_name().to_string_lossy().into_owned();
        if !id_ok(&id) {
            continue;
        }
        ordinary(&e.path())?;
        if !e.path().is_dir() {
            continue;
        }
        let path = e.path().join("app.ini");
        if !path.exists() {
            continue;
        }
        ordinary(&path)?;
        ensure!(fs::metadata(&path)?.len() <= 512, "Oversized app manifest");
        let text = fs::read_to_string(path)?;
        let get = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(&format!("{key}=")))
                .unwrap_or("")
                .to_owned()
        };
        out.push(Installed {
            id,
            name: get("name"),
            version: get("version"),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}
fn copy_existing(source: &Path, target: &Path, total: &mut u64, depth: usize) -> Result<()> {
    ensure!(depth < 8, "App data too deeply nested");
    for e in fs::read_dir(source)? {
        let e = e?;
        ordinary(&e.path())?;
        let dst = target.join(e.file_name());
        if e.path().is_dir() {
            fs::create_dir(&dst)?;
            copy_existing(&e.path(), &dst, total, depth + 1)?;
        } else {
            ensure!(e.path().is_file(), "Special files are not allowed");
            *total += fs::metadata(e.path())?.len();
            ensure!(
                *total <= 64 * 1024 * 1024,
                "App data exceeds 64 MiB update limit"
            );
            fs::copy(e.path(), &dst)?;
            OpenOptions::new().write(true).open(&dst)?.sync_all()?;
        }
    }
    Ok(())
}
fn verify(path: &Path, f: &AppFile) -> Result<()> {
    let b = fs::read(path)?;
    ensure!(
        b.len() as u64 == f.bytes && format!("{:x}", Sha256::digest(&b)) == f.sha256,
        "App file verification failed: {}",
        f.path
    );
    Ok(())
}
pub fn install(root: &Path, id: &str, source: Option<&Path>) -> Result<serde_json::Value> {
    let _operation = crate::device_operation::acquire()?;
    let root = validated(root)?;
    install_at(&root, id, source)
}
fn install_at(root: &Path, id: &str, source: Option<&Path>) -> Result<serde_json::Value> {
    ensure!(id_ok(id), "Invalid app id");
    let c = published(source)?;
    let pkg = c
        .apps
        .iter()
        .find(|p| p.id == id)
        .context("App is not in the published catalog")?;
    ensure!(
        pkg.api == 1 && pkg.target == "sony-a918-armv5-oabi",
        "Incompatible app"
    );
    for f in &pkg.files {
        ensure!(
            plain_name(&f.path) && !f.path.starts_with("state."),
            "Unsafe package filename"
        );
        let b = payload(id, f, source)?;
        ensure!(
            b.len() as u64 == f.bytes && format!("{:x}", Sha256::digest(b)) == f.sha256,
            "Damaged catalog payload"
        );
    }
    verify_runtime_manifest(pkg, source)?;
    let apps = root.join("APPS");
    dir(&apps)?;
    let live = apps.join(id);
    let stage = apps.join(format!(".install-{id}"));
    ensure!(
        !stage.exists(),
        "An interrupted installation exists; inspect APPS/.install-{id} before retrying"
    );
    let mut existing = 0;
    if live.exists() {
        ordinary(&live)?;
        ensure!(live.is_dir(), "App path is not a directory");
        for (count, e) in walkdir::WalkDir::new(&live)
            .follow_links(false)
            .into_iter()
            .enumerate()
        {
            ensure!(count < 4096, "App has too many data entries");
            let e = e?;
            ordinary(e.path())?;
            if e.file_type().is_file() {
                existing += e.metadata()?.len();
            }
        }
        ensure!(
            existing <= 64 * 1024 * 1024,
            "App data exceeds update limit"
        );
    }
    let required = existing + pkg.files.iter().map(|f| f.bytes).sum::<u64>() + 4 * 1024 * 1024;
    ensure!(
        fs2::available_space(&root)? >= required,
        "Not enough free space for a staged app install"
    );
    fs::create_dir(&stage)?;
    let preparation = (|| -> Result<()> {
        if live.exists() {
            copy_existing(&live, &stage, &mut 0, 0)?;
        }
        for f in &pkg.files {
            let target = stage.join(&f.path);
            if target.exists() {
                ordinary(&target)?;
                ensure!(target.is_file(), "Package target is not a regular file");
                fs::remove_file(&target)?;
            }
            let mut out = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            out.write_all(&payload(id, f, source)?)?;
            out.sync_all()?;
            drop(out);
            verify(&target, f)?;
        }
        Ok(())
    })();
    preparation
        .context("Preparation failed; live app unchanged, staged files retained for inspection")?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let backup = apps.join(format!(".backup-{id}-{stamp}"));
    let had_old = live.exists();
    if had_old {
        fs::rename(&live, &backup)?;
    }
    if let Err(err) = fs::rename(&stage, &live) {
        if had_old {
            fs::rename(&backup, &live)
                .context("Commit failed AND old app restore failed; backup is retained in APPS")?;
        }
        return Err(err.into());
    }
    Ok(
        serde_json::json!({"id":id,"version":pkg.version,"installed":true,"backup":if had_old{Some(backup.to_string_lossy().into_owned())}else{None},"firmware_required":c.required_firmware,"hardware_tested":false}),
    )
}
#[derive(Serialize)]
pub struct Storage {
    pub root: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub partitions: Vec<Partition>,
}
#[derive(Serialize)]
pub struct Partition {
    pub name: String,
    pub accessible: bool,
    pub note: String,
}
pub fn storage(root: &Path) -> Result<Storage> {
    let root = validated(root)?;
    let total = fs2::total_space(&root)?;
    let free = fs2::available_space(&root)?;
    Ok(Storage {
        root: root.to_string_lossy().into_owned(),
        total_bytes: total,
        free_bytes: free,
        used_bytes: total.saturating_sub(free),
        partitions: vec![
            Partition {
                name: "WALKMAN data volume".into(),
                accessible: true,
                note: "Live filesystem capacity reported by the computer".into(),
            },
            Partition {
                name: "Linux root / kernel / recovery / settings".into(),
                accessible: false,
                note: "Not exposed by normal USB storage; size and usage unavailable".into(),
            },
        ],
    })
}
#[derive(Serialize)]
pub struct Entry {
    pub name: String,
    pub directory: bool,
    pub bytes: u64,
    pub path: String,
}
pub fn browse(root: &Path, relative: &str) -> Result<Vec<Entry>> {
    let root = validated(root)?;
    ensure!(
        !relative.contains('\\'),
        "Use slash-separated relative paths"
    );
    let mut path = root.clone();
    for c in Path::new(relative).components() {
        match c {
            std::path::Component::Normal(n) => {
                path.push(n);
                ordinary(&path)?;
            }
            _ => anyhow::bail!("Invalid relative path"),
        }
    }
    ensure!(
        path.canonicalize()?.starts_with(&root),
        "Path escaped device root"
    );
    let mut out = vec![];
    for e in fs::read_dir(path)? {
        let e = e?;
        if ordinary(&e.path()).is_err() {
            continue;
        }
        let m = e.metadata()?;
        let name = e.file_name().to_string_lossy().into_owned();
        out.push(Entry {
            name: name.clone(),
            directory: m.is_dir(),
            bytes: if m.is_file() { m.len() } else { 0 },
            path: if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            },
        });
        ensure!(out.len() <= 10000, "Directory has too many entries");
    }
    out.sort_by(|a, b| b.directory.cmp(&a.directory).then(a.name.cmp(&b.name)));
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn device() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        for n in ["DCIM", "VIDEO", "PICTURE"] {
            fs::create_dir(d.path().join(n)).unwrap();
        }
        d
    }
    #[test]
    fn public_catalog_excludes_commercial_game_inputs() {
        for app in catalog().unwrap().apps {
            assert!(
                app.files
                    .iter()
                    .all(|file| file.path != "rom.bin" && file.path != "new.sav")
            );
        }
        assert!(embedded("tamagotchi-id", "rom.bin").is_err());
        assert!(embedded("tamagotchi-id", "new.sav").is_err());
    }
    #[test]
    fn catalog_hashes_match() {
        for a in catalog().unwrap().apps {
            assert!(id_ok(&a.id));
            for f in a.files {
                let b = embedded(&a.id, &f.path).unwrap();
                assert_eq!(format!("{:x}", Sha256::digest(&b)), f.sha256);
            }
        }
    }
    #[test]
    fn traversal_rejected() {
        let d = device();
        assert!(browse(d.path(), "../").is_err());
        assert!(install(d.path(), "../x", None).is_err());
    }
    #[test]
    fn staged_update_preserves_saves_and_backup() {
        let d = device();
        install_at(d.path(), "tetris", None).unwrap();
        let save = d.path().join("APPS/tetris/state.sav");
        fs::write(&save, b"user save").unwrap();
        let result = install_at(d.path(), "tetris", None).unwrap();
        assert_eq!(fs::read(save).unwrap(), b"user save");
        assert!(Path::new(result["backup"].as_str().unwrap()).is_dir());
        assert!(d.path().join("APPS/tetris/app.ini").is_file());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        let d = device();
        std::os::unix::fs::symlink("/", d.path().join("escape")).unwrap();
        assert!(browse(d.path(), "escape").is_err());
    }
    #[test]
    fn incomplete_install_is_not_overwritten() {
        let d = device();
        fs::create_dir(d.path().join("APPS")).unwrap();
        fs::create_dir(d.path().join("APPS/.install-tetris")).unwrap();
        assert!(install_at(d.path(), "tetris", None).is_err());
    }
    #[test]
    fn catalog_rejects_path_escape() {
        let d = tempfile::tempdir().unwrap();
        let mut c = catalog().unwrap();
        c.apps[0].files[0].path = "../outside".into();
        fs::write(
            d.path().join("catalog.json"),
            serde_json::to_vec(&c).unwrap(),
        )
        .unwrap();
        assert!(published(Some(d.path())).is_err());
    }
    #[test]
    fn corrupt_payload_stops_before_live_change() {
        let d = device();
        let repo = tempfile::tempdir().unwrap();
        let c = catalog().unwrap();
        fs::write(
            repo.path().join("catalog.json"),
            serde_json::to_vec(&c).unwrap(),
        )
        .unwrap();
        fs::create_dir(repo.path().join("tetris")).unwrap();
        for f in &c.apps[0].files {
            fs::write(
                repo.path().join("tetris").join(&f.path),
                embedded("tetris", &f.path).unwrap(),
            )
            .unwrap();
        }
        install_at(d.path(), "tetris", None).unwrap();
        fs::write(repo.path().join("tetris/app"), b"broken").unwrap();
        let before = fs::read(d.path().join("APPS/tetris/app")).unwrap();
        assert!(install_at(d.path(), "tetris", Some(repo.path())).is_err());
        assert_eq!(before, fs::read(d.path().join("APPS/tetris/app")).unwrap());
    }
    #[test]
    fn new_published_app_requires_no_desktop_rebuild() {
        let d = device();
        let repo = tempfile::tempdir().unwrap();
        let mut c = catalog().unwrap();
        let mut app = c.apps.remove(0);
        app.id = "demo".into();
        app.name = "DEMO".into();
        let folder = repo.path().join("demo");
        fs::create_dir(&folder).unwrap();
        for f in &mut app.files {
            let mut b = embedded("tetris", &f.path).unwrap().to_vec();
            if f.path == "app.ini" {
                b = String::from_utf8(b)
                    .unwrap()
                    .replace("id=tetris", "id=demo")
                    .replace("name=TETRIS", "name=DEMO")
                    .into_bytes();
            }
            f.bytes = b.len() as u64;
            f.sha256 = format!("{:x}", Sha256::digest(&b));
            fs::write(folder.join(&f.path), b).unwrap();
        }
        c.apps = vec![app];
        fs::write(
            repo.path().join("catalog.json"),
            serde_json::to_vec(&c).unwrap(),
        )
        .unwrap();
        install_at(d.path(), "demo", Some(repo.path())).unwrap();
        assert!(d.path().join("APPS/demo/app").is_file());
    }
    #[test]
    fn windows_aliases_and_case_collisions_rejected() {
        assert!(!plain_name("CON.txt"));
        assert!(!plain_name("app."));
        let d = tempfile::tempdir().unwrap();
        let mut c = catalog().unwrap();
        let mut duplicate = c.apps[0].files[0].clone();
        duplicate.path = "APP".into();
        c.apps[0].files.push(duplicate);
        fs::write(
            d.path().join("catalog.json"),
            serde_json::to_vec(&c).unwrap(),
        )
        .unwrap();
        assert!(published(Some(d.path())).is_err());
    }
}
