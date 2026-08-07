# 압력 워크로드 — 지정 GB를 할당·터치하고 소요 ms를 stdout 한 줄로 출력. 종료 시 OS가 전량 회수.
param([int]$GB = 4)
$chunks = New-Object System.Collections.ArrayList
$sw = [System.Diagnostics.Stopwatch]::StartNew()
for ($g = 0; $g -lt $GB; $g++) {
    $b = New-Object byte[] (1GB)
    for ($i = 0; $i -lt $b.Length; $i += 4096) { $b[$i] = 1 }
    $chunks.Add($b) | Out-Null
}
$sw.Stop()
Write-Output ("ALLOC_MS={0:N1}" -f $sw.Elapsed.TotalMilliseconds)
# 두 번째 통과 — 방금 만진 페이지가 아직 RAM에 있는지(=압력 하 유지력) 측정
$sw2 = [System.Diagnostics.Stopwatch]::StartNew()
foreach ($b in $chunks) { for ($i = 0; $i -lt $b.Length; $i += 4096) { $b[$i] = 2 } }
$sw2.Stop()
Write-Output ("RETOUCH_MS={0:N1}" -f $sw2.Elapsed.TotalMilliseconds)
