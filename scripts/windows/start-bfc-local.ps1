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
$DataDirectory = [IO.Path]::GetFullPath($DataDirectory)
if ($DataDirectory.StartsWith('\\\\')) {
  throw 'BFC data directory must be on a local Windows volume.'
}
# Reject reparse points in every existing path component, not only the leaf.
$cursor = $DataDirectory
while ($cursor) {
  if (Test-Path -LiteralPath $cursor) {
    $entryAtPath = Get-Item -LiteralPath $cursor -Force
    if (($entryAtPath.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "BFC data path contains a reparse point: $cursor"
    }
  }
  $parent = Split-Path -Path $cursor -Parent
  if (-not $parent -or $parent -eq $cursor) { break }
  $cursor = $parent
}
New-Item -ItemType Directory -Path $DataDirectory -Force | Out-Null
# Rebuild the DACL on every launch. Existing explicit grants are removed too.
$currentSid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$systemSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$acl = Get-Acl -LiteralPath $DataDirectory
$acl.SetAccessRuleProtection($true, $false)
foreach ($rule in @($acl.Access)) { $acl.RemoveAccessRuleSpecific($rule) }
$acl.SetOwner($currentSid)
$inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor
  [Security.AccessControl.InheritanceFlags]::ObjectInherit
$propagation = [Security.AccessControl.PropagationFlags]::None
$allow = [Security.AccessControl.AccessControlType]::Allow
$full = [Security.AccessControl.FileSystemRights]::FullControl
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($currentSid, $full, $inheritance, $propagation, $allow))
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($systemSid, $full, $inheritance, $propagation, $allow))
Set-Acl -LiteralPath $DataDirectory -AclObject $acl
$verifiedAcl = Get-Acl -LiteralPath $DataDirectory
$verifiedRules = @($verifiedAcl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
if ($verifiedRules.Count -ne 2 -or
    -not ($verifiedRules | Where-Object { $_.IdentityReference.Value -eq $currentSid.Value -and $_.AccessControlType -eq $allow -and $_.FileSystemRights -eq $full -and -not $_.IsInherited }) -or
    -not ($verifiedRules | Where-Object { $_.IdentityReference.Value -eq $systemSid.Value -and $_.AccessControlType -eq $allow -and $_.FileSystemRights -eq $full -and -not $_.IsInherited })) {
  throw 'BFC data directory DACL is not exactly current-user and SYSTEM FullControl.'
}
$env:BILIBILI_FAVORITES_DATA_DIR = $DataDirectory
$env:BROWSEROS_BFC_DATA_DIR = $DataDirectory
# Standalone foreground service: stop with Ctrl+C. Port always loopback only.
& node $entry
