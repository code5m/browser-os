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
if (Test-Path -LiteralPath $DataDirectory) {
  $existing = Get-Item -LiteralPath $DataDirectory -Force
  if (($existing.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw 'BFC data directory must not be a junction or reparse point.'
  }
}
New-Item -ItemType Directory -Path $DataDirectory -Force | Out-Null
$directory = Get-Item -LiteralPath $DataDirectory -Force
if (($directory.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
  throw 'BFC data directory resolved to a reparse point.'
}
# Re-apply and verify the ACL on every launch, including pre-existing data.
# The identity token file inherits this owner/SYSTEM-only directory ACL.
$currentSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
& icacls.exe $DataDirectory '/inheritance:r' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Cannot disable inherited BFC data ACL.' }
& icacls.exe $DataDirectory '/remove:g' '*S-1-1-0' '*S-1-5-32-545' '*S-1-5-11' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Cannot remove broad BFC data ACL grants.' }
& icacls.exe $DataDirectory '/grant:r' "*${currentSid}:(OI)(CI)F" 'NT AUTHORITY\SYSTEM:(OI)(CI)F' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Cannot authorize current-user BFC data ACL.' }
$acl = (& icacls.exe $DataDirectory | Out-String)
if ($LASTEXITCODE -ne 0 -or $acl -match '(?i)Everyone|BUILTIN\\Users|Authenticated Users') {
  throw 'BFC data directory ACL is not exclusive to the current user and SYSTEM.'
}
$env:BILIBILI_FAVORITES_DATA_DIR = $DataDirectory
$env:BROWSEROS_BFC_DATA_DIR = $DataDirectory
# Standalone foreground service: stop with Ctrl+C. Port always loopback only.
& node $entry
