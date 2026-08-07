$ErrorActionPreference = 'Stop'

# {{VERSION}}/{{CHECKSUM64}}는 release.yml이 태그 빌드 시점에 치환한다(수동 편집 금지).
$packageName = 'nexa-memkeeper'
$version     = '{{VERSION}}'
$url64       = "https://github.com/SosomLab/nexa-memkeeper/releases/download/$version/NexaMemKeeper-$version-setup.exe"

$packageArgs = @{
  packageName    = $packageName
  fileType       = 'exe'
  url64bit       = $url64
  checksum64     = '{{CHECKSUM64}}'
  checksumType64 = 'sha256'
  softwareName   = 'Nexa MemKeeper*'
  # Inno Setup. /ALLUSERS = 머신 전역 설치 강제 — choco는 관리자로 도는데 설치기 기본은
  # PrivilegesRequired=lowest(사용자별)이라, 명시하지 않으면 관리자 계정의 %LOCALAPPDATA%에
  # 설치되어 정작 사용자에게는 보이지 않는다(packaging/NexaMemKeeper.iss 참조).
  # /TASKS="" = 자동 실행 등록 생략 — 트레이 메뉴에서 각자 켠다(설치자 ≠ 사용 계정일 수 있다).
  silentArgs     = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- /ALLUSERS /TASKS=""'
  validExitCodes = @(0)
}

Install-ChocolateyPackage @packageArgs
