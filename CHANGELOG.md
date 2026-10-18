# Changelog

All notable changes are recorded here. Versions follow Semantic Versioning. Unreleased entries describe development state, not a published release.

## [Unreleased]

## [0.3.0] - 2026-10-18

### Added

- Durable append sessions with two checksummed journal generations, complete computer backups, frozen metadata/artwork selection, per-file SHA-256 verification and file-level resume across application restarts.
- Automatic recovery from bounded transient transport failures. Disconnected sessions wait for the same device; repeated I/O errors while it remains connected require review instead of infinite background retries.
- Three-language recovery progress, pause-after-current-file, resume, retry-after-checking and discard-task-with-files-retained controls. Existing confirmation authorizes the saved task; resuming does not require selecting its songs again.
- Device identity markers, Windows volume serial verification, strict recovery path validation, directory ownership checks and safeguards against another device taking the same drive letter.

### Changed

- Windows transfer copies use ordinary CopyFileW, the successful physical diagnostic strategy, with settling intervals and readback verification. POSIX copies retain flush and verification. Existing audio is moved by directory during the final transaction instead of being rewritten.
- Incoming and retained metadata directories use 8.3 names. Saved intents make interrupted directory moves idempotent; completion updates the desktop queue before acknowledging the journal, preventing duplicate appends after a restart.
- Observed system/history-table normalization can regenerate a candidate only when the original track, audio and artwork identities and every other file remain unchanged. Other changes and repeated checksum failures pause for review.
- Cached older installer versions are excluded from staged build artifacts and installed-package CI tests select the current application version explicitly.

### Validation and limits

- The preceding controlled append added eight songs to the physical A918; the user confirmed all nine songs appear and new songs play. This release integrates that strategy, while recovery paths are validated using simulated devices and injected faults. Physical USB reliability is not established by those tests.
- Tests cover transient and persistent errors, partial files, renamed mount paths, wrong-device refusal, checksum mismatches, frozen artwork, changed sources, journal corruption, interruption at every commit boundary and queue preservation on application reconstruction.
- Resume is at verified file boundaries. An incomplete current file may be quarantined and recopied. Automatic CHKDSK, formatting, firmware changes and unbounded write retries are not performed.

## [0.2.2] - 2026-10-18

### Fixed

- Start transfer no longer stays disabled in normally launched desktop installations. A connected device and a nonempty queue now open the transfer confirmation dialog. Confirm the experimental transfer there, then start the backup and transfer without a startup flag.
- The backend accepts an explicit boolean confirmation for one transfer request. Missing, false or string-valued confirmations remain blocked before a job starts. Confirmation does not persist or unlock subsequent transfers.
- Transfer confirmation resets when the dialog closes. English, Chinese and Japanese instructions explain the available workflow. Backups, file verification, target validation and compatible I/O remain in place. Full physical A918 transfer reliability remains unverified.

## [0.2.1] - 2026-10-18

The compatible I/O profile is experimental. Device writes remain disabled by default; physical A918 reliability improvement has not been established.

### Changed

- Experimental transfers default to a conservative device I/O profile: 32 KiB application writes, a 1 MiB/s application-level cap, paced readback and a 100 ms settling interval after each file's final flush. `--device-io standard` retains the previous copy strategy for controlled comparisons. Physical reliability improvement has not been established.
- Device enumeration, device artwork reads and application file browsing are blocked during active jobs to reduce competing I/O. Media I/O failures stop immediately without automatic rewrite retries.

### Added

- Deterministic pacing, short-write and fail-fast tests, no-truncation protection for existing staging files, and virtual-filesystem coverage of compatible copy and busy-device artwork requests.

## [0.2.0] - 2026-10-18

This desktop preview keeps device writes disabled by default while full A918 transfer validation remains in progress.

### Added

- Independent Tauri 2 desktop windows and native music, folder and artwork dialogs.
- Official Animal Island UI artwork in executable, window and installer icons, with attribution.
- Windows setup with optional Start Menu and desktop shortcuts, Debian DEB/AppImage and macOS DMG packages.
- Three-language update checking, release notes, progress, signature verification and installation from this repository's GitHub Releases.
- Signed updater artifacts and a complete platform feed published after every platform build succeeds.
- Native desktop IPC and close-to-exit checks, including installed Windows and Debian packages.

### Changed

- Normal startup opens a desktop window. The optional browser service requires `--headless`.
- Repeated launches focus the running instance; closing the last window exits the application.
- Desktop language preference persists independently of the internal service port.

### Fixed

- The desktop no longer competes for the earlier browser preview's default port.

## [0.1.0] - Development preview

### Added

- CI builds, packages, checksums and packaged-application smoke tests for Windows, Debian and macOS Apple Silicon/Intel.
- English, Simplified Chinese and Japanese UI; English by default, with a persistent language picker.
- Translation completeness and placeholder checks during frontend builds.
- English documentation and dependency/font notices.
- Per-user data directories, macOS player detection and browser launching.
- Recovery journals, per-file flushing and destination paths in copy errors.
- Preflight protection for existing or malformed device playlists.
- Full staged-transfer and append tests using a virtual player, with copy interruption, external modification, commit failure and rollback failure scenarios.
- Low-space staging copies only changed files and reuses unchanged files by same-volume moves, with a complete computer backup and interrupted-commit checks.

### Changed

- Replaced the exploratory database writer with declarative sections, records and views for MIT distribution.
- Artwork uses the firmware-selected thumbnail marker and TIT2/PICP/PIC0/XSOT hierarchy.
- Existing artwork is indexed once per read and retained during index rebuilding.

### Fixed

- Desktop startup selects a free loopback port when the default is occupied, preventing an immediate exit when an earlier preview is running.

- The controlled A918 artwork probe now selects its tree with `STD_TPE1_THUMBNAIL`.
- Albums can use a cover supplied by a later song.
- OMA readback preserves track totals and disc numbers/totals.
- Full Unicode OMA tags round-trip without changing source or audio bytes.
- Artwork readback follows stable track/group IDs from the selected thumbnail tree, preserving covers when artist or album display labels exceed the index text limit.
- Global SYSB serialization uses the accepted A918 profile word and derives its compact span from the actual row data; undocumented flag semantics remain explicitly unclaimed.
- Commit failures now report failed rollback explicitly and retain recovery locations in the journal instead of discarding restoration errors.
- Device-root validation resolves aliases before checks and recognizes Windows verbatim drive prefixes and non-C system drives.

### Known issues

- Full native transfers remain unverified: Windows device I/O error 1117 interrupted two attempts before commit.
- macOS apps are ad-hoc signed, not notarized.
- Playlist editing, device deletion, transcoding and source-file tag writing remain unimplemented.

## [0.1.0] - 2026-10-17

### Added

- Initial Rust local service with React and Animal Island UI.
- MP3 import, search, sorting, pagination, albums/artists, queueing and local playback.
- ID3 v1/v1.1/v2.2/v2.3/v2.4 support and catalog tag/artwork editing.
- Local OMA and database preparation, readback checks and SHA-256 manifests.
- Windows/Linux tests, Debian 12 local validation and Sony binary research through REA.

This milestone does not imply a published binary release or completed hardware validation.
