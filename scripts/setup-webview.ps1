$ErrorActionPreference = 'Stop'

function Get-WebViewVersion {
    $keys = @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
        'HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    )
    foreach ($key in $keys) {
        $value = Get-ItemPropertyValue -LiteralPath $key -Name pv -ErrorAction SilentlyContinue
        if ($value -and [version]$value -gt [version]'0.0.0.0') { return $value }
    }
}

$version = Get-WebViewVersion
if (-not $version) {
    $installer = Join-Path ([IO.Path]::GetTempPath()) ('webview2-' + [Guid]::NewGuid().ToString('N') + '.exe')
    Invoke-WebRequest -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $installer
    $signature = Get-AuthenticodeSignature -LiteralPath $installer
    if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
        throw 'The WebView2 bootstrapper does not have a valid Microsoft signature.'
    }
    $process = Start-Process -FilePath $installer -ArgumentList '/silent', '/install' -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "WebView2 installation failed with exit code $($process.ExitCode)." }
    $version = Get-WebViewVersion
    if (-not $version) { throw 'The WebView2 Runtime is still unavailable after installation.' }
}
Write-Output "WebView2 Runtime $version is available."
