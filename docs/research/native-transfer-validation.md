# Native transfer validation

2026-10-17, Windows 11, connected Sony NW-A918 on F:.

The third artwork probe succeeded according to the user's physical test: artwork displays and playback is normal. This validates the thumbnail tree marker and the tested four-field hierarchy combination. See `firmware/artwork-reader-chain.md`.

The updated Rust program passed 15 tests and locally prepared the purchased Hey Ya! source with OMA audio, JPEGs, four-field 03GINF2D and the thumbnail marker. The original source is unchanged. The real transfer API was then invoked with artwork enabled and the explicit experimental writer flag, retaining the existing track and adding a new native test copy.

The full transfer stopped during copying to `F:/ISLAND-NEW-1792249546891583800`, before the directory commit. Windows returned error 1117 (I/O device error). The System event log records disk event 153 at 23:06:07, retrying LBA 0x21e on disk 2. The new temporary 10F00 directory enumerated invalid names and impossible multi-gigabyte lengths. This is evidence of a storage/transport/filesystem fault; it does not isolate cable, USB port, flash or firmware as the cause.

The accepted `F:/OMGAUDIO` was not renamed or replaced. Its existing audio SHA-256 matches the verified computer backup: `9c17090711946e1d535e14ddce28135791491304a5dd384bccbbffa5bc8755`. The complete host-side backup and candidate stage are retained at `data/rust-device-validation/backups/1792249545165660200/`. The device still has one original song. No retry, deletion, format or filesystem repair was performed after the fault.

The user was asked to reconnect through another direct USB port or cable before a read-only recheck. Full native device transfer and Debian USB transfer remain unverified; the application write gate remains enabled by default. Copying now explicitly flushes each file and reports the exact destination path on errors, but that code change is not a claim to repair the observed media fault.

## Repair and second native attempt

The user changed the direct USB port and reconnected. Read-only CHKDSK identified an unrecoverable folder in the failed staging directory's 10F00. The user explicitly authorized `CHKDSK F: /F`. It converted that damaged temporary folder to a file and reported filesystem corrections. A subsequent read-only pass reported no problems. The original song and 00GTRLST/02TREINF/03GINF2D/04CNTINF SHA-256 values all remained identical to the verified backup.

The current Rust release (17 tests, streamed per-file copy, sync_all, transaction journal) was used for a second native transfer. Backup/stage/journal: `data/rust-device-validation-2/backups/1792249955325762700/`. The transfer again failed before directory commit, this time while copying `ISLAND-NEW-1792249955925706500/01TREE13.DAT`, error 1117. System disk event 153 at 23:12:56 records an I/O retry at LBA 0x2414 on disk 2. After this failure, read-only CHKDSK reported no filesystem problems and the five original file hashes again matched the backup. Some read-only scans warn that the volume is in use; their structural findings are not a hardware health guarantee.

This is a reproduced lower-level storage/transport failure after filesystem repair and a USB-port change. It does not isolate the data cable, connector, device flash or firmware. Further device writes are stopped pending a different WM-PORT cable or another diagnostic change. No format or repeated repair was performed. The accepted library still contains its original single playable song and verified artwork.
