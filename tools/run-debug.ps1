param([switch]$Release)

$ErrorActionPreference = 'Stop'
$project = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$install = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs\Nocan'
$exe = Join-Path $install 'nocan.exe'
$profile = if ($Release) { 'release' } else { 'debug' }

Push-Location $project
try {
    if ($Release) { cargo build --release --offline } else { cargo build --offline }
    if ($LASTEXITCODE -ne 0) { throw 'Build gagal; aplikasi yang sedang berjalan tidak diubah.' }
    $built = Join-Path $project "target\$profile\nocan.exe"
    $projectPrefix = $project.TrimEnd('\') + '\'
    Get-Process -Name nocan -ErrorAction SilentlyContinue | ForEach-Object {
        $path = $_.Path
        if ($path -and ($path -ieq $exe -or $path.StartsWith($projectPrefix, [StringComparison]::OrdinalIgnoreCase))) {
            Stop-Process -Id $_.Id -Force
            $_.WaitForExit()
        }
    }
    New-Item -ItemType Directory -Path $install -Force | Out-Null
    Copy-Item -LiteralPath $built -Destination $exe -Force
    Start-Process -FilePath $exe -WorkingDirectory $install
    Write-Host "Nocan $profile terbaru dijalankan dari $exe"
} finally { Pop-Location }
