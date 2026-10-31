#ifndef AppVersion
  #error AppVersion must be supplied by build-installer.ps1
#endif

[Setup]
AppId={{9F893C52-A1BF-45DE-B912-51957FDE1D74}
AppName=Nocan
AppVersion={#AppVersion}
AppPublisher=Nocan contributors
DefaultDirName={localappdata}\Programs\Nocan
DefaultGroupName=Nocan
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\target
OutputBaseFilename=Nocan-Setup
SetupIconFile=..\assets\mascot\runtime\nocan.ico
UninstallDisplayIcon={app}\nocan.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Files]
Source: "..\target\release\nocan.exe"; DestDir: "{app}"; Flags: ignoreversion

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Icons]
Name: "{autoprograms}\Nocan"; Filename: "{app}\nocan.exe"
Name: "{autodesktop}\Nocan"; Filename: "{app}\nocan.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\nocan.exe"; Description: "Launch Nocan"; Flags: nowait postinstall skipifsilent

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var LegacyPath: String;
begin
  if (CurStep = ssPostInstall) and
     RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Nocan', 'InstallLocation', LegacyPath) and
     (CompareText(RemoveBackslashUnlessRoot(LegacyPath), RemoveBackslashUnlessRoot(ExpandConstant('{app}'))) = 0) then
    RegDeleteKeyIncludingSubkeys(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Nocan');
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Startup: String;
begin
  if (CurUninstallStep = usUninstall) and
     RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Nocan', Startup) and
     ((CompareText(Startup, '"' + ExpandConstant('{app}\nocan.exe') + '"') = 0) or
      (CompareText(Startup, ExpandConstant('{app}\nocan.exe')) = 0)) then
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Nocan');
end;
