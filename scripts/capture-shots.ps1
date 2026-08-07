# 기능별 스크린샷 생성 — Wiki·README에 쓰는 이미지를 **재현 가능하게** 만든다.
#
#   pwsh scripts\capture-shots.ps1            # docs\images\*.png 갱신
#   pwsh scripts\capture-shots.ps1 -Exe path  # 특정 빌드로 촬영
#
# 창을 화면 위로 올려 두고 PrintWindow로 **해당 창만** 담는다(바탕화면·다른 앱이 섞이지 않게).
# 촬영용으로 띄운 워커는 끝나면 모두 정리한다.

[CmdletBinding()]
param(
    [string]$Exe = "$PSScriptRoot\..\target\release\NexaMemKeeper.exe",
    [string]$OutDir = "$PSScriptRoot\..\docs\images"
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Shot {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int m);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessageW(IntPtr h, int m, IntPtr w, string l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, int m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr h, EnumProc p, IntPtr l);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
}
"@

function Find-Win([string]$cls) {
    $script:hit = [IntPtr]::Zero
    $cb = [Shot+EnumProc] {
        param($h, $l)
        $sb = New-Object Text.StringBuilder 128
        [void][Shot]::GetClassName($h, $sb, 128)
        if ($sb.ToString() -eq $cls) { $script:hit = $h }
        return $true
    }
    [void][Shot]::EnumWindows($cb, [IntPtr]::Zero)
    return $script:hit
}

function Find-Child([IntPtr]$parent, [string]$cls) {
    $script:kid = [IntPtr]::Zero
    $cb = [Shot+EnumProc] {
        param($h, $l)
        $sb = New-Object Text.StringBuilder 128
        [void][Shot]::GetClassName($h, $sb, 128)
        if ($sb.ToString() -eq $cls) { $script:kid = $h }
        return $true
    }
    [void][Shot]::EnumChildWindows($parent, $cb, [IntPtr]::Zero)
    return $script:kid
}

function Save-Shot([IntPtr]$hwnd, [string]$name) {
    if ($hwnd -eq [IntPtr]::Zero) { throw "창을 찾지 못했습니다: $name" }
    # 화면 좌상단 근처로 옮겨 창이 잘리지 않게 한다(SWP_NOSIZE|SWP_NOZORDER = 0x0005)
    [void][Shot]::SetWindowPos($hwnd, [IntPtr]::Zero, 40, 40, 0, 0, 0x0005)
    [void][Shot]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 700
    $r = New-Object Shot+RECT
    [void][Shot]::GetWindowRect($hwnd, [ref]$r)
    $bmp = New-Object System.Drawing.Bitmap (($r.R - $r.L), ($r.B - $r.T))
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    # 2 = PW_RENDERFULLCONTENT — 커스텀 드로잉 창도 그대로 담긴다
    [void][Shot]::PrintWindow($hwnd, $dc, 2)
    $g.ReleaseHdc($dc)
    $g.Dispose()
    $path = Join-Path $OutDir $name
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host ("  {0}  ({1}x{2})" -f $name, ($r.R - $r.L), ($r.B - $r.T)) -ForegroundColor Green
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Exe = (Resolve-Path $Exe).Path
Write-Host "== 스크린샷 촬영: $Exe ==" -ForegroundColor Cyan
$spawned = @()

try {
    # ── 1) 메인 화면(프로세스 목록) — 기본 보기 = 앱별 묶음
    $spawned += Start-Process -PassThru $Exe -ArgumentList '--worker', 'list'
    Start-Sleep -Seconds 4
    $list = Find-Win 'NexaMemKeeperList'
    Save-Shot $list 'main-list.png'

    # ── 2) 같은 창에서 검색 — 창 제목까지 걸리는 것을 보여 준다
    $edit = Find-Child $list 'Edit'
    [void][Shot]::SendMessageW($edit, 0x000C, [IntPtr]::Zero, 'explorer')   # WM_SETTEXT
    [void][Shot]::PostMessage($list, 0x0111, [IntPtr](300 -bor (0x0300 -shl 16)), $edit) # EN_CHANGE
    Start-Sleep -Seconds 2
    Save-Shot $list 'main-list-search.png'

    # ── 3) 설정 창
    $spawned += Start-Process -PassThru $Exe -ArgumentList '--worker', 'settings'
    Start-Sleep -Seconds 3
    Save-Shot (Find-Win 'NexaMemKeeperSettings') 'settings.png'

    # ── 4) Top 5 팝업 — 상주 셸에 트레이 좌클릭을 흉내 낸 메시지를 보낸다
    #    (WM_APP+1 = WM_TRAY, lParam = WM_LBUTTONUP)
    $shell = Find-Win 'NexaMemKeeperShell'
    if ($shell -ne [IntPtr]::Zero) {
        [void][Shot]::PostMessage($shell, 0x8001, [IntPtr]1, [IntPtr]0x0202)
        Start-Sleep -Seconds 2
        $popup = Find-Win 'NexaMemKeeperPopup'
        if ($popup -ne [IntPtr]::Zero) { Save-Shot $popup 'tray-popup.png' }
        else { Write-Warning '팝업 창을 찾지 못했습니다 — 트레이 셸이 떠 있는지 확인하세요.' }
    } else {
        Write-Warning '상주 셸이 없어 팝업은 건너뜁니다(NexaMemKeeper.exe를 먼저 실행).'
    }
} finally {
    foreach ($p in $spawned) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
}
Get-ChildItem $OutDir -Filter *.png | Select-Object Name, @{n = 'KB'; e = { [math]::Round($_.Length / 1KB) } }
