# BrowserOS Windows installed payload verification or release-window smoke test.
# Not proof of Bilibili login or a full interactive GUI acceptance.
[CmdletBinding()]
param(
  [string]$ExePath = 'src-tauri\target\release\mvp-browser-os.exe',
  [string]$EvidenceDir = (Join-Path $env:TEMP 'browseros-windows-gui-smoke'),
  [int]$TimeoutSeconds = 45,
  [switch]$CaptureScreenshot,
  [switch]$StopLaunched,
  [switch]$PayloadOnly,
  [string]$ExpectedReleaseDirectory,
  [string]$ExpectedRuntimeDirectory,
  [string]$SourceSha
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $ExePath -PathType Leaf)) {
  throw "Release executable not found: $ExePath. Build or install a Windows release first."
}
$exe = (Resolve-Path -LiteralPath $ExePath).Path
$hash = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
if ($PayloadOnly) {
  if ($SourceSha -notmatch '^[0-9a-f]{40}$' -or
      [string]::IsNullOrWhiteSpace($ExpectedReleaseDirectory) -or
      [string]::IsNullOrWhiteSpace($ExpectedRuntimeDirectory)) {
    throw 'NSIS_PAYLOAD_EXPECTED_SOURCE_REQUIRED'
  }
  $installedRoot = Split-Path -Parent $exe
  $expectedExe = [IO.Path]::GetFullPath((Join-Path $ExpectedReleaseDirectory 'mvp-browser-os.exe'))
  if ([StringComparer]::OrdinalIgnoreCase.Equals($exe, $expectedExe)) {
    throw 'NSIS_PAYLOAD_EXPECTS_INSTALLED_COPY'
  }
  foreach ($directory in @($installedRoot, (Join-Path $installedRoot 'windows-lsp'), $ExpectedReleaseDirectory, $ExpectedRuntimeDirectory)) {
    if (-not (Test-Path -LiteralPath $directory -PathType Container) -or
        ((Get-Item -LiteralPath $directory).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
      throw 'NSIS_PAYLOAD_DIRECTORY_INVALID'
    }
  }
  $expectedFiles = [ordered]@{
    'mvp-browser-os.exe' = Join-Path $ExpectedReleaseDirectory 'mvp-browser-os.exe'
  }
  foreach ($name in @('rust-analyzer.exe', 'runtime.json', 'LICENSE-MIT', 'LICENSE-APACHE', 'Cargo.lock')) {
    $expectedFiles[('windows-lsp/' + $name)] = Join-Path $ExpectedRuntimeDirectory $name
  }
  $verified = [ordered]@{}
  foreach ($entry in $expectedFiles.GetEnumerator()) {
    $installed = Join-Path $installedRoot $entry.Key
    if (-not (Test-Path -LiteralPath $entry.Value -PathType Leaf) -or
        -not (Test-Path -LiteralPath $installed -PathType Leaf)) {
      throw "NSIS_PAYLOAD_FILE_MISSING:$($entry.Key)"
    }
    $sourceItem = Get-Item -LiteralPath $entry.Value
    $installedItem = Get-Item -LiteralPath $installed
    if (($sourceItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
        ($installedItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
      throw "NSIS_PAYLOAD_REPARSE_POINT:$($entry.Key)"
    }
    $expectedHash = (Get-FileHash -LiteralPath $entry.Value -Algorithm SHA256).Hash.ToLowerInvariant()
    $actualHash = (Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -cne $expectedHash -or $installedItem.Length -ne $sourceItem.Length) {
      throw "NSIS_PAYLOAD_HASH_MISMATCH:$($entry.Key)"
    }
    $verified[$entry.Key] = $actualHash
  }
  New-Item -ItemType Directory -Path $EvidenceDir -Force | Out-Null
  $payloadReport = [ordered]@{
    status = 'PACKAGE_PAYLOAD_PASS_GUI_PENDING'
    source_sha = $SourceSha
    gui_pass = $false
    real_bfc_login_verified = $false
    files = $verified
  }
  $json = $payloadReport | ConvertTo-Json -Depth 5
  [IO.File]::WriteAllText(
    (Join-Path $EvidenceDir 'package-payload.json'),
    $json + [Environment]::NewLine,
    [Text.UTF8Encoding]::new($false)
  )
  Write-Host 'NSIS_INSTALLED_PAYLOAD=PASS files=6 GUI=GUI_PENDING'
  Write-Host $json
  return
}
$existing = @(Get-Process -Name 'mvp-browser-os' -ErrorAction SilentlyContinue)
if ($existing.Count -gt 0) {
  throw 'BrowserOS is already running. Close the application normally before this isolated smoke test.'
}
New-Item -ItemType Directory -Path $EvidenceDir -Force | Out-Null
$started = Get-Date
$proc = Start-Process -FilePath $exe -PassThru
try {
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  $windowFound = $false
  while ((Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 500
    $proc.Refresh()
    if ($proc.HasExited) {
      throw "BrowserOS exited before window creation. ExitCode=$($proc.ExitCode)"
    }
    if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
      $windowFound = $true
      break
    }
  }
  if (-not $windowFound) {
    throw 'WINDOW_SMOKE_PENDING: release process did not expose a top-level window.'
  }
  $report = [ordered]@{
    status = 'WINDOW_SMOKE_PASS_GUI_PENDING'
    gui_pass = $false
    real_bfc_login_verified = $false
    executable = $exe
    sha256 = $hash
    pid = $proc.Id
    window_handle = $proc.MainWindowHandle.ToInt64()
    window_title = $proc.MainWindowTitle
    started_at = $started.ToString('o')
  }
  if ($CaptureScreenshot) {
    # Screenshot stays on the local device and can contain private account data.
    try {
      Add-Type -AssemblyName System.Drawing
      $source = @'
using System;
using System.Runtime.InteropServices;
public static class BrowserOSWindowRect {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
}
'@
      Add-Type -TypeDefinition $source -ErrorAction Stop
      $rect = New-Object BrowserOSWindowRect+RECT
      if (-not [BrowserOSWindowRect]::GetWindowRect($proc.MainWindowHandle,[ref]$rect)) {
        throw 'Cannot query main window rectangle'
      }
      $width = $rect.Right - $rect.Left
      $height = $rect.Bottom - $rect.Top
      if ($width -lt 300 -or $height -lt 200) {
        throw "Window suspiciously small: $width x $height"
      }
      $bitmap = [System.Drawing.Bitmap]::new($width,$height)
      $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
      try {
        $graphics.CopyFromScreen($rect.Left,$rect.Top,0,0,$bitmap.Size)
        $output = Join-Path $EvidenceDir "browseros-window-pid$($proc.Id).png"
        $bitmap.Save($output,[System.Drawing.Imaging.ImageFormat]::Png)
        $report.screenshot_local_only = $output
        $report.window_width = $width
        $report.window_height = $height
      } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
      }
    } catch {
      $report.screenshot_pending_reason = $_.Exception.Message
    }
  }
  $evidencePath = Join-Path $EvidenceDir "browseros-window-pid$($proc.Id).json"
  $report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $evidencePath -Encoding utf8
  Write-Host ($report | ConvertTo-Json -Depth 5)
} finally {
  # Only stop this script's process when explicitly requested. Never stop
  # other BrowserOS or unrelated user processes.
  if ($StopLaunched -and $proc -and -not $proc.HasExited) {
    Stop-Process -Id $proc.Id -ErrorAction SilentlyContinue
  }
}
