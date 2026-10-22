# Changelog

All notable changes are recorded here. Versions follow Semantic Versioning. Unreleased entries describe development state, not a published release.

## [0.4.0-alpha.8] - 2026-10-22

Public prerelease. A918 music-transfer I/O errors remain unresolved; this is not a verified scan-and-repair fix. Firmware flashing remains unavailable.

### Added

- Application library with bundled Tetris and color iD emulator, local publisher catalogs, validated installation and save-preserving updates.
- Read-only device-storage browser and honest capacity reporting for the exposed data volume; hidden Linux partitions remain unavailable.
- Version consistency checks across native/backend/frontend packages and prerelease publishing without replacing the stable updater feed.

### Fixed

- Include the alpha.7 transfer error-handling improvements: stop automatic storage-error retries and retain the journal for review.
- Bundle the CFW alpha.3 application executables and matching sources. The English CFW launcher and apps have a successful A918 user test report; this is not desktop transfer qualification.

### Distribution

- Exclude commercial ROMs and initialization saves from the built-in catalog, executable and public source. Color iD requires compatible user-supplied `rom.bin` and `new.sav` in its device application directory.
- Retain earlier private candidates locally. During later app-copy testing, storage errors also occurred outside the music path before a retry succeeded; app installation is not a guarantee against the storage issue.

## [0.4.0-alpha.7] - 2026-10-21

Local safety candidate; the A918 write-timeout/repair-prompt root cause is not resolved or hardware-qualified.

### Fixed

- Stop the resumable transfer on the first reported storage I/O/CRC/read/write/timeout/corruption failure instead of automatically retrying it as a connection interruption. Retain the journal and require explicit review, including when the volume also disappears.
- Avoid additional device-resolution reads in the storage-failure handler. Genuine connection-interruption recovery remains separate.
- Cover first-copy failure, commit failure/recovery and context-wrapped error classification with fault-injection regressions. Tests model connection interruptions separately from storage failures.

### Investigation

- The installed color-app package includes a 4MiB ROM and totals 5,139,454 bytes. Successful app installation is therefore a meaningful control against a simple file-size explanation; it does not isolate write granularity, metadata operations, placement, or commit behavior.
- Keep application installation behavior unchanged while comparing it with music transfer. No new firmware or physical-device writes are part of this change.

## [0.4.0-alpha.6] - 2026-10-20

Local diagnostic candidate; no publication or verified A918 transfer fix.

### Fixed

- Pass the selected device-I/O profile into the active resumable transfer path. Compatible mode now uses paced 32KiB writes, explicit file synchronization and readback checks instead of bypassing that mode through CopyFileW.
- Explicitly synchronize files copied by the Windows standard profile before transaction verification. This is a file-level barrier, not automatic volume ejection or proof of power-loss safety.

### Investigation

- The preserved 0.4.0-alpha.1 executable's real transfer stopped at 23/72 files with Windows I/O error 1117 and a readback mismatch, before library commit. A separate CopyFileW-only replay into an isolated folder reproduced error 1117 on a freshly formatted A918 volume. Hardware compatibility of this candidate remains unverified.

## [Unreleased]

### Security review

- Complete all 758 records in the bounded Sony firmware/recovery/backup catalog query, proving coverage of the remaining 558 with distinct identifiers and saved-response hashes. Review ambiguous item file catalogs and reject other-model/text false positives. Add offline pagination-integrity regression tests; resident firmware and persistent flag-clear callers remain unrecovered, and flashing stays unavailable.

- Retain a bounded A910 resident-image candidate search ledger covering public catalogs, historical Sony downloads, additional official source archives and local image triage. Exclude application discs, other-model material and generated rootfs capacity fixtures from factory-image evidence; the resident installer, fwpup and persistent flag-clear caller remain unrecovered.

- Complete the offline original 1.02/1.03 rootfs comparison; bind wrldr resident-image table construction and nvpflag FUP access to pinned ELF bytes. Record table-search and control-block failure limits without treating wrldr as fwpup or claiming recovery safety. Add synthetic archive-reader regression coverage; no device access or flashing route is introduced.

- Recover Sony's explicitly shared A916/A918/A919 U-Boot and kernel source distributions; retain verified archive provenance and source-level boot-selection/failure evidence. Distinguish the RAM BoOt clear from persistent NVP fWUp clearing. Resident images and fwpup remain missing; no device access or firmware execution is added.

## [0.4.0-alpha.5] - 2026-10-19

Local development candidate; no hardware flashing or release publication.

### Fixed

- Restore the shared A916/A918/A919 firmware profile interface required by the existing decoder/session/inspection modules after an inconsistent device configuration prevented compilation. No firmware executor is added.

### Security review

- Reconstruct device-side NVP flag persistence, normal initrd/startup contents and NAND error boundaries from pinned local inputs; add static inventories, 21 device-side tests and 55 offline power-loss/failure scenarios. Boot-time flag consumption/clearing, resident installer and fwpup erase/program/verify order remain unverified; native flashing stays unavailable.

### Security review

- Extend offline FWUpdater analysis with hash-pinned COM interface binding, configured MSC copy/activation and cleanup routes, exact timer/error branches, static version-prompt resources, 23 symbolic replay tests and 28 retained fault traces. No application firmware executor or live device access is added.

- Audit firmware updater parity against pinned Sony binaries and stock/R14 components; retain a no-flash verdict and explicit implementation/recovery gaps. Add authenticated HTTP regression coverage proving music-write authorization and force fields cannot expose firmware mutation routes. No firmware executor or activation capability is added.

## [0.4.0-alpha.4] - 2026-10-19

Local development candidate; hardware validation remains pending.

### Added

- Reserve and decode IWnn custom firmware subrevision declarations separately from Sony base versions in capture/Windows diagnostic reports. Legacy 1.03/00 remains unidentified; no exact build, launcher ABI or authenticity is inferred. Firmware-side emission and live app-install identity gating remain pending; existing firmware packages are unchanged.


## [0.4.0-alpha.3] - 2026-10-19

Local development candidate; hardware validation remains pending.

### Fixed

- Model A916/A918/A919 as sharing one firmware profile and package signature. Separate fixture provenance from compatibility; accept all three in offline firmware sessions while retaining exact device identity binding.


## [0.4.0-alpha.2] - 2026-10-19

Local development candidate; hardware validation remains pending.

### Changed

- Centralize device-family discovery and per-model offline firmware validation in device profiles; remove fixed A918 branding from the device UI. USB candidates never imply exact model verification.
- Serialize music transfers and firmware device diagnostics with a shared host lock, including separate application instances; future flash executors must retain this guard through reconnect/readback. Live firmware writing remains disabled.


## [0.4.0-alpha.1] - 2026-10-19

Local prerelease only; not published to GitHub or the stable updater feed. Adds application/storage features to 0.3.0; actual-device app installation and R14 firmware remain unverified.

### Added

- Synchronized application SemVer, build/tag consistency gates and metadata-driven UI version labels. Prereleases cannot become the stable GitHub latest release.
- Add local and publisher-directory disk-app catalogs, staged/hash-verified installs that retain saves and previous app folders, and dynamic R14 launcher compatibility checks.
- Add a read-only device directory browser and live data-volume space overview; inaccessible Linux partitions remain explicitly unknown.

- Local installation/recovery evidence and host-only pipeline fault tests that demonstrate a potentially masked producer failure in the stock script's ordinary pipeline semantics. Resident recovery and hardware power-loss behavior remain unverified; firmware activation stays disabled.

- Strict MSC device-information and power-response decoders, an explicit Windows read-only diagnostic restricted to serial-bound fixed data-in queries, and a safety-report command/API. No firmware write or activation capability is added.
- Offline firmware transaction/checkpoint modeling with ambiguity review, no post-intent automatic resend, exact identity/version readback, same-version/downgrade refusal and malformed-frame/driver-response/checkpoint fault tests.

- Read-only firmware research card with English, Chinese and Japanese UI, native USB inventory on Windows/Linux/macOS and bounded local A91x UPG container validation.
- Firmware CLI inspection that exits before catalog loading and transfer recovery, plus independent synthetic-container smoke coverage in all four CI targets.
- REA/Ghidra evidence for the original updater and an offline device-information command specification cross-checked against the device kernel. No vendor-command execution or firmware flashing is enabled.

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
