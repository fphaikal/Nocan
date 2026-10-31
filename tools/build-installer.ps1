param([string]$InnoCompiler = 'ISCC.exe')

$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $project
try {
    cargo build --release --locked --offline
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $version = [Diagnostics.FileVersionInfo]::GetVersionInfo((Join-Path $project 'target\release\nocan.exe')).ProductVersion
    if (-not $version) { throw 'Could not determine the application version.' }
    & $InnoCompiler "/DAppVersion=$version" (Join-Path $PSScriptRoot 'nocan.iss')
    if ($LASTEXITCODE -ne 0) { throw 'Installer creation failed.' }
    Write-Host "Installer: $(Join-Path $project 'target\Nocan-Setup.exe')"
} finally { Pop-Location }
