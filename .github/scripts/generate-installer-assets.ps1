Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$root = Resolve-Path (Join-Path $PSScriptRoot '../..')
$output = Join-Path $root 'src-tauri/installer/assets'
New-Item -ItemType Directory -Force -Path $output | Out-Null

# Draw a crisp rounded application tile. The bitmap is centered by NSIS,
# so no DPI-dependent full-window background image is required.
$icon = [System.Drawing.Bitmap]::new(96, 96, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$graphics = [System.Drawing.Graphics]::FromImage($icon)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([System.Drawing.Color]::FromArgb(245, 248, 252))

$path = [System.Drawing.Drawing2D.GraphicsPath]::new()
$path.AddArc(8, 8, 36, 36, 180, 90)
$path.AddArc(52, 8, 36, 36, 270, 90)
$path.AddArc(52, 52, 36, 36, 0, 90)
$path.AddArc(8, 52, 36, 36, 90, 90)
$path.CloseFigure()
$iconRectangle = [System.Drawing.Rectangle]::new(8, 8, 80, 80)
$iconBrush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
  $iconRectangle,
  [System.Drawing.Color]::FromArgb(255, 26, 190, 177),
  [System.Drawing.Color]::FromArgb(255, 55, 108, 235),
  [System.Drawing.Drawing2D.LinearGradientMode]::ForwardDiagonal
)
$graphics.FillPath($iconBrush, $path)
$iconBrush.Dispose()
$path.Dispose()

$pen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(235, 255, 255, 255), 4.5)
$pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$graphics.DrawLine($pen, 48, 35, 31, 65)
$graphics.DrawLine($pen, 48, 35, 66, 65)
$pen.Dispose()
$node = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
$graphics.FillEllipse($node, 37, 23, 23, 23)
$graphics.FillEllipse($node, 21, 55, 20, 20)
$graphics.FillEllipse($node, 56, 55, 20, 20)
$node.Dispose()

$icon.Save((Join-Path $output 'app-icon.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$graphics.Dispose()
$icon.Dispose()

Write-Host "Astral NSIS icon generated in $output"
