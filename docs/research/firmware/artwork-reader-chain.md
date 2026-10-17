# Firmware artwork reader and thumbnail tree selection

Source: immutable original `gemini-ICX1101`, SHA-256 `a5c94ce492fb7ec9cd7a740d30cb79a1c2b43ee782e2da036a72f57dabdcd177`. REA/Ghidra 12.1.4 evidence is retained in `rea/`. Names below are analytical labels; pseudocode is not Sony source.

## Observed chain

- `0x11d608` requests metadata for a track ID with filter `0x1001` on the jacket page. `0x93d7c` allocates the metadata command; `0x189318` dispatches its `0x1000` component through `0x189018` and worker `0x188124`.
- `0x1dc060` maps the request to backend metadata, then opens image data through `0x1ff8c0`, a vtable slot `+0x1c` call. The OMA reader's table at `0x48d870` selects a branch thunk to `0x1f8480`.
- `0x1f8480` first tries a file-specific image source (`0x1fa320`), then falls back through `0x1fa294` / `0x1fbbac` / `0x1fb8dc` to database group artwork. Some short branch entry points are not recognized as functions by Ghidra; their original ARM instructions were independently decoded with Capstone. Failed REA resolutions are retained, not treated as successful evidence.
- `0x1fb8dc` maps a track index through a 16-bit array at object+`0x60`. A zero group returns `0xa0a`. For a nonzero group it reads that GPIF record, recognizes numeric tags `0x50494350` and `0x50494330` (PICP/PIC0), converts the text, replaces backslashes with slashes and selects an image descriptor.
- `0x1fb6c8` initializes the mapping from the selected `01TREE%02X.DAT` and opens the corresponding `03GINF%02X.DAT`. `0x1fa0b0` supplies that selected tree number.
- The OMA backend initializer `0x1fa8c4` reads `00GTRLST.DAT` and `02TREINF.DAT`, matches tree TIT2 values against the profile configuration, and looks up logical tree ID `0x1e` before initializing the database. `0x1fa78c` is an exact, NUL-terminated two-byte string equality check.
- `0x1e4bd4` selects one of four profile tables. Profiles 1, 2 and 3 all have logical tree ID 30 with UTF-16BE name `STD_TPE1_THUMBNAIL` at `0x490768` and no fixed physical tree number. Profile 4 instead has a fixed tree 32. The currently installed runtime profile is not independently observed.

This establishes a real reader-side use for the thumbnail marker, beyond its separate occurrence in mgscsi ioctl configuration. It does not establish that changing the marker alone will display a cover: the selected tree's track/group mapping must also select a record containing a usable picture descriptor.

## Third controlled experiment

The current device's tree 2D marker was `STD_TPE1`. The second experiment had already added PICP/PIC0 to its album record. A new candidate changes only that existing marker to `STD_TPE1_THUMBNAIL` in `02TREINF.DAT`, with no length change and ten changed bytes. All other library hashes, including audio and images, are preserved.

Tool: `tools/probe_thumbnail_tree.py`. Complete verified backup and install report: `data/cover-installs/thumbnail-tree-1792249378707900400/`. The script requires an exact match to the post-update baseline, checks the original slot and table layout, verifies a full backup, stages and flushes the replacement, then checks the entire final library manifest.

The user confirmed **artwork displays and playback is normal** after safely ejecting this third probe. This confirms the combination of the marker, the existing 2D track/group map, four-field GPIF album record and JPEG descriptors on this A918. It does not independently establish which JPEG size was selected, every supported firmware profile, or that all newly generated indexes work.

The Rust database writer now emits this four-field hierarchy and marker when artwork is enabled. A regression fixture covers sparse track IDs, two artists with identical album names, a cover supplied by a later track and an album without artwork. Native full-transfer validation is tracked separately.
