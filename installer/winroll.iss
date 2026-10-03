#ifndef PackageDir
  #error PackageDir must point to the staged public package
#endif
#ifndef OutputDir
  #error OutputDir must point to the release output directory
#endif
#ifndef AppVersion
  #error AppVersion must be supplied by the package script
#endif

[Setup]
AppId={{4A9106DA-A87A-4CD2-8BD8-2177786DA580}
AppName=WinRoll RS
AppVersion={#AppVersion}
AppPublisher=Kin Tam
AppPublisherURL=https://github.com/jedipi/WinRoll-RS
AppSupportURL=https://github.com/jedipi/WinRoll-RS/issues
DefaultDirName={localappdata}\Programs\WinRoll RS
DefaultGroupName=WinRoll RS
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
MinVersion=10.0.19045
AppMutex=Local\WinRoll-RS.Experiment
CloseApplications=no
RestartApplications=no
UninstallDisplayIcon={app}\winroll.exe
SetupIconFile=..\assets\icons\winroll-expanded.ico
OutputDir={#OutputDir}
OutputBaseFilename=winroll-{#AppVersion}-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "{#PackageDir}\winroll.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\compatibility-report.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\RUST-COPYRIGHT.html"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\SHA256SUMS.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\WinRoll RS"; Filename: "{app}\winroll.exe"
Name: "{group}\Uninstall WinRoll RS"; Filename: "{uninstallexe}"

[Code]
function InitializeSetup: Boolean;
begin
  Result := ProcessorArchitecture = paX64;
  if not Result then
    SuppressibleMsgBox('WinRoll RS requires x64 Windows 10 22H2 or Windows 11 21H2 or newer.',
      mbCriticalError, MB_OK, IDOK);
end;
