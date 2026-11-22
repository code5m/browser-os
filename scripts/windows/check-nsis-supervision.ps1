# Real command-line boundary regression using the same bounded Job supervisor
# and the NSIS toolset already verified by the locked Tauri CLI.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$MakeNsis,
  [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
  [Parameter(Mandatory = $true)][string]$SourceSha
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:GITHUB_ACTIONS -ne 'true' -or $SourceSha -notmatch '^[0-9a-f]{40}$') {
  throw 'NSIS_ARGUMENT_REGRESSION_REQUIRES_FROZEN_DISPOSABLE_CI'
}
. (Join-Path $PSScriptRoot '..\windows-native-test-runner.ps1')
if (-not (Test-Path -LiteralPath $MakeNsis -PathType Leaf) -or
    ((Get-Item -LiteralPath $MakeNsis).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
  throw 'NSIS_ARGUMENT_REGRESSION_COMPILER_INVALID'
}
$MakeNsis = (Resolve-Path -LiteralPath $MakeNsis).Path
$EvidenceDirectory = [IO.Path]::GetFullPath($EvidenceDirectory)
if (Test-Path -LiteralPath $EvidenceDirectory) {
  throw 'NSIS_ARGUMENT_REGRESSION_REQUIRES_FRESH_DIRECTORY'
}
New-Item -ItemType Directory -Path $EvidenceDirectory | Out-Null

function Invoke-RecordedArgumentProbe {
  param(
    [Parameter(Mandatory = $true)][string]$Label,
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][AllowEmptyCollection()][AllowEmptyString()][string[]]$Arguments,
    [int]$ExpectedExitCode = 0,
    [int]$TimeoutSeconds = 15
  )
  $invoke = @{
    Executable = $Executable
    Arguments = $Arguments
    LogPrefix = Join-Path $EvidenceDirectory $Label
    TimeoutSeconds = $TimeoutSeconds
  }
  $run = Invoke-SupervisedNativeProcess @invoke
  $report = [ordered]@{
    source_sha = $SourceSha
    phase = $Label
    expected_exit_code = $ExpectedExitCode
    supervision = $run.supervision
  }
  $json = $report | ConvertTo-Json -Depth 6
  [IO.File]::WriteAllText(
    (Join-Path $EvidenceDirectory ($Label + '.supervision.json')),
    $json + [Environment]::NewLine,
    [Text.UTF8Encoding]::new($false)
  )
  Write-Host ('NSIS_PROBE_SUPERVISION=' + ($report | ConvertTo-Json -Depth 6 -Compress))
  Assert-NativeSupervision -Run $run -Phase $Label
  if ($run.supervision.ExitCode -ne $ExpectedExitCode) {
    throw "NSIS_ARGUMENT_REGRESSION_EXIT:$Label"
  }
  return $run
}

$utf8 = [Text.UTF8Encoding]::new($false)
$echoPath = Join-Path $EvidenceDirectory 'argument echo.ps1'
$echoSource = @'
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::Out.WriteLine((ConvertTo-Json -InputObject @($args) -Compress))
'@
[IO.File]::WriteAllText($echoPath, $echoSource + [Environment]::NewLine, $utf8)
[string[]]$expected = @(
  '',
  'plain',
  'space value',
  ('tab' + [char]9 + 'value'),
  'quote"value',
  'C:\plain\',
  'C:\space path\',
  'slashes\\\"value',
  '中文路径',
  'literal&|<>^%$();'
)
# Use only this PowerShell process's executable; no PID/tree enumeration or shell.
$pwsh = (Get-Process -Id $PID).Path
$echoArguments = @('-NoLogo', '-NoProfile', '-NonInteractive', '-File', $echoPath) + $expected
$echoRun = Invoke-RecordedArgumentProbe -Label 'argv-roundtrip' -Executable $pwsh -Arguments $echoArguments
$output = Read-BoundedNativeOutput -Run $echoRun
$actual = @(ConvertFrom-Json -InputObject $output.stdout)
if ($actual.Count -ne $expected.Count) { throw 'NSIS_ARGUMENT_REGRESSION_ARGC' }
for ($index = 0; $index -lt $expected.Count; $index++) {
  if ($actual[$index] -isnot [string] -or $actual[$index] -cne $expected[$index]) {
    throw "NSIS_ARGUMENT_REGRESSION_ARGV:$index"
  }
}

$probeSource = @'
Unicode true
RequestExecutionLevel user
Name "BrowserOS native supervisor argument regression"
OutFile "browseros silent probe.exe"
SilentInstall normal

Function .onInit
  IfSilent silent not_silent
silent:
  SetErrorLevel 0
  Quit
not_silent:
  SetErrorLevel 41
  Quit
FunctionEnd

Section
SectionEnd
'@
$probePath = Join-Path $EvidenceDirectory 'silent probe.nsi'
[IO.File]::WriteAllText($probePath, $probeSource + [Environment]::NewLine, $utf8)
Push-Location $EvidenceDirectory
try {
  $null = Invoke-RecordedArgumentProbe -Label 'nsis-compile' -Executable $MakeNsis -Arguments @('/V2', '/NOCD', $probePath) -TimeoutSeconds 30
  $probeExe = Join-Path $EvidenceDirectory 'browseros silent probe.exe'
  if (-not (Test-Path -LiteralPath $probeExe -PathType Leaf)) {
    throw 'NSIS_ARGUMENT_REGRESSION_BINARY_MISSING'
  }
  # Both paths Quit from .onInit before any UI; non-silent is an explicit code.
  # The executable name and directory deliberately contain spaces.
  $null = Invoke-RecordedArgumentProbe -Label 'nsis-silent' -Executable $probeExe -Arguments @('/S')
  $null = Invoke-RecordedArgumentProbe -Label 'nsis-no-silent-switch' -Executable $probeExe -Arguments @() -ExpectedExitCode 41
  $null = Invoke-RecordedArgumentProbe -Label 'nsis-lowercase-switch' -Executable $probeExe -Arguments @('/s') -ExpectedExitCode 41
} finally {
  Pop-Location
}
Write-Host 'NSIS_ARGUMENT_REGRESSION=PASS cases=4 argv_values=10 silent=0 non_silent=41 lowercase=41'
