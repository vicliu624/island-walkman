# Validation record: 2026-10-17

Windows 11, Rust/Cargo 1.95 and the TypeScript/Vite production build were verified. The suite grew from 13 to 17 tests as metadata round trips, artwork trees and playlist preflight were added. Older logs retain the count for their historical run.

Earlier API checks used a purchased MP3 to verify import, cover extraction, Unicode catalog edits, queue deduplication, local preparation, the write gate and unchanged source hashes. Raw logs are local research artifacts.

Official Rust Linux archives were checked against published SHA-256 values and installed in task workspace directories. Ubuntu 24.04/WSL tested GNU and musl builds. The static executable also completed real-song import, artwork caching, OMA/database generation and readback in Debian 12 through PRoot.

The Debian root filesystem was an official debuerreotype artifact; verified OCI layer SHA-256: `7fd1ebe418b5079d4aa36f416af9b143d127bc75b0155d71cb5fd559b5e04d39`. No host package upgrade was performed. These checks do not validate USB hardware.

Uppercase indexes made the original track visible and playable. Two artwork probes failed to display artwork. A third changed the 2D tree marker to `STD_TPE1_THUMBNAIL`; the user confirmed artwork and playback both worked.

Two full native transfers stopped before commit with Windows I/O error 1117. A filesystem repair was authorized between attempts. Original audio and key index hashes remained unchanged. See [native transfer validation](native-transfer-validation.md).

New CI/localization checks are separate from these historical results. Local builds do not substitute for macOS runner results or complete physical transfer tests.
