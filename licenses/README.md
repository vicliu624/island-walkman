# Bundled license provenance

These files retain upstream license and copyright notices. They do not change the MIT license of Island Walkman's own source.

- `NotoSansSC-OFL.txt` and `Nunito-OFL.txt`: upstream Google Fonts OFL notices.
- `rust-1.95/COPYRIGHT-library.html` and its `licenses/` directory: copied unchanged from the official installed Rust 1.95.0 toolchain's `share/doc/rust` directory. The compiler version is pinned in CI. The library notice describes the standard library and its build dependencies across supported targets, not only components linked into one particular application binary.
- `musl-COPYRIGHT.txt`: preserved from the Debian-published musl `1.2.5-3.1~deb13u1` package's `usr/share/doc/musl/copyright`. It retains musl's upstream permissions and third-party notices. This identifies the notice source, not an assertion that the Rust toolchain uses Debian's binary package.

The packager copies this directory into `third-party-licenses/bundled/`. Cargo crate notices and production npm dependency notices are collected separately from their installed source packages.

When updating the Rust toolchain or dependencies, review and refresh the corresponding notices without editing upstream legal text. Applications remain subject to their components' original terms.
