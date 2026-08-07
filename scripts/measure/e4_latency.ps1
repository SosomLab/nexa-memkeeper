# E4 — 정리가 "체감 성능"에 미치는 영향: 앱 기동 지연 + 파일 캐시 재적재
# 기본값(①②)과 공격적 설정(③ 전체 스탠바이 퍼지)을 같은 잣대로 비교 — 기본값 근거 검증.
$sp = $PSScriptRoot
. "$sp\memlib.ps1"
$out = "$PSScriptRoot\results"; New-Item -ItemType Directory -Force -Path $out | Out-Null
$iniDir = Split-Path $script:Exe; $ini = "$iniDir\data\settings.ini"

# --- 워크로드 1: 앱 기동 지연 (pwsh — DLL/어셈블리 100여 개 적재) ---
function Measure-Launch {
    param($n = 8)
    $t = @()
    for ($i = 0; $i -lt $n; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $p = Start-Process -FilePath "pwsh" -ArgumentList "-NoProfile","-NoLogo","-Command","exit" -Wait -PassThru -WindowStyle Hidden
        $sw.Stop(); $t += $sw.Elapsed.TotalMilliseconds
    }
    return $t
}

# --- 워크로드 2: 파일 캐시 재적재 (고정 코퍼스 읽기) ---
$corpus = Get-ChildItem "C:\Windows\System32\*.dll" -File -ErrorAction SilentlyContinue |
          Sort-Object Name | Select-Object -First 400
$corpusMB = [math]::Round((($corpus | Measure-Object Length -Sum).Sum)/1MB, 1)
function Measure-Read {
    $buf = New-Object byte[] 1048576
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    foreach ($f in $corpus) {
        $fs = [System.IO.File]::OpenRead($f.FullName)
        while ($fs.Read($buf, 0, $buf.Length) -gt 0) { }
        $fs.Dispose()
    }
    $sw.Stop(); return $sw.Elapsed.TotalMilliseconds
}

function Set-Areas {
    param([string]$standbyAll)
    New-Item -ItemType Directory -Force -Path "$iniDir\data" | Out-Null
    @"
[areas]
working_sets = true
standby_low = true
standby_all = $standbyAll
modified = false
system_ws = false
combine = false
"@ | Set-Content -Path $ini -Encoding UTF8
}

$stats = { param($a) [pscustomobject]@{
    n=$a.Count; MedianMs=[math]::Round((($a|Sort-Object)[[int]($a.Count/2)]),1)
    MinMs=[math]::Round(($a|Measure-Object -Minimum).Minimum,1)
    MaxMs=[math]::Round(($a|Measure-Object -Maximum).Maximum,1)
    MeanMs=[math]::Round(($a|Measure-Object -Average).Average,1) } }

$rows = [System.Collections.ArrayList]::new()
"[E4] 상주 셸 중지"; Stop-Shell | Out-Null
"[E4] 코퍼스: $($corpus.Count)개 파일 / $corpusMB MB"

# 1) 워밍업 + 정상 상태 기준선
"[E4] 워밍업"
Measure-Launch 4 | Out-Null; Measure-Read | Out-Null; Measure-Read | Out-Null
Start-Sleep -Seconds 5

$a = Get-MemSnap; $l = Measure-Launch 8; $rd = Measure-Read; $b = Get-MemSnap
$d = Get-MemDelta $a $b
$s = & $stats $l
$rows.Add([pscustomobject]@{ cond="A_정상(캐시 따뜻함)"; LaunchMedianMs=$s.MedianMs; LaunchMeanMs=$s.MeanMs; LaunchMinMs=$s.MinMs; LaunchMaxMs=$s.MaxMs; ReadMs=[math]::Round($rd,1); ReadMBps=[math]::Round($corpusMB/($rd/1000),1); HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB }) | Out-Null
"  A 정상: 기동 중앙값 $($s.MedianMs)ms | 코퍼스 읽기 $([math]::Round($rd,1))ms"

# 2) 기본값(①②) 정리 직후
"[E4] 기본값(①②) 정리 직후"
Set-Areas "false"
$r = Invoke-Clean; "  $($r.Diag)"
$a = Get-MemSnap; $l = Measure-Launch 8; $rd = Measure-Read; $b = Get-MemSnap
$d = Get-MemDelta $a $b
$s = & $stats $l
$rows.Add([pscustomobject]@{ cond="B_기본정리(①②) 직후"; LaunchMedianMs=$s.MedianMs; LaunchMeanMs=$s.MeanMs; LaunchMinMs=$s.MinMs; LaunchMaxMs=$s.MaxMs; ReadMs=[math]::Round($rd,1); ReadMBps=[math]::Round($corpusMB/($rd/1000),1); HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB }) | Out-Null
"  B 기본정리: 기동 중앙값 $($s.MedianMs)ms | 코퍼스 읽기 $([math]::Round($rd,1))ms | hard=$($d.HardFaults)"

# 3) 회복 대기 후 공격적(③ 포함) 정리 직후
"[E4] 회복 대기 90s"
Start-Sleep -Seconds 30; Measure-Read | Out-Null; Measure-Read | Out-Null; Start-Sleep -Seconds 30

"[E4] 공격적(③ 전체 스탠바이 퍼지) 정리 직후"
Set-Areas "true"
$r = Invoke-Clean; "  $($r.Diag)"
$a = Get-MemSnap; $l = Measure-Launch 8; $rd = Measure-Read; $b = Get-MemSnap
$d = Get-MemDelta $a $b
$s = & $stats $l
$rows.Add([pscustomobject]@{ cond="C_공격적정리(①②③) 직후"; LaunchMedianMs=$s.MedianMs; LaunchMeanMs=$s.MeanMs; LaunchMinMs=$s.MinMs; LaunchMaxMs=$s.MaxMs; ReadMs=[math]::Round($rd,1); ReadMBps=[math]::Round($corpusMB/($rd/1000),1); HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB }) | Out-Null
"  C 공격적: 기동 중앙값 $($s.MedianMs)ms | 코퍼스 읽기 $([math]::Round($rd,1))ms | hard=$($d.HardFaults) pagesIn=$($d.PagesInMB)MB"

# 4) 원상 복구 + 재확인
Remove-Item $ini -ErrorAction SilentlyContinue
Start-Sleep -Seconds 20
$a = Get-MemSnap; $l = Measure-Launch 8; $rd = Measure-Read; $b = Get-MemSnap
$d = Get-MemDelta $a $b
$s = & $stats $l
$rows.Add([pscustomobject]@{ cond="D_회복후(재캐시)"; LaunchMedianMs=$s.MedianMs; LaunchMeanMs=$s.MeanMs; LaunchMinMs=$s.MinMs; LaunchMaxMs=$s.MaxMs; ReadMs=[math]::Round($rd,1); ReadMBps=[math]::Round($corpusMB/($rd/1000),1); HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB }) | Out-Null

$rows | Export-Csv "$out\latency.csv" -NoTypeInformation
$rows | Format-Table -AutoSize | Out-String -Width 220
"[E4] 완료 (settings.ini 삭제 복구됨) -> $out"
