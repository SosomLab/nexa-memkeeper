; Nexa MemKeeper — 설치형 배포 스크립트 (Inno Setup 6, DR-9)
;
; 설계 의도
;  - **사용자별 설치**(PrivilegesRequired=lowest): 앱 자체가 asInvoker로 돌고(DR-6) 자동 실행도
;    HKCU Run(DR-11)이라, 관리자 승격 없이 설치하는 편이 설계와 일치한다. UAC 창이 뜨지 않고
;    제거 목록(프로그램 및 기능)에는 사용자 항목으로 등록된다.
;  - 설치 위치가 %LOCALAPPDATA%\Programs 밑이므로 **exe 옆 data\settings.ini가 그대로 쓰인다**
;    (포터블과 같은 경로 규칙 — config.rs). Program Files에 넣었다면 %LOCALAPPDATA% 폴백을 탔을 것이다.
;  - 자동 실행 질문은 **기본 체크**(사용자 확정 08-07 — DR-9).
;
; 빌드: ISCC.exe /DAppVersion=0.0.1 packaging\NexaMemKeeper.iss  (packaging\pack.ps1이 대신 호출한다)

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define AppName "Nexa MemKeeper"
#define AppExe "NexaMemKeeper.exe"
#define AppPublisher "SosomLab"
#define AppUrl "https://sosomlab.com"
#define RepoUrl "https://github.com/SosomLab/nexa-memkeeper"

[Setup]
; AppId는 **절대 바꾸지 않는다** — 바뀌면 기존 설치를 갱신하지 못하고 중복 등록된다.
AppId={{8C6A5E31-7C4B-4B2E-9F3A-2B1D0E7A94C5}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#RepoUrl}
AppUpdatesURL={#RepoUrl}/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE.md
OutputDir=..\target\pack
OutputBaseFilename=NexaMemKeeper-{#AppVersion}-setup
SetupIconFile=..\crates\mk-app\assets\nexa-memkeeper.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
; 승격 없이 사용자별 설치 — 앱의 권한 모델(asInvoker + HKCU Run)과 맞춘다
PrivilegesRequired=lowest
; 패키지 매니저(winget·choco)는 **승격된 세션**에서 조용히 설치한다. 그대로 사용자별 설치를 하면
; 관리자 계정의 %LOCALAPPDATA%에 들어가 정작 사용자에게는 보이지 않는다. 그래서 명령줄 `/ALLUSERS`로
; 전체 사용자 설치를 허용해 둔다(docs/20 §7-2). 대화형 설치는 지금처럼 사용자별이 기본.
PrivilegesRequiredOverridesAllowed=commandline dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; 설치 중 실행 파일이 잠겨 있으면 알려 준다(상주형이라 흔한 상황)
CloseApplications=yes
CloseApplicationsFilter=*.exe
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
; 기본 체크(플래그에 unchecked를 주지 않는다) — DR-9
Name: "startup"; Description: "Start {#AppName} when I sign in"; GroupDescription: "Startup:"
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE.ko.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; 자동 실행 = HKCU Run 값 1개(DR-11). 앱의 [자동 실행] 메뉴가 쓰는 값과 **같은 이름**이라
; 설치 후 메뉴에서 끄면 그대로 해제된다. 제거 시에는 값을 지운다.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; \
    ValueName: "NexaMemKeeper"; ValueData: """{app}\{#AppExe}"""; Tasks: startup; \
    Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; 상주 중이면 먼저 내린다(파일 잠금 방지)
Filename: "{sys}\taskkill.exe"; Parameters: "/f /im {#AppExe}"; Flags: runhidden; RunOnceId: "StopApp"

; 설정 파일(data\settings.ini)은 **지우지 않는다** — 재설치 시 사용자 설정을 잃지 않기 위해서다.
; 완전히 지우려면 설치 폴더의 data 폴더를 직접 삭제하면 된다(문서 20 §5).
