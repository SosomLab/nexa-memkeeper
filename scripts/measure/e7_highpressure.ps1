# E7 — 고압력 구간 재측정(E-3): "여유가 빠듯할 때는 정리가 도움이 되는가"
#
# 21 §10의 한계 2를 메운다. 앞선 실험(E1~E6)은 여유가 넉넉한 평상시 상태에서 측정했으므로,
# "진짜 부족할 때는 다를 수 있다"는 반론이 열려 있었다. 여기서는 **압력을 걸어 둔 채로**
# D축(체감 성능 = 앱 기동·파일 읽기)을 [무정리] / [직전 정리] 교차로 비교한다.
#
#   pwsh scripts\measure\e7_highpressure.ps1 [-TargetAvailMB 800] [-Rounds 3]
#
# ⚠️ 측정 중 메모리를 의도적으로 채우므로 PC가 느려진다. 점유 프로세스는 **어떤 경우에도**
#    finally에서 종료한다(Ctrl-C 포함). 하한(HardFloorMB) 아래로는 내려가지 않는다.

[CmdletBinding()]
param(
    [int]$TargetAvailMB = 800,   # 이 값까지 여유를 떨어뜨린 뒤 측정
    [int]$HardFloorMB = 400,     # 이 아래로는 절대 내려가지 않는다(시스템 보호)
    [int]$Rounds = 3
)

$ErrorActionPreference = 'Stop'
$sp = $PSScriptRoot
. "$sp\memlib.ps1"
$out = "$sp\results"; New-Item -ItemType Directory -Force -Path $out | Out-Null

# --- 워크로드: 앱 기동(pwsh 100여 DLL 적재) / 고정 코퍼스 읽기 --- (E4와 같은 잣대)
function Measure-Launch {
    param($n = 5)
    $t = @()
    for ($i = 0; $i -lt $n; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $null = Start-Process -FilePath "pwsh" -ArgumentList "-NoProfile", "-NoLogo", "-Command", "exit" `
            -Wait -PassThru -WindowStyle Hidden
        $sw.Stop(); $t += $sw.Elapsed.TotalMilliseconds
    }
    ($t | Sort-Object)[[int]($t.Count / 2)]   # 중앙값(기동은 편차가 커 평균보다 안정적)
}

$corpus = Get-ChildItem "C:\Windows\System32\*.dll" -File -ErrorAction SilentlyContinue |
    Sort-Object Name | Select-Object -First 200
function Measure-Read {
    $buf = New-Object byte[] 1048576
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    foreach ($f in $corpus) {
        $fs = [System.IO.File]::OpenRead($f.FullName)
        while ($fs.Read($buf, 0, $buf.Length) -gt 0) { }
        $fs.Dispose()
    }
    $sw.Stop(); $sw.Elapsed.TotalMilliseconds
}

# --- 압력 유지 프로세스: 목표 여유까지 채우고 **붙들고 있는다**(측정 내내 압력 유지) ---
$hogScript = @'
param([int]$TargetAvailMB, [int]$HardFloorMB, [int]$CapMB)
# 페이지를 **계속 만져야** 압력이 유지된다. PowerShell 루프로는 수 GB를 주기적으로 만지기에 너무
# 느려서(첫 시도에서 압력이 1~2분 만에 풀렸다) 접촉 루프만 C#으로 컴파일해 쓴다.
Add-Type @"
public static class Hog {
  public static void Touch(byte[] b, byte v) {
    for (int i = 0; i < b.Length; i += 4096) b[i] = v;
  }
}
"@
$held = New-Object System.Collections.ArrayList
$chunk = 256MB
$total = 0
while ($true) {
    $avail = (Get-CimInstance Win32_PerfRawData_PerfOS_Memory).AvailableMBytes
    if ($avail -le $TargetAvailMB -or $avail -le $HardFloorMB -or $total -ge $CapMB) { break }
    try {
        $b = New-Object byte[] $chunk
        [Hog]::Touch($b, 1)      # 커밋만으로는 압력이 안 생긴다 — 실제로 물리 페이지를 차지시킨다
        [void]$held.Add($b); $total += 256
    } catch { break }
}
"HOG_HELD_MB=$total"
# 부모가 종료시킬 때까지 붙들고 **매 주기 전 구간을 다시 만진다**. 첫 바이트만 만지면 나머지가
# 대기 목록으로 밀려나 여유가 도로 늘어난다(실측: 2분 만에 849MB → 8GB로 압력 소멸).
$v = 2
while ($true) {
    foreach ($b in $held) { [Hog]::Touch($b, $v) }
    $v = ($v + 1) % 250 + 1
    Start-Sleep -Milliseconds 500
}
'@
$hogPath = Join-Path $env:TEMP "mk_hog.ps1"
Set-Content -Path $hogPath -Value $hogScript -Encoding UTF8

$rows = [System.Collections.ArrayList]::new()
$hog = $null
$shellWasUp = $false

try {
    "[E7] 상주 셸 중지(자동 정리가 통제 실험을 오염시키지 않게)"
    $shellWasUp = Stop-Shell
    $base = Get-MemSnap
    Show-Snap $base "기준(압력 전)"

    $cap = [int]($script:TotalMB * 0.7)
    "[E7] 압력 생성 — 여유 {0}MB까지(하한 {1}MB, 상한 {2}MB 점유)" -f $TargetAvailMB, $HardFloorMB, $cap
    $hog = Start-Process -PassThru -WindowStyle Hidden pwsh `
        -ArgumentList "-NoProfile", "-File", $hogPath, "-TargetAvailMB", $TargetAvailMB,
        "-HardFloorMB", $HardFloorMB, "-CapMB", $cap
    # 채우는 데 시간이 걸린다 — 목표에 닿거나 3분이 지나면 진행
    $deadline = (Get-Date).AddMinutes(3)
    do {
        Start-Sleep -Seconds 3
        $now = Get-MemSnap
    } while ($now.AvailMB -gt $TargetAvailMB -and (Get-Date) -lt $deadline -and -not $hog.HasExited)
    Show-Snap $now "압력 후"
    "[E7] 정착 30s"; Start-Sleep -Seconds 30

    foreach ($r in 1..$Rounds) {
        # 라운드마다 순서를 뒤집어 드리프트(캐시 예열 등)를 상쇄한다
        $order = if ($r % 2 -eq 1) { @("NOCLEAN", "CLEAN") } else { @("CLEAN", "NOCLEAN") }
        foreach ($cond in $order) {
            $pre = Get-MemSnap
            $cleanMs = 0; $diag = ""
            if ($cond -eq "CLEAN") {
                $c = Invoke-Clean; $cleanMs = $c.WallMs; $diag = $c.Diag
                Start-Sleep -Seconds 2
            }
            $start = Get-MemSnap
            $launchMs = Measure-Launch
            $readMs = Measure-Read
            $end = Get-MemSnap
            $d = Get-MemDelta $start $end
            [void]$rows.Add([pscustomobject]@{
                    round       = $r; cond = $cond
                    PreAvailMB  = $pre.AvailMB
                    PreActiveMB = [math]::Round($script:TotalMB - $pre.AvailMB - $pre.ModifiedMB, 1)
                    CleanMs     = $cleanMs
                    LaunchMs    = [math]::Round($launchMs, 1)
                    ReadMs      = [math]::Round($readMs, 1)
                    HardFaults  = $d.HardFaults; PagesInMB = $d.PagesInMB; PagesOutMB = $d.PagesOutMB
                    SoftFaults  = $d.SoftFaults
                })
            "  [{0} r{1}] avail={2}MB launch={3:N0}ms read={4:N0}ms hard={5} pagesIn={6}MB" -f `
                $cond, $r, $pre.AvailMB, $launchMs, $readMs, $d.HardFaults, $d.PagesInMB
            if ($diag) { "     clean: $diag" }
            Start-Sleep -Seconds 10   # 다음 조건 전 안정화
        }
    }
} finally {
    if ($hog -and -not $hog.HasExited) { Stop-Process -Id $hog.Id -Force -ErrorAction SilentlyContinue }
    Remove-Item $hogPath -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 3
    Show-Snap (Get-MemSnap) "정리 후(압력 해제)"
    if ($shellWasUp) { Start-Shell; "[E7] 상주 셸 재기동" }
}

$rows | Export-Csv "$out\highpressure.csv" -NoTypeInformation -Encoding UTF8
""
"=== 요약(중앙값) ==="
foreach ($c in @("NOCLEAN", "CLEAN")) {
    $g = $rows | Where-Object cond -EQ $c
    if (-not $g) { continue }
    $l = ($g.LaunchMs | Sort-Object)[[int]($g.Count / 2)]
    $rd = ($g.ReadMs | Sort-Object)[[int]($g.Count / 2)]
    $hf = ($g.HardFaults | Measure-Object -Average).Average
    "{0,-8} launch={1,8:N1}ms  read={2,8:N1}ms  hardFaults={3,8:N0}  avail={4:N0}MB" -f `
        $c, $l, $rd, $hf, (($g.PreAvailMB | Measure-Object -Average).Average)
}
"결과 CSV: $out\highpressure.csv"
