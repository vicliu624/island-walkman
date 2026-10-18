$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
Push-Location ui
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw 'npm ci failed' }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw 'UI build failed' }
} finally { Pop-Location }
python tools/collect-notices.py
if ($LASTEXITCODE -ne 0) { throw 'Notice collection failed' }
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
npx --prefix ui tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed' }
Write-Output 'Installer: target/release/bundle/nsis/'
