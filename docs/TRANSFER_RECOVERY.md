# Transfer recovery

Version 0.3.0 uses a durable append engine. It integrates the short-name staging and native Windows copy path that completed eight additions on the physical A918. The user confirmed all nine songs were visible and newly transferred songs played. The integrated restart/reconnect behavior is validated using simulated devices and injected faults; it has not been retested by writing this release to the physical player.

## User behavior

- Start a new task through the existing transfer confirmation dialog.
- Pause after the current file; a final commit is allowed to finish safely.
- Resume the saved selection without another song-selection or write-confirmation dialog.
- Reconnect the same device when the UI says it is waiting. Recovery starts automatically once its identity matches.
- Persistent errors and data mismatches require checking the connection and task details, then an explicit retry. A user pause remains paused across application restarts.
- Discarding a pre-commit task retains its files and backups. A commit in progress must be recovered before it can be discarded.

## Persistent state

Each task has a 128-bit session identifier and a computer-side directory at `transfers/<id>/` beneath the configured data directory. Source paths, source hashes, metadata snapshots, artwork hashes, a complete original library backup and the prepared candidate are retained there. Artwork is copied to a private task snapshot so later library edits do not alter the saved task.

Two alternating journal files contain a serialized payload and its SHA-256 checksum. Each journal generation is synced before a corresponding device mutation. A torn newer file falls back to the previous valid generation. Recovery inspects locations and hashes to handle operations that completed before the next journal update. If neither generation is valid, recovery stops and retains the backups.

Phases are Preparing, Copying, Ready, Committing, Committed and Finalized. Status separately records Active, WaitingDevice, UserPaused or NeedsReview. Completion remains Committed until the desktop catalog has removed only this task's source keys and persisted the resulting device view. Only then is the journal Finalized. Repeating finalization therefore does not duplicate songs or drop later queue additions.

## Copy and resume

Windows uses CopyFileW with no forced write-through flag. Debian and macOS use new-file buffered copies and a final sync. Copy completion is followed by a settling delay (three seconds for files below 128 KiB, one second otherwise) and SHA-256 readback. A mismatch is read again through a fresh handle before pausing for review. These checks are not a guarantee against physical power loss.

New audio is copied through one short temporary filename inside the existing audio directory, verified, then moved into the task's incoming directory. Non-audio files are staged separately. On resume, already complete files are rehashed and reused. A saved interrupted copy can retain or quarantine its incomplete temporary file and recopy that file. Unknown checksum failures require explicit review rather than repeated automatic rewriting.

Capacity checks reserve space for remaining new audio, non-audio staging, directories and a margin. The original audio is not copied to another device directory. Unindexed audio, case-colliding paths and reserved staging-name conflicts stop before a new task overwrites them.

## Commit and process interruption

1. Move verified new audio into the existing audio directories without changing the old database.
2. Persist the root-retain intent, then rename OMGAUDIO to the reserved short backup directory.
3. Persist the move intent, then move each entire audio directory from the retained root into the incoming root.
4. Persist the install intent, then rename the incoming root to OMGAUDIO.
5. Verify the final file manifest and parsed song/audio/artwork identities before recording Committed.

Every move is inspected on restart. A move whose destination already contains the expected data is not repeated. Unambiguous retained/incoming layouts are verified by hashes before continuing, including when an older valid journal generation preceded the last rename. Conflicting locations, unexpected libraries and data changes pause for review.

After audio-directory moves, the device-side retained directory contains metadata and artwork but can lack audio. The full original library remains in the computer backup. No recursive player cleanup, automatic CHKDSK, formatting or firmware operation is part of recovery.

## Identity and transport recovery

A random device marker in IWIDENT.DAT identifies the saved task's player. Windows additionally checks the filesystem volume serial. The device is resolved again after a transport failure, allowing a new mount path while refusing a different marker at the old path. Directory names, journal paths, ownership markers and original audio hashes are checked before recovery writes. The drive letter is never the sole authorization.

Transient device errors receive three automatic retries after 2, 5 and 10 seconds. Each attempt releases old handles, resolves identity and rechecks state. A device still reporting errors after that bound requires review. A genuinely missing device waits; the idle monitor checks for its matching identity every five seconds. It does not endlessly relaunch failed writes against a device that remains present. Background device artwork and enumeration are blocked while jobs are running.

## Player normalization

The physical investigation observed independent changes to 00GTRLST.DAT, 02TREINF.DAT and the 0C/0E/0F system/history table set, along with volume-change errors. Their underlying cause was not isolated. Recovery accepts only that specific file set, requires every other file to be unchanged, and checks parsed original IDs, text metadata, properties, audio hashes and referenced artwork hashes. It refreshes the complete backup and regenerates the candidate before committing. Title/content changes, unknown files and audio changes are not classified as harmless normalization.

## Validation

Tests cover complete multi-song append; no original audio rewrites; opaque and audio-directory auxiliary data; blank-player initialization; the 256-track audio-directory boundary; frozen artwork; changed sources; partial copies; bounded persistent I/O failures; disk full; checksum review; changed mount paths; wrong devices; colliding short directories; torn journal slots; every commit rename failure; simulated process death at preparation, copy and commit boundaries; recovery with a torn latest intent; and queue preservation after application reconstruction.

UI verification uses an isolated, synthetic player under the project data directory. It checks saved progress, explicit resume without another confirmation, completion, and English/Chinese/Japanese status labels. No development test writes to the user's physical A918. Physical USB faults, complete device failure and arbitrary filesystem corruption remain outside automatic recovery guarantees.
