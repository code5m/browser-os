# BrowserOS BFC independent local service for Windows 11 (Node.js 20+).
# Usage: pwsh -File scripts/windows/start-bfc-local.ps1
# Does not read browser Cookies, call Bilibili APIs or install system services.
[CmdletBinding()]
param(
  [string]$ServiceDirectory = (Join-Path $env:LOCALAPPDATA 'BrowserOS\bfc-workbench'),
  [string]$DataDirectory = (Join-Path $env:LOCALAPPDATA 'BrowserOS\Bilibili-Favorites-Review')
)
$ErrorActionPreference = 'Stop'
if (-not $env:LOCALAPPDATA) {
  throw 'LOCALAPPDATA is required for the per-user BFC service.'
}
# Resolve the real OS directory rather than trusting inherited SystemRoot.
$icacls = Join-Path ([Environment]::SystemDirectory) 'icacls.exe'
if (-not (Test-Path -LiteralPath $icacls -PathType Leaf)) {
  throw 'Windows icacls.exe is unavailable.'
}
$nodeCommand = Get-Command node -CommandType Application -ErrorAction SilentlyContinue |
  Select-Object -First 1
if ($null -eq $nodeCommand) {
  throw 'Node.js 20+ required. Install Node before starting the local BFC service.'
}
$nodeExe = [IO.Path]::GetFullPath([string]$nodeCommand.Source)

function Assert-NoReparsePath([string]$Path) {
  $full = [IO.Path]::GetFullPath($Path)
  $root = [IO.Path]::GetPathRoot($full)
  $cursor = $root
  $remaining = $full.Substring($root.Length)
  foreach ($part in $remaining.Split([char[]]@('\','/'), [StringSplitOptions]::RemoveEmptyEntries)) {
    $cursor = Join-Path $cursor $part
    if (Test-Path -LiteralPath $cursor) {
      $attributes = [IO.File]::GetAttributes($cursor)
      if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "BFC path contains a reparse point: $cursor"
      }
    }
  }
}

function Set-OwnerSystemDirectoryAcl([string]$Path) {
  $acl = Get-Acl -LiteralPath $Path
  $acl.SetAccessRuleProtection($true, $false)
  foreach ($rule in @($acl.Access)) { $acl.RemoveAccessRuleSpecific($rule) | Out-Null }
  $acl.SetOwner($currentSid)
  $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor
    [Security.AccessControl.InheritanceFlags]::ObjectInherit
  $propagation = [Security.AccessControl.PropagationFlags]::None
  $allow = [Security.AccessControl.AccessControlType]::Allow
  $full = [Security.AccessControl.FileSystemRights]::FullControl
  $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($currentSid, $full, $inheritance, $propagation, $allow))
  $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($systemSid, $full, $inheritance, $propagation, $allow))
  Set-Acl -LiteralPath $Path -AclObject $acl
  $verified = Get-Acl -LiteralPath $Path
  $rules = @($verified.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
  if ($rules.Count -ne 2 -or
      @($rules | Where-Object {
        $_.IdentityReference.Value -notin @($currentSid.Value, $systemSid.Value) -or
        $_.AccessControlType -ne $allow -or
        $_.FileSystemRights -ne $full -or
        $_.IsInherited
      }).Count -ne 0) {
    throw "Directory ACL is not exactly current-user and SYSTEM: $Path"
  }
}

$currentSid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$systemSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
# Pass the OS-resolved SID, never a caller-controlled DOMAIN\USERNAME string,
# so the Node service applies the same current-user ACL as this launcher.
$env:BROWSEROS_USER_SID = $currentSid.Value
$env:BROWSEROS_ICACLS_PATH = $icacls

# The service source is executable input. Reject junctions and protect the
# checkout before resolving server.mjs or invoking the Node runtime.
$ServiceDirectory = [IO.Path]::GetFullPath($ServiceDirectory)
if ($ServiceDirectory.StartsWith('\\')) {
  throw 'BFC service directory must be on a local Windows volume.'
}
Assert-NoReparsePath $ServiceDirectory
if (-not (Test-Path -LiteralPath $ServiceDirectory -PathType Container)) {
  throw "BFC service directory does not exist: $ServiceDirectory"
}
Set-OwnerSystemDirectoryAcl $ServiceDirectory
$servicePending = [System.Collections.Generic.Stack[string]]::new()
$serviceChildren = [System.Collections.Generic.List[string]]::new()
$servicePending.Push($ServiceDirectory)
$maxServiceObjects = 50000
while ($servicePending.Count -gt 0) {
  $directory = $servicePending.Pop()
  foreach ($child in [IO.Directory]::EnumerateFileSystemEntries($directory)) {
    $attributes = [IO.File]::GetAttributes($child)
    if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "BFC service source contains a reparse point: $child"
    }
    $serviceChildren.Add($child)
    if ($serviceChildren.Count -gt $maxServiceObjects) {
      throw 'BFC service source object limit exceeded.'
    }
    if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) {
      $servicePending.Push($child)
    }
  }
}
foreach ($child in $serviceChildren) {
  & $icacls $child '/setowner' "*$($currentSid.Value)" | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Cannot set BFC service source owner: $child" }
  & $icacls $child '/reset' | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Cannot reset BFC service source ACL: $child" }
}
$serviceAllowedSids = @($currentSid.Value, $systemSid.Value)
$serviceAllow = [Security.AccessControl.AccessControlType]::Allow
$serviceFull = [Security.AccessControl.FileSystemRights]::FullControl
foreach ($child in $serviceChildren) {
  $acl = Get-Acl -LiteralPath $child
  $rules = @($acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
  if ($rules.Count -ne 2 -or
      @($rules | Where-Object {
        $_.IdentityReference.Value -notin $serviceAllowedSids -or
        $_.AccessControlType -ne $serviceAllow -or
        $_.FileSystemRights -ne $serviceFull -or
        -not $_.IsInherited
      }).Count -ne 0) {
    throw "BFC service source ACL is not inherited exclusively from the protected root: $child"
  }
}
$entry = Join-Path $ServiceDirectory 'server.mjs'
Assert-NoReparsePath $entry
if (-not (Test-Path -LiteralPath $entry -PathType Leaf)) {
  throw "BFC service not installed at $entry. Use the pinned code5m/bfc-workbench source."
}
Assert-NoReparsePath $nodeExe
$version = (& $nodeExe -p "Number(process.versions.node.split('.')[0])").Trim()
if ([int]$version -lt 20) { throw 'Node.js >= 20 is required.' }

$DataDirectory = [IO.Path]::GetFullPath($DataDirectory)
if ($DataDirectory.StartsWith('\\')) {
  throw 'BFC data directory must be on a local Windows volume.'
}
Assert-NoReparsePath $DataDirectory
New-Item -ItemType Directory -Path $DataDirectory -Force | Out-Null
Assert-NoReparsePath $DataDirectory
Set-OwnerSystemDirectoryAcl $DataDirectory

# Enumerate without following junctions/reparse points. Do not use
# Get-ChildItem -Recurse or icacls /T against an untrusted existing tree.
$pending = [System.Collections.Generic.Stack[string]]::new()
$children = [System.Collections.Generic.List[string]]::new()
$pending.Push($DataDirectory)
$maxDataObjects = 50000
while ($pending.Count -gt 0) {
  $directory = $pending.Pop()
  foreach ($child in [IO.Directory]::EnumerateFileSystemEntries($directory)) {
    $attributes = [IO.File]::GetAttributes($child)
    if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
      throw "BFC data tree contains a reparse point: $child"
    }
    $children.Add($child)
    if ($children.Count -gt $maxDataObjects) {
      throw 'BFC data tree object limit exceeded.'
    }
    if (($attributes -band [IO.FileAttributes]::Directory) -ne 0) {
      $pending.Push($child)
    }
  }
}

# Reset each existing child separately so it inherits only the protected root
# DACL. Then verify every object's effective explicit/inherited rules.
foreach ($child in $children) {
  & $icacls $child '/setowner' "*$($currentSid.Value)" | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Cannot set BFC data object owner: $child" }
  & $icacls $child '/reset' | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "Cannot reset BFC data ACL: $child" }
}
$allowedSids = @($currentSid.Value, $systemSid.Value)
$allow = [Security.AccessControl.AccessControlType]::Allow
$full = [Security.AccessControl.FileSystemRights]::FullControl
foreach ($child in $children) {
  $acl = Get-Acl -LiteralPath $child
  $rules = @($acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
  if ($rules.Count -ne 2 -or
      @($rules | Where-Object {
        $_.IdentityReference.Value -notin $allowedSids -or
        $_.AccessControlType -ne $allow -or
        $_.FileSystemRights -ne $full -or
        -not $_.IsInherited
      }).Count -ne 0) {
    throw "BFC data object ACL is not inherited exclusively from the protected root: $child"
  }
}

# BrowserOS and the service may be launched independently. Persist the
# selected data directory in a protected per-user config file so both resolve
# the same identity token even when -DataDirectory is customized.
$configDirectory = Join-Path $env:LOCALAPPDATA 'BrowserOS\bfc-config'
Assert-NoReparsePath $configDirectory
New-Item -ItemType Directory -Path $configDirectory -Force | Out-Null
Assert-NoReparsePath $configDirectory
Set-OwnerSystemDirectoryAcl $configDirectory
$configPath = Join-Path $configDirectory 'data-directory.json'
Assert-NoReparsePath $configPath
$tempConfig = Join-Path $configDirectory ("data-directory.{0}.tmp" -f [Guid]::NewGuid().ToString('N'))
Assert-NoReparsePath $tempConfig
$configJson = @{ dataDirectory = $DataDirectory } | ConvertTo-Json -Compress
[IO.File]::WriteAllText($tempConfig, $configJson, [Text.UTF8Encoding]::new($false))
Move-Item -LiteralPath $tempConfig -Destination $configPath -Force
Assert-NoReparsePath $configPath
& $icacls $configPath '/setowner' "*$($currentSid.Value)" | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Cannot set BFC data-directory config owner.' }
$configAcl = Get-Acl -LiteralPath $configPath
$configOwnerSid = $configAcl.GetOwner([Security.Principal.SecurityIdentifier]).Value
if ($configOwnerSid -ne $currentSid.Value) {
  throw 'BFC data-directory config is not owned by the current user.'
}
$configRules = @($configAcl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]))
if ($configRules.Count -ne 2 -or
    @($configRules | Where-Object {
      $_.IdentityReference.Value -notin $allowedSids -or
      $_.AccessControlType -ne $allow -or
      $_.FileSystemRights -ne $full -or
      -not $_.IsInherited
    }).Count -ne 0) {
  throw 'BFC data-directory config ACL is not inherited exclusively from its protected directory.'
}

$env:BILIBILI_FAVORITES_DATA_DIR = $DataDirectory
$env:BROWSEROS_BFC_DATA_DIR = $DataDirectory
# Standalone foreground service: stop with Ctrl+C. Port always loopback only.
& $nodeExe $entry
