; Inno Setup 6 script for Audio Mixer Manager.
;
; Build the release executable first, then from the repository root:
;   iscc installer\audiomixermanager.iss
; The setup is written to target\installer\.
;
; Per-user install: no administrator rights, files in
; %LOCALAPPDATA%\Programs\Audio Mixer Manager, uninstall from Settings > Apps.

#define AppName "Audio Mixer Manager"
#define AppExe "audiomixermanager.exe"
#define AppSource "..\target\release\" + AppExe
#define RunKey "Software\Microsoft\Windows\CurrentVersion\Run"
; Must match VALUE_NAME in crates/app/src/autostart.rs.
#define RunValue "AudioMixerManager"

#ifndef AppVersion
  #define AppVersion GetStringFileInfo(AppSource, "ProductVersion")
#endif

[Setup]
; Never change AppId: it identifies the installation for upgrades and uninstall.
AppId={{60049364-CC2E-43B1-A382-A7474D21496F}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppName}
DefaultDirName={localappdata}\Programs\{#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Windows 10 1803: first build with per-app audio routing.
MinVersion=10.0.17134
OutputDir=..\target\installer
OutputBaseFilename=AudioMixerManager-Setup-{#AppVersion}
SetupIconFile=app.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
; The app keeps running in the tray when its window is closed, so the Restart
; Manager cannot close it: it is stopped in [Code] instead.
CloseApplications=no

[Languages]
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
italian.AutostartTask=Avvia con Windows (nascosta nell'area di notifica)
english.AutostartTask=Start with Windows (hidden in the notification area)
italian.LaunchApp=Avvia {#AppName}
english.LaunchApp=Launch {#AppName}
italian.DeleteSettings=Eliminare anche impostazioni e log di {#AppName}?
english.DeleteSettings=Also delete {#AppName} settings and logs?

[Tasks]
Name: "autostart"; Description: "{cm:AutostartTask}"
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#AppSource}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Same entry the in-app "Start with Windows" toggle writes.
Root: HKCU; Subkey: "{#RunKey}"; ValueType: string; ValueName: "{#RunValue}"; ValueData: """{app}\{#AppExe}"" --hidden"; Tasks: autostart

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchApp}"; Flags: nowait postinstall skipifsilent

[Code]
procedure StopApp();
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe}', '', SW_HIDE,
    ewWaitUntilTerminated, ResultCode);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopApp();
  Result := '';
end;

{ Unchecking the task on an upgrade removes an entry left by a previous install. }
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and not WizardIsTaskSelected('autostart') then
    RegDeleteValue(HKEY_CURRENT_USER, '{#RunKey}', '{#RunValue}');
end;

function InitializeUninstall(): Boolean;
begin
  StopApp();
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    { The entry may come from the in-app toggle, not from the installer task. }
    RegDeleteValue(HKEY_CURRENT_USER, '{#RunKey}', '{#RunValue}');
    if not UninstallSilent and
       (MsgBox(CustomMessage('DeleteSettings'), mbConfirmation, MB_YESNO) = IDYES) then
    begin
      DelTree(ExpandConstant('{userappdata}\AudioMixerManager'), True, True, True);
      DelTree(ExpandConstant('{localappdata}\AudioMixerManager'), True, True, True);
    end;
  end;
end;
