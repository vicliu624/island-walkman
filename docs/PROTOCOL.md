# Observed protocol and implementation provenance

The current serializer is expressed as documents containing typed sections. Each section owns its row count, row width, flags, payload and padding. The directory derives offsets and lengths from encoded sections, rather than hard-coding a sequence of writes. Flat views and the artist/album hierarchy are independent projections of the library.

`src/database.rs` replaces the earlier monolithic exploratory writer. The current implementation does not link to, invoke, include or translate JSymphonic source. JSymphonic was an early behavioral reference; this history must remain visible rather than being disguised by a license-header change. Runtime code, dependency licenses and research inputs are separate: see [third-party notices](../THIRD_PARTY_NOTICES.md).

## Format facts

- Directory records contain a four-byte section identifier, big-endian offset and length, then reserved bytes.
- Sections begin with an identifier, big-endian row count and row width, then two flag words.
- Fixed text fields contain a four-byte tag, encoding identifier 2 and UTF-16BE text. Slots are 128 bytes. A full two-byte terminator is retained and surrogate pairs are not split.
- The content index (`04CNTINF.DAT`) maps stable one-based IDs to audio properties and title, artist, album, genre and sorting fields. Sparse IDs remain sparse.
- Tree files combine group records (`GPLB`) with track-ID lists (`TPLB`). Group metadata lives in the corresponding `03GINFxx.DAT` file.
- The tested artwork hierarchy uses TIT2/PICP/PIC0/XSOT and the tree marker `STD_TPE1_THUMBNAIL`. Firmware selects logical artwork tree 30 by the exact title marker, then resolves tracks to groups and picture descriptors.
- Artwork descriptors contain two three-digit dimensions, a semicolon and a relative JPEG path. Additional descriptors are separated by `|`. Square images avoid assuming an unresolved dimension ordering.

Evidence for the field names, filename generation, text slots and reader behavior comes from original SonicStage binaries and the player's firmware. See [Sony inventory](research/sony-inventory.md) and [firmware reader chain](research/firmware/artwork-reader-chain.md). Those documents retain hashes, addresses and evidence identifiers. No proprietary binary or recovered listing belongs in the public repository.

## Verification boundaries

A controlled A918 probe displayed artwork and played normally. Full native Rust transfers have not yet completed because Windows reports reproducible device I/O failures while writing temporary files, before library commit. Passing serialization and readback tests does not prove full hardware compatibility. Writes remain gated until native transfer validation succeeds.

The current writer does not preserve arbitrary foreign playlists. Preflight rejects existing playlist contents instead of silently replacing them. Unknown files are retained in the staged library, but this is not a claim that every foreign index semantic is understood.
