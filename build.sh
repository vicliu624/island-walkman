#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
(cd ui && npm ci && npm run build)
python3 tools/collect-notices.py
cargo test --locked
case "$(uname -s)" in
  Linux) bundles=deb,appimage ;;
  Darwin) bundles=app,dmg ;;
  *) printf '%s\n' 'Use build.ps1 on Windows.' >&2; exit 2 ;;
esac
npx --prefix ui tauri build --bundles "$bundles"
