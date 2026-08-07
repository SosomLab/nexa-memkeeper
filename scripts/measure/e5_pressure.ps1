# E5 — 핵심 검증: "미리 정리해두면 메모리가 빠듯할 때 실제로 빨라지는가"
# 동일 압력 워크로드(4GB 할당·터치)를 [무정리] / [직전 정리] 조건으로 교차 반복(드리프트 상쇄).
$sp = $PSScriptRoot
. "$sp\memlib.ps1"
$out = "$PSScriptRoot\results"; New-Item -ItemType Directory -Force -Path $out | Out-Null
$GB = 4
$rows = [System.Collections.ArrayList]::new()

function Run-Workload {
    param($cond, $round)
    # 상주 셸이 되살아나 자동 정리(임계 90%·저메모리 이벤트)를 쏘면 NOCLEAN 조건이 오염된다.
    # 매 워크로드 직전에 확인·중지하고, 되살아났던 사실은 기록에 남긴다.
    $shellWasUp = (Get-Process -Name NexaMemKeeper -ErrorAction SilentlyContinue) -ne $null
    if ($shellWasUp) { Stop-Shell | Out-Null; "  ⚠ 상주 셸이 실행 중이어서 중지함(오염 방지)" }
    $pre = Get-MemSnap
    $cleanMs = 0; $cleanDiag = ""
    if ($cond -eq "CLEAN") {
        $r = Invoke-Clean; $cleanMs = $r.WallMs; $cleanDiag = $r.Diag
        Start-Sleep -Seconds 2
    }
    $start = Get-MemSnap
    $o = & pwsh -NoProfile -File "$sp\alloc_child.ps1" -GB $GB 2>&1
    $end = Get-MemSnap
    $d = Get-MemDelta $start $end
    $allocMs   = [double](($o | Select-String 'ALLOC_MS=(.+)').Matches.Groups[1].Value -replace ',','')
    $retouchMs = [double](($o | Select-String 'RETOUCH_MS=(.+)').Matches.Groups[1].Value -replace ',','')
    $rows.Add([pscustomobject]@{
        round=$round; cond=$cond
        PreAvailMB=$pre.AvailMB; PreActiveMB=[math]::Round($script:TotalMB-$pre.AvailMB-$pre.ModifiedMB,1)
        StartAvailMB=$start.AvailMB
        CleanMs=$cleanMs
        AllocMs=[math]::Round($allocMs,1); AllocMBps=[math]::Round(($GB*1024)/($allocMs/1000),1)
        RetouchMs=[math]::Round($retouchMs,1); RetouchMBps=[math]::Round(($GB*1024)/($retouchMs/1000),1)
        HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB; PagesOutMB=$d.PagesOutMB
        SoftFaults=$d.SoftFaults; WindowSecs=$d.Secs
    }) | Out-Null
    "  [{0} r{1}] preAvail={2}MB alloc={3:N0}ms ({4:N0}MB/s) retouch={5:N0}ms hard={6} pagesOut={7}MB pagesIn={8}MB" -f `
        $cond, $round, $pre.AvailMB, $allocMs, (($GB*1024)/($allocMs/1000)), $retouchMs, $d.HardFaults, $d.PagesOutMB, $d.PagesInMB
    if ($cleanDiag) { "     clean: $cleanDiag" }
}

"[E5] 상주 셸 중지"; Stop-Shell | Out-Null
"[E5] 워크로드 = {0}GB 할당·터치 / 교차 3라운드" -f $GB
"[E5] 초기 정착 90s"; Start-Sleep -Seconds 90

for ($round = 1; $round -le 3; $round++) {
    Run-Workload "NOCLEAN" $round
    "  정착 90s..."; Start-Sleep -Seconds 90
    Run-Workload "CLEAN" $round
    if ($round -lt 3) { "  정착 90s..."; Start-Sleep -Seconds 90 }
}

$rows | Export-Csv "$out\pressure.csv" -NoTypeInformation
""
"=== 조건별 요약 ==="
$rows | Group-Object cond | ForEach-Object {
    $g = $_.Group
    [pscustomobject]@{
        cond=$_.Name; n=$g.Count
        AllocMs_mean=[math]::Round(($g.AllocMs|Measure-Object -Average).Average,1)
        AllocMBps_mean=[math]::Round(($g.AllocMBps|Measure-Object -Average).Average,1)
        RetouchMs_mean=[math]::Round(($g.RetouchMs|Measure-Object -Average).Average,1)
        HardFaults_mean=[math]::Round(($g.HardFaults|Measure-Object -Average).Average,0)
        PagesInMB_mean=[math]::Round(($g.PagesInMB|Measure-Object -Average).Average,1)
        PagesOutMB_mean=[math]::Round(($g.PagesOutMB|Measure-Object -Average).Average,1)
    }
} | Format-Table -AutoSize | Out-String -Width 200
$rows | Format-Table -AutoSize | Out-String -Width 220
"[E5] 완료 -> $out"
