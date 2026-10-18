//! Durable append transactions. Player paths are short, owned and independently
//! verified; journals and complete backups live on the computer.
use crate::{
    metadata::{self, Track},
    protocol,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub type Manifest = BTreeMap<String, String>;
const MARKER: &str = "IWIDENT.DAT";
const MUTABLE: &[&str] = &[
    "00GTRLST.DAT",
    "02TREINF.DAT",
    "01TREE0C.DAT",
    "03GINF0C.DAT",
    "01TREE0E.DAT",
    "03GINF0E.DAT",
    "01TREE0F.DAT",
];
const MANAGED: &[&str] = &[
    "00GTRLST.DAT",
    "01TREE01.DAT",
    "01TREE02.DAT",
    "01TREE03.DAT",
    "01TREE04.DAT",
    "01TREE22.DAT",
    "01TREE2D.DAT",
    "02TREINF.DAT",
    "03GINF01.DAT",
    "03GINF02.DAT",
    "03GINF03.DAT",
    "03GINF04.DAT",
    "03GINF22.DAT",
    "03GINF2D.DAT",
    "04CNTINF.DAT",
    "05CIDLST.DAT",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Preparing,
    Copying,
    Ready,
    Committing,
    Committed,
    Finalized,
    Abandoned,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Active,
    WaitingDevice,
    UserPaused,
    NeedsReview,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Step {
    AddAudio,
    RetainRoot,
    MoveAudio,
    Install,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    id: u16,
    title: String,
    artist: String,
    album: String,
    album_artist: String,
    genre: String,
    year: i32,
    track: u32,
    disc: u32,
    duration: u32,
    properties: [u8; 4],
    audio: String,
    artwork: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    version: u32,
    pub id: String,
    generation: u64,
    pub root_hint: PathBuf,
    token: String,
    volume: Option<u32>,
    marker_new: bool,
    pub phase: Phase,
    pub status: Status,
    step: Step,
    stage_name: String,
    backup_name: String,
    quarantine_name: String,
    temp_name: String,
    pub selected: Vec<Track>,
    input_covers: BTreeMap<String, String>,
    covers: bool,
    original: Manifest,
    generated: Manifest,
    known: BTreeMap<String, BTreeSet<String>>,
    identities: Vec<Identity>,
    target_identities: Vec<Identity>,
    audio_dirs: Vec<String>,
    pub files_done: usize,
    pub files_total: usize,
    pub percent: u8,
    pub error: String,
    copy_intent: Option<String>,
    authorized: bool,
}

impl Session {
    pub fn pending(&self) -> bool {
        !matches!(self.phase, Phase::Finalized | Phase::Abandoned)
    }
    pub fn auto_resume(&self) -> bool {
        self.pending() && matches!(self.status, Status::Active | Status::WaitingDevice)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1 && self.authorized,
            "Unsupported or unauthorized transfer journal"
        );
        ensure!(
            self.id.len() == 32 && self.id.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid session ID"
        );
        ensure!(
            self.token.len() == 64 && self.token.bytes().all(|b| b.is_ascii_hexdigit()),
            "Invalid device identity"
        );
        for (name, prefix) in [
            (&self.stage_name, "IW"),
            (&self.backup_name, "IB"),
            (&self.quarantine_name, "IQ"),
        ] {
            ensure!(
                name.len() == 8
                    && name.starts_with(prefix)
                    && name[2..].bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid transaction directory"
            );
        }
        ensure!(
            self.temp_name == format!("IT{}.TMP", &self.stage_name[2..]),
            "Invalid temporary filename"
        );
        for name in self
            .original
            .keys()
            .chain(self.generated.keys())
            .chain(self.known.keys())
        {
            checked_relative(name)?;
        }
        for name in &self.audio_dirs {
            ensure!(audio_dir(name), "Invalid audio directory");
        }
        ensure!(
            self.selected
                .iter()
                .all(|t| t.key.len() == 64 && t.key.bytes().all(|b| b.is_ascii_hexdigit())),
            "Invalid source identity"
        );
        Ok(())
    }
}

pub struct Update {
    pub phase: String,
    pub percent: u8,
    pub done: usize,
    pub total: usize,
    pub message: String,
    pub commit_in_progress: bool,
    pub root: PathBuf,
}
pub enum Outcome {
    Complete(Session),
    Paused(Session),
}

/// Device operations have a narrow seam for deterministic unplug/crash tests.
pub trait DeviceOps {
    fn copy(&mut self, source: &Path, target: &Path) -> Result<()>;
    fn rename(&mut self, source: &Path, target: &Path) -> Result<()> {
        fs::rename(source, target).map_err(Into::into)
    }
    fn wait(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
    fn checkpoint(&mut self, _name: &str) {}
}
pub struct Native;
impl DeviceOps for Native {
    fn copy(&mut self, source: &Path, target: &Path) -> Result<()> {
        native_copy(source, target)
    }
}

fn checked_relative(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && !name.contains('\\')
            && !name.contains(':')
            && Path::new(name)
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "Unsafe path in transfer journal"
    );
    Ok(())
}
fn audio_dir(name: &str) -> bool {
    name.len() == 5 && name.starts_with("10F") && name[3..].bytes().all(|b| b.is_ascii_hexdigit())
}
fn in_audio_directory(name: &str) -> bool {
    name.split_once('/').is_some_and(|(dir, _)| audio_dir(dir))
}
fn is_audio(name: &str) -> bool {
    name.split_once('/')
        .is_some_and(|(dir, file)| audio_dir(dir) && file.ends_with(".OMA") && !file.contains('/'))
}
pub fn manifest(root: &Path) -> Result<Manifest> {
    let mut map = Manifest::new();
    if !root.exists() {
        return Ok(map);
    }
    ensure!(
        !root.symlink_metadata()?.file_type().is_symlink(),
        "Symbolic links are not allowed in a device library"
    );
    for entry in walkdir::WalkDir::new(root).follow_links(false).min_depth(1) {
        let entry = entry?;
        ensure!(
            !entry.file_type().is_symlink(),
            "Symbolic links are not allowed in a device library"
        );
        if entry.file_type().is_file() {
            let name = entry
                .path()
                .strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/");
            checked_relative(&name)?;
            map.insert(name, metadata::hash_file(entry.path())?);
        }
    }
    Ok(map)
}
fn device_tracks(root: &Path) -> Result<Vec<Track>> {
    let library = root.join("OMGAUDIO");
    if !library.exists() || fs::read_dir(&library)?.next().is_none() {
        ensure!(
            !fs::read_dir(root)?.filter_map(|e| e.ok()).any(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                name.starts_with("ISLAND-BACKUP-")
                    || (name.len() == 8
                        && name.starts_with("IB")
                        && name[2..].bytes().all(|b| b.is_ascii_hexdigit()))
            }),
            "A retained device library exists; recover it before initializing an empty library"
        );
        return Ok(vec![]);
    }
    protocol::read_device(root)
}
fn identities(root: &Path) -> Result<Vec<Identity>> {
    device_tracks(root)?
        .iter()
        .map(|t| {
            Ok(Identity {
                id: t.device_id.context("Missing device track ID")?,
                title: t.title.clone(),
                artist: t.artist.clone(),
                album: t.album.clone(),
                album_artist: t.album_artist.clone(),
                genre: t.genre.clone(),
                year: t.year,
                track: t.track,
                disc: t.disc,
                duration: t.duration_ms,
                properties: t.properties,
                audio: metadata::hash_file(Path::new(&t.path))?,
                artwork: t
                    .cover
                    .as_ref()
                    .map(|p| metadata::hash_file(Path::new(p)))
                    .transpose()?,
            })
        })
        .collect()
}
fn uid() -> String {
    format!(
        "{:032x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
fn directory(data: &Path, id: &str) -> Result<PathBuf> {
    ensure!(
        id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid session ID"
    );
    let path = data.join("transfers").join(id);
    if path.exists() {
        ensure!(
            !path.symlink_metadata()?.file_type().is_symlink(),
            "Unsafe transfer directory"
        );
    }
    Ok(path)
}
fn envelope(session: &Session) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(session)?;
    Ok(serde_json::to_vec_pretty(
        &serde_json::json!({"sha256":format!("{:x}",Sha256::digest(&payload)),"payload":String::from_utf8(payload)?}),
    )?)
}
fn save(data: &Path, s: &mut Session) -> Result<()> {
    s.validate()?;
    s.generation += 1;
    let dir = directory(data, &s.id)?;
    fs::create_dir_all(&dir)?;
    // Keep two independently synced generations. A torn newer slot never erases
    // the previous intent; operations are inspected rather than blindly replayed.
    let path = dir.join(format!("journal-{}.json", s.generation % 2));
    let mut file = File::create(path)?;
    file.write_all(&envelope(s)?)?;
    file.sync_all()?;
    Ok(())
}
pub fn load(data: &Path, id: &str) -> Result<Session> {
    let dir = directory(data, id)?;
    let mut valid = vec![];
    for slot in 0..2 {
        let result = (|| -> Result<Session> {
            let value: serde_json::Value =
                serde_json::from_slice(&fs::read(dir.join(format!("journal-{slot}.json")))?)?;
            let payload = value["payload"]
                .as_str()
                .context("Missing journal payload")?;
            ensure!(
                value["sha256"].as_str()
                    == Some(&format!("{:x}", Sha256::digest(payload.as_bytes()))),
                "Journal checksum mismatch"
            );
            let session: Session = serde_json::from_str(payload)?;
            session.validate()?;
            ensure!(session.id == id, "Journal directory mismatch");
            Ok(session)
        })();
        if let Ok(session) = result {
            valid.push(session);
        }
    }
    valid
        .into_iter()
        .max_by_key(|s| s.generation)
        .context("Transfer journal is unreadable; backups have been preserved")
}
pub fn pending(data: &Path) -> Result<Option<Session>> {
    let dir = data.join("transfers");
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut result = None;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let session = load(data, &id)?;
        if session.pending() {
            ensure!(
                result.is_none(),
                "Multiple pending transfers require review"
            );
            result = Some(session);
        }
    }
    Ok(result)
}
pub fn acknowledge(data: &Path, id: &str) -> Result<()> {
    let mut s = load(data, id)?;
    ensure!(s.phase == Phase::Committed, "Transfer is not committed");
    s.phase = Phase::Finalized;
    save(data, &mut s)
}
pub fn discard(data: &Path, id: &str) -> Result<()> {
    let mut s = load(data, id)?;
    ensure!(
        matches!(s.phase, Phase::Preparing | Phase::Copying | Phase::Ready),
        "Finish device recovery before discarding this transfer"
    );
    if s.phase == Phase::Ready {
        ensure!(
            s.root_hint.is_dir() && current_original(&s, &s.root_hint)? == s.original,
            "Reconnect the expected device and finish recovery before discarding this transfer"
        );
    }
    s.phase = Phase::Abandoned;
    save(data, &mut s)
}

pub fn create(data: &Path, root: &Path, selected: Vec<Track>, covers: bool) -> Result<Session> {
    fs::create_dir_all(data)?;
    let registry = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(data.join("transfers.lock"))?;
    fs2::FileExt::try_lock_exclusive(&registry)
        .context("Another process is creating a transfer")?;
    ensure!(
        pending(data)?.is_none(),
        "Resume the existing transfer before starting another"
    );
    ensure!(!selected.is_empty(), "Transfer queue is empty");
    ensure!(
        selected
            .iter()
            .map(|t| &t.key)
            .collect::<BTreeSet<_>>()
            .len()
            == selected.len(),
        "Transfer queue contains duplicate source identities"
    );
    let original = manifest(&root.join("OMGAUDIO"))?;
    ensure!(
        original.keys().all(|name| {
            let upper = name.to_ascii_uppercase();
            (!is_audio(&upper) && !MANAGED.contains(&upper.as_str())) || name == &upper
        }),
        "Noncanonical Sony audio or database filenames require review before append"
    );
    ensure!(
        !original
            .keys()
            .any(|name| name.eq_ignore_ascii_case("SESSION.DAT")),
        "Reserved staging filename exists in the library; no write attempted"
    );
    ensure!(
        original
            .keys()
            .map(|name| name.to_uppercase())
            .collect::<BTreeSet<_>>()
            .len()
            == original.len(),
        "Case-colliding device paths require review"
    );
    let identities = identities(root)?;
    let registered: BTreeSet<_> = identities
        .iter()
        .map(|t| {
            protocol::oma_name(t.id)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    ensure!(
        original
            .keys()
            .filter(|name| is_audio(name))
            .all(|name| registered.contains(name)),
        "Unindexed device audio needs review before adding songs"
    );
    let id = uid();
    let marker = root.join(MARKER);
    let marker_new = !marker.exists();
    let token = if marker_new {
        format!(
            "{:x}",
            Sha256::digest(format!("{id}-{}", root.display()).as_bytes())
        )
    } else {
        ensure!(
            fs::metadata(&marker)?.len() == 64,
            "Device identity marker needs review"
        );
        fs::read_to_string(marker)?
    };
    ensure!(
        token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "Device identity marker needs review"
    );
    let mut names = None;
    for value in 1..=0xffffffu32 {
        let stage = format!("IW{value:06X}");
        let backup = format!("IB{value:06X}");
        let quarantine = format!("IQ{value:06X}");
        let temp = format!("IT{value:06X}.TMP");
        if [
            root.join(&stage),
            root.join(&backup),
            root.join(&quarantine),
            root.join("OMGAUDIO/10F00").join(&temp),
        ]
        .iter()
        .all(|p| !p.exists())
        {
            names = Some((stage, backup, quarantine, temp));
            break;
        }
    }
    let (stage_name, backup_name, quarantine_name, temp_name) =
        names.context("No free short transaction names")?;
    let input_covers = selected
        .iter()
        .filter_map(|t| t.cover.as_ref().map(|p| (t.key.clone(), p)))
        .map(|(key, path)| Ok((key, metadata::hash_file(Path::new(path))?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut s = Session {
        version: 1,
        id,
        generation: 0,
        root_hint: root.into(),
        token,
        volume: volume_serial(root)?,
        marker_new,
        phase: Phase::Preparing,
        status: Status::Active,
        step: Step::AddAudio,
        stage_name,
        backup_name,
        quarantine_name,
        temp_name,
        selected,
        input_covers,
        covers,
        original,
        generated: Manifest::new(),
        known: BTreeMap::new(),
        identities,
        target_identities: vec![],
        audio_dirs: vec![],
        files_done: 0,
        files_total: 0,
        percent: 0,
        error: String::new(),
        copy_intent: None,
        authorized: true,
    };
    save(data, &mut s)?;
    Ok(s)
}

fn host_copy(source: &Path, target: &Path, expected: &str) -> Result<()> {
    if target.is_file() && metadata::hash_file(target)? == expected {
        return Ok(());
    }
    fs::create_dir_all(target.parent().context("Missing parent directory")?)?;
    ensure!(
        !target.exists() || !target.symlink_metadata()?.file_type().is_symlink(),
        "Unsafe host backup path"
    );
    if target.is_file() {
        writable_host_file(target)?;
    }
    fs::copy(source, target)?;
    writable_host_file(target)?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(target)?
        .sync_all()?;
    ensure!(
        metadata::hash_file(target)? == expected,
        "Host backup verification failed"
    );
    Ok(())
}
fn writable_host_file(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[cfg(windows)]
    permissions.set_readonly(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(permissions.mode() | 0o200);
    }
    fs::set_permissions(path, permissions)?;
    Ok(())
}
fn fingerprint_matches(s: &Session, root: &Path) -> Result<bool> {
    Ok(identities(root)? == s.identities)
}
fn current_original(s: &Session, root: &Path) -> Result<Manifest> {
    let mut actual = manifest(&root.join("OMGAUDIO"))?;
    // The only accepted-library temporary path belongs to this saved session.
    if s.copy_intent.as_ref().is_some_and(|name| is_audio(name)) {
        actual.remove(&format!("10F00/{}", s.temp_name));
    }
    Ok(actual)
}
fn normalization(s: &Session, actual: &Manifest, root: &Path) -> Result<bool> {
    let changed: BTreeSet<_> = s
        .original
        .keys()
        .chain(actual.keys())
        .filter(|n| s.original.get(*n) != actual.get(*n))
        .collect();
    Ok(changed.iter().all(|n| MUTABLE.contains(&n.as_str())) && fingerprint_matches(s, root)?)
}
#[cfg(test)]
fn prepare(data: &Path, s: &mut Session, root: &Path) -> Result<()> {
    prepare_with(data, s, root, &mut || {})
}
fn prepare_with(
    data: &Path,
    s: &mut Session,
    root: &Path,
    checkpoint: &mut dyn FnMut(),
) -> Result<()> {
    for t in &s.selected {
        ensure!(
            metadata::hash_file(Path::new(&t.path))? == t.key,
            "Source changed; restore it or discard this transfer: {}",
            t.title
        );
    }
    let dir = directory(data, &s.id)?;
    for track in &mut s.selected {
        if let Some(hash) = s.input_covers.get(&track.key) {
            let snapshot = dir.join("input-covers").join(format!("{}.jpg", track.key));
            let source = Path::new(
                track
                    .cover
                    .as_deref()
                    .context("Saved artwork source is missing")?,
            );
            host_copy(source, &snapshot, hash)?;
            track.cover = Some(snapshot.to_string_lossy().into());
        }
    }
    let current = current_original(s, root)?;
    if current != s.original {
        ensure!(
            normalization(s, &current, root)?,
            "Existing device library changed; no write attempted"
        );
        for name in s.original.keys().filter(|n| !current.contains_key(*n)) {
            for base in [dir.join("before/OMGAUDIO"), dir.join("stage/OMGAUDIO")] {
                let path = base.join(name);
                if path.is_file() {
                    fs::remove_file(path)?;
                }
            }
        }
        s.original = current;
    }
    let before = dir.join("before/OMGAUDIO");
    for (name, hash) in &s.original {
        host_copy(&root.join("OMGAUDIO").join(name), &before.join(name), hash)?;
    }
    ensure!(
        manifest(&before)? == s.original,
        "Complete host backup verification failed"
    );
    let stage = dir.join("stage/OMGAUDIO");
    fs::create_dir_all(&stage)?;
    for (name, hash) in &s.original {
        host_copy(&before.join(name), &stage.join(name), hash)?;
    }
    let mut tracks = device_tracks(root)?;
    let mut maximum = tracks.iter().filter_map(|t| t.device_id).max().unwrap_or(0);
    for original in &s.selected {
        maximum = maximum
            .checked_add(1)
            .context("Device track IDs are exhausted")?;
        let mut t = original.clone();
        t.device_id = Some(maximum);
        let path = stage.join(protocol::oma_name(maximum));
        fs::create_dir_all(path.parent().unwrap())?;
        let mut wrapped = t.clone();
        wrapped.cover = None;
        protocol::write_oma(&wrapped, &path)?;
        ensure!(
            metadata::hash_file(Path::new(&t.path))? == t.key,
            "Source changed during preparation"
        );
        tracks.push(t);
    }
    for name in MANAGED {
        let p = stage.join(name);
        if p.exists() {
            fs::remove_file(p)?;
        }
    }
    protocol::write_database(&stage, &tracks, s.covers)?;
    let generated = manifest(&stage)?;
    ensure!(
        protocol::read_device(&dir.join("stage"))?.len() == tracks.len(),
        "Prepared database readback failed"
    );
    s.target_identities = identities(&dir.join("stage"))?;
    for (name, hash) in &generated {
        s.known
            .entry(name.clone())
            .or_default()
            .insert(hash.clone());
    }
    s.generated = generated;
    checkpoint();
    let latest = current_original(s, root)?;
    if latest != s.original {
        ensure!(
            normalization(s, &latest, root)?,
            "Device changed during preparation; progress retained"
        );
        s.phase = Phase::Preparing;
        save(data, s)?;
        return Ok(());
    }
    s.phase = Phase::Copying;
    s.percent = 20;
    save(data, s)
}

pub fn connected(session: &Session, roots: &[PathBuf]) -> Result<bool> {
    Ok(resolve(session, roots)?.is_some())
}
fn resolve(s: &Session, roots: &[PathBuf]) -> Result<Option<PathBuf>> {
    resolve_mode(s, roots, false)
}
fn resolve_mode(
    s: &Session,
    roots: &[PathBuf],
    initial_confirmed: bool,
) -> Result<Option<PathBuf>> {
    for root in std::iter::once(&s.root_hint).chain(roots) {
        if !root.is_dir() {
            continue;
        }
        if volume_serial(root)? != s.volume {
            continue;
        }
        let marker = root.join(MARKER);
        let contents = if marker.is_file() && fs::metadata(&marker)?.len() <= 64 {
            Some(fs::read_to_string(&marker)?)
        } else {
            None
        };
        if contents.as_deref() == Some(s.token.as_str()) {
            return Ok(Some(root.clone()));
        }
        if s.marker_new
            && matches!(s.phase, Phase::Preparing | Phase::Copying)
            && s.files_done == 0
            && s.copy_intent.is_none()
            && (!marker.exists()
                || contents
                    .as_ref()
                    .is_some_and(|text| text.len() < s.token.len() && s.token.starts_with(text)))
            && root == &s.root_hint
            && fingerprint_matches(s, root)?
        {
            let actual = current_original(s, root)?;
            if actual == s.original || normalization(s, &actual, root)? {
                ensure!(
                    initial_confirmed || s.volume.is_some() || !s.identities.is_empty(),
                    "Confirm the expected player before creating its identity marker"
                );
                return Ok(Some(root.clone()));
            }
        }
    }
    Ok(None)
}
fn ensure_marker(data: &Path, s: &mut Session, root: &Path) -> Result<()> {
    let path = root.join(MARKER);
    if path.exists() {
        ensure!(
            fs::metadata(&path)?.len() <= 64,
            "Device identity marker needs review"
        );
        let contents = fs::read_to_string(&path)?;
        if s.marker_new
            && contents.len() < s.token.len()
            && s.token.starts_with(&contents)
            && fingerprint_matches(s, root)?
        {
            let actual = current_original(s, root)?;
            ensure!(
                actual == s.original || normalization(s, &actual, root)?,
                "Existing library changed while creating its identity"
            );
            let mut file = OpenOptions::new().write(true).open(&path)?;
            file.write_all(s.token.as_bytes())?;
            file.sync_all()?;
        }
        ensure!(
            fs::read_to_string(path)? == s.token,
            "Device identity changed"
        );
        if s.marker_new {
            s.marker_new = false;
            save(data, s)?;
        }
        return Ok(());
    }
    ensure!(
        s.marker_new && s.phase == Phase::Copying,
        "Expected device identity is missing"
    );
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(s.token.as_bytes())?;
    file.sync_all()?;
    s.marker_new = false;
    save(data, s)?;
    Ok(())
}
fn owned_stage(s: &Session, root: &Path) -> Result<PathBuf> {
    let path = root.join(&s.stage_name);
    let owner = path.join("SESSION.DAT");
    if !path.exists() {
        fs::create_dir(&path)?;
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&owner)?;
        f.write_all(s.id.as_bytes())?;
        f.sync_all()?;
    }
    ensure!(
        !path.symlink_metadata()?.file_type().is_symlink(),
        "Unsafe staging directory"
    );
    if owner.exists() {
        ensure!(
            fs::metadata(&owner)?.len() <= s.id.len() as u64,
            "Staging ownership mismatch"
        );
    }
    if !owner.exists() && fs::read_dir(&path)?.next().is_none() {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&owner)?;
        file.write_all(s.id.as_bytes())?;
        file.sync_all()?;
    }
    if owner.is_file() && s.files_done == 0 && s.copy_intent.is_none() {
        let contents = fs::read(&owner)?;
        if contents.len() < s.id.len()
            && s.id.as_bytes().starts_with(&contents)
            && fs::read_dir(&path)?.count() == 1
        {
            let mut file = OpenOptions::new().write(true).open(&owner)?;
            file.write_all(s.id.as_bytes())?;
            file.sync_all()?;
        }
    }
    ensure!(
        fs::read_to_string(owner)? == s.id,
        "Staging directory ownership mismatch"
    );
    Ok(path)
}
fn quarantine<O: DeviceOps>(s: &Session, root: &Path, path: &Path, ops: &mut O) -> Result<()> {
    let dir = root.join(&s.quarantine_name);
    let owner = dir.join("SESSION.DAT");
    if !dir.exists() {
        fs::create_dir(&dir)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&owner)?;
        file.write_all(s.id.as_bytes())?;
        file.sync_all()?;
    }
    ensure!(
        !dir.symlink_metadata()?.file_type().is_symlink() && fs::read_to_string(owner)? == s.id,
        "Quarantine ownership mismatch"
    );
    for i in 0..0xffffffu32 {
        let target = dir.join(format!("Q{i:07X}.TMP"));
        if !target.exists() {
            ops.rename(path, &target)?;
            return Ok(());
        }
    }
    bail!("Quarantine is full; retained files require review")
}

fn emit(s: &Session, phase: &str, message: &str, observer: &mut dyn FnMut(Update)) {
    observer(Update {
        phase: phase.into(),
        percent: s.percent,
        done: s.files_done,
        total: s.files_total,
        message: message.into(),
        commit_in_progress: matches!(s.phase, Phase::Committing | Phase::Committed),
        root: s.root_hint.clone(),
    });
}

fn copy_files<O: DeviceOps>(
    data: &Path,
    s: &mut Session,
    root: &Path,
    ops: &mut O,
    explicit: bool,
    pause: &AtomicBool,
    observer: &mut dyn FnMut(Update),
) -> Result<()> {
    let current = current_original(s, root)?;
    if current != s.original {
        ensure!(
            normalization(s, &current, root)?,
            "Existing device library changed; no write attempted"
        );
        s.phase = Phase::Preparing;
        save(data, s)?;
        prepare_with(data, s, root, &mut || ops.checkpoint("candidate_prepared"))?;
        if s.phase == Phase::Preparing {
            return Ok(());
        }
    }
    ensure_marker(data, s, root)?;
    let stage = owned_stage(s, root)?;
    let host = directory(data, &s.id)?.join("stage/OMGAUDIO");
    ensure!(
        manifest(&host)? == s.generated,
        "Prepared files changed on the computer; backups preserved"
    );
    let files: Vec<_> = s
        .generated
        .iter()
        .filter(|(name, _)| !in_audio_directory(name) || !s.original.contains_key(*name))
        .map(|(a, b)| (a.clone(), b.clone()))
        .collect();
    s.files_total = files.len();
    s.files_done = 0;
    emit(s, "verifying", "Checking saved transfer files", observer);
    let mut remaining = Manifest::new();
    let mut verified = BTreeSet::new();
    for (name, hash) in &files {
        let target = stage.join(name);
        if target.is_file() && metadata::hash_file(&target)? == *hash {
            s.files_done += 1;
            verified.insert(name.clone());
        } else {
            remaining.insert(name.clone(), hash.clone());
        }
    }
    let required = crate::space::required_bytes(&host, &remaining)?;
    ensure!(
        fs2::available_space(root)? >= required,
        "Not enough space to resume safely; verified files retained"
    );
    let mut ordered = files;
    ordered.sort_by_key(|(name, _)| (!is_audio(name), name.clone()));
    for (name, hash) in ordered {
        if pause.load(Ordering::Relaxed) {
            s.status = Status::UserPaused;
            save(data, s)?;
            return Ok(());
        }
        let target = stage.join(&name);
        if verified.contains(&name) {
            continue;
        }
        if target.exists() {
            let actual = metadata::hash_file(&target)?;
            let previous = s
                .known
                .get(&name)
                .is_some_and(|values| values.contains(&actual));
            ensure!(
                previous || explicit,
                "Readback mismatch; check the connection before resuming"
            );
            quarantine(s, root, &target, ops)?;
        }
        let temp = root.join("OMGAUDIO/10F00").join(&s.temp_name);
        let destination = if is_audio(&name) { &temp } else { &target };
        if destination == &temp && temp.exists() {
            ensure!(
                s.copy_intent.as_deref() == Some(&name),
                "Unexpected audio temporary file; no overwrite attempted"
            );
            if metadata::hash_file(&temp)? == hash {
                fs::create_dir_all(target.parent().unwrap())?;
                ops.rename(&temp, &target)?;
                s.files_done += 1;
                s.copy_intent = None;
                save(data, s)?;
                continue;
            }
            // Only a saved in-progress copy may own a partial audio temporary.
            quarantine(s, root, &temp, ops)?;
        }
        s.copy_intent = Some(name.clone());
        save(data, s)?;
        fs::create_dir_all(destination.parent().unwrap())?;
        emit(
            s,
            "copying",
            &format!("Copying and verifying: {name}"),
            observer,
        );
        ops.copy(&host.join(&name), destination)?;
        ops.wait(if fs::metadata(host.join(&name))?.len() < 128 * 1024 {
            Duration::from_secs(3)
        } else {
            Duration::from_secs(1)
        });
        let mut actual = metadata::hash_file(destination)?;
        if actual != hash {
            ops.wait(Duration::from_secs(1));
            actual = metadata::hash_file(destination)?;
        }
        ensure!(
            actual == hash,
            "Readback mismatch; check the connection before resuming"
        );
        ensure!(
            metadata::hash_file(&host.join(&name))? == hash,
            "Prepared source changed during copying"
        );
        if is_audio(&name) {
            fs::create_dir_all(target.parent().unwrap())?;
            ops.rename(destination, &target)?;
        }
        ops.checkpoint("file_copied");
        s.copy_intent = None;
        s.files_done += 1;
        s.percent = 20 + (s.files_done * 65 / s.files_total.max(1)) as u8;
        save(data, s)?;
        emit(
            s,
            "copying",
            &format!("Verified files: {} / {}", s.files_done, s.files_total),
            observer,
        );
    }
    let mut staged = manifest(&stage)?;
    staged.remove("SESSION.DAT");
    let expected: Manifest = s
        .generated
        .iter()
        .filter(|(n, _)| !in_audio_directory(n) || !s.original.contains_key(*n))
        .map(|(n, h)| (n.clone(), h.clone()))
        .collect();
    ensure!(
        staged == expected,
        "Staged library does not match the verified plan"
    );
    let latest = current_original(s, root)?;
    if latest != s.original {
        ensure!(
            normalization(s, &latest, root)?,
            "Existing device library changed; no commit attempted"
        );
        s.phase = Phase::Preparing;
        save(data, s)?;
        return Ok(());
    }
    s.copy_intent = None;
    s.audio_dirs = fs::read_dir(root.join("OMGAUDIO"))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_ok_and(|t| t.is_dir()) && audio_dir(&e.file_name().to_string_lossy())
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    for name in s.generated.keys().filter(|n| in_audio_directory(n)) {
        let dir = name.split('/').next().unwrap().to_string();
        if !s.audio_dirs.contains(&dir) {
            s.audio_dirs.push(dir);
        }
    }
    s.audio_dirs.sort();
    s.phase = Phase::Ready;
    s.percent = 90;
    save(data, s)
}

fn verify_final(s: &Session, root: &Path) -> Result<()> {
    let actual = manifest(&root.join("OMGAUDIO"))?;
    if actual != s.generated {
        let changed: BTreeSet<_> = actual
            .keys()
            .chain(s.generated.keys())
            .filter(|n| actual.get(*n) != s.generated.get(*n))
            .collect();
        ensure!(
            changed.iter().all(|n| MUTABLE.contains(&n.as_str()))
                && identities(root)? == s.target_identities,
            "Committed library needs review; complete backups retained"
        );
    }
    ensure!(
        identities(root)? == s.target_identities,
        "Committed songs or artwork do not match the prepared library"
    );
    Ok(())
}

fn commit<O: DeviceOps>(
    data: &Path,
    s: &mut Session,
    root: &Path,
    ops: &mut O,
    observer: &mut dyn FnMut(Update),
) -> Result<()> {
    let library = root.join("OMGAUDIO");
    let stage = root.join(&s.stage_name);
    let backup = root.join(&s.backup_name);
    ensure!(
        s.original
            .iter()
            .filter(|(n, _)| in_audio_directory(n))
            .all(|(n, h)| s.generated.get(n) == Some(h)),
        "Existing audio must never be replaced by append recovery"
    );
    // A checksum-valid older journal generation may precede the last durable
    // rename. Infer only unambiguous graphs whose file hashes prove ownership.
    if !library.exists() && backup.is_dir() && stage.is_dir() {
        let before = manifest(&backup)?;
        let incoming = manifest(&stage)?;
        let non_audio = |m: &Manifest| {
            m.iter()
                .filter(|(n, _)| !in_audio_directory(n))
                .map(|(n, h)| (n.clone(), h.clone()))
                .collect::<Manifest>()
        };
        ensure!(
            non_audio(&before) == non_audio(&s.original)
                && non_audio(&incoming) == non_audio(&s.generated),
            "Retained or staged metadata changed during recovery"
        );
        for (name, hash) in s.generated.iter().filter(|(n, _)| in_audio_directory(n)) {
            let locations = [backup.join(name), stage.join(name)];
            let present: Vec<_> = locations.iter().filter(|p| p.is_file()).collect();
            ensure!(
                present.len() == 1 && metadata::hash_file(present[0])? == *hash,
                "Audio has ambiguous or invalid recovery locations"
            );
        }
        s.phase = Phase::Committing;
        s.step = Step::MoveAudio;
        save(data, s)?;
    } else if library.is_dir() && backup.is_dir() && !stage.exists() {
        verify_final(s, root)?;
        s.phase = Phase::Committed;
        s.percent = 100;
        save(data, s)?;
        return Ok(());
    } else if s.phase == Phase::Ready && library.is_dir() && !backup.exists() {
        let added = s
            .generated
            .keys()
            .filter(|n| is_audio(n) && !s.original.contains_key(*n))
            .any(|n| library.join(n).is_file());
        if added {
            s.phase = Phase::Committing;
            s.step = Step::AddAudio;
            save(data, s)?;
        }
    }
    emit(
        s,
        "committing",
        "Finishing the verified transfer; keep the device connected",
        observer,
    );
    if s.phase == Phase::Ready {
        ensure!(
            current_original(s, root)? == s.original,
            "Device changed before commit; verified files retained"
        );
        s.phase = Phase::Committing;
        s.step = Step::AddAudio;
        save(data, s)?;
    }
    if s.step == Step::AddAudio {
        ensure!(
            library.is_dir() && !backup.exists(),
            "Unexpected directory state before commit"
        );
        let mut original = current_original(s, root)?;
        for (name, hash) in s
            .generated
            .iter()
            .filter(|(n, _)| is_audio(n) && !s.original.contains_key(*n))
        {
            let target = library.join(name);
            let from = stage.join(name);
            if target.exists() {
                ensure!(
                    metadata::hash_file(&target)? == *hash && !from.exists(),
                    "New audio has conflicting locations"
                );
                original.remove(name);
            } else {
                ensure!(
                    metadata::hash_file(&from)? == *hash,
                    "Prepared audio missing before commit"
                );
            }
        }
        ensure!(
            original == s.original,
            "Old library changed during commit preparation"
        );
        for (name, hash) in s
            .generated
            .iter()
            .filter(|(n, _)| is_audio(n) && !s.original.contains_key(*n))
        {
            let target = library.join(name);
            let from = stage.join(name);
            if !target.exists() {
                fs::create_dir_all(target.parent().unwrap())?;
                ops.rename(&from, &target)?;
            }
            ensure!(
                metadata::hash_file(&target)? == *hash,
                "New audio verification failed during commit"
            );
            ops.checkpoint("audio_added");
        }
        for dir in &s.audio_dirs {
            let path = stage.join(dir);
            if path.exists() {
                fs::remove_dir(path).context("Audio staging directory is not empty")?;
            }
        }
        let owner = stage.join("SESSION.DAT");
        if owner.exists() {
            ensure!(
                fs::read_to_string(&owner)? == s.id,
                "Staging ownership changed"
            );
            fs::remove_file(owner)?;
        }
        s.step = Step::RetainRoot;
        save(data, s)?;
    }
    if s.step == Step::RetainRoot {
        if library.exists() {
            ensure!(!backup.exists(), "Backup directory already exists");
            ops.rename(&library, &backup)?;
        }
        ensure!(
            backup.is_dir() && !library.exists(),
            "Cannot locate the retained original library"
        );
        ops.checkpoint("root_retained");
        s.step = Step::MoveAudio;
        save(data, s)?;
    }
    if s.step == Step::MoveAudio {
        ensure!(
            !library.exists() && backup.is_dir() && stage.is_dir(),
            "Unexpected directory state during audio recovery"
        );
        for name in &s.audio_dirs {
            let from = backup.join(name);
            let to = stage.join(name);
            if from.exists() {
                ensure!(!to.exists(), "Audio directory has conflicting locations");
                ops.rename(&from, &to)?;
            }
            ensure!(to.is_dir(), "Cannot locate the preserved audio directory");
            ops.checkpoint("audio_directory_moved");
        }
        s.step = Step::Install;
        save(data, s)?;
    }
    if s.step == Step::Install {
        if stage.exists() {
            ensure!(
                !library.exists(),
                "A different library appeared during recovery"
            );
            ops.rename(&stage, &library)?;
        }
        ensure!(
            library.is_dir() && !stage.exists(),
            "Cannot locate the installed library"
        );
        ops.checkpoint("library_installed");
        s.step = Step::Verify;
        save(data, s)?;
    }
    verify_final(s, root)?;
    s.phase = Phase::Committed;
    s.percent = 100;
    s.error.clear();
    save(data, s)?;
    ops.checkpoint("commit_recorded");
    Ok(())
}

pub fn transient(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|e| e.downcast_ref::<std::io::Error>())
        .any(|e| {
            #[cfg(windows)]
            {
                matches!(
                    e.raw_os_error(),
                    Some(6 | 21 | 32 | 121 | 1006 | 1117 | 1167)
                )
            }
            #[cfg(not(windows))]
            {
                matches!(e.raw_os_error(), Some(5 | 6 | 19 | 116))
            }
        })
}

pub fn run<O: DeviceOps>(
    data: &Path,
    id: &str,
    roots: impl Fn() -> Vec<PathBuf>,
    explicit: bool,
    pause: &AtomicBool,
    ops: &mut O,
    observer: &mut dyn FnMut(Update),
) -> Result<Outcome> {
    let mut s = load(data, id)?;
    ensure!(s.pending(), "This transfer is already finalized");
    if !explicit && matches!(s.status, Status::UserPaused | Status::NeedsReview) {
        return Ok(Outcome::Paused(s));
    }
    s.status = Status::Active;
    s.error.clear();
    save(data, &mut s)?;
    let mut failures = 0;
    let mut rebases = 0;
    loop {
        if pause.load(Ordering::Relaxed) && !matches!(s.phase, Phase::Committing | Phase::Committed)
        {
            s.status = Status::UserPaused;
            save(data, &mut s)?;
            return Ok(Outcome::Paused(s));
        }
        let result = (|| -> Result<()> {
            let root = resolve_mode(&s, &roots(), explicit)?
                .context("Expected device is not connected")?;
            if s.root_hint != root {
                s.root_hint = root.clone();
                save(data, &mut s)?;
            }
            if s.phase == Phase::Preparing {
                emit(
                    &s,
                    "preparing",
                    "Backing up and preparing the append",
                    observer,
                );
                prepare_with(data, &mut s, &root, &mut || {
                    ops.checkpoint("candidate_prepared")
                })?;
                if s.phase == Phase::Preparing {
                    rebases += 1;
                    ensure!(
                        rebases <= 3,
                        "Device metadata keeps changing; verified progress retained"
                    );
                    return Ok(());
                }
                ops.checkpoint("prepared");
            }
            if pause.load(Ordering::Relaxed) && matches!(s.phase, Phase::Copying | Phase::Ready) {
                s.status = Status::UserPaused;
                save(data, &mut s)?;
                return Ok(());
            }
            if matches!(s.phase, Phase::Copying | Phase::Ready) {
                for track in &s.selected {
                    ensure!(
                        metadata::hash_file(Path::new(&track.path))? == track.key,
                        "Source changed before commit; restore it or discard this transfer: {}",
                        track.title
                    );
                }
            }
            ensure_marker(data, &mut s, &root)?;
            let lock_path = root.join(".island-walkman.lock");
            let lock = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(lock_path)?;
            fs2::FileExt::try_lock_exclusive(&lock)
                .context("Another transfer is using this device")?;
            if s.phase == Phase::Copying {
                copy_files(data, &mut s, &root, ops, explicit, pause, observer)?;
                if s.phase == Phase::Preparing {
                    rebases += 1;
                    ensure!(
                        rebases <= 3,
                        "Device metadata keeps changing; verified progress retained"
                    );
                    return Ok(());
                }
                if s.status == Status::UserPaused {
                    return Ok(());
                }
            }
            if s.phase == Phase::Ready && pause.load(Ordering::Relaxed) {
                s.status = Status::UserPaused;
                save(data, &mut s)?;
                return Ok(());
            }
            if matches!(s.phase, Phase::Ready | Phase::Committing) {
                commit(data, &mut s, &root, ops, observer)?;
            }
            if s.phase == Phase::Committed {
                verify_final(&s, &root)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) if s.phase == Phase::Committed => {
                emit(
                    &s,
                    "completed",
                    "Transfer complete; safely eject and verify playback",
                    observer,
                );
                return Ok(Outcome::Complete(s));
            }
            Ok(()) if s.status == Status::UserPaused => return Ok(Outcome::Paused(s)),
            Ok(()) => continue,
            Err(error) => {
                if s.phase == Phase::Preparing
                    && error
                        .to_string()
                        .contains("Host backup verification failed")
                    && rebases < 3
                {
                    if let Ok(Some(root)) = resolve_mode(&s, &roots(), explicit) {
                        if let Ok(actual) = current_original(&s, &root) {
                            if actual != s.original
                                && normalization(&s, &actual, &root).unwrap_or(false)
                            {
                                rebases += 1;
                                save(data, &mut s)?;
                                continue;
                            }
                        }
                    }
                }
                let disconnected = matches!(resolve(&s, &roots()), Ok(None));
                s.error = format!("{error:#}");
                if (transient(&error) || disconnected) && failures < 3 {
                    failures += 1;
                    s.status = Status::WaitingDevice;
                    save(data, &mut s)?;
                    emit(
                        &s,
                        "recovering",
                        "Connection interrupted; restoring verified progress",
                        observer,
                    );
                    ops.wait(Duration::from_secs([2, 5, 10][failures - 1]));
                    s = load(data, id)?;
                    s.status = Status::Active;
                    continue;
                }
                s.status = if disconnected {
                    Status::WaitingDevice
                } else {
                    Status::NeedsReview
                };
                save(data, &mut s)?;
                emit(
                    &s,
                    if s.status == Status::WaitingDevice {
                        "waiting_device"
                    } else {
                        "needs_review"
                    },
                    &s.error,
                    observer,
                );
                return Ok(Outcome::Paused(s));
            }
        }
    }
}

#[cfg(windows)]
fn wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    let text = path.as_os_str().to_string_lossy();
    let path = if text.starts_with("\\\\?\\") && !text.starts_with("\\\\?\\UNC\\") {
        Path::new(&text[4..])
    } else {
        path
    };
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CopyFileW(from: *const u16, to: *const u16, fail: i32) -> i32;
    fn GetVolumeInformationW(
        root: *const u16,
        name: *mut u16,
        name_size: u32,
        serial: *mut u32,
        max_component: *mut u32,
        flags: *mut u32,
        fs_name: *mut u16,
        fs_size: u32,
    ) -> i32;
}
#[cfg(windows)]
fn volume_serial(root: &Path) -> Result<Option<u32>> {
    use std::path::Prefix;
    let drive = match root.components().next() {
        Some(Component::Prefix(p)) => match p.kind() {
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => d,
            _ => bail!("Expected a local device drive"),
        },
        _ => bail!("Expected a local device drive"),
    };
    let path = PathBuf::from(format!("{}:\\", drive as char));
    let mut serial = 0;
    let ok = unsafe {
        GetVolumeInformationW(
            wide(&path).as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(Some(serial))
}
#[cfg(not(windows))]
fn volume_serial(_root: &Path) -> Result<Option<u32>> {
    Ok(None)
}
#[cfg(windows)]
fn native_copy(from: &Path, to: &Path) -> Result<()> {
    if unsafe { CopyFileW(wide(from).as_ptr(), wide(to).as_ptr(), 1) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn paused_fixture(data: &Path, session: &Session) -> Result<Session> {
    let mut s = staged_fixture(data, session)?;
    fs::remove_file(s.root_hint.join(&s.stage_name).join("04CNTINF.DAT"))?;
    s.files_done = s.files_done.saturating_sub(1);
    s.phase = Phase::Copying;
    s.status = Status::UserPaused;
    save(data, &mut s)?;
    Ok(s)
}

#[cfg(test)]
pub(crate) fn staged_fixture(data: &Path, session: &Session) -> Result<Session> {
    struct Fast;
    impl DeviceOps for Fast {
        fn copy(&mut self, source: &Path, target: &Path) -> Result<()> {
            native_copy(source, target)
        }
        fn wait(&mut self, _duration: Duration) {}
    }
    let mut s = session.clone();
    let root = s.root_hint.clone();
    prepare(data, &mut s, &root)?;
    ensure_marker(data, &mut s, &root)?;
    copy_files(
        data,
        &mut s,
        &root,
        &mut Fast,
        true,
        &AtomicBool::new(false),
        &mut |_| {},
    )?;
    ensure!(s.phase == Phase::Ready, "Fixture did not reach ready phase");
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{Arc, Mutex},
    };

    #[derive(Default)]
    struct Ops {
        copies: usize,
        renames: usize,
        waits: usize,
        fail_copy: usize,
        fail_rename: usize,
        fail_renames_from: usize,
        disk_full: bool,
        always_fail: bool,
        corrupt: bool,
        crash: Option<&'static str>,
        changed: bool,
        normalize: Option<PathBuf>,
        normalize_preparing: Option<PathBuf>,
        mutate_title: Option<PathBuf>,
        unplug: Option<(PathBuf, PathBuf)>,
        move_on_wait: Option<(PathBuf, PathBuf)>,
    }
    fn media_error() -> anyhow::Error {
        #[cfg(windows)]
        let code = 1117;
        #[cfg(not(windows))]
        let code = 5;
        std::io::Error::from_raw_os_error(code).into()
    }
    impl DeviceOps for Ops {
        fn copy(&mut self, source: &Path, target: &Path) -> Result<()> {
            self.copies += 1;
            if self.disk_full {
                #[cfg(windows)]
                let code = 112;
                #[cfg(not(windows))]
                let code = 28;
                return Err(std::io::Error::from_raw_os_error(code).into());
            }
            if let Some((from, to)) = self.unplug.take() {
                fs::write(target, b"partial")?;
                self.move_on_wait = Some((from, to));
                return Err(media_error());
            }
            if self.always_fail || self.copies == self.fail_copy {
                return Err(media_error());
            }
            native_copy(source, target)?;
            if self.corrupt {
                fs::write(target, b"corrupted device bytes")?;
                self.corrupt = false;
            }
            Ok(())
        }
        fn rename(&mut self, source: &Path, target: &Path) -> Result<()> {
            self.renames += 1;
            if self.renames == self.fail_rename
                || (self.fail_renames_from > 0 && self.renames >= self.fail_renames_from)
            {
                return Err(media_error());
            }
            fs::rename(source, target)?;
            Ok(())
        }
        fn wait(&mut self, _duration: Duration) {
            self.waits += 1;
            if let Some((from, to)) = self.move_on_wait.take() {
                fs::rename(from, to).unwrap();
            }
        }
        fn checkpoint(&mut self, name: &str) {
            if self.crash == Some(name) {
                self.crash = None;
                panic!("simulated process death: {name}")
            }
            if name == "candidate_prepared" && !self.changed {
                if let Some(root) = &self.normalize_preparing {
                    let p = root.join("OMGAUDIO/00GTRLST.DAT");
                    let mut bytes = fs::read(&p).unwrap();
                    let last = bytes.len() - 1;
                    bytes[last] ^= 1;
                    fs::write(p, bytes).unwrap();
                    self.changed = true;
                }
            }
            if name == "file_copied" && !self.changed {
                if let Some(root) = &self.normalize {
                    let p = root.join("OMGAUDIO/00GTRLST.DAT");
                    let mut b = fs::read(&p).unwrap();
                    let last = b.len() - 1;
                    b[last] ^= 1;
                    fs::write(p, b).unwrap();
                    self.changed = true;
                }
                if let Some(root) = &self.mutate_title {
                    let p = root.join("OMGAUDIO/04CNTINF.DAT");
                    let mut b = fs::read(&p).unwrap();
                    let text: Vec<u8> = "Existing"
                        .encode_utf16()
                        .flat_map(u16::to_be_bytes)
                        .collect();
                    let pos = b.windows(text.len()).position(|w| w == text).unwrap();
                    b[pos + 1] = b'X';
                    fs::write(p, b).unwrap();
                    self.changed = true;
                }
            }
        }
    }
    struct Fixture {
        _temp: tempfile::TempDir,
        data: PathBuf,
        root: PathBuf,
        selected: Vec<Track>,
        original: Manifest,
    }
    fn fixture(count: usize) -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("player");
        let data = temp.path().join("application");
        for name in ["DCIM", "VIDEO", "PICTURE", "OMGAUDIO"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }
        fs::create_dir_all(&data).unwrap();
        let mut selected = vec![];
        for i in 0..=count {
            let source = temp.path().join(format!("source-{i}.mp3"));
            fs::write(
                &source,
                [vec![0xff, 0xfb, 0x90, 0], vec![i as u8; 413]]
                    .concat()
                    .repeat(100 + i),
            )
            .unwrap();
            let mut track = metadata::read_mp3(&source, &data.join("covers")).unwrap();
            track.title = if i == 0 {
                "Existing".into()
            } else {
                format!("新しい歌 {i}")
            };
            track.artist = format!("Artist {i}");
            track.album = format!("Album {i}");
            let image = temp.path().join(format!("artwork-{i}.jpg"));
            image::RgbImage::from_pixel(60, 40, image::Rgb([i as u8, 120, 30]))
                .save(&image)
                .unwrap();
            track.cover = Some(image.to_string_lossy().into());
            if i == 0 {
                track.device_id = Some(1);
                let audio = root.join("OMGAUDIO").join(protocol::oma_name(1));
                fs::create_dir_all(audio.parent().unwrap()).unwrap();
                let mut wrapped = track.clone();
                wrapped.cover = None;
                protocol::write_oma(&wrapped, &audio).unwrap();
                protocol::write_database(&root.join("OMGAUDIO"), &[track], true).unwrap();
            } else {
                selected.push(track);
            }
        }
        fs::write(
            root.join("OMGAUDIO/OPAQUE.BIN"),
            b"unknown data must survive",
        )
        .unwrap();
        let original = manifest(&root.join("OMGAUDIO")).unwrap();
        Fixture {
            data,
            root,
            selected,
            original,
            _temp: temp,
        }
    }
    fn execute(f: &Fixture, s: &Session, ops: &mut Ops, explicit: bool) -> Outcome {
        run(
            &f.data,
            &s.id,
            || vec![f.root.clone()],
            explicit,
            &AtomicBool::new(false),
            ops,
            &mut |_| {},
        )
        .unwrap()
    }
    fn complete(outcome: Outcome) -> Session {
        match outcome {
            Outcome::Complete(s) => s,
            Outcome::Paused(s) => panic!("unexpected pause: {} ({:?})", s.error, s.phase),
        }
    }
    #[test]
    fn successful_append_preserves_old_audio_and_source_and_can_finalize_twice_safely() {
        let f = fixture(4);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let done = complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 5);
        assert_eq!(
            metadata::hash_file(&f.root.join("OMGAUDIO").join(protocol::oma_name(1))).unwrap(),
            f.original["10F00/10000001.OMA"]
        );
        assert_eq!(
            fs::read(f.root.join("OMGAUDIO/OPAQUE.BIN")).unwrap(),
            b"unknown data must survive"
        );
        for t in &f.selected {
            assert_eq!(metadata::hash_file(Path::new(&t.path)).unwrap(), t.key);
        }
        assert_eq!(
            manifest(&directory(&f.data, &s.id).unwrap().join("before/OMGAUDIO")).unwrap(),
            f.original
        );
        assert!(done.stage_name.len() == 8 && done.backup_name.len() == 8);
        let copies = complete(execute(&f, &s, &mut Ops::default(), false));
        assert_eq!(copies.phase, Phase::Committed);
        acknowledge(&f.data, &s.id).unwrap();
        assert!(pending(&f.data).unwrap().is_none());
    }
    #[test]
    fn crashes_after_each_durable_boundary_resume_without_duplicate_tracks() {
        for point in [
            "prepared",
            "file_copied",
            "audio_added",
            "root_retained",
            "audio_directory_moved",
            "library_installed",
            "commit_recorded",
        ] {
            let f = fixture(2);
            let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
            let mut ops = Ops {
                crash: Some(point),
                ..Default::default()
            };
            assert!(
                catch_unwind(AssertUnwindSafe(|| execute(&f, &s, &mut ops, true))).is_err(),
                "{point}"
            );
            let done = complete(execute(&f, &s, &mut Ops::default(), false));
            assert_eq!(done.phase, Phase::Committed, "{point}");
            assert_eq!(protocol::read_device(&f.root).unwrap().len(), 3, "{point}");
        }
    }

    #[test]
    fn a_torn_latest_intent_after_a_rename_is_inferred_from_verified_locations() {
        for point in [
            "audio_added",
            "root_retained",
            "audio_directory_moved",
            "library_installed",
            "commit_recorded",
        ] {
            let f = fixture(1);
            let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
            let mut ops = Ops {
                crash: Some(point),
                ..Default::default()
            };
            assert!(catch_unwind(AssertUnwindSafe(|| execute(&f, &s, &mut ops, true))).is_err());
            let latest = load(&f.data, &s.id).unwrap();
            fs::write(
                directory(&f.data, &s.id)
                    .unwrap()
                    .join(format!("journal-{}.json", latest.generation % 2)),
                b"torn latest slot",
            )
            .unwrap();
            complete(execute(&f, &s, &mut Ops::default(), false));
            assert_eq!(protocol::read_device(&f.root).unwrap().len(), 2, "{point}");
        }
    }
    #[test]
    fn an_empty_player_and_a_new_audio_directory_can_be_initialized() {
        let f = fixture(1);
        fs::rename(f.root.join("OMGAUDIO"), f.root.join("UNUSED")).unwrap();
        fs::create_dir(f.root.join("OMGAUDIO")).unwrap();
        let s = create(&f.data, &f.root, f.selected.clone(), false).unwrap();
        complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 1);
    }
    #[test]
    fn crossing_the_256_track_boundary_moves_both_audio_directories() {
        let f = fixture(2);
        let mut old = protocol::read_device(&f.root).unwrap().remove(0);
        old.device_id = Some(255);
        fs::rename(
            f.root.join("OMGAUDIO").join(protocol::oma_name(1)),
            f.root.join("OMGAUDIO").join(protocol::oma_name(255)),
        )
        .unwrap();
        protocol::write_database(&f.root.join("OMGAUDIO"), &[old], true).unwrap();
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(
            protocol::read_device(&f.root)
                .unwrap()
                .iter()
                .map(|t| t.device_id.unwrap())
                .collect::<Vec<_>>(),
            vec![255, 256, 257]
        );
    }
    #[test]
    fn saved_artwork_is_frozen_when_local_cover_is_replaced_while_paused() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let mut prepared = staged_fixture(&f.data, &s).unwrap();
        prepared.phase = Phase::Copying;
        save(&f.data, &mut prepared).unwrap();
        fs::write(
            f.selected[0].cover.as_deref().unwrap(),
            b"replacement no longer matches the saved artwork",
        )
        .unwrap();
        complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 2);
    }
    #[test]
    fn source_change_after_preparation_requires_review_before_commit() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        staged_fixture(&f.data, &s).unwrap();
        fs::write(&f.selected[0].path, b"changed after preparation").unwrap();
        assert!(matches!(
            execute(&f, &s, &mut Ops::default(), true),
            Outcome::Paused(Session {
                status: Status::NeedsReview,
                ..
            })
        ));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 1);
    }
    #[test]
    fn each_commit_rename_transport_failure_recovers_automatically() {
        // Four additions, one root retain, one audio-directory move, one install.
        for fail in 1..=7 {
            let f = fixture(4);
            let s = create(&f.data, &f.root, f.selected.clone(), false).unwrap();
            // Copying uses native renames too. Inject only once after Ready.
            let mut paused = Ops::default();
            let flag = AtomicBool::new(false);
            let mut session = load(&f.data, &s.id).unwrap();
            prepare(&f.data, &mut session, &f.root).unwrap();
            ensure_marker(&f.data, &mut session, &f.root).unwrap();
            copy_files(
                &f.data,
                &mut session,
                &f.root,
                &mut paused,
                true,
                &flag,
                &mut |_| {},
            )
            .unwrap();
            assert_eq!(session.phase, Phase::Ready);
            let mut ops = Ops {
                fail_rename: fail,
                ..Default::default()
            };
            complete(execute(&f, &s, &mut ops, true));
            assert_eq!(
                protocol::read_device(&f.root).unwrap().len(),
                5,
                "rename {fail}"
            );
        }
    }
    #[test]
    fn verified_files_survive_restart_and_are_not_copied_again() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let flag = AtomicBool::new(false);
        let mut ops = Ops::default();
        let result = run(
            &f.data,
            &s.id,
            || vec![f.root.clone()],
            true,
            &flag,
            &mut ops,
            &mut |u| {
                if u.done > 0 {
                    flag.store(true, Ordering::Relaxed);
                }
            },
        )
        .unwrap();
        let paused = match result {
            Outcome::Paused(s) => s,
            _ => panic!("not paused"),
        };
        assert_eq!(paused.status, Status::UserPaused);
        let done_before = paused.files_done;
        assert!(done_before > 0);
        let mut resumed = Ops::default();
        let done = complete(execute(&f, &s, &mut resumed, true));
        assert_eq!(resumed.copies + done_before, done.files_total);
    }
    #[test]
    fn partial_transport_copy_reconnects_to_same_identity_at_new_path() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let moved = f.root.with_file_name("reconnected");
        let candidates = Arc::new(Mutex::new(vec![moved.clone()]));
        let mut ops = Ops {
            unplug: Some((f.root.clone(), moved.clone())),
            ..Default::default()
        };
        let out = run(
            &f.data,
            &s.id,
            || candidates.lock().unwrap().clone(),
            true,
            &AtomicBool::new(false),
            &mut ops,
            &mut |_| {},
        )
        .unwrap();
        let done = complete(out);
        assert_eq!(done.root_hint, moved);
        assert_eq!(protocol::read_device(&done.root_hint).unwrap().len(), 2);
    }
    #[test]
    fn repeated_io_errors_pause_after_bounded_attempts_without_touching_old_library() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let mut ops = Ops {
            always_fail: true,
            ..Default::default()
        };
        let out = execute(&f, &s, &mut ops, true);
        let paused = match out {
            Outcome::Paused(s) => s,
            _ => panic!("not paused"),
        };
        assert_eq!(paused.status, Status::NeedsReview);
        assert_eq!(ops.copies, 4);
        assert_eq!(manifest(&f.root.join("OMGAUDIO")).unwrap(), f.original);
        complete(execute(&f, &s, &mut Ops::default(), true));
    }
    #[test]
    fn checksum_mismatch_requires_review_and_never_autorewrites() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let mut ops = Ops {
            corrupt: true,
            ..Default::default()
        };
        let out = execute(&f, &s, &mut ops, true);
        let paused = match out {
            Outcome::Paused(s) => s,
            _ => panic!("not paused"),
        };
        assert_eq!(paused.status, Status::NeedsReview);
        let mut auto = Ops::default();
        assert!(matches!(
            execute(&f, &s, &mut auto, false),
            Outcome::Paused(_)
        ));
        assert_eq!(auto.copies, 0);
        complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 2);
    }
    #[test]
    fn known_system_normalization_rebases_but_title_changes_do_not() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        complete(execute(
            &f,
            &s,
            &mut Ops {
                normalize: Some(f.root.clone()),
                ..Default::default()
            },
            true,
        ));
        let g = fixture(1);
        let t = create(&g.data, &g.root, g.selected.clone(), true).unwrap();
        let out = execute(
            &g,
            &t,
            &mut Ops {
                mutate_title: Some(g.root.clone()),
                ..Default::default()
            },
            true,
        );
        assert!(matches!(
            out,
            Outcome::Paused(Session {
                status: Status::NeedsReview,
                ..
            })
        ));
        assert_eq!(protocol::read_device(&g.root).unwrap().len(), 1);
    }
    #[test]
    fn system_normalization_during_preparation_rebases_before_any_copy() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        complete(execute(
            &f,
            &s,
            &mut Ops {
                normalize_preparing: Some(f.root.clone()),
                ..Default::default()
            },
            true,
        ));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 2);
    }
    #[test]
    fn a_different_device_at_the_same_path_is_not_written() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        execute(
            &f,
            &s,
            &mut Ops {
                always_fail: true,
                ..Default::default()
            },
            true,
        );
        fs::write(f.root.join(MARKER), "f".repeat(64)).unwrap();
        let before = manifest(&f.root).unwrap();
        let mut ops = Ops::default();
        let out = execute(&f, &s, &mut ops, true);
        assert!(matches!(out, Outcome::Paused(_)));
        assert_eq!(ops.copies, 0);
        assert_eq!(manifest(&f.root).unwrap(), before);
    }
    #[test]
    fn torn_new_journal_slot_falls_back_and_both_bad_slots_block() {
        let f = fixture(1);
        let mut s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        save(&f.data, &mut s).unwrap();
        let dir = directory(&f.data, &s.id).unwrap();
        fs::write(
            dir.join(format!("journal-{}.json", s.generation % 2)),
            b"torn",
        )
        .unwrap();
        assert_eq!(load(&f.data, &s.id).unwrap().generation, s.generation - 1);
        fs::write(
            dir.join(format!("journal-{}.json", (s.generation + 1) % 2)),
            b"also torn",
        )
        .unwrap();
        assert!(load(&f.data, &s.id).is_err());
    }
    #[test]
    fn journal_paths_cannot_escape_and_source_changes_stop_before_copy() {
        for name in [
            "../other",
            "/absolute",
            "C:/other",
            "10F00/../../other",
            "10F00\\other",
        ] {
            assert!(checked_relative(name).is_err(), "{name}");
        }
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        fs::write(&f.selected[0].path, b"changed source").unwrap();
        let mut ops = Ops::default();
        assert!(matches!(
            execute(&f, &s, &mut ops, true),
            Outcome::Paused(Session {
                status: Status::NeedsReview,
                ..
            })
        ));
        assert_eq!(ops.copies, 0);
        assert_eq!(manifest(&f.root.join("OMGAUDIO")).unwrap(), f.original);
    }
    #[test]
    fn new_session_never_overwrites_unrelated_short_name_directories() {
        let f = fixture(1);
        fs::create_dir(f.root.join("IW000001")).unwrap();
        fs::write(f.root.join("IW000001/USER.TXT"), b"unrelated").unwrap();
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        assert_ne!(s.stage_name, "IW000001");
        complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(
            fs::read(f.root.join("IW000001/USER.TXT")).unwrap(),
            b"unrelated"
        );
    }
    #[test]
    fn auxiliary_files_inside_audio_directories_move_without_being_rewritten() {
        let f = fixture(1);
        fs::write(
            f.root.join("OMGAUDIO/10F00/NOTES.TXT"),
            b"retained auxiliary data",
        )
        .unwrap();
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        complete(execute(&f, &s, &mut Ops::default(), true));
        assert_eq!(
            fs::read(f.root.join("OMGAUDIO/10F00/NOTES.TXT")).unwrap(),
            b"retained auxiliary data"
        );
    }
    #[test]
    fn persistent_commit_errors_keep_recovery_coordinates_and_finish_on_manual_resume() {
        for fail_from in [2, 3] {
            let f = fixture(1);
            let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
            staged_fixture(&f.data, &s).unwrap();
            let out = execute(
                &f,
                &s,
                &mut Ops {
                    fail_renames_from: fail_from,
                    ..Default::default()
                },
                true,
            );
            assert!(matches!(
                out,
                Outcome::Paused(Session {
                    phase: Phase::Committing,
                    status: Status::NeedsReview,
                    ..
                })
            ));
            assert!(discard(&f.data, &s.id).is_err());
            complete(execute(&f, &s, &mut Ops::default(), true));
            assert_eq!(protocol::read_device(&f.root).unwrap().len(), 2);
        }
    }
    #[test]
    fn disk_full_stops_without_automatic_retries_or_original_library_changes() {
        let f = fixture(1);
        let s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        let mut ops = Ops {
            disk_full: true,
            ..Default::default()
        };
        assert!(matches!(
            execute(&f, &s, &mut ops, true),
            Outcome::Paused(Session {
                status: Status::NeedsReview,
                ..
            })
        ));
        assert_eq!(ops.copies, 1);
        assert_eq!(manifest(&f.root.join("OMGAUDIO")).unwrap(), f.original);
    }
    #[test]
    fn unindexed_audio_and_case_insensitive_reserved_names_are_not_overwritten() {
        let f = fixture(1);
        fs::write(
            f.root.join("OMGAUDIO/10F00/10000002.OMA"),
            b"unindexed user data",
        )
        .unwrap();
        assert!(create(&f.data, &f.root, f.selected.clone(), true).is_err());
        let g = fixture(1);
        fs::write(
            g.root.join("OMGAUDIO/session.dat"),
            b"unrelated reserved data",
        )
        .unwrap();
        assert!(create(&g.data, &g.root, g.selected.clone(), true).is_err());
    }
    #[test]
    fn interrupted_initial_identity_and_empty_stage_markers_are_completed_safely() {
        let f = fixture(1);
        let mut s = create(&f.data, &f.root, f.selected.clone(), true).unwrap();
        prepare(&f.data, &mut s, &f.root).unwrap();
        fs::write(f.root.join(MARKER), &s.token.as_bytes()[..16]).unwrap();
        let stage = f.root.join(&s.stage_name);
        fs::create_dir(&stage).unwrap();
        fs::write(stage.join("SESSION.DAT"), &s.id.as_bytes()[..8]).unwrap();
        complete(execute(&f, &s, &mut Ops::default(), false));
        assert_eq!(protocol::read_device(&f.root).unwrap().len(), 2);
    }
}
#[cfg(not(windows))]
fn native_copy(from: &Path, to: &Path) -> Result<()> {
    let mut source = File::open(from)?;
    let mut target = OpenOptions::new().create_new(true).write(true).open(to)?;
    std::io::copy(&mut source, &mut target)?;
    target.sync_all()?;
    Ok(())
}
