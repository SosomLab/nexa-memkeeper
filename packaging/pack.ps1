# 배포 산출물 생성 — 포터블 ZIP + 설치형 exe (DR-9, 절차 SSOT = docs/20-packaging.md)
#
#   pwsh packaging\pack.ps1            # 릴리스 빌드 + 예산 게이트 + ZIP (+ Inno 있으면 설치본)
#   pwsh packaging\pack.ps1 -SkipBuild # 이미 빌드된 산출물로 포장만
#
# 게이트(DR-8)를 통과하지 못하면 **포장하지 않고 중단**한다 — 예산 초과분이 배포로 새어 나가지 않게.

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$SkipInstaller
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$exeName = 'NexaMemKeeper.exe'
$relExe = Join-Path $root "target\release\$exeName"
$outDir = Join-Path $root 'target\pack'

# 버전은 워크스페이스 Cargo.toml 한 곳에서만 읽는다(단일 원천)
$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1).Matches[0].Groups[1].Value
if (-not $version) { throw 'Cargo.toml에서 버전을 찾지 못했습니다.' }
Write-Host "== Nexa MemKeeper $version 포장 ==" -ForegroundColor Cyan

if (-not $SkipBuild) {
    # 상주 중이면 exe가 잠겨 링크가 실패한다(docs/18 §3-1) — 먼저 내린다
    Get-Process NexaMemKeeper -ErrorAction SilentlyContinue | Stop-Process -Force
    Push-Location $root
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw '릴리스 빌드 실패' }
    } finally { Pop-Location }
}
if (-not (Test-Path $relExe)) { throw "산출물이 없습니다: $relExe" }

# ── 예산 게이트(DR-8) — exe 크기 · 외부 DLL 0
$sizeMb = [math]::Round((Get-Item $relExe).Length / 1MB, 2)
Write-Host ("exe 크기 : {0} MB (기준 ≤ 3MB)" -f $sizeMb)
if ($sizeMb -gt 3) { throw "예산 초과: exe $sizeMb MB > 3MB (DR-8)" }

$dumpbin = Get-Command dumpbin.exe -ErrorAction SilentlyContinue
if (-not $dumpbin) {
    # PATH에 없으면 Visual Studio 설치본에서 찾는다(GitHub 러너·일반 개발 PC 모두 여기에 있다)
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        # -requires로 **MSVC 도구 세트가 있는** 설치본만 고른다. 안 그러면 VS 셸만 얹은 제품
        # (예: SSMS)이 latest로 잡혀 VC\Tools 폴더가 없다(08-07 실측).
        $vs = & $vswhere -latest -products * `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($vs) {
            $found = Get-ChildItem -Path (Join-Path $vs 'VC\Tools\MSVC') -Filter dumpbin.exe -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $_.FullName -match '\\Hostx64\\x64\\' } | Select-Object -First 1
            if ($found) { $dumpbin = $found }
        }
    }
}
if ($dumpbin) {
    $sys = @('kernel32.dll', 'user32.dll', 'gdi32.dll', 'shell32.dll', 'advapi32.dll',
        'ntdll.dll', 'comctl32.dll', 'version.dll', 'msvcrt.dll', 'ole32.dll', 'oleaut32.dll',
        'bcrypt.dll', 'bcryptprimitives.dll', 'ucrtbase.dll', 'psapi.dll', 'shlwapi.dll',
        'userenv.dll', 'ws2_32.dll')
    $imports = & $dumpbin /dependents $relExe |
        Select-String -Pattern '^\s{4}(\S+\.dll)$' | ForEach-Object { $_.Matches[0].Groups[1].Value.ToLower() }
    # api-ms-win-*는 **API 세트**(OS가 제공하는 가상 DLL 이름)라 개별 나열이 무의미하다 — 접두사로 통과.
    # 반면 vcruntime140.dll 같은 재배포 런타임은 화이트리스트에 없으므로 그대로 걸린다(의도).
    $extra = $imports | Where-Object { $sys -notcontains $_ -and $_ -notlike 'api-ms-win-*' }
    if ($extra) { throw "외부 DLL 의존 발견(DR-8 위반): $($extra -join ', ')" }
    Write-Host ("의존 DLL : {0} (모두 OS 기본)" -f ($imports -join ', '))
} else {
    Write-Host '의존 DLL : dumpbin 없음 — 확인 건너뜀(Developer PowerShell에서 실행하면 검사한다)' -ForegroundColor Yellow
}

# ── 포터블 ZIP — 압축을 풀어 바로 실행하는 형태(무서명 설치 차단 회피, DR-9)
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$stage = Join-Path $outDir "NexaMemKeeper-$version"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
Copy-Item $relExe $stage
Copy-Item (Join-Path $root 'README.md'), (Join-Path $root 'LICENSE.md'), (Join-Path $root 'LICENSE.ko.md') $stage

# 설정을 exe 옆에 두는 포터블 모드임을 명시(config.rs의 포터블 우선 규칙)
@"
Nexa MemKeeper $version — portable

Run NexaMemKeeper.exe. Settings are stored next to the executable in
  data\settings.ini
so the whole folder can be moved or carried on a USB drive.
If the folder is read-only, settings fall back to %LOCALAPPDATA%\NexaMemKeeper.

Autostart is off in the portable build. Enable it from the tray menu when you
want it — it writes a single HKCU\...\Run value and removing it undoes that.

https://sosomlab.com  |  https://github.com/SosomLab/nexa-memkeeper
"@ | Set-Content -Path (Join-Path $stage 'PORTABLE.txt') -Encoding UTF8

$zip = Join-Path $outDir "NexaMemKeeper-$version-portable.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
Write-Host ("포터블   : {0} ({1} KB)" -f $zip, [math]::Round((Get-Item $zip).Length / 1KB)) -ForegroundColor Green
Write-Host ("  SHA256 : {0}" -f (Get-FileHash $zip -Algorithm SHA256).Hash)

# ── 설치형 exe — Inno Setup이 있을 때만(없어도 ZIP 배포는 성립한다)
if ($SkipInstaller) { return }
$iscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue
if (-not $iscc) {
    foreach ($p in @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe")) {
        if (Test-Path $p) { $iscc = Get-Item $p; break }
    }
}
if (-not $iscc) {
    Write-Host '설치본   : Inno Setup 6(ISCC.exe)이 없어 건너뜁니다 — https://jrsoftware.org/isdl.php' -ForegroundColor Yellow
    return
}
& $iscc.Source "/DAppVersion=$version" (Join-Path $PSScriptRoot 'NexaMemKeeper.iss')
if ($LASTEXITCODE -ne 0) { throw '설치본 생성 실패' }
$setup = Join-Path $outDir "NexaMemKeeper-$version-setup.exe"
Write-Host ("설치본   : {0} ({1} KB)" -f $setup, [math]::Round((Get-Item $setup).Length / 1KB)) -ForegroundColor Green
# 해시는 winget·choco 매니페스트가 요구한다(docs/20 §8) — 릴리스 노트에도 같이 싣는다
Write-Host ("  SHA256 : {0}" -f (Get-FileHash $setup -Algorithm SHA256).Hash)
"$((Get-FileHash $setup -Algorithm SHA256).Hash)  $(Split-Path $setup -Leaf)`n$((Get-FileHash $zip -Algorithm SHA256).Hash)  $(Split-Path $zip -Leaf)" |
    Set-Content (Join-Path $outDir 'SHA256SUMS.txt') -Encoding ASCII
