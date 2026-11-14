# BrowserOS BFC independent local service for Windows 11 (Node.js 20+).
# Usage: pwsh -File scripts/windows/start-bfc-local.ps1
# Does not read browser Cookies, call Bilibili APIs or install system services.
[CmdletBinding()]
param(
  [string]$ServiceDirectory = (Join-Path $env:LOCALAPPDATA 'BrowserOS\bfc-workbench'),
  [string]$DataDirectory = (Join-Path $env:LOCALAPPDATA 'BrowserOS\Bilibili-Favorites-Review')
)
$ErrorActionPreference = 'Stop'
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  throw 'Node.js 20+ required. Install Node before starting the local BFC service.'
}
$version = (& node -p "Number(process.versions.node.split('.')[0])").Trim()
if ([int]$version -lt 20) { throw 'Node.js >= 20 is required.' }
$entry = Join-Path $ServiceDirectory 'server.mjs'
if (-not (Test-Path -LiteralPath $entry -PathType Leaf)) {
  throw "BFC service not installed at $entry. Use the pinned code5m/bfc-workbench source."
}
New-Item -ItemType Directory -Path $DataDirectory -Force | Out-Null
$env:BILIBILI_FAVORITES_DATA_DIR = $DataDirectory
# Standalone foreground service: stop with Ctrl+C. Port always loopback only.
& node $entry
