# Island Walkman

A local music library and transfer application for Sony NW-A918, built with Rust, React and Animal Island UI. No Java runtime or cloud music service is required.

**Development preview:** library management and local preparation work. A controlled A918 artwork probe succeeded. Full native transfers remain under investigation after reproducible device I/O errors. Device writes are disabled by default. See [hardware validation](docs/research/native-transfer-validation.md).

## Features

- Import MP3 files and folders; search, sort and browse albums or artists.
- Read ID3 v1/v1.1/v2.2/v2.3/v2.4, Unicode and multivalue tags, album artist, track/disc totals and year.
- Edit library metadata without modifying the original MP3. Full text is retained in OMA tags while indexes respect device limits.
- Extract front-cover artwork, replace it locally and prepare Sony JPEG descriptors and artwork indexes.
- Preview source MP3 audio in the browser; queue songs and prepare a local library without a connected player.
- Detect mounted players and read existing songs and artwork.
- Stage transfers with host backups, per-file flush/checks, SHA-256 manifests and a recovery journal.
- English, Simplified Chinese and Japanese interface. **English is the default**, independent of OS language. The language picker saves your choice locally. Song metadata is never translated.

## Installation

The **Build and package** GitHub Actions workflow produces these artifacts on pushes, pull requests and manual runs:

| System | Architecture | Package |
| --- | --- | --- |
| Windows 11 | x86-64 | Portable ZIP |
| Debian 12 or newer | x86-64 | DEB and static portable TAR.GZ |
| macOS 14 or newer | Apple Silicon | TAR.GZ containing Island Walkman.app |
| macOS 15 or newer | Intel | TAR.GZ containing Island Walkman.app |

Windows: extract the complete archive and open `island-walkman.exe`. Keep `ui/` beside it.

Debian: install with `sudo apt install ./island-walkman-*.deb`, then launch Island Walkman from the applications menu or run `island-walkman`. The portable archive can also be extracted and run directly. `xdg-utils` opens the browser.

macOS: extract the archive and copy `Island Walkman.app` to Applications. CI apps are ad-hoc signed, not Apple-notarized; macOS may require approval through its normal security settings. Developer ID signing and notarization credentials are not configured. The app launches a local service and opens the system browser.

The service binds only to `127.0.0.1`, normally port `18718`. Keep the application running while using its browser tab. Do not expose the local service through a proxy.

## Data and configuration

| System | Default data directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%/IslandWalkman` |
| Debian | `$XDG_DATA_HOME/island-walkman`, or `~/.local/share/island-walkman` |
| macOS | `~/Library/Application Support/IslandWalkman` |

Use `--data <directory>` to select another library. Data and backups live outside installed application files. Earlier development builds used a project-local `data/`; pass that path explicitly to reuse an earlier library.

```text
--no-open                 Do not open the browser automatically
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

Experimental writes require `--allow-experimental-device-write`. This development gate is not a compatibility guarantee. The available player currently produces lower-level I/O errors during full transfers. Passing unit tests and the separate successful artwork probe do not establish full native transfer support.

Before commit the program stages and hashes new files, verifies the original library and retains a host backup. A host `transaction.json` records paths, manifests and commit phase. After success, `ISLAND-BACKUP-*` retains the previous device library. An incomplete `ISLAND-NEW-*` folder does not mean a transfer committed.

Safely eject the player and check playback after a successful transfer. On I/O errors, stop writing and inspect the connection and filesystem. Automatic filesystem repair, formatting and firmware changes are outside the application.

## Build from source

Prerequisites: Rust **1.95.0**, Node.js **24**, Python **3.13** for packaging and the platform Rust linker. Linux static builds need `musl-tools`; Debian packages need `dpkg-deb`. macOS packaging uses `codesign` from developer command-line tools.

```sh
cd ui
npm ci
npm run build
cd ..
cargo test --locked
cargo build --release --locked
```

Run `cargo run --locked -- --no-open` for development after building `ui/dist`. `npm run dev` is available for frontend development.

The CI matrix builds the frontend, tests native Rust targets, creates packages and runs a smoke test against the **packaged executable**. Synthetic audio verifies source preservation, local preparation, the API and packaged UI assets. No personal music or hardware is used in CI.

Manual Windows packaging:

```sh
python tools/package.py --platform windows --arch x86_64 --binary target/release/island-walkman.exe
python tools/smoke.py --executable dist/stage/island-walkman-0.1.0-windows-x86_64/island-walkman.exe
```

Use `--platform debian` or `macos` on that OS. Packaging refuses to overwrite an existing stage. SHA-256 files accompany archives. CI does not publish a GitHub Release automatically.

## Current limits

- MP3 import only; automatic FLAC/WAV/AAC transcoding is not implemented.
- Edits affect the application library and future transfers, not source-file ID3 tags.
- Device deletion and playlist editing are not implemented. Existing playlists prevent unsupported index rebuilding.
- Full native A918 transfers, power-loss recovery and Debian/macOS USB behavior remain unverified.
- The system browser supplies the UI; the macOS app is a service launcher.
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
