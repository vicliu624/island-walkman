# Third-party notices

Island Walkman application code is distributed under the MIT License. Dependencies retain their original licenses. The release archives include this notice and available dependency/font license texts.

| Component | License | Purpose |
| --- | --- | --- |
| Tauri 2 and plugins | MIT OR Apache-2.0 | Native windows, dialogs, lifecycle and signed updates |
| React and React DOM | MIT | User interface runtime |
| Animal Island UI | MIT | Interface components, styles and bundled illustration assets |
| Naive Icons | MIT | Interface icons |
| Noto Sans SC | SIL Open Font License 1.1 | Bundled interface font |
| Nunito | SIL Open Font License 1.1 | Bundled interface font |
| Rust dependencies | See Cargo metadata and individual crate licenses | Metadata, images, hashing, JSON, HTTP and filesystem access |
| Rust standard library | MIT OR Apache-2.0, with retained component notices | Executable runtime |

The source repository includes `Cargo.lock` and `ui/package-lock.json`. These identify exact dependency versions. The `licenses/` directory retains font texts, Rust standard-library notices and musl notices. See its provenance README. Runtime notices may describe components for more than one target; their presence does not imply that every listed component is linked into every package.

JSymphonic was consulted during early protocol exploration. It is GPL-3.0-or-later, is not a dependency, and no Java binary or source is bundled. The earlier monolithic layout writer was removed before the MIT publication. The current `src/database.rs` implements declarative binary sections, rows and views from an observed-format specification. Historical observations remain attributed in research notes; they are not an inclusion of GPL implementation code.

REA and Ghidra were research tools, not runtime dependencies. Sony firmware, Sony installers, decompiled listings, proprietary DLLs, user music, album artwork and device backups are excluded from source publication and release archives. The research notes describe observations and their limitations; they do not grant redistribution rights to Sony software.

Sony and WALKMAN are trademarks of their respective owners. This project is independent and is not endorsed by Sony.

The application icon is `demo/assets/logo.png` from [Animal Island UI](https://github.com/guokaigdg/animal-island-ui), copyright guokaigdg, under its MIT license. Generated platform icons retain this attribution. Use does not imply endorsement by its author.

The customized Windows installer template derives from Tauri (`crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi`), copyright Tauri Programme within The Commons Conservancy, under MIT OR Apache-2.0. Upstream license texts are retained in `licenses/tauri/`.
