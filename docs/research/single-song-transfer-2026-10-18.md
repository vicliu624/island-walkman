# Single-song transfer diagnostic

The user explicitly authorized transferring the eight queued songs, one at a time if necessary. The normally installed v0.2.2 app's previous batch job had failed with Windows error 1117 while copying the old Hey Ya! OMA into a new staging directory. The accepted OMGAUDIO had not been committed or replaced.

Before the new attempt, all 43 accepted-library files matched the complete host backup from the failed batch job. All eight source MP3 hashes matched their catalog identities. Read-only CHKDSK reported no structural problems. The original application queue was snapshotted and left unchanged; an isolated host catalog contained only the smallest queued song, Adele's Make You Feel My Love (Live), 3,725,834 source bytes.

The local diagnostic build used `--device-io legacy --reuse-unchanged`: 256 KiB/s application pacing, Windows FILE_FLAG_WRITE_THROUGH, readback hashing and the existing transaction/rollback path. Unchanged files were reused rather than rewritten. Forty-six Rust tests passed, including the new write-through/no-overwrite and ample-space reuse cases. These local flags are not in the published v0.2.2 installer.

The actual request failed before copying new audio: creating `F:/ISLAND-NEW-1792297170622363200/10F00` returned error 1117. The journal remained at `copying; original library unchanged`. Windows recorded disk event 153 at local time 12:19:52, LBA 0x2952 on disk 2. A read-only check identified an unrecoverable entry at that staging subdirectory. No further song transfers or alternative naming probes were attempted.

Under the user's prior explicit authorization for CHKDSK F: /F repairs, CHKDSK converted the damaged staging folder to a file and reported filesystem corrections. The subsequent read-only check found no structural problems. All 43 accepted-library hashes still matched the baseline, and the original application still had all eight queued items. The isolated writer process was stopped. No format, firmware change, original-library replacement or deletion was performed.

Result: zero of eight new songs transferred. The current evidence identifies failure in the Windows-to-device storage path, including directory creation, but does not isolate USB controller/driver, hub, cable/connector, player firmware or flash. A changed connection or host environment is needed before further writes. A short staging-name hypothesis was not tested after corruption appeared.

Host diagnostic data: `data/real-single-transfer-20261018/`. The complete pre-attempt host backup and journal remain under its `backups/1792297168712176500/` directory; the original installed app's complete backup remains outside the project under its own application data directory.

## Follow-up: all eight songs transferred

After the user requested continued transfer, controlled short-name staging successfully copied and verified the Adele audio. Subsequent small-file operations still produced intermittent I/O errors and a corrupted JPEG readback; validation blocked committing that file until an explicit buffered rewrite and readback succeeded. The user confirmed all other access to F: had stopped. Plain Windows CopyFileW, short staging names, explicit per-file hashes and pauses proved usable, though intermittent failures still required stopping, inspecting the accepted state and resuming verified staging files.

A per-file backed-up append installed Adele, and a later append installed the next song. Directory transactions that move the audio directory as one unit, rather than copying unchanged audio or moving each old file, then completed the remaining additions. Single-song and four-song transaction tests injected a failure at every rename and verified restoration. Complete host backups were retained before each preparation. Failed staging data and recovery coordinates were preserved.

Some failed operations returned WinError 1006 (volume externally altered). System/history table changes were observed independently of successful host commits; the changes are consistent with player-side normalization after a reconnect/reset, but their cause was not conclusively isolated. Candidate generation was repeated from the fresh accepted device state rather than bypassing the unchanged-library checks.

The final append coordinator reported all eight requested source identities completed and the Rust device reader returned nine tracks. Independent verification checked each source MP3 hash, each device OMA's audio payload against the corresponding source bytes after the source tag offset, the unchanged original Hey Ya! OMA hash, and decoding of every referenced artwork JPEG. All nine audio payloads passed. Eight tracks have valid referenced artwork; the source for the remaining track has no artwork. The closed desktop app's catalog was backed up and synchronized to nine device tracks, an empty transfer queue and Completed status.

The final source and device verification records are in the chat outputs as `a918-transfer-completed-2026-10-18.json` and `a918-transferred-songs-2026-10-18.csv`. Operational recovery data remains in `data/sequential-append-20261018/`. Physical playback after safe removal is still awaiting the user's check. The transfer used local diagnostic helpers; these changes are not part of the published v0.2.2 installer and are not proof that the USB fault is resolved.

## Physical confirmation

On 2026-10-18, after safe removal, the user confirmed that all nine songs appear on the A918 and newly transferred songs play. This validates the completed eight-song append on this device. Artwork display for the newly added tracks was not separately confirmed. The successful diagnostic append path has not yet been integrated into a new published installer, and the intermittent storage/USB fault remains unresolved.
