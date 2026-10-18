# Changelog

All notable changes are recorded here. Versions follow Semantic Versioning. Unreleased entries describe development state, not a published release.

## [Unreleased]

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
