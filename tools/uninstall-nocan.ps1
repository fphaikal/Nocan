$ErrorActionPreference = 'Stop'
$install = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs\Nocan'
$exe = Join-Path $install 'nocan.exe'
Get-Process -Name nocan -ErrorAction SilentlyContinue | Where-Object { $_.Path -ieq $exe } | Stop-Process -Force
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$startup = (Get-ItemProperty -Path $run -Name Nocan -ErrorAction SilentlyContinue).Nocan
if ($startup -and $startup.Trim('"') -ieq $exe) { Remove-ItemProperty -Path $run -Name Nocan }
foreach ($folder in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop'))) {
    $link = Join-Path $folder 'Nocan.lnk'
    if (Test-Path -LiteralPath $link) {
        $shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($link)
        if ($shortcut.TargetPath -ieq $exe) { Remove-Item -LiteralPath $link }
    }
}
Remove-Item -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Nocan' -Recurse -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $exe -Force -ErrorAction SilentlyContinue
Write-Host 'Nocan dihapus. Pengaturan dan data pengguna tetap disimpan.'
