$ErrorActionPreference = 'Stop'

# 포터블 = 설치기 없이 ZIP을 풀어 둔다. 설정은 exe 옆 data\settings.ini에 생기고,
# choco 제거 시 tools 폴더째 사라진다(설정도 함께 — 보존이 필요하면 설치형을 쓴다).
# {{VERSION}}/{{CHECKSUM64}}는 release.yml이 주입한다(수동 편집 금지).
$packageName = 'nexa-memkeeper.portable'
$version     = '{{VERSION}}'
$toolsDir    = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64       = "https://github.com/SosomLab/nexa-memkeeper/releases/download/$version/NexaMemKeeper-$version-portable.zip"

Install-ChocolateyZipPackage `
  -PackageName $packageName `
  -Url64bit $url64 `
  -Checksum64 '{{CHECKSUM64}}' `
  -ChecksumType64 'sha256' `
  -UnzipLocation $toolsDir

# choco는 tools 아래 exe마다 실행용 shim을 만든다. '<exe>.gui' 표시 파일이 있으면 shim이
# 이 프로그램을 GUI 앱으로 다뤄 콘솔 창을 붙잡지 않고 곧바로 돌아간다(C-3).
# exe가 ZIP에서 풀리므로 패키지에 넣지 않고 설치 시점에 만든다.
New-Item -ItemType File -Path (Join-Path $toolsDir 'NexaMemKeeper.exe.gui') -Force | Out-Null
