param([Parameter(Mandatory = $true)][string]$Installer)
$ErrorActionPreference = 'Stop'

if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_OS -ne 'Windows' -or -not $env:RUNNER_TEMP) {
    throw 'Run installer lifecycle tests only on a disposable Windows GitHub runner.'
}
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Resonance'
if (Test-Path -LiteralPath $uninstallKey) {
    throw 'An existing Resonance installation must not be modified by this test.'
}
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$runnerRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
$installDirectory = [IO.Path]::GetFullPath((Join-Path $runnerRoot ('resonance-install-' + [Guid]::NewGuid().ToString('N'))))
if (-not $installDirectory.StartsWith($runnerRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unexpected installer test directory.'
}

function Install-TestApplication {
    $process = Start-Process -FilePath $installerPath -ArgumentList @('/S', '/NS', "/D=$installDirectory") -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Installer failed with exit code $($process.ExitCode)." }
    if (-not (Test-Path -LiteralPath (Join-Path $installDirectory 'resonance.exe'))) { throw 'Installed executable is missing.' }
    if (-not (Test-Path -LiteralPath $uninstallKey)) { throw 'Uninstall registration is missing.' }
}

try {
    Install-TestApplication
    node scripts/smoke.mjs --release --binary (Join-Path $installDirectory 'resonance.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Installed application failed its startup test.' }
    Install-TestApplication
    node scripts/smoke.mjs --release --binary (Join-Path $installDirectory 'resonance.exe')
    if ($LASTEXITCODE -ne 0) { throw 'Reinstalled application failed its startup test.' }
} finally {
    $uninstaller = Join-Path $installDirectory 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller) {
        $process = Start-Process -FilePath $uninstaller -ArgumentList @('/S', "_?=$installDirectory") -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -ne 0) { throw "Uninstaller failed with exit code $($process.ExitCode)." }
    }
}
if (Test-Path -LiteralPath (Join-Path $installDirectory 'resonance.exe')) { throw 'Uninstaller left the executable behind.' }
if (Test-Path -LiteralPath $uninstallKey) { throw 'Uninstaller left its registration behind.' }
Write-Output 'Installation, reinstallation, startup, and uninstallation passed.'
