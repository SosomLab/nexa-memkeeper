# E1 — 정리 1회의 회수 효과·출처 분해·사후 지속성·되돌림 비용
# 구간: [정착 180s] → [대조 기준선 90s 무정리] → [정리] → [사후 관찰 300s]
$sp = $PSScriptRoot
. "$sp\memlib.ps1"
$out = "$PSScriptRoot\results"
New-Item -ItemType Directory -Force -Path $out | Out-Null

function Sample-Loop {
    param($seconds, $tag, $rows)
    $end = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $end) {
        $s = Get-MemSnap
        $rows.Add([pscustomobject]@{
            tag = $tag; t = $s.t.ToString("HH:mm:ss")
            AvailMB = $s.AvailMB; FreeMB = $s.FreeMB; ModifiedMB = $s.ModifiedMB
            SbResMB = $s.SbResMB; SbNormMB = $s.SbNormMB; StandbyMB = $s.StandbyMB
            ActiveMB = [math]::Round($script:TotalMB - $s.AvailMB - $s.ModifiedMB, 1)
            CommitMB = $s.CommitMB
            cPageReads = $s.cPageReads; cPagesOut = $s.cPagesOut; cTransFlt = $s.cTransFlt
        }) | Out-Null
        Start-Sleep -Seconds 5
    }
}

# 상위 프로세스 워킹셋(되돌림 관찰용)
function Snap-Procs {
    param($tag)
    Get-Process | Sort-Object WorkingSet64 -Descending | Select-Object -First 12 |
      ForEach-Object { [pscustomobject]@{ tag=$tag; Name=$_.ProcessName; Id=$_.Id; WsMB=[math]::Round($_.WorkingSet64/1MB,1) } }
}

$log = [System.Collections.ArrayList]::new()
$procs = [System.Collections.ArrayList]::new()

"[{0}] 상주 셸 중지" -f (Get-Date -Format HH:mm:ss)
$wasRunning = Stop-Shell
"  stopped=$wasRunning"

"[{0}] 정착 대기 180s (직전 스모크 정리의 여파 소멸)" -f (Get-Date -Format HH:mm:ss)
Sample-Loop 180 "settle" $log

"[{0}] 대조 기준선 90s (무정리) — 자연 변동·자연 하드폴트 측정" -f (Get-Date -Format HH:mm:ss)
$baseA = Get-MemSnap
Sample-Loop 90 "baseline" $log
$baseB = Get-MemSnap
$dBase = Get-MemDelta $baseA $baseB
$dBase | Export-Csv "$out\baseline_delta.csv" -NoTypeInformation
"  기준선: dAvail=$($dBase.dAvailMB)MB hard=$($dBase.HardFaults) soft=$($dBase.SoftFaults) pagesOut=$($dBase.PagesOutMB)MB"

Snap-Procs "t0_before" | ForEach-Object { $procs.Add($_) | Out-Null }

"[{0}] 정리 실행" -f (Get-Date -Format HH:mm:ss)
$pre = Get-MemSnap
$r = Invoke-Clean
$post = Get-MemSnap
$dClean = Get-MemDelta $pre $post
[pscustomobject]@{
    WallMs=$r.WallMs; CpuMs=$r.CpuMs; Exit=$r.Exit; Diag=$r.Diag
    PreAvail=$pre.AvailMB; PostAvail=$post.AvailMB
    PreActive=[math]::Round($script:TotalMB-$pre.AvailMB-$pre.ModifiedMB,1)
    PostActive=[math]::Round($script:TotalMB-$post.AvailMB-$post.ModifiedMB,1)
    PreModified=$pre.ModifiedMB; PostModified=$post.ModifiedMB
    dAvailMB=$dClean.dAvailMB; SoftFaults=$dClean.SoftFaults; HardFaults=$dClean.HardFaults
} | Export-Csv "$out\clean_event.csv" -NoTypeInformation
"  $($r.Diag)"
"  wall=$($r.WallMs)ms cpu=$($r.CpuMs)ms | Active $($pre.ModifiedMB) -> Modified $($post.ModifiedMB)MB"

Snap-Procs "t1_after" | ForEach-Object { $procs.Add($_) | Out-Null }

"[{0}] 사후 관찰 300s" -f (Get-Date -Format HH:mm:ss)
$postA = Get-MemSnap
Sample-Loop 300 "after" $log
$postB = Get-MemSnap
$dAfter = Get-MemDelta $postA $postB
$dAfter | Export-Csv "$out\after_delta.csv" -NoTypeInformation
"  사후300s: dAvail=$($dAfter.dAvailMB)MB hard=$($dAfter.HardFaults) soft=$($dAfter.SoftFaults) pagesOut=$($dAfter.PagesOutMB)MB pagesIn=$($dAfter.PagesInMB)MB"

Snap-Procs "t2_plus300" | ForEach-Object { $procs.Add($_) | Out-Null }

$log | Export-Csv "$out\timeline.csv" -NoTypeInformation
$procs | Export-Csv "$out\procs.csv" -NoTypeInformation
"[{0}] E1 완료 -> $out" -f (Get-Date -Format HH:mm:ss)
