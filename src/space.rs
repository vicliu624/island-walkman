//! Low-space staging plans and reversible reuse of unchanged on-device files.
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub type Manifest = BTreeMap<String, String>;
pub struct Plan {
    pub changed: Manifest,
    pub reused: Vec<String>,
    pub required_bytes: u64,
}
fn allocation(bytes: u64) -> Result<u64> {
    // Conservative for the tested FAT32 player: reserve up to 64 KiB per file.
    let clusters = bytes
        .max(1)
        .checked_add(65535)
        .context("Staging size overflow")?
        / 65536;
    clusters.checked_mul(65536).context("Staging size overflow")
}
pub fn required_bytes(root: &Path, files: &Manifest) -> Result<u64> {
    let directories = (directories(files).len() as u64)
        .checked_mul(65536)
        .context("Staging size overflow")?;
    let initial = (1024 * 1024u64)
        .checked_add(directories)
        .context("Staging size overflow")?;
    files.keys().try_fold(initial, |total, name| {
        let size = fs::metadata(root.join(name))?.len();
        total
            .checked_add(allocation(size)?)
            .context("Staging size overflow")
    })
}
fn directories(files: &Manifest) -> BTreeSet<PathBuf> {
    files
        .keys()
        .flat_map(|name| Path::new(name).ancestors().skip(1).map(Path::to_path_buf))
        .collect()
}
pub fn plan(root: &Path, original: &Manifest, generated: &Manifest) -> Result<Plan> {
    let mut changed = Manifest::new();
    let mut reused = vec![];
    for (path, hash) in generated {
        if original.get(path) == Some(hash) {
            reused.push(path.clone());
        } else {
            changed.insert(path.clone(), hash.clone());
        }
    }
    // Reused audio needs no new data clusters, but its destination directory
    // entries still require space in the new incoming directory tree.
    let changed_dirs = directories(&changed);
    let additional_dirs = directories(generated).difference(&changed_dirs).count() as u64;
    let required_bytes = required_bytes(root, &changed)?
        .checked_add(
            additional_dirs
                .checked_mul(65536)
                .context("Staging size overflow")?,
        )
        .context("Staging size overflow")?;
    Ok(Plan {
        changed,
        reused,
        required_bytes,
    })
}

/// The host has a complete verified backup before this is called. On the
/// player, only unchanged files are moved; replaced files stay in `backup`.
/// The intent journal contains the complete reuse list before the first move.
pub fn commit(
    original: &Path,
    incoming: &Path,
    backup: &Path,
    reused: &[String],
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
    verify: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    ensure!(
        original.is_dir(),
        "Low-space commit requires an existing library"
    );
    rename(original, backup).context("Cannot retain the old library; new data remains staged")?;
    let mut moved = vec![];
    let outcome = (|| -> Result<()> {
        for name in reused {
            let target = incoming.join(name);
            fs::create_dir_all(target.parent().unwrap())?;
            rename(&backup.join(name), &target)
                .with_context(|| format!("Cannot reuse device file: {name}"))?;
            moved.push(name);
        }
        verify(incoming)?;
        rename(incoming, original).context("Cannot commit the reused library")?;
        Ok(())
    })();
    if let Err(error) = outcome {
        let mut failures = vec![];
        for name in moved.into_iter().rev() {
            if let Err(rollback) = rename(&incoming.join(name), &backup.join(name)) {
                failures.push(format!("{name}: {rollback}"));
            }
        }
        if failures.is_empty() {
            if let Err(rollback) = rename(backup, original) {
                failures.push(rollback.to_string());
            }
        }
        if failures.is_empty() {
            return Err(error).context("Low-space commit failed; original library restored");
        }
        bail!(
            "Low-space commit and rollback failed; use the complete host backup. Device data is in {} and {}: {}; {}",
            backup.display(),
            incoming.display(),
            error,
            failures.join("; ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destination_directories_are_reserved_even_when_the_audio_is_reused() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("new.oma"), b"new audio").unwrap();
        let original = Manifest::from([("nested/audio/old.oma".into(), "old-hash".into())]);
        let mut generated = original.clone();
        generated.insert("new.oma".into(), "new-hash".into());
        let plan = plan(temp.path(), &original, &generated).unwrap();
        let free = 1200 * 1024;
        assert!(required_bytes(temp.path(), &plan.changed).unwrap() <= free);
        assert!(
            plan.required_bytes > free,
            "Directory allocation must also fit before commit starts"
        );
    }
    #[test]
    fn a_small_append_fits_without_duplicating_the_existing_audio() {
        let temp = tempfile::tempdir().unwrap();
        fs::File::create(temp.path().join("old.oma"))
            .unwrap()
            .set_len(8 * 1024 * 1024)
            .unwrap();
        fs::write(temp.path().join("new.oma"), b"new audio").unwrap();
        let original = Manifest::from([("old.oma".into(), "old-hash".into())]);
        let generated = Manifest::from([
            ("old.oma".into(), "old-hash".into()),
            ("new.oma".into(), "new-hash".into()),
        ]);
        let plan = plan(temp.path(), &original, &generated).unwrap();
        assert!(required_bytes(temp.path(), &generated).unwrap() > 4 * 1024 * 1024);
        assert!(plan.required_bytes < 4 * 1024 * 1024);
        assert_eq!(plan.reused, vec!["old.oma"]);
        assert_eq!(plan.changed.keys().collect::<Vec<_>>(), vec!["new.oma"]);
    }
    fn fixture() -> (
        tempfile::TempDir,
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("library");
        let incoming = temp.path().join("incoming");
        let backup = temp.path().join("backup");
        fs::create_dir_all(original.join("audio")).unwrap();
        fs::create_dir_all(incoming.join("audio")).unwrap();
        fs::write(original.join("audio/old.oma"), b"old audio").unwrap();
        fs::write(original.join("index.dat"), b"old index").unwrap();
        fs::write(incoming.join("index.dat"), b"new index").unwrap();
        (temp, original, incoming, backup)
    }
    #[test]
    fn final_rename_failure_moves_reused_audio_back_before_restoring() {
        let (_temp, original, incoming, backup) = fixture();
        let mut calls = 0;
        let error = commit(
            &original,
            &incoming,
            &backup,
            &["audio/old.oma".into()],
            |from, to| {
                calls += 1;
                if calls == 3 {
                    return Err(std::io::Error::other("Injected final rename failure"));
                }
                fs::rename(from, to)
            },
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(error.to_string().contains("original library restored"));
        assert_eq!(
            fs::read(original.join("audio/old.oma")).unwrap(),
            b"old audio"
        );
        assert_eq!(fs::read(original.join("index.dat")).unwrap(), b"old index");
        assert!(!backup.exists());
    }
    #[test]
    fn verification_failure_preserves_the_original() {
        let (_temp, original, incoming, backup) = fixture();
        let error = commit(
            &original,
            &incoming,
            &backup,
            &["audio/old.oma".into()],
            |from, to| fs::rename(from, to),
            |_| bail!("Injected verification failure"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("original library restored"));
        assert_eq!(
            fs::read(original.join("audio/old.oma")).unwrap(),
            b"old audio"
        );
        assert_eq!(fs::read(original.join("index.dat")).unwrap(), b"old index");
    }
    #[test]
    fn persistent_rollback_failure_retains_both_halves_for_host_recovery() {
        let (_temp, original, incoming, backup) = fixture();
        let mut calls = 0;
        let error = commit(
            &original,
            &incoming,
            &backup,
            &["audio/old.oma".into()],
            |from, to| {
                calls += 1;
                if calls >= 3 {
                    return Err(std::io::Error::other("Injected persistent failure"));
                }
                fs::rename(from, to)
            },
            |_| Ok(()),
        )
        .unwrap_err();
        assert!(error.to_string().contains("complete host backup"));
        assert!(!original.exists());
        assert_eq!(fs::read(backup.join("index.dat")).unwrap(), b"old index");
        assert_eq!(
            fs::read(incoming.join("audio/old.oma")).unwrap(),
            b"old audio"
        );
    }
}
