# E6 — 사용자 질문 직격: "주기적 정리"가 평상시 성능에 도움이 되는가
# 5분 블록을 [무정리] / [60초마다 정리] 교대 4블록. 각 블록에서 60초마다 동일 벤치마크를 돌려 지연 비교.
$sp = $PSScriptRoot
. "$sp\memlib.ps1"
$out = "$PSScriptRoot\results"; New-Item -ItemType Directory -Force -Path $out | Out-Null

$corpus = Get-ChildItem "C:\Windows\System32\*.dll" -File -ErrorAction SilentlyContinue |
          Sort-Object Name | Select-Object -First 150
$corpusMB = [math]::Round((($corpus | Measure-Object Length -Sum).Sum)/1MB, 1)

function Bench {
    # 앱 기동 3회 + 코퍼스 읽기 1회 = 1 표본
    $lt = @()
    for ($i = 0; $i -lt 3; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        Start-Process -FilePath "pwsh" -ArgumentList "-NoProfile","-NoLogo","-Command","exit" -Wait -WindowStyle Hidden
        $sw.Stop(); $lt += $sw.Elapsed.TotalMilliseconds
    }
    $buf = New-Object byte[] 1048576
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    foreach ($f in $corpus) {
        $fs = [System.IO.File]::OpenRead($f.FullName)
        while ($fs.Read($buf, 0, $buf.Length) -gt 0) { }
        $fs.Dispose()
    }
    $sw.Stop()
    [pscustomobject]@{
        LaunchMs = [math]::Round((($lt | Sort-Object)[1]), 1)   # 3회 중앙값
        ReadMs   = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
    }
}

$samples = [System.Collections.ArrayList]::new()
$blocks  = [System.Collections.ArrayList]::new()

"[E6] 상주 셸 중지"; Stop-Shell | Out-Null
"[E6] 코퍼스 $($corpus.Count)개 / $corpusMB MB · 워밍업"
Bench | Out-Null; Bench | Out-Null
Start-Sleep -Seconds 20

$plan = @("CONTROL","PERIODIC","CONTROL","PERIODIC")
$blockIdx = 0
foreach ($cond in $plan) {
    $blockIdx++
    "[E6] 블록 $blockIdx = $cond (5분) $(Get-Date -Format HH:mm:ss)"
    $bA = Get-MemSnap
    $cleanCount = 0; $cleanMsTotal = 0
    for ($s = 1; $s -le 5; $s++) {
        $tick = Get-Date
        # 상주 셸이 되살아나면 CONTROL 블록에 자동 정리가 섞인다 — 매 표본 직전 차단
        if (Get-Process -Name NexaMemKeeper -ErrorAction SilentlyContinue) {
            Stop-Shell | Out-Null; "    ⚠ 상주 셸 중지(오염 방지)"
        }
        if ($cond -eq "PERIODIC") {
            $r = Invoke-Clean; $cleanCount++; $cleanMsTotal += $r.WallMs
            Start-Sleep -Seconds 2
        }
        $m0 = Get-MemSnap
        $bm = Bench
        $m1 = Get-MemSnap
        $d = Get-MemDelta $m0 $m1
        $samples.Add([pscustomobject]@{
            block=$blockIdx; cond=$cond; sample=$s
            AvailMB=$m0.AvailMB; ModifiedMB=$m0.ModifiedMB
            ActiveMB=[math]::Round($script:TotalMB-$m0.AvailMB-$m0.ModifiedMB,1)
            LaunchMs=$bm.LaunchMs; ReadMs=$bm.ReadMs
            HardFaults=$d.HardFaults; PagesInMB=$d.PagesInMB
        }) | Out-Null
        "    s$s avail=$($m0.AvailMB)MB launch=$($bm.LaunchMs)ms read=$($bm.ReadMs)ms hard=$($d.HardFaults)"
        $rest = 60 - ((Get-Date) - $tick).TotalSeconds
        if ($rest -gt 0) { Start-Sleep -Seconds $rest }
    }
    $bB = Get-MemSnap
    $bd = Get-MemDelta $bA $bB
    $blocks.Add([pscustomobject]@{
        block=$blockIdx; cond=$cond; Secs=$bd.Secs; Cleans=$cleanCount
        CleanMsTotal=[math]::Round($cleanMsTotal,1)
        HardFaults=$bd.HardFaults; PagesInMB=$bd.PagesInMB; PagesOutMB=$bd.PagesOutMB
        SoftFaults=$bd.SoftFaults
    }) | Out-Null
    "  블록 $blockIdx 합계: hard=$($bd.HardFaults) pagesIn=$($bd.PagesInMB)MB pagesOut=$($bd.PagesOutMB)MB soft=$($bd.SoftFaults)"
}

$samples | Export-Csv "$out\samples.csv" -NoTypeInformation
$blocks  | Export-Csv "$out\blocks.csv" -NoTypeInformation

""
"=== 조건별 요약 (표본 10개씩) ==="
$samples | Group-Object cond | ForEach-Object {
    $g = $_.Group
    $lm = ($g.LaunchMs | Sort-Object); $rm = ($g.ReadMs | Sort-Object)
    [pscustomobject]@{
        cond=$_.Name; n=$g.Count
        Launch_median=$lm[[int]($lm.Count/2)]
        Launch_mean=[math]::Round(($g.LaunchMs|Measure-Object -Average).Average,1)
        Read_median=$rm[[int]($rm.Count/2)]
        Read_mean=[math]::Round(($g.ReadMs|Measure-Object -Average).Average,1)
        Avail_mean=[math]::Round(($g.AvailMB|Measure-Object -Average).Average,1)
        Active_mean=[math]::Round(($g.ActiveMB|Measure-Object -Average).Average,1)
        HardFaults_sum=($g.HardFaults|Measure-Object -Sum).Sum
    }
} | Format-Table -AutoSize | Out-String -Width 200
"=== 블록별 디스크 I/O ==="
$blocks | Format-Table -AutoSize | Out-String -Width 200
"[E6] 완료 -> $out"
