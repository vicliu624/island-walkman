# Island Walkman

A native desktop music library and transfer application for Sony NW-A918, built with Tauri 2, Rust, React and Animal Island UI. No Java runtime or cloud music service is required.

**Development preview:** library management and local preparation work. A controlled A918 artwork probe succeeded. Full native transfers remain under investigation after reproducible device I/O errors. Device writes are disabled by default. See [hardware validation](docs/research/native-transfer-validation.md).

## Features

- Import MP3 files and folders; search, sort and browse albums or artists.
- Read ID3 v1/v1.1/v2.2/v2.3/v2.4, Unicode and multivalue tags, album artist, track/disc totals and year.
- Edit library metadata without modifying the original MP3. Full text is retained in OMA tags while indexes respect device limits.
- Extract front-cover artwork, replace it locally and prepare Sony JPEG descriptors and artwork indexes.
- Preview source MP3 audio in the application; queue songs and prepare a local library without a connected player.
- Detect mounted players and read existing songs and artwork.
- Stage transfers with host backups, per-file flush/checks, SHA-256 manifests and a recovery journal.
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
--device-io compatible    Conservative experimental device copy (v0.2.1 and later)
--device-io standard      Previous copy strategy for controlled comparisons
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

In v0.2.2 and later, choose **Start transfer**, acknowledge the experimental transfer in the confirmation dialog, then choose **Back up and transfer**. No command-line flag is needed. Confirmation applies only to that request and resets when the dialog closes. The explicit `--allow-experimental-device-write` development flag remains available. Neither path is a compatibility guarantee. The available player currently produces lower-level I/O errors during full transfers. Passing unit tests and the separate successful artwork probe do not establish full native transfer support.

Version 0.2.1 adds a default compatible copy profile: 32 KiB application writes, at most 1 MiB/s of application-issued payload, one final flush per file, a 100 ms settling interval and paced SHA-256 readback. It fails on a device I/O error rather than automatically replaying writes; existing staging files are never truncated in compatible mode. Background device enumeration and device artwork reads are blocked during jobs. These controls do not change USB packet sizes or bypass the host controller. They remain unverified on the physical A918. Unconfirmed device writes remain blocked by default; the desktop confirmation dialog authorizes one experimental transfer.

Before commit the program stages and hashes new files, verifies the original library and retains a complete host backup. A host `transaction.json` records paths, manifests and commit phase. An incomplete `ISLAND-NEW-*` folder does not mean a transfer committed.

When enough space is available, the player receives a complete staged library and `ISLAND-BACKUP-*` retains the previous library. With less free space, only new/changed files are copied; unchanged files are moved on the same volume during commit. The journal records `mode: reuse_unchanged` and the reuse paths. In this mode the device backup is **partial**, and the verified computer backup is the complete recovery source. An interruption can leave files split between incoming and backup directories; do not delete either until recovery is resolved. Unfinished commits block a new transfer instead of being silently retried.

Space estimates reserve up to 64 KiB per copied file and destination directory, plus a 1 MiB margin. Reused files still require destination directory entries. Host backups contain the full library, so the computer also needs sufficient disk space. Low-space commit and rollback are covered by simulated filesystem tests; physical disconnect and FAT32 power-loss behavior remain unverified.

Safely eject the player and check playback after a successful transfer. On I/O errors, stop writing and inspect the connection and filesystem. Automatic filesystem repair, formatting and firmware changes are outside the application.

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
