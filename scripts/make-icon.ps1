# make-icon.ps1 — 앱 아이콘 생성(재현 가능한 소스). 산출물: crates/mk-app/assets/
#
# 디자인 = nexa-dir2 계열 계승(assets/nexa-dir-1024.png 참조):
#   둥근 사각 컨테이너 + 파랑 외곽선(#3B82F6) + 남색 그라데이션 바디 + 초록 글리프(#22C55E)
# 이 앱의 정체성: 메모리 회수 + 사용률 —
#   ① 도넛 게이지 80% 호(트레이의 색 단계 임계와 같은 값 — docs/05 §4-3)
#   ② 가운데 메모리 모듈(RAM) 글리프 + 아래 방향 화살표(회수)
#
# 실행: pwsh -File scripts/make-icon.ps1   (System.Drawing만 사용 — 외부 도구 0)

param([string]$OutDir = "crates/mk-app/assets")

Add-Type -AssemblyName System.Drawing

$ErrorActionPreference = 'Stop'
if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Force $OutDir | Out-Null }

# ── 팔레트(nexa-dir2 아이콘에서 추출)
$cBorder = [System.Drawing.Color]::FromArgb(255, 59, 130, 246)   # #3B82F6 파랑 외곽
$cBodyTop = [System.Drawing.Color]::FromArgb(255, 46, 55, 72)    # #2E3748
$cBodyBot = [System.Drawing.Color]::FromArgb(255, 24, 29, 38)    # #181D26
$cGreen = [System.Drawing.Color]::FromArgb(255, 34, 197, 94)     # #22C55E 초록 글리프
$cTrack = [System.Drawing.Color]::FromArgb(255, 55, 65, 81)      # 게이지 트랙(어두운 회청)

function New-RoundedPath([single]$x, [single]$y, [single]$w, [single]$h, [single]$r) {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $r * 2
  $p.AddArc($x, $y, $d, $d, 180, 90)
  $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
  $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
  $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
  $p.CloseFigure()
  return $p
}

# ── 1024 마스터 렌더
$S = 1024
$bmp = New-Object System.Drawing.Bitmap($S, $S, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.PixelOffsetMode = 'HighQuality'
$g.Clear([System.Drawing.Color]::Transparent)

# 컨테이너(둥근 사각) — 남색 세로 그라데이션 + 파랑 외곽선
$inset = 44.0
$body = New-RoundedPath $inset $inset ($S - $inset * 2) ($S - $inset * 2) 190
$rect = New-Object System.Drawing.RectangleF($inset, $inset, ($S - $inset * 2), ($S - $inset * 2))
$grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $cBodyTop, $cBodyBot, 90)
$g.FillPath($grad, $body)
$penBorder = New-Object System.Drawing.Pen($cBorder, 34)
$penBorder.Alignment = 'Center'
$g.DrawPath($penBorder, $body)

# ── 도넛 게이지 — 80%(트레이 '주의' 임계와 동일 값)
$cx = 512.0; $cy = 500.0; $rad = 285.0; $thick = 86.0
$arcRect = New-Object System.Drawing.RectangleF(($cx - $rad), ($cy - $rad), ($rad * 2), ($rad * 2))
$penTrack = New-Object System.Drawing.Pen($cTrack, $thick)
$penTrack.StartCap = 'Round'; $penTrack.EndCap = 'Round'
$g.DrawArc($penTrack, $arcRect, 0, 360)
$penGauge = New-Object System.Drawing.Pen($cGreen, $thick)
$penGauge.StartCap = 'Round'; $penGauge.EndCap = 'Round'
# 12시(-90°)에서 시계 방향 80% = 288°
$g.DrawArc($penGauge, $arcRect, -90, 288)

# ── 가운데 메모리 모듈(RAM) — 초록 글리프
$mw = 250.0; $mh = 150.0
$mx = $cx - $mw / 2; $my = $cy - $mh / 2 - 26
$chip = New-RoundedPath $mx $my $mw $mh 26
$brGreen = New-Object System.Drawing.SolidBrush($cGreen)
$g.FillPath($brGreen, $chip)
# 모듈 핀(아래 4개) — 바디 색으로 파내 칩처럼 보이게
$brBody = New-Object System.Drawing.SolidBrush($cBodyBot)
$pinW = 30.0; $pinH = 34.0; $gap = 22.0
$startX = $mx + ($mw - ($pinW * 4 + $gap * 3)) / 2
for ($i = 0; $i -lt 4; $i++) {
  $px = $startX + $i * ($pinW + $gap)
  $g.FillRectangle($brBody, $px, ($my + $mh - $pinH), $pinW, $pinH)
}
# 모듈 노치(왼쪽 아래) — RAM 식별 요소
$g.FillRectangle($brBody, ($mx + 40), ($my + $mh - $pinH), 16, $pinH)

# ── 회수 화살표(아래 방향) — 모듈 아래
$ax = $cx; $ay = $my + $mh + 52
$penArrow = New-Object System.Drawing.Pen($cGreen, 44)
$penArrow.StartCap = 'Round'; $penArrow.EndCap = 'Round'
$penArrow.LineJoin = 'Round'
$g.DrawLine($penArrow, $ax, $ay, $ax, ($ay + 86))
$chev = New-Object System.Drawing.Drawing2D.GraphicsPath
$chev.AddLine(($ax - 62), ($ay + 34), $ax, ($ay + 96))
$chev.AddLine($ax, ($ay + 96), ($ax + 62), ($ay + 34))
$g.DrawPath($penArrow, $chev)

$g.Dispose()

# ── PNG 마스터 저장
$png1024 = Join-Path $OutDir "nexa-memkeeper-1024.png"
$bmp.Save($png1024, [System.Drawing.Imaging.ImageFormat]::Png)

# ── 다중 해상도 ICO 조립(각 프레임 PNG 인코딩 — Vista+ 지원)
$sizes = @(16, 20, 24, 32, 40, 48, 64, 128, 256)
$frames = @()
foreach ($sz in $sizes) {
  $fb = New-Object System.Drawing.Bitmap($sz, $sz, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $fg = [System.Drawing.Graphics]::FromImage($fb)
  $fg.InterpolationMode = 'HighQualityBicubic'
  $fg.SmoothingMode = 'AntiAlias'
  $fg.PixelOffsetMode = 'HighQuality'
  $fg.Clear([System.Drawing.Color]::Transparent)
  $fg.DrawImage($bmp, (New-Object System.Drawing.Rectangle(0, 0, $sz, $sz)))
  $fg.Dispose()
  $ms = New-Object System.IO.MemoryStream
  $fb.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $frames += , @{ Size = $sz; Bytes = $ms.ToArray() }
  $ms.Dispose(); $fb.Dispose()
}
$bmp.Dispose()

$icoPath = Join-Path $OutDir "nexa-memkeeper.ico"
$fs = [System.IO.File]::Create($icoPath)
$bw = New-Object System.IO.BinaryWriter($fs)
$bw.Write([uint16]0); $bw.Write([uint16]1); $bw.Write([uint16]$frames.Count)   # ICONDIR
$offset = 6 + 16 * $frames.Count
foreach ($f in $frames) {
  $dim = if ($f.Size -ge 256) { 0 } else { $f.Size }
  $bw.Write([byte]$dim); $bw.Write([byte]$dim)      # width, height (0 = 256)
  $bw.Write([byte]0); $bw.Write([byte]0)            # colorCount, reserved
  $bw.Write([uint16]1); $bw.Write([uint16]32)       # planes, bitCount
  $bw.Write([uint32]$f.Bytes.Length)
  $bw.Write([uint32]$offset)
  $offset += $f.Bytes.Length
}
foreach ($f in $frames) { $bw.Write($f.Bytes) }
$bw.Flush(); $bw.Close(); $fs.Close()

"생성 완료:"
"  {0} ({1:N0} bytes)" -f $png1024, (Get-Item $png1024).Length
"  {0} ({1:N0} bytes, {2}종 해상도)" -f $icoPath, (Get-Item $icoPath).Length, $frames.Count
