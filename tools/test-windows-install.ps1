param([Parameter(Mandatory = $true)][string]$Installer)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This installer integration test is restricted to an isolated CI runner.' }
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$desktopLink = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Island Walkman.lnk'
$programs = [Environment]::GetFolderPath('Programs')
function Get-MenuLinks {
    @(Get-ChildItem -LiteralPath $programs -Filter 'Island Walkman.lnk' -Recurse -ErrorAction SilentlyContinue)
}
if ((Test-Path -LiteralPath $desktopLink) -or (Get-MenuLinks).Count -gt 0) {
    throw 'A pre-existing shortcut would make this test unsafe or ambiguous.'
}
foreach ($shortcuts in @($false, $true)) {
    $installDir = Join-Path $env:RUNNER_TEMP ('IslandWalkmanInstallerTest-' + [Guid]::NewGuid().ToString('N'))
    try {
        $installArgs = @('/S')
        if (-not $shortcuts) { $installArgs += '/NS' }
        $installArgs += "/D=$installDir"
        $process = Start-Process -FilePath $installerPath -ArgumentList $installArgs -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -ne 0) { throw "Installation failed: $($process.ExitCode)" }
        if ((Test-Path -LiteralPath $desktopLink) -ne $shortcuts) { throw 'Desktop shortcut choice was ignored.' }
        if (((Get-MenuLinks).Count -gt 0) -ne $shortcuts) { throw 'Start Menu shortcut choice was ignored.' }
        python tools/desktop-smoke.py --installed --executable "$installDir/island-walkman.exe"
        if ($LASTEXITCODE -ne 0) { throw 'Installed desktop validation failed.' }
        $update = Start-Process -FilePath $installerPath -ArgumentList '/S', '/UPDATE', "/D=$installDir" -WindowStyle Hidden -Wait -PassThru
        if ($update.ExitCode -ne 0) { throw "Installer update failed: $($update.ExitCode)" }
        if ((Test-Path -LiteralPath $desktopLink) -ne $shortcuts) { throw 'Update changed the desktop shortcut choice.' }
        if (((Get-MenuLinks).Count -gt 0) -ne $shortcuts) { throw 'Update changed the Start Menu shortcut choice.' }
    } finally {
        $uninstaller = Join-Path $installDir 'uninstall.exe'
        if (Test-Path -LiteralPath $uninstaller) {
            $uninstall = Start-Process -FilePath $uninstaller -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
            if ($uninstall.ExitCode -ne 0) { throw "Test uninstall failed: $($uninstall.ExitCode)" }
        }
    }
    if ((Test-Path -LiteralPath $desktopLink) -or (Get-MenuLinks).Count -gt 0) { throw 'Uninstall left test shortcuts behind.' }
}
Write-Output 'Installer integration passed: optional shortcuts, native launch, update preservation and uninstall.'
