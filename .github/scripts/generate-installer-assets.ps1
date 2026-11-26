Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$root = Resolve-Path (Join-Path $PSScriptRoot '../..')
$output = Join-Path $root 'src-tauri/installer/assets'
New-Item -ItemType Directory -Force -Path $output | Out-Null

$background = [System.Drawing.Bitmap]::new(640, 460, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$graphics = [System.Drawing.Graphics]::FromImage($background)
$rectangle = [System.Drawing.Rectangle]::new(0, 0, 640, 460)
$brush = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
  $rectangle,
  [System.Drawing.Color]::FromArgb(255, 255, 244, 236),
  [System.Drawing.Color]::FromArgb(232, 248, 255),
  [System.Drawing.Drawing2D.LinearGradientMode]::ForwardDiagonal
)
$graphics.FillRectangle($brush, $rectangle)
$background.Save((Join-Path $output 'background.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$brush.Dispose()
$graphics.Dispose()
$background.Dispose()

$source = [System.Drawing.Image]::FromFile((Join-Path $root 'app-icon.png'))
$icon = [System.Drawing.Bitmap]::new(96, 96, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$graphics = [System.Drawing.Graphics]::FromImage($icon)
$graphics.Clear([System.Drawing.Color]::Black)
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$graphics.DrawImage($source, 0, 0, 96, 96)
$icon.Save((Join-Path $output 'app-icon.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$graphics.Dispose()
$icon.Dispose()
$source.Dispose()

Write-Host "Astral NSIS assets generated in $output"
