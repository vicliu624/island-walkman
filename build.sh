#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
(cd ui && npm ci && npm run build)
cargo test --locked
case "${1:-}" in
  --static)
    cargo build --release --locked --target x86_64-unknown-linux-musl
    executable=target/x86_64-unknown-linux-musl/release/island-walkman
    ;;
  '')
    cargo build --release --locked
    executable=target/release/island-walkman
    ;;
  *) printf '%s\n' 'Usage: build.sh [--static]' >&2; exit 2 ;;
esac
mkdir -p bundle/ui
cp "$executable" bundle/island-walkman
cp -R ui/dist/. bundle/ui/
cp README.md LICENSE bundle/
printf '%s\n' 'Built bundle/island-walkman'
