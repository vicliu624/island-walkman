# Disk applications and storage browser

This prerelease adds an application library and a read-only storage browser. It does not flash firmware or provide a Linux remote shell. Apps require the separate R14 disk-app launcher ABI 1 firmware. R13 does not support executable disk apps. The English CFW alpha.3 launcher and applications have a successful NW-A918 user report. This does not establish transfer or save durability.

## Browse and install

Open **Application library** to browse bundled local candidates: Tetris and color Tamagotchi iD. Connect a mounted WALKMAN using the existing device controls. Select **Install app**. Installation runs as an exclusive task, validates the known A918 directory structure, checks free space and each payload SHA-256, stages files and reads them back before directory commit. Updates copy existing app data, including saves, into the new version and retain the old directory under `APPS/.backup-<id>-<timestamp>`. Backup and incomplete staging directories are excluded from launcher discovery. No OMGAUDIO, NVP, root partition, format or raw-device operation is involved.

An interrupted `APPS/.install-<id>` blocks another installation rather than silently deleting data. A power failure during the directory swap can leave a backup without a live app; inspect the retained directories before restoring. FAT writes are not claimed to be power-loss atomic. Installation authorization applies only to the explicitly selected app, not firmware flashing. Safely eject the player when complete.

**Open published app repository** loads a local publisher directory containing `catalog.json` and `<app-id>/` payload directories. A repository can add apps without rebuilding Island Walkman. The path field and refresh control also work in the headless interface. The bundled catalog is available offline. This implementation does not download an Internet catalog or publish to GitHub; the public packages exclude commercial game inputs.

Catalog schema 1 declares `required_firmware` and `apps`, each with id, name, version, api=1, target=`sony-a918-armv5-oabi`, status, description, and files with path/bytes/sha256. Application files are bounded and flat; paths and reparse points/symlinks are rejected. A valid `app.ini` and matching static ARM32 `app` executable are required. The tool checks app size, CRC32, manifest consistency and target before writes. Catalog hashes detect corruption, not author identity. Only install apps from trusted authors: these programs are not sandboxed and execute with the firmware's privileges.

## Storage overview

Capacity is queried from the mounted data filesystem, not the firmware image or disk size estimates. Used, available and total space are shown. The read-only directory browser navigates the exposed volume, displays file sizes and blocks path traversal and symbolic/reparse links. It does not edit, execute or delete files.

Normal A918 USB storage exposes the data volume only. Linux root, kernel, recovery and settings partitions are explicitly shown as unavailable with no fabricated size/usage. Access to those would require a separately established and tested device-side communication service; this build does not add one.

## Locations and migration

- `WALKMAN:/APPS/tetris/app.ini`, `app`
- `WALKMAN:/APPS/tamagotchi-id/app.ini`, `app`, `rom.bin`, `new.sav`
- iD creates `state.sav`, `state.bak`, and temporary `state.tmp` in its app directory.

Commercial ROM and reset seed files are not bundled. Supply your compatible `rom.bin` and `new.sav` separately in `APPS/tamagotchi-id/` after installing the emulator. Existing R13 saves in `GAMES/TAMAGOTCHI/ID/` are not moved automatically. After backing them up, copy `state.sav` and optionally `state.bak` into `APPS/tamagotchi-id/`. Keep the old directory until the new app has been verified.

## Validation

Rust tests cover bundled hashes, external repositories including a new app ID, staged update/save preservation, retained backups, corrupt payload rejection, traversal and incomplete-install protection. Desktop smoke checks both catalog cards, disabled installation without a device and storage browser rendering. Host filesystem tests are not actual-player FAT/USB/power-loss validation.
