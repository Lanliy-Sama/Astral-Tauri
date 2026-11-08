# NSIS 工具链预热：tauri bundler 构建 NSIS 安装包前需要下载 nsis-3.13.zip，
# GitHub release 直连在 runner 上持续 504，这里多源轮换下载并解压到 tauri 缓存目录。
$ErrorActionPreference = "Continue"

$dest = Join-Path $env:LOCALAPPDATA "tauri"
New-Item -ItemType Directory -Force -Path $dest | Out-Null
$makensis = Join-Path $dest "NSIS\makensis.exe"
if (Test-Path $makensis) {
    Write-Host "NSIS cached, skip"
    exit 0
}

$urls = @(
    "https://ghfast.top/https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip",
    "https://gh-proxy.com/https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip",
    "https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip"
)

$ok = $false
foreach ($u in $urls) {
    if ($ok) { break }
    for ($i = 1; $i -le 2; $i++) {
        try {
            Write-Host "Downloading NSIS from $u (attempt $i)"
            Invoke-WebRequest -Uri $u -OutFile "$dest\nsis.zip" -UserAgent astral -ErrorAction Stop
            Expand-Archive "$dest\nsis.zip" -DestinationPath $dest -Force
            Remove-Item "$dest\nsis.zip" -ErrorAction SilentlyContinue
            if (Test-Path $makensis) {
                Write-Host "NSIS preheat OK"
                $ok = $true
                break
            }
            Write-Host "makensis not found after extract"
        }
        catch {
            Write-Host "Failed: $($_.Exception.Message)"
            Start-Sleep -Seconds 5
        }
    }
}

if (-not $ok) {
    Write-Host "##[warning]NSIS preheat failed; tauri bundler download may 504"
}
