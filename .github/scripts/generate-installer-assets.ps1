Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$root = Resolve-Path (Join-Path $PSScriptRoot '../..')
$output = Join-Path $root 'src-tauri/installer/assets'
New-Item -ItemType Directory -Force -Path $output | Out-Null

# All decoration and text live in one scalable bitmap. NSIS only owns the
# centered native install button, so DPI cannot separate icon/text alignment.
$bitmap = [System.Drawing.Bitmap]::new(640, 460, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$rectangle = [System.Drawing.Rectangle]::new(0, 0, 640, 460)
$brush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
  $rectangle,
  [System.Drawing.Color]::FromArgb(255, 246, 251, 255),
  [System.Drawing.Color]::FromArgb(255, 246, 241, 253),
  [System.Drawing.Drawing2D.LinearGradientMode]::ForwardDiagonal
)
$graphics.FillRectangle($brush, $rectangle)
$brush.Dispose()

$glow = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(52, 80, 210, 235))
$graphics.FillEllipse($glow, 430, -145, 350, 350)
$glow.Dispose()
$glow = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(36, 150, 116, 230))
$graphics.FillEllipse($glow, -175, 295, 380, 260)
$glow.Dispose()
$glow = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(24, 35, 190, 180))
$graphics.FillEllipse($glow, 45, 52, 12, 12)
$graphics.FillEllipse($glow, 574, 342, 9, 9)
$graphics.FillEllipse($glow, 530, 78, 6, 6)
$glow.Dispose()

$path = [System.Drawing.Drawing2D.GraphicsPath]::new()
$path.AddArc(280, 44, 32, 32, 180, 90)
$path.AddArc(328, 44, 32, 32, 270, 90)
$path.AddArc(328, 92, 32, 32, 0, 90)
$path.AddArc(280, 92, 32, 32, 90, 90)
$path.CloseFigure()
$iconRectangle = [System.Drawing.Rectangle]::new(280, 44, 80, 80)
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
$graphics.DrawLine($pen, 320, 71, 303, 101)
$graphics.DrawLine($pen, 320, 71, 338, 101)
$pen.Dispose()
$node = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
$graphics.FillEllipse($node, 309, 59, 23, 23)
$graphics.FillEllipse($node, 293, 91, 20, 20)
$graphics.FillEllipse($node, 328, 91, 20, 20)
$node.Dispose()

$format = [System.Drawing.StringFormat]::new()
$format.Alignment = [System.Drawing.StringAlignment]::Center
$format.LineAlignment = [System.Drawing.StringAlignment]::Center
$titleBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 23, 50, 77))
$font = [System.Drawing.Font]::new('Segoe UI', 31, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$graphics.DrawString('Astral', $font, $titleBrush, [System.Drawing.RectangleF]::new(170, 148, 300, 44), $format)
$font.Dispose()
$titleBrush.Dispose()

$subtitleBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 96, 117, 138))
$subtitle = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('5a6J5YWo44CB6Ieq55Sx44CB566A5rSB55qE6IGU5py65L2T6aqM'))
$font = [System.Drawing.Font]::new('Microsoft YaHei UI', 15, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$graphics.DrawString($subtitle, $font, $subtitleBrush, [System.Drawing.RectangleF]::new(100, 202, 440, 32), $format)
$font.Dispose()
$subtitleBrush.Dispose()
$format.Dispose()

$bitmap.Save((Join-Path $output 'background.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$graphics.Dispose()
$bitmap.Dispose()

Write-Host "Astral scalable NSIS background generated in $output"
