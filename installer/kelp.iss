; Inno Setup Script for Kelp
; To compile, open this file in Inno Setup Compiler.

#define MyAppName "Kelp"
#define MyAppVersion "0.1.0-alpha"
#define MyAppPublisher "Vibbudu"
#define MyAppURL "https://github.com/Vibbudu/kelp"
#define MyAppExeName "kelp.exe"

[Setup]
; Keep same AppId so it cleanly upgrades/replaces the previous registration
AppId={{E1D3B89E-9D1B-4E38-95C4-05A9EB5BC8B3}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
; Output folder for the compiled setup file
OutputDir=output
OutputBaseFilename=KelpSetup-v0.1.0-alpha
Compression=lzma
SolidCompression=yes
WizardStyle=modern
MinVersion=10.0
; Auto-close running launcher instances during install/uninstall
CloseApplications=yes
AppMutex=KelpMutex_3f6c8d10-7e82-4f36-9a2c-982d6b38c20d

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "runatstartup"; Description: "Run Kelp automatically at Windows startup"; GroupDescription: "Additional options:"

[InstallDelete]
; Clean up legacy Nova Launcher shortcuts and installations if present
Type: files; Name: "{autodesktop}\Nova Launcher.lnk"
Type: files; Name: "{group}\Nova Launcher.lnk"
Type: filesandordirs; Name: "{autopf}\Nova Launcher"

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon
Name: "{userstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: runatstartup

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: files; Name: "{localappdata}\{#MyAppName}\config.json"
Type: files; Name: "{localappdata}\{#MyAppName}\kelp.db"
Type: files; Name: "{localappdata}\{#MyAppName}\kelp.db-shm"
Type: files; Name: "{localappdata}\{#MyAppName}\kelp.db-wal"
Type: filesandordirs; Name: "{localappdata}\{#MyAppName}\logs"
Type: dirifempty; Name: "{localappdata}\{#MyAppName}"
