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
