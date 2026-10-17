$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
Push-Location ui
try {
    npm ci
    if ($LASTEXITCODE -ne 0) { throw 'npm ci failed' }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw 'UI build failed' }
} finally { Pop-Location }
cargo test --locked
if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed' }
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
New-Item -ItemType Directory -Path bundle -Force | Out-Null
Copy-Item -LiteralPath target/release/island-walkman.exe -Destination bundle/island-walkman.exe -Force
New-Item -ItemType Directory -Path bundle/ui -Force | Out-Null
Copy-Item -Path (Join-Path $PSScriptRoot 'ui/dist/*') -Destination bundle/ui -Recurse -Force
Copy-Item -LiteralPath README.md,LICENSE -Destination bundle -Force
Write-Output 'Built bundle/island-walkman.exe; keep bundle/ui beside it.'
