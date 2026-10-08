; Windows installer for Jellyfin-MDL-RPC. Build with Inno Setup 6:
;   iscc installer\jellyfin-mdl-rpc.iss /DAppVersion=1.4.0
; Expects target\release\jellyfin-rpc.exe, target\release\jellyfin-rpc-background.exe
; and target\mdl_fetch\mdl_fetch.exe (built by PyInstaller) to exist.
;
; The wizard asks for the Jellyfin and MyDramaList details and writes
; %APPDATA%\jellyfin-rpc\main.json, so nobody has to edit JSON by hand.

#ifndef AppVersion
  #define AppVersion "1.4.0"
#endif
#ifndef BinDir
  #define BinDir "..\target\release"
#endif
#ifndef FetcherDir
  #define FetcherDir "..\target\mdl_fetch"
#endif

#define AppName "Jellyfin-MDL-RPC"
#define Launcher "jellyfin-rpc-background.exe"
; Stops only copies running from the install folder, not a jellyfin-rpc started elsewhere.
; "{{" is Inno's escape for a literal "{".
#define StopParams "-NoProfile -WindowStyle Hidden -Command Get-Process jellyfin-rpc -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -like (Join-Path $env:LOCALAPPDATA 'Programs\" + AppName + "\*') } | Stop-Process -Force"
#define PowerShell "{sys}\WindowsPowerShell\v1.0\powershell.exe"

[Setup]
AppId={{2764F52A-B475-4961-A190-4DCEA9E4DED6}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=ImSe4n
AppPublisherURL=https://github.com/ImSe4n/jellyfin-mdl-rpc
AppSupportURL=https://github.com/ImSe4n/jellyfin-mdl-rpc/issues
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=yes
; Per-user install: no admin prompt, and the config lives in this user's AppData.
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\target\installer
OutputBaseFilename={#AppName}-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; A running copy holds its files open; close it so they can be replaced.
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\jellyfin-rpc.exe
UninstallDisplayName={#AppName}

[Tasks]
Name: autostart; Description: "Start {#AppName} when I log in"

[Files]
Source: "{#BinDir}\jellyfin-rpc.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BinDir}\{#Launcher}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#FetcherDir}\mdl_fetch.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#Launcher}"
Name: "{group}\Stop {#AppName}"; Filename: "{#PowerShell}"; Parameters: "{#StopParams}"
Name: "{group}\Edit settings"; Filename: "{sys}\notepad.exe"; Parameters: """{userappdata}\jellyfin-rpc\main.json"""
Name: "{group}\View log"; Filename: "{sys}\notepad.exe"; Parameters: """{userappdata}\jellyfin-rpc\jellyfin-rpc.log"""
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{userstartup}\{#AppName}"; Filename: "{app}\{#Launcher}"; Tasks: autostart

[Run]
Filename: "{app}\{#Launcher}"; Description: "Start {#AppName} now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{#PowerShell}"; Parameters: "{#StopParams}"; Flags: runhidden; RunOnceId: "StopJellyfinRpc"

[Messages]
FinishedLabel=Setup has finished installing [name].%n%nOpen the Discord desktop app, then play something on Jellyfin. Your status updates within a few seconds.%n%nTo change your settings later, use "Edit settings" in the Start menu, then stop and start [name] again.

[Code]
var
  ExistingPage: TInputOptionWizardPage;
  JellyfinPage: TInputQueryWizardPage;
  MdlPage: TInputQueryWizardPage;
  DisplayPage: TInputOptionWizardPage;

function ConfigPath: String;
begin
  Result := ExpandConstant('{userappdata}\jellyfin-rpc\main.json');
end;

function KeepExisting: Boolean;
begin
  Result := (ExistingPage <> nil) and ExistingPage.Values[0];
end;

{ Escapes text for a JSON string literal. }
function JsonEscape(const S: String): String;
var
  I: Integer;
  C: Char;
begin
  Result := '';
  for I := 1 to Length(S) do
  begin
    C := S[I];
    if C = '\' then
      Result := Result + '\\'
    else if C = '"' then
      Result := Result + '\"'
    else if Ord(C) < 32 then
      Result := Result + Format('\u%.4x', [Ord(C)])
    else
      Result := Result + C;
  end;
end;

function JsonBool(const Value: Boolean): String;
begin
  if Value then
    Result := 'true'
  else
    Result := 'false';
end;

{ Accepts what people paste from the browser, e.g. http://host:8096/web/#/home. }
function NormalizeUrl(S: String): String;
var
  WebAt: Integer;
begin
  S := Trim(S);
  WebAt := Pos('/web/', LowerCase(S));
  if WebAt > 0 then
    S := Copy(S, 1, WebAt - 1);
  while (Length(S) > 0) and (S[Length(S)] = '/') do
    Delete(S, Length(S), 1);
  if (Length(S) >= 4) and (LowerCase(Copy(S, Length(S) - 3, 4)) = '/web') then
    Delete(S, Length(S) - 3, 4);
  Result := S;
end;

function IsHttpUrl(const S: String): Boolean;
var
  Lower: String;
begin
  Lower := LowerCase(S);
  Result := ((Pos('http://', Lower) = 1) and (Length(S) > 7))
    or ((Pos('https://', Lower) = 1) and (Length(S) > 8));
end;

{ Same rule as jellyfin-rpc: letters, digits and _ . - only. Blank is allowed (MDL off). }
function IsValidMdlName(const S: String): Boolean;
var
  I: Integer;
  C: Char;
begin
  Result := Length(S) <= 64;
  for I := 1 to Length(S) do
  begin
    C := S[I];
    if not (((C >= 'a') and (C <= 'z')) or ((C >= 'A') and (C <= 'Z'))
      or ((C >= '0') and (C <= '9')) or (C = '_') or (C = '.') or (C = '-')) then
      Result := False;
  end;
end;

{ "alice, bob" -> "alice,bob": jellyfin-rpc splits on commas without trimming. }
function CleanUsernames(S: String): String;
begin
  S := Trim(S);
  StringChangeEx(S, ', ', ',', True);
  StringChangeEx(S, ' ,', ',', True);
  Result := S;
end;

procedure InitializeWizard;
var
  AfterId: Integer;
begin
  AfterId := wpWelcome;

  if FileExists(ConfigPath) then
  begin
    ExistingPage := CreateInputOptionPage(AfterId,
      'Existing settings', 'You already have a Jellyfin-RPC config.',
      'Found ' + ConfigPath + '.' + #13#10 + 'Keep it, or replace it with new settings?',
      True, False);
    ExistingPage.Add('Keep my current settings');
    ExistingPage.Add('Replace them (the old file is kept as a backup)');
    ExistingPage.Values[0] := True;
    AfterId := ExistingPage.ID;
  end;

  JellyfinPage := CreateInputQueryPage(AfterId,
    'Jellyfin', 'Which Jellyfin server should your status come from?',
    'To make an API key, open Jellyfin in your browser and go to Dashboard > API Keys > +.');
  JellyfinPage.Add('Server address (the one you open in your browser, e.g. http://192.168.1.10:8096):', False);
  JellyfinPage.Add('API key:', False);
  JellyfinPage.Add('Your Jellyfin username (for several, separate them with commas):', False);

  MdlPage := CreateInputQueryPage(JellyfinPage.ID,
    'MyDramaList', 'Show your MyDramaList stats on Discord?',
    'Your username is the last part of mydramalist.com/profile/<username>, and your profile must be public.' + #13#10 +
    'Leave this blank to skip the MyDramaList cards.');
  MdlPage.Add('MyDramaList username (optional):', False);

  DisplayPage := CreateInputOptionPage(MdlPage.ID,
    'Display', 'How should your Discord status look?',
    'Posters are uploaded to litterbox.catbox.moe, a temporary public image host, because Discord can only show images from a public link.',
    False, False);
  DisplayPage.Add('Show posters');
  DisplayPage.Add('Keep showing my status while paused');
  DisplayPage.Values[0] := True;
  DisplayPage.Values[1] := True;
end;

function ShouldSkipPage(PageID: Integer): Boolean;
begin
  Result := KeepExisting and ((PageID = JellyfinPage.ID) or (PageID = MdlPage.ID)
    or (PageID = DisplayPage.ID));
end;

function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if CurPageID = JellyfinPage.ID then
  begin
    if not IsHttpUrl(NormalizeUrl(JellyfinPage.Values[0])) then
    begin
      MsgBox('The server address has to start with http:// or https://', mbError, MB_OK);
      Result := False;
    end
    else if Trim(JellyfinPage.Values[1]) = '' then
    begin
      MsgBox('Paste the API key from Dashboard > API Keys in Jellyfin.', mbError, MB_OK);
      Result := False;
    end
    else if CleanUsernames(JellyfinPage.Values[2]) = '' then
    begin
      MsgBox('Enter the Jellyfin username you watch with.', mbError, MB_OK);
      Result := False;
    end;
  end
  else if CurPageID = MdlPage.ID then
  begin
    if not IsValidMdlName(Trim(MdlPage.Values[0])) then
    begin
      MsgBox('MyDramaList usernames only use letters, numbers, _ . and -' + #13#10 +
        'Copy it from the end of your profile link.', mbError, MB_OK);
      Result := False;
    end;
  end;
end;

function BuildConfig: String;
var
  Nl, Json, MdlName: String;
begin
  Nl := #13#10;
  Json := '{' + Nl +
    '  "jellyfin": {' + Nl +
    '    "url": "' + JsonEscape(NormalizeUrl(JellyfinPage.Values[0])) + '",' + Nl +
    '    "api_key": "' + JsonEscape(Trim(JellyfinPage.Values[1])) + '",' + Nl +
    '    "username": "' + JsonEscape(CleanUsernames(JellyfinPage.Values[2])) + '"' + Nl +
    '  },' + Nl +
    '  "discord": {' + Nl +
    '    "show_paused": ' + JsonBool(DisplayPage.Values[1]) + Nl +
    '  }';
  if DisplayPage.Values[0] then
    Json := Json + ',' + Nl +
      '  "images": {' + Nl +
      '    "enable_images": true,' + Nl +
      '    "litterbox_images": true' + Nl +
      '  }';
  MdlName := Trim(MdlPage.Values[0]);
  if MdlName <> '' then
    Json := Json + ',' + Nl +
      '  "mdl": {' + Nl +
      '    "username": "' + JsonEscape(MdlName) + '"' + Nl +
      '  }';
  Result := Json + Nl + '}';
end;

procedure WriteConfig;
var
  Lines: TArrayOfString;
  Backup: String;
begin
  if not ForceDirectories(ExtractFileDir(ConfigPath)) then
  begin
    MsgBox('Could not create ' + ExtractFileDir(ConfigPath), mbError, MB_OK);
    Exit;
  end;
  if FileExists(ConfigPath) then
  begin
    Backup := ConfigPath + '.backup-' + GetDateTimeString('yyyymmdd-hhnnss', '-', '-');
    if not RenameFile(ConfigPath, Backup) then
    begin
      MsgBox('Could not back up your old settings, so they were left unchanged:' + #13#10 + ConfigPath,
        mbError, MB_OK);
      Exit;
    end;
  end;
  SetArrayLength(Lines, 1);
  Lines[0] := BuildConfig;
  if not SaveStringsToUTF8File(ConfigPath, Lines, False) then
    MsgBox('Could not write ' + ConfigPath, mbError, MB_OK);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  { A silent install has no answers to write; it keeps whatever config exists. }
  if (CurStep = ssPostInstall) and not KeepExisting
    and (Trim(JellyfinPage.Values[0]) <> '') then
    WriteConfig;
end;
