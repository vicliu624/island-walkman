use crate::{
    metadata::{self, Track},
    protocol,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tiny_http::{Header, Request, Response, Server, StatusCode};

#[derive(Clone, Default, Serialize, Deserialize)]
struct Job {
    running: bool,
    percent: u8,
    message: String,
    logs: Vec<String>,
    backup: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct State {
    tracks: Vec<Track>,
    queue: Vec<String>,
    device_root: String,
    device_tracks: Vec<Track>,
    job: Job,
}
struct App {
    state: Mutex<State>,
    persist: Mutex<()>,
    requests: Mutex<()>,
    data: PathBuf,
    ui: PathBuf,
    token: String,
    allow_device_write: bool,
    device_io_profile: crate::device_io::Profile,
}
fn stamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_string()
}
fn save(app: &App) -> Result<()> {
    let _persist = app.persist.lock().unwrap();
    let s = app.state.lock().unwrap();
    let temp = app.data.join("catalog.tmp");
    let mut f = File::create(&temp)?;
    f.write_all(&serde_json::to_vec_pretty(&*s)?)?;
    f.sync_all()?;
    let dest = app.data.join("catalog.json");
    if dest.exists() {
        fs::copy(&dest, app.data.join("catalog.previous.json"))?;
    }
    fs::rename(&temp, &dest)?;
    Ok(())
}
fn log(app: &App, percent: u8, text: &str) {
    let mut s = app.state.lock().unwrap();
    s.job.percent = percent;
    s.job.message = text.into();
    s.job.logs.push(text.into());
    if s.job.logs.len() > 200 {
        s.job.logs.remove(0);
    }
}
fn ensure_idle(app: &App) -> Result<()> {
    ensure!(
        !app.state.lock().unwrap().job.running,
        "A task is running; try again later"
    );
    Ok(())
}
fn launch(
    app: Arc<App>,
    name: &str,
    work: impl FnOnce(&Arc<App>) -> Result<()> + Send + 'static,
) -> Result<()> {
    {
        let mut s = app.state.lock().unwrap();
        ensure!(!s.job.running, "Another task is already running");
        s.job = Job {
            running: true,
            message: name.into(),
            ..Default::default()
        };
    }
    std::thread::spawn(move || {
        let result = work(&app);
        let mut s = app.state.lock().unwrap();
        s.job.running = false;
        match result {
            Ok(()) => {
                s.job.percent = 100;
                s.job.message = "Completed".into();
            }
            Err(e) => {
                s.job.message = format!("Operation failed: {e:#}");
                let msg = s.job.message.clone();
                s.job.logs.push(msg);
            }
        }
        drop(s);
        let _ = save(&app);
    });
    Ok(())
}
fn import(app: &Arc<App>, paths: Vec<String>) -> Result<()> {
    let mut files = vec![];
    for input in paths {
        let p = PathBuf::from(input);
        if p.is_dir() {
            for entry in walkdir::WalkDir::new(&p).follow_links(false) {
                let e = entry?;
                if e.file_type().is_file()
                    && e.path()
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("mp3"))
                {
                    files.push(e.path().to_path_buf());
                }
            }
        } else {
            ensure!(
                p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp3")),
                "This version imports MP3 only; convert other formats first"
            );
            files.push(p);
        }
    }
    ensure!(!files.is_empty(), "No MP3 files found");
    let n = files.len();
    let mut failures = vec![];
    for (i, p) in files.iter().enumerate() {
        log(
            app,
            (i * 100 / n) as u8,
            &format!(
                "Reading {}",
                p.file_name().unwrap_or_default().to_string_lossy()
            ),
        );
        match metadata::read_mp3(p, &app.data.join("covers")) {
            Ok(t) => {
                let mut s = app.state.lock().unwrap();
                if !s.tracks.iter().any(|x| x.key == t.key) {
                    s.tracks.push(t);
                }
            }
            Err(e) => failures.push(format!("{}：{e:#}", p.display())),
        }
    }
    if !failures.is_empty() {
        bail!("Some files could not be imported: {}", failures.join("；"))
    }
    Ok(())
}
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    copy_tree_with_profile(src, dst, crate::device_io::Profile::Standard)
}
fn copy_tree_with_profile(
    src: &Path,
    dst: &Path,
    profile: crate::device_io::Profile,
) -> Result<()> {
    fs::create_dir_all(dst)?;
    for e in walkdir::WalkDir::new(src).follow_links(false).min_depth(1) {
        let e = e?;
        ensure!(
            !e.file_type().is_symlink(),
            "Device library contains a symbolic link; copy stopped"
        );
        let to = dst.join(e.path().strip_prefix(src)?);
        if e.file_type().is_dir() {
            fs::create_dir_all(&to)
                .with_context(|| format!("Cannot create directory: {}", to.display()))?;
        } else if e.file_type().is_file() {
            if profile == crate::device_io::Profile::Compatible {
                crate::device_io::copy_file(e.path(), &to)?;
                continue;
            }
            // Flush each completed file before verifying and before directory
            // commit. Include the path in errors from removable media.
            let mut source = fs::File::open(e.path())
                .with_context(|| format!("Cannot read file: {}", e.path().display()))?;
            let mut target = fs::File::create(&to)
                .with_context(|| format!("Cannot create file: {}", to.display()))?;
            std::io::copy(&mut source, &mut target)
                .with_context(|| format!("Cannot copy file: {}", to.display()))?;
            target
                .sync_all()
                .with_context(|| format!("Cannot flush device file: {}", to.display()))?;
            drop(target);
            ensure!(
                metadata::hash_file(e.path())? == metadata::hash_file(&to)?,
                "Copy verification failed: {}",
                to.display()
            );
        }
    }
    Ok(())
}
fn transfer(app: &Arc<App>, covers: bool) -> Result<()> {
    ensure!(
        app.allow_device_write,
        "Device writes are disabled pending independent format validation. Library, tags and artwork previews remain available."
    );
    let state = app.state.lock().unwrap().clone();
    ensure!(!state.queue.is_empty(), "Transfer queue is empty");
    let root = PathBuf::from(&state.device_root).canonicalize()?;
    protocol::validate_device(&root)?;
    protocol::validate_rebuild_supported(&root)?;
    if app.device_io_profile == crate::device_io::Profile::Compatible {
        log(
            app,
            0,
            "Compatible device I/O: 32 KiB chunks, 1 MiB/s cap and paced verification",
        );
    }
    transfer_staged(app, covers, &root, state, |source, destination| {
        copy_tree_with_profile(source, destination, app.device_io_profile)
    })
}

// The public operation enforces policy above; this private core accepts an I/O
// seam so tests can reproduce interrupted copying on a temporary filesystem.
fn transfer_staged(
    app: &Arc<App>,
    covers: bool,
    root: &Path,
    state: State,
    copy_to_device: impl FnOnce(&Path, &Path) -> Result<()>,
) -> Result<()> {
    transfer_with_capacity(app, covers, root, state, copy_to_device, || {
        fs2::available_space(root).map_err(Into::into)
    })
}
fn transfer_with_capacity(
    app: &Arc<App>,
    covers: bool,
    root: &Path,
    state: State,
    copy_to_device: impl FnOnce(&Path, &Path) -> Result<()>,
    available_space: impl FnOnce() -> Result<u64>,
) -> Result<()> {
    ensure_recovery_safe(&app.data, root)?;
    let _lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join(".island-walkman.lock"))?;
    fs2::FileExt::try_lock_exclusive(&_lock).context("Another transfer is using the device")?;
    let current = protocol::read_device(&root)?;
    let mut tracks = current.clone();
    let mut max = tracks.iter().filter_map(|t| t.device_id).max().unwrap_or(0);
    let selected: Vec<Track> = state
        .queue
        .iter()
        .map(|key| {
            state
                .tracks
                .iter()
                .find(|t| &t.key == key)
                .cloned()
                .context("A queued track was removed from the library")
        })
        .collect::<Result<_>>()?;
    // Snapshot current file hashes before preparing output. Reject external changes at commit.
    let device_dir = root.join("OMGAUDIO");
    let original = manifest(&device_dir)?;
    let run = app.data.join("backups").join(stamp());
    fs::create_dir_all(&run)?;
    if device_dir.exists() {
        log(app, 2, "Backing up and verifying the device library");
        copy_tree(&device_dir, &run.join("before/OMGAUDIO"))?;
    }
    app.state.lock().unwrap().job.backup = Some(run.to_string_lossy().into());
    let stage = run.join("stage/OMGAUDIO");
    fs::create_dir_all(&stage)?;
    // Stage a complete replacement on the host. Retain unknown device files unchanged.
    if device_dir.exists() {
        copy_tree(&device_dir, &stage)?;
    }
    for (i, mut t) in selected.into_iter().enumerate() {
        ensure!(
            metadata::hash_file(Path::new(&t.path))? == t.key,
            "Source changed after import; import it again: {}",
            t.title
        );
        // Display metadata is not audio identity. Never replace another recording implicitly.
        max = max
            .checked_add(1)
            .context("Device track IDs are exhausted")?;
        t.device_id = Some(max);
        let id = max;
        let dest = stage.join(protocol::oma_name(id));
        fs::create_dir_all(dest.parent().unwrap())?;
        log(
            app,
            10 + (i * 55 / state.queue.len()) as u8,
            &format!("Wrapping audio: {}", t.title),
        );
        // APIC/EA3 cover handling has not been validated against Sony output.
        let mut device_track = t.clone();
        device_track.cover = None;
        protocol::write_oma(&device_track, &dest)?;
        ensure!(
            metadata::hash_file(Path::new(&t.path))? == t.key,
            "Source changed during wrapping; commit stopped: {}",
            t.title
        );
        tracks.push(t);
    }
    log(app, 68, "Generating device database and indexes");
    // Remove known writer tables from the host stage so Windows FAT never retains lowercase LFN entries.
    for e in fs::read_dir(&stage)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().to_uppercase();
        if e.file_type()?.is_file()
            && [
                "01TREE01.DAT",
                "01TREE02.DAT",
                "01TREE03.DAT",
                "01TREE04.DAT",
                "01TREE22.DAT",
                "01TREE2D.DAT",
                "03GINF01.DAT",
                "03GINF02.DAT",
                "03GINF03.DAT",
                "03GINF04.DAT",
                "03GINF22.DAT",
                "03GINF2D.DAT",
                "00GTRLST.DAT",
                "02TREINF.DAT",
                "04CNTINF.DAT",
                "04PATLST.DAT",
                "05CIDLST.DAT",
            ]
            .contains(&name.as_str())
        {
            fs::remove_file(e.path())?;
        }
    }
    protocol::write_database(&stage, &tracks, covers)?;
    let staged_root = run.join("stage");
    let parsed = protocol::read_device(&staged_root)?;
    ensure!(
        parsed.len() == tracks.len(),
        "New database readback count mismatch"
    );
    let generated = manifest(&stage)?;
    let full_required = crate::space::required_bytes(&stage, &generated)?;
    let free = available_space()?;
    let reuse_mode = free < full_required;
    let plan = crate::space::plan(&stage, &original, &generated)?;
    let needed = if reuse_mode {
        plan.required_bytes
    } else {
        full_required
    };
    ensure!(
        free >= needed,
        "Not enough space to stage the new library safely"
    );
    ensure!(
        manifest(&device_dir)? == original,
        "Device changed during preparation; not committed"
    );
    let incoming = root.join(format!("ISLAND-NEW-{}", stamp()));
    let old = root.join(format!("ISLAND-BACKUP-{}", stamp()));
    // Retain recovery coordinates and expected hashes even when copying fails.
    fs::write(
        run.join("manifest.json"),
        serde_json::to_vec_pretty(&generated)?,
    )?;
    let journal = run.join("transaction.json");
    let transaction = |phase: &str| -> Result<()> {
        fs::write(
            &journal,
            serde_json::to_vec_pretty(&json!({
                "phase":phase,"original_library":device_dir,"incoming":incoming,
                "device_backup":old,"host_backup":run.join("before/OMGAUDIO"),
                "source_manifest":original,"expected_manifest":generated,
                "mode":if reuse_mode {"reuse_unchanged"}else{"full_directory"},
                "reused_paths":if reuse_mode {plan.reused.clone()}else{vec![]},
                "device_staging_bytes":needed
            }))?,
        )?;
        Ok(())
    };
    transaction("copying; original library unchanged")?;
    log(app, 76, "Copying and verifying the new device library");
    let delta = run.join("delta/OMGAUDIO");
    if reuse_mode {
        log(
            app,
            76,
            "Low-space staging; complete backup retained on computer",
        );
        fs::create_dir_all(&delta)?;
        for name in plan.changed.keys() {
            let target = delta.join(name);
            fs::create_dir_all(target.parent().unwrap())?;
            fs::copy(stage.join(name), target)?;
        }
        ensure!(
            manifest(&delta)? == plan.changed,
            "New device library verification failed"
        );
    }
    copy_to_device(if reuse_mode { &delta } else { &stage }, &incoming)?;
    ensure!(
        manifest(&incoming)?
            == if reuse_mode {
                plan.changed.clone()
            } else {
                generated.clone()
            },
        "New device library verification failed"
    );
    ensure!(
        manifest(&device_dir)? == original,
        "Original library changed during copying; not committed"
    );
    transaction("verified; ready to commit")?;
    let commit_result = if reuse_mode {
        crate::space::commit(
            &device_dir,
            &incoming,
            &old,
            &plan.reused,
            |from, to| fs::rename(from, to),
            |path| {
                ensure!(
                    manifest(path)? == generated,
                    "New device library verification failed"
                );
                Ok(())
            },
        )
    } else {
        commit_library(&device_dir, &incoming, &old, |from, to| {
            fs::rename(from, to)
        })
    };
    if let Err(error) = commit_result {
        let phase = if manifest(&device_dir).is_ok_and(|current| current == original) {
            "commit failed; original library available"
        } else if reuse_mode {
            "commit failed; device data may be split; host_backup is complete"
        } else if old.exists() {
            "commit and rollback failed; original library in device_backup"
        } else {
            "commit failed; inspect incoming and host_backup"
        };
        if let Err(journal_error) = transaction(phase) {
            return Err(error).context(format!("Recovery journal update failed: {journal_error}"));
        }
        return Err(error);
    }
    transaction("committed; old library retained")?;
    log(
        app,
        98,
        "Write complete; old library retained. Safely eject and verify playback.",
    );
    fs::write(
        run.join("manifest.json"),
        serde_json::to_vec_pretty(&generated)?,
    )?;
    let mut s = app.state.lock().unwrap();
    s.device_tracks = protocol::read_device(&root)?;
    s.queue.clear();
    Ok(())
}

fn ensure_recovery_safe(data: &Path, root: &Path) -> Result<()> {
    let library = root.join("OMGAUDIO");
    if !library.is_dir()
        && fs::read_dir(root)?.any(|entry| {
            entry.is_ok_and(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("ISLAND-BACKUP-")
            })
        })
    {
        bail!(
            "Interrupted transfer detected; restore the library before starting another transfer"
        );
    }
    let backups = data.join("backups");
    if !backups.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(backups)? {
        let path = entry?.path().join("transaction.json");
        if !path.is_file() {
            continue;
        }
        let journal: Value = serde_json::from_slice(&fs::read(&path)?).with_context(|| {
            format!("Cannot read transfer recovery journal: {}", path.display())
        })?;
        if journal["original_library"].as_str().map(Path::new) != Some(library.as_path()) {
            continue;
        }
        let phase = journal["phase"].as_str().unwrap_or("");
        if phase.starts_with("committed;") || phase.starts_with("copying;") {
            continue;
        }
        let expected: BTree = serde_json::from_value(journal["source_manifest"].clone())?;
        ensure!(
            manifest(&library)? == expected,
            "Unfinished commit requires recovery; complete host backup: {}",
            journal["host_backup"].as_str().unwrap_or("unknown")
        );
    }
    Ok(())
}

fn commit_library(
    original: &Path,
    incoming: &Path,
    backup: &Path,
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    if original.exists() {
        rename(original, backup)
            .context("Cannot retain the old library; new data remains staged")?;
    }
    if let Err(commit_error) = rename(incoming, original) {
        if backup.exists() {
            if let Err(rollback_error) = rename(backup, original) {
                bail!(
                    "Commit and rollback failed; original library retained at {}: commit: {}; rollback: {}",
                    backup.display(),
                    commit_error,
                    rollback_error
                );
            }
            return Err(commit_error).context("Commit failed; original library restored");
        }
        return Err(commit_error).context("Commit failed; new library remains staged");
    }
    Ok(())
}
fn prepare(app: &Arc<App>) -> Result<()> {
    let state = app.state.lock().unwrap().clone();
    let chosen: Vec<_> = state
        .queue
        .iter()
        .map(|key| {
            state
                .tracks
                .iter()
                .find(|t| &t.key == key)
                .cloned()
                .context("Queued track does not exist")
        })
        .collect::<Result<_>>()?;
    ensure!(!chosen.is_empty(), "Transfer queue is empty");
    let root = app.data.join("preparations").join(stamp());
    let dir = root.join("OMGAUDIO");
    fs::create_dir_all(&dir)?;
    let mut tracks = vec![];
    for (i, mut t) in chosen.into_iter().enumerate() {
        ensure!(
            metadata::hash_file(Path::new(&t.path))? == t.key,
            "Source file changed; import it again"
        );
        t.device_id = Some((i + 1).try_into().context("Track ID exceeds limits")?);
        let p = dir.join(protocol::oma_name(t.device_id.unwrap()));
        fs::create_dir_all(p.parent().unwrap())?;
        let mut audio = t.clone();
        audio.cover = None;
        protocol::write_oma(&audio, &p)?;
        ensure!(
            metadata::hash_file(Path::new(&t.path))? == t.key,
            "Source file changed during preparation"
        );
        tracks.push(t);
        log(
            app,
            ((i + 1) * 80 / state.queue.len()) as u8,
            "Generating local OMA files",
        );
    }
    protocol::write_database(&dir, &tracks, true)?;
    let read = protocol::read_device(&root)?;
    ensure!(
        read.len() == tracks.len(),
        "Generated library readback count mismatch"
    );
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest(&dir)?)?,
    )?;
    app.state.lock().unwrap().job.backup = Some(root.to_string_lossy().into());
    log(
        app,
        100,
        &format!(
            "Local preparation complete; player not accessed: {}",
            root.display()
        ),
    );
    Ok(())
}
fn prepare_cover_probe(app: &Arc<App>, source: &Path, hierarchy: bool) -> Result<()> {
    let track = app
        .state
        .lock()
        .unwrap()
        .tracks
        .first()
        .cloned()
        .context("Import a test MP3 with artwork first")?;
    let picture = track
        .cover
        .as_ref()
        .context("Test track has no usable artwork")?;
    let source = source.join("OMGAUDIO");
    ensure!(source.is_dir(), "Source has no OMGAUDIO directory");
    let root = app.data.join("cover-probes").join(stamp());
    let target = root.join("OMGAUDIO");
    copy_tree(&source, &target)?;
    for e in fs::read_dir(&target)? {
        let e = e?;
        let name = e.file_name().to_string_lossy().into_owned();
        if e.file_type()?.is_file()
            && name.to_lowercase().ends_with(".dat")
            && name != name.to_uppercase()
        {
            let tmp = target.join(format!("{name}.case-tmp"));
            fs::rename(e.path(), &tmp)?;
            fs::rename(tmp, target.join(name.to_uppercase()))?;
        }
    }
    let before = manifest(&target)?;
    let group = if hierarchy {
        crate::sony_artwork::patch_hierarchical_album_cover(
            &target,
            Path::new(picture),
            &track.album,
            0x2d,
        )?
    } else {
        crate::sony_artwork::patch_album_cover(
            &target,
            Path::new(picture),
            &track.album,
            &track.artist,
        )?
    };
    let after = manifest(&target)?;
    let changed: Vec<_> = before
        .iter()
        .filter(|(path, hash)| after.get(*path) != Some(*hash))
        .map(|(path, _)| path.clone())
        .collect();
    ensure!(
        changed
            == vec![if hierarchy {
                "03GINF2D.DAT"
            } else {
                "03GINF01.DAT"
            }],
        "Artwork probe changed files outside its album table: {changed:?}"
    );
    let new_files: Vec<_> = after
        .keys()
        .filter(|path| !before.contains_key(*path))
        .cloned()
        .collect();
    ensure!(new_files.len() == 3, "Unexpected artwork probe image count");
    fs::write(
        root.join("probe.json"),
        serde_json::to_vec_pretty(
            &json!({"source":source,"group":group,"changed":changed,"new_files":new_files,"before":before,"after":after,"device_written":false,"experiment":if hierarchy {"hierarchical-cover"}else{"album-cover"}}),
        )?,
    )?;
    println!(
        "Artwork probe prepared locally; one table and three images, no device writes: {}",
        root.display()
    );
    Ok(())
}
fn manifest(root: &Path) -> Result<BTree> {
    let mut m = BTree::new();
    if !root.exists() {
        return Ok(m);
    }
    for e in walkdir::WalkDir::new(root).follow_links(false).min_depth(1) {
        let e = e?;
        ensure!(
            !e.file_type().is_symlink(),
            "Library contains a symbolic link"
        );
        if e.file_type().is_file() {
            m.insert(
                e.path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/"),
                metadata::hash_file(e.path())?,
            );
        }
    }
    Ok(m)
}
type BTree = std::collections::BTreeMap<String, String>;
fn detect() -> Vec<String> {
    let mut roots = vec![];
    #[cfg(windows)]
    for c in b'D'..=b'Z' {
        let p = PathBuf::from(format!("{}:/", c as char));
        if protocol::validate_device(&p).is_ok() {
            roots.push(p.to_string_lossy().into())
        }
    }
    #[cfg(unix)]
    for base in ["/media", "/run/media", "/mnt", "/Volumes"] {
        for e in walkdir::WalkDir::new(base)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if e.file_type().is_dir() && protocol::validate_device(e.path()).is_ok() {
                roots.push(e.path().to_string_lossy().into())
            }
        }
    }
    roots
}
pub fn run() -> Result<()> {
    run_inner(None)
}

pub fn run_desktop(ui: PathBuf, ready: std::sync::mpsc::Sender<Result<String, String>>) {
    if let Err(error) = run_inner(Some((ui, ready.clone()))) {
        let _ = ready.send(Err(format!("{error:#}")));
    }
}

fn run_inner(
    desktop: Option<(PathBuf, std::sync::mpsc::Sender<Result<String, String>>)>,
) -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let arg = |key: &str| {
        args.iter()
            .position(|x| x == key)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data = arg("--data")
        .map(PathBuf::from)
        .unwrap_or_else(default_data_directory);
    fs::create_dir_all(&data)?;
    let mut state: State = fs::read(data.join("catalog.json"))
        .ok()
        .map(|b| serde_json::from_slice(&b))
        .transpose()?
        .unwrap_or_default();
    state.job.running = false;
    let ui = if let Some((ui, _)) = &desktop {
        ui.clone()
    } else if base.join("ui/index.html").exists() {
        base.join("ui")
    } else if base.join("../Resources/ui/index.html").exists() {
        base.join("../Resources/ui")
    } else {
        project.join("ui/dist")
    };
    let app = Arc::new(App {
        state: Mutex::new(state),
        persist: Mutex::new(()),
        requests: Mutex::new(()),
        data,
        ui,
        token: stamp(),
        allow_device_write: args.contains(&"--allow-experimental-device-write".into()),
        device_io_profile: crate::device_io::Profile::parse(arg("--device-io").as_deref())?,
    });
    if let Some(path) = arg("--import") {
        import(&app, vec![path])?;
        save(&app)?;
    }
    if let Some(source) = arg("--cover-probe") {
        prepare_cover_probe(&app, Path::new(&source), false)?;
        return Ok(());
    }
    if let Some(source) = arg("--cover-probe-tree") {
        prepare_cover_probe(&app, Path::new(&source), true)?;
        return Ok(());
    }
    if args.contains(&"--prepare".into()) {
        let mut s = app.state.lock().unwrap();
        s.queue = s.tracks.iter().map(|t| t.key.clone()).collect();
        drop(s);
        prepare(&app)?;
        println!("{}", app.state.lock().unwrap().job.message);
        save(&app)?;
        return Ok(());
    }
    let requested_port = arg("--port");
    let port = requested_port
        .as_deref()
        .unwrap_or("18718")
        .parse::<u16>()
        .context("--port must be a number between 0 and 65535")?;
    let listener = bind_loopback(
        if desktop.is_some() { 0 } else { port },
        requested_port.is_none(),
    )?;
    let server = Server::from_listener(listener, None).map_err(|e| anyhow::anyhow!("{e}"))?;
    let url = format!("http://{}", server.server_addr());
    println!("Island Walkman {url}");
    if let Some((_, ready)) = desktop {
        let _ = ready.send(Ok(url.clone()));
    } else if !args.contains(&"--no-open".into()) {
        open_browser(&url);
    }
    for req in server.incoming_requests() {
        let app = app.clone();
        std::thread::spawn(move || {
            handle(req, &app);
        });
    }
    Ok(())
}
fn open_browser(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
pub(crate) fn default_data_directory() -> PathBuf {
    #[cfg(windows)]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(path).join("IslandWalkman");
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = std::env::var_os("HOME") {
        return PathBuf::from(path).join("Library/Application Support/IslandWalkman");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
            if Path::new(&path).is_absolute() {
                return PathBuf::from(path).join("island-walkman");
            }
        }
        if let Some(path) = std::env::var_os("HOME") {
            return PathBuf::from(path).join(".local/share/island-walkman");
        }
    }
    PathBuf::from("island-walkman-data")
}
fn respond(req: Request, code: u16, data: Vec<u8>, mime: &str) {
    let response = Response::from_data(data)
        .with_status_code(StatusCode(code))
        .with_header(Header::from_bytes("Content-Type", mime).unwrap())
        .with_header(Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap())
        .with_header(Header::from_bytes("Cache-Control", "no-store").unwrap());
    let _ = req.respond(response);
}
fn handle(mut req: Request, app: &Arc<App>) {
    if req.method() == &tiny_http::Method::Get && req.url().starts_with("/api/audio/") {
        let valid = req
            .headers()
            .iter()
            .any(|h| h.field.equiv("Host") && h.value.as_str().starts_with("127.0.0.1:"));
        if !valid {
            respond(req, 400, b"Invalid Host".to_vec(), "text/plain");
            return;
        }
        let key = req
            .url()
            .split('?')
            .next()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap();
        let path = app
            .state
            .lock()
            .unwrap()
            .tracks
            .iter()
            .find(|t| t.key == key)
            .map(|t| t.path.clone());
        if let Some(path) = path {
            if let Ok(file) = File::open(path) {
                let _ = req.respond(
                    Response::from_file(file)
                        .with_header(Header::from_bytes("Content-Type", "audio/mpeg").unwrap()),
                );
                return;
            }
        }
        respond(req, 404, b"Audio not found".to_vec(), "text/plain");
        return;
    }
    let result = (|| -> Result<(Vec<u8>, String)> {
        let host = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("Host"))
            .map(|h| h.value.as_str())
            .unwrap_or("");
        ensure!(host.starts_with("127.0.0.1:"), "Invalid Host header");
        let url = req.url().split('?').next().unwrap_or("/").to_string();
        if req.method() == &tiny_http::Method::Get {
            if url == "/api/state" {
                let s = app.state.lock().unwrap().clone();
                return Ok((
                    serde_json::to_vec(
                        &json!({"state":s,"token":app.token,"device_write_enabled":app.allow_device_write,"device_io_profile":app.device_io_profile.name()}),
                    )?,
                    "application/json".into(),
                ));
            }
            if url == "/api/devices" {
                if std::env::args().any(|arg| arg == "--desktop-smoke") {
                    return Ok((b"[]".to_vec(), "application/json".into()));
                }
                let _request = app.requests.lock().unwrap();
                ensure_idle(app)?;
                return Ok((serde_json::to_vec(&detect())?, "application/json".into()));
            }
            if url.starts_with("/api/cover/") {
                let key = url.rsplit('/').next().unwrap();
                let s = app.state.lock().unwrap();
                let t = s
                    .tracks
                    .iter()
                    .chain(s.device_tracks.iter())
                    .find(|t| t.key == key)
                    .context("Track does not exist")?;
                ensure!(
                    t.device_id.is_none() || !s.job.running,
                    "A task is running; try again later"
                );
                return Ok((
                    fs::read(t.cover.as_ref().context("No artwork")?)?,
                    "image/jpeg".into(),
                ));
            }
            let path = app.ui.join(if url == "/" {
                "index.html"
            } else {
                url.trim_start_matches('/')
            });
            let canonical = path.canonicalize()?;
            ensure!(
                canonical.starts_with(app.ui.canonicalize()?),
                "Invalid resource path"
            );
            let mime = match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
                "js" => "text/javascript",
                "css" => "text/css",
                "woff2" => "font/woff2",
                "html" => "text/html; charset=utf-8",
                _ => "application/octet-stream",
            };
            return Ok((fs::read(canonical)?, mime.into()));
        }
        ensure!(
            req.method() == &tiny_http::Method::Post,
            "Only GET and POST are supported"
        );
        ensure!(
            req.headers()
                .iter()
                .any(|h| h.field.equiv("X-Island-Token") && h.value.as_str() == app.token),
            "Request validation failed; refresh the interface"
        );
        // Keep idle checks and their subsequent mutations atomic relative to launch.
        let _request = app.requests.lock().unwrap();
        ensure!(
            req.body_length().unwrap_or(0) < 2 * 1024 * 1024,
            "Request is too large"
        );
        let mut body = String::new();
        req.as_reader()
            .take(2 * 1024 * 1024)
            .read_to_string(&mut body)?;
        let v: Value = serde_json::from_str(&body)?;
        if url == "/api/desktop-smoke-report"
            && std::env::args().any(|arg| arg == "--desktop-smoke")
        {
            let args: Vec<_> = std::env::args().collect();
            if let Some(path) = args
                .iter()
                .position(|arg| arg == "--smoke-report")
                .and_then(|index| args.get(index + 1))
            {
                fs::write(path, serde_json::to_vec(&v)?)?;
            }
            return Ok((b"{}".to_vec(), "application/json".into()));
        }
        match url.as_str() {
            "/api/browse" => {
                ensure_idle(app)?;
                let home = std::env::var("USERPROFILE")
                    .or_else(|_| std::env::var("HOME"))
                    .unwrap_or("/".into());
                let input = v["path"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from(home).join("Music"));
                let path = input.canonicalize()?;
                let mut entries = vec![];
                for e in fs::read_dir(&path)? {
                    let e = e?;
                    let kind = e.file_type()?;
                    if kind.is_dir()
                        || e.path().extension().is_some_and(|e| {
                            ["mp3", "jpg", "jpeg", "png", "webp"]
                                .iter()
                                .any(|x| e.eq_ignore_ascii_case(x))
                        })
                    {
                        entries.push(json!({"name":e.file_name().to_string_lossy(),"path":e.path().to_string_lossy(),"directory":kind.is_dir()}));
                    }
                }
                entries.sort_by_key(|e| {
                    (
                        e["directory"] != true,
                        e["name"].as_str().unwrap_or("").to_lowercase(),
                    )
                });
                return Ok((
                    serde_json::to_vec(
                        &json!({"path":path.to_string_lossy(),"parent":path.parent().map(|p|p.to_string_lossy()),"entries":entries}),
                    )?,
                    "application/json".into(),
                ));
            }
            "/api/import" => {
                let paths: Vec<String> = serde_json::from_value(v["paths"].clone())?;
                launch(app.clone(), "Importing", move |a| import(a, paths))?;
            }
            "/api/connect" => {
                ensure_idle(app)?;
                let root = v["path"].as_str().context("Missing device path")?;
                protocol::validate_device(Path::new(root))?;
                let tracks = protocol::read_device(Path::new(root))?;
                let mut s = app.state.lock().unwrap();
                s.device_root = root.into();
                s.device_tracks = tracks;
            }
            "/api/queue" => {
                ensure_idle(app)?;
                let keys: Vec<String> = serde_json::from_value(v["keys"].clone())?;
                let mut s = app.state.lock().unwrap();
                for key in keys {
                    ensure!(
                        s.tracks.iter().any(|t| t.key == key),
                        "Track does not exist"
                    );
                    if !s.queue.contains(&key) {
                        s.queue.push(key);
                    }
                }
            }
            "/api/queue/remove" => {
                ensure_idle(app)?;
                let key = v["key"].as_str().context("Missing track key")?;
                app.state.lock().unwrap().queue.retain(|k| k != key);
            }
            "/api/edit" => {
                ensure_idle(app)?;
                let edited: Track = serde_json::from_value(v["track"].clone())?;
                let mut s = app.state.lock().unwrap();
                let t = s
                    .tracks
                    .iter_mut()
                    .find(|t| t.key == edited.key)
                    .context("Track does not exist")?;
                t.title = edited.title;
                t.artist = edited.artist;
                t.album = edited.album;
                t.album_artist = edited.album_artist;
                t.genre = edited.genre;
                t.year = edited.year;
                t.track = edited.track;
                t.track_total = edited.track_total;
                t.disc = edited.disc;
                t.disc_total = edited.disc_total;
            }
            "/api/cover" => {
                ensure_idle(app)?;
                let key = v["key"].as_str().context("Missing track key")?;
                let path = v["path"].as_str().context("Missing image path")?;
                let dest = metadata::save_cover(&fs::read(path)?, &app.data.join("covers"), key)?;
                let mut s = app.state.lock().unwrap();
                s.tracks
                    .iter_mut()
                    .find(|t| t.key == key)
                    .context("Track does not exist")?
                    .cover = Some(dest.to_string_lossy().into());
            }
            "/api/transfer" => {
                let covers = v["covers"].as_bool().unwrap_or(false);
                launch(app.clone(), "Preparing transfer", move |a| {
                    transfer(a, covers)
                })?;
            }
            "/api/prepare" => {
                launch(app.clone(), "Preparing local library", prepare)?;
            }
            _ => bail!("Unknown endpoint"),
        }
        save(app)?;
        Ok((
            serde_json::to_vec(&json!({"ok":true}))?,
            "application/json".into(),
        ))
    })();
    match result {
        Ok((body, mime)) => respond(req, 200, body, &mime),
        Err(e) => {
            if e.to_string() != "__responded" {
                respond(
                    req,
                    400,
                    serde_json::to_vec(&json!({"error":format!("{e:#}")})).unwrap(),
                    "application/json",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compatible_tree_copy_verifies_nested_files_and_preserves_source() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("incoming");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("nested/audio.oma"), vec![0x59; 33 * 1024]).unwrap();
        fs::write(source.join("index.dat"), b"small index").unwrap();
        let before = manifest(&source).unwrap();
        copy_tree_with_profile(&source, &target, crate::device_io::Profile::Compatible).unwrap();
        assert_eq!(manifest(&target).unwrap(), before);
        assert_eq!(manifest(&source).unwrap(), before);
    }

    #[test]
    fn busy_device_cover_requests_stop_before_opening_the_file() {
        let (temp, app, _, _) = virtual_player();
        {
            let mut state = app.state.lock().unwrap();
            let mut track = state.tracks[0].clone();
            track.key = "busy-device-cover".into();
            track.device_id = Some(99);
            track.cover = Some(
                temp.path()
                    .join("must-not-be-opened.jpg")
                    .to_string_lossy()
                    .into(),
            );
            state.device_tracks = vec![track];
            state.job.running = true;
        }
        let server = Server::http("127.0.0.1:0").unwrap();
        let address = server.server_addr().to_ip().unwrap();
        let worker = std::thread::spawn(move || handle(server.recv().unwrap(), &app));
        let mut client = std::net::TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        write!(client, "GET /api/cover/busy-device-cover HTTP/1.0\r\nHost: {address}\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        worker.join().unwrap();
        assert!(response.contains("400"));
        assert!(response.contains("A task is running; try again later"));
        assert!(!response.contains("cannot find"));
    }
    #[test]
    fn missing_current_library_with_a_device_backup_blocks_a_new_transfer() {
        let (_temp, app, root, _source) = virtual_player();
        fs::rename(
            root.join("OMGAUDIO"),
            root.join("ISLAND-BACKUP-interrupted"),
        )
        .unwrap();
        let state = app.state.lock().unwrap().clone();
        let error = transfer_staged(&app, true, &root, state, copy_tree).unwrap_err();
        assert!(error.to_string().contains("Interrupted transfer"));
        assert!(!root.join("OMGAUDIO").exists());
        assert!(!root.join(".island-walkman.lock").exists());
        assert!(root.join("ISLAND-BACKUP-interrupted/opaque.bin").is_file());
    }
    #[test]
    fn an_unfinished_commit_is_not_silently_retried_after_restart() {
        let (_temp, app, root, _source) = virtual_player();
        let before = manifest(&root.join("OMGAUDIO")).unwrap();
        let run = app.data.join("backups/interrupted");
        fs::create_dir_all(&run).unwrap();
        fs::write(
            run.join("transaction.json"),
            serde_json::to_vec(&json!({
                "phase":"verified; ready to commit","original_library":root.join("OMGAUDIO"),
                "source_manifest":before,"host_backup":run.join("before/OMGAUDIO")
            }))
            .unwrap(),
        )
        .unwrap();
        ensure_recovery_safe(&app.data, &root).unwrap();
        fs::write(
            root.join("OMGAUDIO/new-committed-file.bin"),
            b"commit happened but journal did not finish",
        )
        .unwrap();
        let error = ensure_recovery_safe(&app.data, &root).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Unfinished commit requires recovery")
        );
        assert!(root.join("OMGAUDIO/new-committed-file.bin").is_file());
    }
    #[test]
    fn failed_commit_restores_the_original_library() {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("OMGAUDIO");
        let incoming = temp.path().join("incoming");
        let backup = temp.path().join("backup");
        fs::create_dir(&original).unwrap();
        fs::create_dir(&incoming).unwrap();
        fs::write(original.join("track.oma"), b"original audio").unwrap();
        let mut operations = 0;
        let error = commit_library(&original, &incoming, &backup, |from, to| {
            operations += 1;
            if operations == 2 {
                return Err(std::io::Error::other("Injected commit I/O failure"));
            }
            fs::rename(from, to)
        })
        .unwrap_err();
        assert!(error.to_string().contains("original library restored"));
        assert_eq!(
            fs::read(original.join("track.oma")).unwrap(),
            b"original audio"
        );
        assert!(incoming.is_dir());
        assert!(!backup.exists());
    }

    #[test]
    fn failed_rollback_reports_the_actual_original_library_location() {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("OMGAUDIO");
        let incoming = temp.path().join("incoming");
        let backup = temp.path().join("backup");
        fs::create_dir(&original).unwrap();
        fs::create_dir(&incoming).unwrap();
        fs::write(original.join("track.oma"), b"original audio").unwrap();
        let mut operations = 0;
        let error = commit_library(&original, &incoming, &backup, |from, to| {
            operations += 1;
            if operations > 1 {
                return Err(std::io::Error::other("Injected persistent I/O failure"));
            }
            fs::rename(from, to)
        })
        .unwrap_err();
        assert!(error.to_string().contains("Commit and rollback failed"));
        assert!(error.to_string().contains(backup.to_str().unwrap()));
        assert_eq!(
            fs::read(backup.join("track.oma")).unwrap(),
            b"original audio"
        );
        assert!(!original.exists());
        assert!(incoming.is_dir());
    }
    fn virtual_player() -> (tempfile::TempDir, Arc<App>, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("player");
        fs::create_dir_all(root.join("OMGAUDIO")).unwrap();
        protocol::write_database(&root.join("OMGAUDIO"), &[], false).unwrap();
        fs::write(
            root.join("OMGAUDIO/opaque.bin"),
            b"preserve unknown device data",
        )
        .unwrap();
        let data = temp.path().join("app");
        fs::create_dir(&data).unwrap();
        let source = temp.path().join("generated.mp3");
        let frame = [vec![0xff, 0xfb, 0x90, 0], vec![0; 413]].concat();
        fs::write(&source, frame.repeat(100)).unwrap();
        let image = temp.path().join("artwork.jpg");
        image::RgbImage::from_pixel(60, 40, image::Rgb([220, 90, 30]))
            .save(&image)
            .unwrap();
        let app = Arc::new(App {
            state: Mutex::new(State::default()),
            persist: Mutex::new(()),
            requests: Mutex::new(()),
            data,
            ui: temp.path().into(),
            token: "test".into(),
            allow_device_write: true,
            device_io_profile: crate::device_io::Profile::Standard,
        });
        import(&app, vec![source.to_string_lossy().into_owned()]).unwrap();
        {
            let mut state = app.state.lock().unwrap();
            let track = &mut state.tracks[0];
            track.title = "試験曲名 🎵 ".repeat(15);
            track.artist = "艺术家".into();
            track.album = "テストアルバム".into();
            track.track = 9;
            track.track_total = 20;
            track.disc = 2;
            track.disc_total = 3;
            track.cover = Some(image.to_string_lossy().into_owned());
            state.queue = vec![track.key.clone()];
            state.device_root = root.to_string_lossy().into_owned();
        }
        (temp, app, root, source)
    }

    #[test]
    fn complete_staged_transfer_and_append_preserve_existing_audio_and_artwork() {
        let (_temp, app, root, source) = virtual_player();
        let source_hash = metadata::hash_file(&source).unwrap();
        for expected_count in 1..=2 {
            let snapshot = app.state.lock().unwrap().clone();
            let title = snapshot.tracks[0].title.clone();
            transfer_staged(&app, true, &root, snapshot, copy_tree).unwrap();
            assert_eq!(metadata::hash_file(&source).unwrap(), source_hash);
            assert_eq!(
                fs::read(root.join("OMGAUDIO/opaque.bin")).unwrap(),
                b"preserve unknown device data"
            );
            let tracks = protocol::read_device(&root).unwrap();
            assert_eq!(tracks.len(), expected_count);
            for track in tracks {
                assert_eq!(track.title, title);
                assert_eq!(
                    (track.track, track.track_total, track.disc, track.disc_total),
                    (9, 20, 2, 3)
                );
                assert!(Path::new(track.cover.as_ref().unwrap()).is_file());
            }
            let state = app.state.lock().unwrap();
            let run = Path::new(state.job.backup.as_ref().unwrap());
            let journal: Value =
                serde_json::from_slice(&fs::read(run.join("transaction.json")).unwrap()).unwrap();
            assert_eq!(journal["phase"], "committed; old library retained");
            let backup = Path::new(journal["device_backup"].as_str().unwrap());
            assert!(backup.join("opaque.bin").is_file());
            if expected_count == 2 {
                let old = backup.join(protocol::oma_name(1));
                assert_eq!(
                    metadata::hash_file(&old).unwrap(),
                    metadata::hash_file(&root.join("OMGAUDIO").join(protocol::oma_name(1)))
                        .unwrap()
                );
            }
            assert!(state.queue.is_empty());
            let key = state.tracks[0].key.clone();
            drop(state);
            app.state.lock().unwrap().queue = vec![key];
        }
    }

    #[test]
    fn near_full_player_appends_without_copying_unchanged_files_twice() {
        let (_temp, app, root, source) = virtual_player();
        let large = root.join("OMGAUDIO/large-unchanged.bin");
        fs::File::create(&large)
            .unwrap()
            .set_len(8 * 1024 * 1024)
            .unwrap();
        let original_hash = metadata::hash_file(&large).unwrap();
        let source_hash = metadata::hash_file(&source).unwrap();
        let snapshot = app.state.lock().unwrap().clone();
        transfer_with_capacity(
            &app,
            true,
            &root,
            snapshot,
            |delta, incoming| {
                assert!(!delta.join("large-unchanged.bin").exists());
                copy_tree(delta, incoming)
            },
            || Ok(4 * 1024 * 1024),
        )
        .unwrap();
        assert_eq!(metadata::hash_file(&large).unwrap(), original_hash);
        assert_eq!(metadata::hash_file(&source).unwrap(), source_hash);
        let tracks = protocol::read_device(&root).unwrap();
        assert_eq!(tracks.len(), 1);
        assert!(tracks[0].cover.is_some());
        let state = app.state.lock().unwrap();
        let run = Path::new(state.job.backup.as_ref().unwrap());
        assert_eq!(
            metadata::hash_file(&run.join("before/OMGAUDIO/large-unchanged.bin")).unwrap(),
            original_hash
        );
        let journal: Value =
            serde_json::from_slice(&fs::read(run.join("transaction.json")).unwrap()).unwrap();
        assert_eq!(journal["mode"], "reuse_unchanged");
        assert!(journal["device_staging_bytes"].as_u64().unwrap() < 4 * 1024 * 1024);
        assert!(
            journal["reused_paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|path| path == "large-unchanged.bin")
        );
        assert!(
            !Path::new(journal["device_backup"].as_str().unwrap())
                .join("large-unchanged.bin")
                .exists()
        );
        assert!(state.queue.is_empty());
    }

    #[test]
    fn long_artist_and_album_tags_keep_artwork_after_device_readback() {
        let (_temp, app, root, _source) = virtual_player();
        let artist = "艺术家 🎸 ".repeat(15);
        let album = "長いアルバム名 🎵 ".repeat(15);
        {
            let mut state = app.state.lock().unwrap();
            state.tracks[0].artist = artist.clone();
            state.tracks[0].album = album.clone();
        }
        let snapshot = app.state.lock().unwrap().clone();
        transfer_staged(&app, true, &root, snapshot, copy_tree).unwrap();
        let track = protocol::read_device(&root).unwrap().remove(0);
        assert_eq!(track.artist, artist);
        assert_eq!(track.album, album);
        assert!(
            track.cover.is_some(),
            "Artwork association must not depend on truncated index labels"
        );
    }

    #[test]
    fn interrupted_device_copy_preserves_original_and_records_recovery_paths() {
        let (_temp, app, root, _source) = virtual_player();
        let before = manifest(&root.join("OMGAUDIO")).unwrap();
        let snapshot = app.state.lock().unwrap().clone();
        let error = transfer_staged(&app, true, &root, snapshot, |_, incoming| {
            fs::create_dir(incoming)?;
            fs::write(incoming.join("partial.tmp"), b"incomplete")?;
            bail!("Injected device I/O interruption")
        })
        .unwrap_err();
        assert!(error.to_string().contains("Injected device I/O"));
        assert_eq!(manifest(&root.join("OMGAUDIO")).unwrap(), before);
        let state = app.state.lock().unwrap();
        assert_eq!(state.queue.len(), 1);
        let run = Path::new(state.job.backup.as_ref().unwrap());
        assert_eq!(manifest(&run.join("before/OMGAUDIO")).unwrap(), before);
        let journal: Value =
            serde_json::from_slice(&fs::read(run.join("transaction.json")).unwrap()).unwrap();
        assert_eq!(journal["phase"], "copying; original library unchanged");
        assert!(
            Path::new(journal["incoming"].as_str().unwrap())
                .join("partial.tmp")
                .is_file()
        );
        assert!(!Path::new(journal["device_backup"].as_str().unwrap()).exists());
    }

    #[test]
    fn external_device_changes_during_copy_prevent_commit() {
        let (_temp, app, root, _source) = virtual_player();
        let snapshot = app.state.lock().unwrap().clone();
        let error = transfer_staged(&app, true, &root, snapshot, |source, incoming| {
            copy_tree(source, incoming)?;
            fs::write(
                root.join("OMGAUDIO/external-change.bin"),
                b"another application",
            )?;
            Ok(())
        })
        .unwrap_err();
        assert!(error.to_string().contains("Original library changed"));
        assert!(root.join("OMGAUDIO/external-change.bin").is_file());
        assert!(protocol::read_device(&root).unwrap().is_empty());
        assert_eq!(app.state.lock().unwrap().queue.len(), 1);
    }

    #[test]
    fn device_write_gate_is_checked_before_any_device_access() {
        let d = tempfile::tempdir().unwrap();
        let app = Arc::new(App {
            state: Mutex::new(State::default()),
            persist: Mutex::new(()),
            requests: Mutex::new(()),
            data: d.path().into(),
            ui: d.path().into(),
            token: "test".into(),
            allow_device_write: false,
            device_io_profile: crate::device_io::Profile::Compatible,
        });
        let e = transfer(&app, true).unwrap_err();
        assert!(e.to_string().contains("Device writes are disabled"));
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 0);
    }
}

// Explicit ports remain strict; only an occupied default gets a free port.
fn bind_loopback(port: u16, fallback: bool) -> std::io::Result<std::net::TcpListener> {
    match std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)) {
        Err(error) if fallback && error.kind() == std::io::ErrorKind::AddrInUse => {
            std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        }
        result => result,
    }
}

#[cfg(test)]
mod startup_tests {
    use super::bind_loopback;
    #[test]
    fn occupied_default_port_falls_back_to_loopback() {
        let occupied = bind_loopback(0, false).unwrap();
        let address = occupied.local_addr().unwrap();
        let alternative = bind_loopback(address.port(), true).unwrap();
        assert!(alternative.local_addr().unwrap().ip().is_loopback());
        assert_ne!(alternative.local_addr().unwrap().port(), address.port());
        assert!(bind_loopback(address.port(), false).is_err());
    }
}
