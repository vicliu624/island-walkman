# Island Walkman

A native desktop music library and transfer application for Sony NW-A918, built with Tauri 2, Rust, React and Animal Island UI. No Java runtime or cloud music service is required.

**Development preview:** an eight-song append was confirmed on a physical A918, bringing its library to nine playable songs. Version 0.3.0 integrates that append strategy with durable recovery. Restart and reconnection paths are tested on simulated devices; the player's intermittent USB/storage fault has not been isolated or eliminated. See [the physical transfer record](docs/research/single-song-transfer-2026-10-18.md) and [recovery behavior](docs/TRANSFER_RECOVERY.md).

## Features

- Import MP3 files and folders; search, sort and browse albums or artists.
- Read ID3 v1/v1.1/v2.2/v2.3/v2.4, Unicode and multivalue tags, album artist, track/disc totals and year.
- Edit library metadata without modifying the original MP3. Full text is retained in OMA tags while indexes respect device limits.
- Extract front-cover artwork, replace it locally and prepare Sony JPEG descriptors and artwork indexes.
- Preview source MP3 audio in the application; queue songs and prepare a local library without a connected player.
- Detect mounted players and read existing songs and artwork.
- Append without rewriting existing audio; preserve complete computer backups, short staging names, per-file SHA-256 verification and two-generation recovery journals.
- Pause, resume verified files, recover interrupted commits and automatically reconnect to the same identified player.
- English, Simplified Chinese and Japanese interface. **English is the default**, independent of OS language. The language picker saves your choice locally. Song metadata is never translated.

## Installation

The **Build and package** GitHub Actions workflow produces these artifacts on pushes, pull requests and manual runs:

| System | Architecture | Package |
| --- | --- | --- |
| Windows 11 | x86-64 | NSIS setup EXE and signed updater |
| Debian 12 or newer | x86-64 | DEB and AppImage |
| macOS 14 or newer | Apple Silicon | DMG and signed APP archive |
| macOS 15 or newer | Intel | DMG and signed APP archive |

Download installers from [GitHub Releases](https://github.com/vicliu624/island-walkman/releases).

Windows: run the setup EXE. Installation is per-user. The Start Menu page offers an option to skip shortcuts; the final page independently offers a desktop shortcut checkbox. Updates preserve existing shortcuts. WebView2 is installed if missing.

Debian: install the DEB using `sudo apt install ./<downloaded-package>.deb`, or make the AppImage executable and run it. The desktop uses WebKitGTK 4.1 and GTK 3. Both package types have signed update assets. DEB installation uses the system package manager and requires administrator authorization.

macOS: open the DMG and copy Island Walkman to Applications. Developer ID signing and Apple notarization credentials are not configured.

Normal startup opens an independent desktop window. Closing the last window exits the process; repeated launches focus the existing window. A loopback-only service runs inside the same process on an OS-selected port. Native permissions are restricted to that window and its exact origin. No external browser is opened.

### Updates

Open **Settings & help > Software updates > Check for updates**. Review the available release and select **Download and install**. The application displays progress, verifies the signature, installs the update and restarts. Finish active transfers first. Checking alone never installs an update.

The feed is [latest.json from this repository's Releases](https://github.com/vicliu624/island-walkman/releases/latest/download/latest.json). No GitHub token is needed. See [desktop distribution](docs/DISTRIBUTION.md) for publishing, keys and platform limitations.

## Data and configuration

| System | Default data directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%/IslandWalkman` |
| Debian | `$XDG_DATA_HOME/island-walkman`, or `~/.local/share/island-walkman` |
| macOS | `~/Library/Application Support/IslandWalkman` |

Use `--data <directory>` to select another library. Data and backups live outside installed application files. Earlier development builds used a project-local `data/`; pass that path explicitly to reuse an earlier library.

```text
--headless                Run the optional local HTTP interface
--no-open                 Do not open a browser in headless mode
--port 18719              Use another local port
--data <directory>        Choose the application data directory
--import <file-or-folder> Import local MP3 files at startup
--prepare                 Generate and verify a local library, then exit
```

Example without a connected player:

```sh
island-walkman --data ./my-library --import ./music --prepare --no-open
```

Preparation writes to `<data>/preparations/<run>/`. It does not write to a player.

## Device transfers and recovery

Connect the A918 in USB storage mode. Typical roots are `F:/`, `/media/user/WALKMAN` and `/Volumes/WALKMAN`. Detect a mounted player or enter its root manually. The process needs filesystem write permission; no custom USB driver is installed.

Device roots are resolved before validation. Windows targets must use a local drive letter and cannot be on the current system drive, including verbatim path forms. Unix filesystem-root aliases are rejected. These checks apply to player targets; source music and application data can remain on the system drive.

Choose **Start transfer**, acknowledge the experimental operation and choose **Back up and transfer**. The saved task retains that authorization, so **Resume transfer** does not ask you to select or confirm its songs again. A new task still requires its own confirmation. The explicit `--allow-experimental-device-write` development flag remains available.

Version 0.3.0 copies new audio one file at a time using ordinary Windows CopyFileW, or flushed buffered copies on Debian/macOS. It waits after copies and checks their SHA-256 hashes. Existing audio is retained and moved by directory during a short final transaction. New indexes and artwork are staged before commit; the player is never formatted or repaired automatically. The older `--device-io` profiles are retained for diagnostic compatibility and do not select the normal resumable engine.

Progress is saved under `<data>/transfers/<session-id>/`, including a complete `before/OMGAUDIO` backup, prepared files, frozen artwork, and two checksummed journal generations. Verified files are rechecked and reused after interruption. Resume is **file-level**: an incomplete current file can be quarantined and recopied, while complete verified songs are not transferred again. Pausing takes effect after the current file; an active commit finishes or retains a recoverable intent.

Transient transport errors receive at most three automatic retries with waits of 2, 5 and 10 seconds. Each attempt opens fresh handles and checks the expected device identity. A missing device waits for reconnection; the idle monitor resumes when the matching player returns. Persistent I/O errors while it remains connected, checksum mismatches, changed imported sources, unknown library changes and damaged journals require review. Use **Retry after checking** after addressing the reported problem. A manually paused or review-required task never restarts automatically.

Unexpected application termination resumes an authorized saved task on startup when the identified player is available. A committed task updates the desktop catalog before its journal is acknowledged, so a restart cannot append it again. Later queue additions are retained for the next task. Source MP3s are not modified; edits and artwork choices are frozen for each saved task.

The player stores `IWIDENT.DAT` as a random identity marker; Windows also checks its volume serial. A drive letter alone does not authorize recovery. Short `IWxxxxxx`, `IBxxxxxx` and `IQxxxxxx` directories hold incoming data, retained metadata and quarantined copies. The on-player backup can be **partial** after audio directories move; the computer backup is the complete recovery source. Retain recovery directories until the task is resolved. Previous 0.2.x journals are not silently converted into new recovery sessions.

Only the observed system/history-table set can be treated as player normalization, and only when all other file hashes and the original song, audio and artwork identities still match. This regenerates the candidate from current device state. Unknown changes stop the commit instead of being overwritten.

Space checks account for new audio, staged metadata/artwork, destination directories and a reserve. Existing audio does not need a second device copy. The computer needs space for a complete backup and prepared library. Safe removal and physical playback verification remain necessary after successful transfer. See [the detailed state machine and limits](docs/TRANSFER_RECOVERY.md).

## Build from source

Prerequisites: Rust **1.95.0**, Node.js **24**, Python **3.13**, and the [Tauri platform prerequisites](https://v2.tauri.app/start/prerequisites/). Linux desktop builds require WebKitGTK 4.1, GTK 3, librsvg and patchelf development packages.

```sh
cd ui
npm ci
npm run build
cd ..
python tools/collect-notices.py
cargo test --locked
cargo run --locked
```

Use `build.ps1` on Windows or `./build.sh` on Linux/macOS to produce desktop installers. `npm run dev` is available for frontend development.

The CI matrix builds the frontend and native installers, runs Rust tests, and validates the rendered desktop and IPC. Windows and Debian additionally install and test their packages. Windows tests both shortcut modes and their preservation during updates. Synthetic audio verifies source preservation and local preparation. No personal music or hardware is used in CI.

Legacy headless archive packaging, retained for protocol research:

```sh
python tools/package.py --platform windows --arch x86_64 --binary target/release/island-walkman.exe
python tools/smoke.py --executable dist/stage/island-walkman-0.2.0-windows-x86_64/island-walkman.exe
```

Desktop distribution uses Tauri packaging instead. Pushing a version tag publishes a GitHub Release only after all four platform builds pass, including signed updater artifacts and a complete feed. See the distribution guide for local unsigned builds and key configuration.

## Current limits

- MP3 import only; automatic FLAC/WAV/AAC transcoding is not implemented.
- Edits affect the application library and future transfers, not source-file ID3 tags.
- Device deletion and playlist editing are not implemented. Existing playlists prevent unsupported index rebuilding.
- Full native A918 transfers, power-loss recovery and Debian/macOS USB behavior remain unverified.
- Windows installers are not Authenticode-signed; macOS applications are not Developer ID signed or notarized. Updater signatures are separate.
- DEB updates require administrator authorization through the system package manager; physical Linux update installation remains to be verified.
- Protocol coverage is limited to recovered and tested cases; see [protocol notes](docs/PROTOCOL.md).

## Documentation and license

- [Changelog](CHANGELOG.md)
- [Protocol and provenance](docs/PROTOCOL.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)
- [Frontend](ui/README.md)
- [Sony research](docs/research/sony-inventory.md)
- [Firmware reader chain](docs/research/firmware/artwork-reader-chain.md)
- [Native transfer validation](docs/research/native-transfer-validation.md)

Application code is [MIT licensed](LICENSE). Dependencies retain their licenses. Sony software, decompiled listings, user music, artwork and backups are not distributed. This project is independent and is not endorsed by Sony.

## Firmware research preview (unreleased)

A read-only Device firmware card inspects native USB identity and local A91x UPG
containers. Windows, Debian and macOS share the Rust implementation. No firmware
write, reset, arbitrary vendor-command sender or Sony updater launch is enabled.
An explicit Windows developer command supports only fixed, serial-bound data-in
information queries; it is not called by the UI or background scanning. Offline
identity/version/power decoders and a fail-closed transaction model are included.
Live identity, firmware and battery responses remain unmeasured in this stage. See [Firmware research](docs/FIRMWARE_RESEARCH.md)
for evidence, validation limits and the remaining work toward a native updater.

See [Protocol phase 2](docs/FIRMWARE_PROTOCOL_PHASE2.md) for the response layouts,
read-only diagnostic boundary, modeled fault tests and unresolved resident-loader
recovery contract. Linux/macOS storage-query adapters and all firmware activation
remain unavailable.
