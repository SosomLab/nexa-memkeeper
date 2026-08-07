# E3 — 트림의 직접 비용: 워킹셋을 뺏긴 프로세스가 되돌리는 데 드는 실제 시간
# 통제 워크로드 1200MB를 만들고, (1) 정상 재접근 (2) 트림 직후 재접근 (3) 재정착 후 재접근 시간을 비교.
$sp = $PSScriptRoot
. "$sp\memlib.ps1"
$out = "$PSScriptRoot\results"; New-Item -ItemType Directory -Force -Path $out | Out-Null

$SizeMB = 1200
$me = [System.Diagnostics.Process]::GetCurrentProcess()

function Touch-All {
    param($buf)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    for ($i = 0; $i -lt $buf.Length; $i += 4096) { $buf[$i] = [byte](($buf[$i] + 1) -band 0xFF) }
    $sw.Stop()
    return $sw.Elapsed.TotalMilliseconds
}
function WsMB { $me.Refresh(); return [math]::Round($me.WorkingSet64/1MB, 1) }

$rows = [System.Collections.ArrayList]::new()
function Rec {
    param($phase, $ms, $d)
    $rows.Add([pscustomobject]@{
        phase=$phase; TouchMs=[math]::Round($ms,1)
        MBps=[math]::Round($SizeMB/($ms/1000),1)
        WsMB=(WsMB)
        HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB; PagesOutMB=$d.PagesOutMB
        SoftFaults=$d.SoftFaults
    }) | Out-Null
}

"[E3] 상주 셸 중지"; Stop-Shell | Out-Null
"[E3] {0}MB 할당·초기 터치" -f $SizeMB
$buf = New-Object byte[] ($SizeMB * 1MB)
$a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
Rec "1_초기터치(할당직후)" $ms (Get-MemDelta $a $b)

# 정상 상태 재접근 3회 — 워킹셋이 살아 있을 때의 기준값
for ($k = 1; $k -le 3; $k++) {
    Start-Sleep -Seconds 2
    $a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
    Rec "2_정상재접근#$k" $ms (Get-MemDelta $a $b)
}

# --- 표적 트림(영역 ① 단일 프로세스) 직후 재접근 ---
"[E3] 표적 트림 후 재접근"
$myPid = $PID
& $script:Exe --worker trim $myPid 2>&1 | Out-Null
Start-Sleep -Milliseconds 500
"  트림 후 WS = $(WsMB)MB"
$a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
Rec "3_표적트림직후재접근" $ms (Get-MemDelta $a $b)

Start-Sleep -Seconds 2
$a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
Rec "4_트림후재정착" $ms (Get-MemDelta $a $b)

# --- 전체 정리(clean) 직후 재접근 ---
"[E3] 전체 정리 후 재접근"
Start-Sleep -Seconds 5
$r = Invoke-Clean
"  $($r.Diag)"
Start-Sleep -Milliseconds 500
"  정리 후 WS = $(WsMB)MB"
$a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
Rec "5_전체정리직후재접근" $ms (Get-MemDelta $a $b)

Start-Sleep -Seconds 2
$a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
Rec "6_정리후재정착" $ms (Get-MemDelta $a $b)

# --- 트림 후 60초 방치(모디파이드 라이터가 페이지파일로 밀어낼 시간) 뒤 재접근 ---
"[E3] 정리 후 60s 방치 뒤 재접근(페이지파일 왕복 여부 확인)"
$r = Invoke-Clean; "  $($r.Diag)"
$a = Get-MemSnap
Start-Sleep -Seconds 60
$b = Get-MemSnap
$dIdle = Get-MemDelta $a $b
"  방치 60s 동안: pagesOut=$($dIdle.PagesOutMB)MB hard=$($dIdle.HardFaults)"
$a = Get-MemSnap; $ms = Touch-All $buf; $b = Get-MemSnap
Rec "7_정리+60s방치후재접근" $ms (Get-MemDelta $a $b)

$buf = $null; [GC]::Collect()
$rows | Export-Csv "$out\trimcost.csv" -NoTypeInformation
$rows | Format-Table -AutoSize | Out-String -Width 200
"[E3] 완료 -> $out"
