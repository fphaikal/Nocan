$ErrorActionPreference = 'Stop'
$install = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs\Nocan'
$exe = Join-Path $install 'nocan.exe'
New-Item -ItemType Directory -Path $install -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'uninstall-nocan.ps1') -Destination (Join-Path $install 'uninstall-nocan.ps1') -Force
& (Join-Path $PSScriptRoot 'run-debug.ps1') -Release
$shell = New-Object -ComObject WScript.Shell
foreach ($folder in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop'))) {
    $shortcut = $shell.CreateShortcut((Join-Path $folder 'Nocan.lnk'))
    $shortcut.TargetPath = $exe
    $shortcut.WorkingDirectory = $install
    $shortcut.IconLocation = "$exe,0"
    $shortcut.Save()
}
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Nocan'
New-Item -Path $key -Force | Out-Null
$uninstall = Join-Path $install 'uninstall-nocan.ps1'
Set-ItemProperty $key DisplayName 'Nocan'
Set-ItemProperty $key Publisher 'Nocan contributors'
Set-ItemProperty $key InstallLocation $install
Set-ItemProperty $key DisplayIcon $exe
Set-ItemProperty $key UninstallString "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$uninstall`""
Set-ItemProperty $key NoModify 1 -Type DWord
Set-ItemProperty $key NoRepair 1 -Type DWord
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
if ((Get-ItemProperty -Path $run -Name Nocan -ErrorAction SilentlyContinue).Nocan) {
    Set-ItemProperty -Path $run -Name Nocan -Value "`"$exe`""
}
