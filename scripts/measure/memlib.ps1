# memlib.ps1 — Nexa MemKeeper 효용성 측정 공용 함수
# 모든 값은 Win32_PerfRawData_PerfOS_Memory(로케일 무관 영문 속성) + GlobalMemoryStatusEx 기준.

$script:TotalMB = [math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1MB)
$script:Exe = Join-Path (Split-Path (Split-Path $PSScriptRoot)) "target\release\NexaMemKeeper.exe"

function Get-MemSnap {
    $m = Get-CimInstance Win32_PerfRawData_PerfOS_Memory
    $sbCore = $m.StandbyCacheCoreBytes / 1MB
    $sbNorm = $m.StandbyCacheNormalPriorityBytes / 1MB
    $sbRes  = $m.StandbyCacheReserveBytes / 1MB
    [pscustomobject]@{
        t          = (Get-Date)
        AvailMB    = [math]::Round($m.AvailableMBytes, 1)
        InUseMB    = [math]::Round($script:TotalMB - $m.AvailableMBytes, 1)
        FreeMB     = [math]::Round($m.FreeAndZeroPageListBytes / 1MB, 1)
        ModifiedMB = [math]::Round($m.ModifiedPageListBytes / 1MB, 1)
        SbCoreMB   = [math]::Round($sbCore, 1)
        SbNormMB   = [math]::Round($sbNorm, 1)     # 우선순위 1~7 (가치 있는 캐시)
        SbResMB    = [math]::Round($sbRes, 1)      # 우선순위 0 (영역 ② 표적)
        StandbyMB  = [math]::Round($sbCore + $sbNorm + $sbRes, 1)
        CommitMB   = [math]::Round($m.CommittedBytes / 1MB, 1)
        # 누적 카운터(구간 차분용)
        cPageReads = [long]$m.PageReadsPersec        # 하드 폴트 I/O 횟수(디스크 읽기)
        cPagesIn   = [long]$m.PagesInputPersec       # 하드 폴트로 읽어온 페이지 수
        cPagesOut  = [long]$m.PagesOutputPersec      # 페이지파일/디스크로 쓴 페이지 수
        cPageWrite = [long]$m.PageWritesPersec       # 페이지 쓰기 I/O 횟수
        cTransFlt  = [long]$m.TransitionFaultsPersec # 소프트 폴트(스탠바이/수정 목록에서 복귀)
        cPageFlt   = [long]$m.PageFaultsPersec       # 전체 페이지 폴트
        cDemandZ   = [long]$m.DemandZeroFaultsPersec
    }
}

# 두 스냅샷 사이의 구간 통계
function Get-MemDelta {
    param($a, $b)
    $secs = ($b.t - $a.t).TotalSeconds
    [pscustomobject]@{
        Secs        = [math]::Round($secs, 2)
        dAvailMB    = [math]::Round($b.AvailMB - $a.AvailMB, 1)
        dInUseMB    = [math]::Round($b.InUseMB - $a.InUseMB, 1)
        dFreeMB     = [math]::Round($b.FreeMB - $a.FreeMB, 1)
        dModifiedMB = [math]::Round($b.ModifiedMB - $a.ModifiedMB, 1)
        dSbNormMB   = [math]::Round($b.SbNormMB - $a.SbNormMB, 1)
        dSbResMB    = [math]::Round($b.SbResMB - $a.SbResMB, 1)
        dStandbyMB  = [math]::Round($b.StandbyMB - $a.StandbyMB, 1)
        dCommitMB   = [math]::Round($b.CommitMB - $a.CommitMB, 1)
        HardFaults  = $b.cPageReads - $a.cPageReads
        PagesInMB   = [math]::Round(($b.cPagesIn - $a.cPagesIn) * 4096 / 1MB, 1)
        PagesOutMB  = [math]::Round(($b.cPagesOut - $a.cPagesOut) * 4096 / 1MB, 1)
        PageWrites  = $b.cPageWrite - $a.cPageWrite
        SoftFaults  = $b.cTransFlt - $a.cTransFlt
        AllFaults   = $b.cPageFlt - $a.cPageFlt
        HardFltPS   = [math]::Round(($b.cPageReads - $a.cPageReads) / [math]::Max($secs, 0.001), 1)
        SoftFltPS   = [math]::Round(($b.cTransFlt - $a.cTransFlt) / [math]::Max($secs, 0.001), 1)
    }
}

# 정리 1회 실행 — 워커를 직접 호출(상주 셸 무관). 반환: 소요 ms · CPU ms · stderr 진단줄
function Invoke-Clean {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $script:Exe
    $psi.Arguments = "--worker clean"
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $p = [System.Diagnostics.Process]::Start($psi)
    $err = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    $sw.Stop()
    [pscustomobject]@{
        WallMs = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
        CpuMs  = [math]::Round($p.TotalProcessorTime.TotalMilliseconds, 1)
        Exit   = $p.ExitCode
        Diag   = $err.Trim()
    }
}

# 상주 셸 중지/재개 (자동 정리가 통제 실험을 오염시키지 않도록)
function Stop-Shell {
    $ps = Get-Process -Name NexaMemKeeper -ErrorAction SilentlyContinue
    if ($ps) { $ps | Stop-Process -Force; Start-Sleep -Milliseconds 800; return $true }
    return $false
}
function Start-Shell {
    Start-Process -FilePath $script:Exe -WindowStyle Hidden
}

function Show-Snap {
    param($s, $label)
    "{0,-22} Avail {1,7} | InUse {2,7} | Free {3,6} | Mod {4,6} | Sb(0) {5,6} | Sb(1-7) {6,7} | Commit {7,7}" -f `
        $label, $s.AvailMB, $s.InUseMB, $s.FreeMB, $s.ModifiedMB, $s.SbResMB, $s.SbNormMB, $s.CommitMB
}
