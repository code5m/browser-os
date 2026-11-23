# Local Windows NSIS release build: no GitHub Actions, gates, tests, installation or merge.
# Requires PowerShell 7, Node, Python, Rust, Git, MSVC and Windows SDK.
[CmdletBinding()]
param(
    [ValidateSet("terser", "esbuild")]
    [string]$Minifier = "terser",
    [ValidateRange(1, 8)]
    [int]$RetryCount = 4,
    [ValidateRange(1, 16)]
    [int]$CargoJobs = 2,
    [string]$CacheDirectory = "",
    [string]$VsInstallDir = "",
    [string]$WindowsSdkRoot = "",
    [string]$ReuseBundleDirectory = ""
)
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if (-not $IsWindows -or $PSVersionTable.PSVersion.Major -lt 7) {
    throw "WINDOWS_LOCAL_BUILD_REQUIRES_POWERSHELL_7"
}
$Root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($CacheDirectory)) {
    $CacheDirectory = Join-Path ([Environment]::GetFolderPath("LocalApplicationData")) "BrowserOS/BuildCache"
}
$CacheDirectory = [System.IO.Path]::GetFullPath($CacheDirectory)
[System.IO.Directory]::CreateDirectory($CacheDirectory) | Out-Null

function Find-Executable([string]$Name, [string]$Fallback) {
    $Command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($null -ne $Command -and $Command.Source -notmatch "[\\/]WindowsApps[\\/]") {
        return $Command.Source
    }
    if ([System.IO.File]::Exists($Fallback)) { return $Fallback }
    throw "WINDOWS_LOCAL_BUILD_EXECUTABLE_NOT_FOUND:$Name"
}
function Invoke-External([string]$Executable, [string[]]$Arguments, [string]$Stage) {
    # Native tools legitimately write progress to stderr.
    $ErrorActionPreference = "Continue"
    $PSNativeCommandUseErrorActionPreference = $false
    & $Executable @Arguments
    $Exit = $LASTEXITCODE
    if ($Exit -ne 0) { throw "WINDOWS_LOCAL_BUILD_FAILED:$Stage exit=$Exit" }
}

$Node = Find-Executable "node.exe" (Join-Path $env:ProgramFiles "nodejs/node.exe")
$Python = Get-Command python3.exe, python.exe, py.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.Source -notmatch "[\\/]WindowsApps[\\/]" } |
    Select-Object -First 1
if ($null -eq $Python) { throw "WINDOWS_LOCAL_BUILD_PYTHON_NOT_FOUND" }
$PythonArgs = @()
if ($Python.Name -ieq "py.exe") { $PythonArgs = @("-3") }

$ClCommand = Get-Command cl.exe -ErrorAction SilentlyContinue
$Cl = if ($null -ne $ClCommand -and [System.IO.File]::Exists($ClCommand.Source)) {
    $ClCommand.Source
} else { "" }
if (-not $Cl) {
    if (-not $VsInstallDir) {
        $VsWhere = Join-Path ([Environment]::GetFolderPath("ProgramFilesX86")) "Microsoft Visual Studio/Installer/vswhere.exe"
        if ([System.IO.File]::Exists($VsWhere)) {
            $Candidates = @(& $VsWhere -latest -products "*" -requires "Microsoft.VisualStudio.Component.VC.Tools.x86.x64" -property installationPath)
            $VsInstallDir = [string]($Candidates | Where-Object { $_ } | Select-Object -First 1)
        }
    }
    if (-not $VsInstallDir) { throw "WINDOWS_LOCAL_BUILD_MSVC_NOT_FOUND: specify -VsInstallDir" }
    $Toolsets = @(Get-ChildItem -LiteralPath (Join-Path $VsInstallDir "VC/Tools/MSVC") -Directory |
        Sort-Object { [version]$_.Name } -Descending)
    foreach ($Toolset in $Toolsets) {
        $Candidate = Join-Path $Toolset.FullName "bin/Hostx64/x64/cl.exe"
        if ([System.IO.File]::Exists($Candidate)) { $Cl = $Candidate; break }
    }
}
if (-not $Cl -or -not [System.IO.File]::Exists($Cl)) {
    throw "WINDOWS_LOCAL_BUILD_CL_NOT_FOUND"
}
$MsvcBin = Split-Path -Parent $Cl
$MsvcRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MsvcBin))
$Linker = Join-Path $MsvcBin "link.exe"
if (-not [System.IO.File]::Exists($Linker)) { throw "WINDOWS_LOCAL_BUILD_LINKER_NOT_FOUND" }

if (-not $WindowsSdkRoot) {
    try {
        $WindowsSdkRoot = [string](Get-ItemProperty -Path "HKLM:/SOFTWARE/Microsoft/Windows Kits/Installed Roots" -Name KitsRoot10).KitsRoot10
    } catch {
        $WindowsSdkRoot = Join-Path ([Environment]::GetFolderPath("ProgramFilesX86")) "Windows Kits/10"
    }
}
$SdkInclude = Join-Path $WindowsSdkRoot "Include"
if (-not [System.IO.Directory]::Exists($SdkInclude)) {
    throw "WINDOWS_LOCAL_BUILD_SDK_NOT_FOUND: specify -WindowsSdkRoot"
}
$SdkVersions = @(Get-ChildItem -LiteralPath $SdkInclude -Directory |
    Where-Object { $_.Name -match "^10[.]0[.][0-9]+[.]0$" } |
    Sort-Object { [version]$_.Name } -Descending)
$SdkVersion = ""
foreach ($Entry in $SdkVersions) {
    $Candidate = $Entry.Name
    if ([System.IO.File]::Exists((Join-Path $WindowsSdkRoot "Lib/$Candidate/um/x64/kernel32.lib")) -and
        [System.IO.File]::Exists((Join-Path $WindowsSdkRoot "Lib/$Candidate/ucrt/x64/ucrt.lib"))) {
        $SdkVersion = $Candidate
        break
    }
}
if (-not $SdkVersion) { throw "WINDOWS_LOCAL_BUILD_SDK_LIBRARIES_NOT_FOUND" }

$Includes = @(
    (Join-Path $MsvcRoot "include"),
    (Join-Path $WindowsSdkRoot "Include/$SdkVersion/ucrt"),
    (Join-Path $WindowsSdkRoot "Include/$SdkVersion/shared"),
    (Join-Path $WindowsSdkRoot "Include/$SdkVersion/um"),
    (Join-Path $WindowsSdkRoot "Include/$SdkVersion/winrt")
)
$OptionalCppWinRt = Join-Path $WindowsSdkRoot "Include/$SdkVersion/cppwinrt"
if ([System.IO.Directory]::Exists($OptionalCppWinRt)) { $Includes += $OptionalCppWinRt }
$Libraries = @(
    (Join-Path $MsvcRoot "lib/x64"),
    (Join-Path $WindowsSdkRoot "Lib/$SdkVersion/ucrt/x64"),
    (Join-Path $WindowsSdkRoot "Lib/$SdkVersion/um/x64")
)
foreach ($Directory in @($Includes) + @($Libraries)) {
    if (-not [System.IO.Directory]::Exists($Directory)) {
        throw "WINDOWS_LOCAL_BUILD_TOOLCHAIN_DIRECTORY_MISSING:$Directory"
    }
}
$LibPaths = @((Join-Path $MsvcRoot "lib/x64"))
$OneCore = Join-Path $MsvcRoot "lib/onecore/x64"
if ([System.IO.Directory]::Exists($OneCore)) { $LibPaths += $OneCore }
$env:PATH = (@($MsvcBin, $PSHOME, (Split-Path -Parent $Node), $env:PATH) -join ";")
$env:INCLUDE = (@($Includes) + @($env:INCLUDE | Where-Object { $_ })) -join ";"
$env:LIB = (@($Libraries) + @($env:LIB | Where-Object { $_ })) -join ";"
$env:LIBPATH = (@($LibPaths) + @($env:LIBPATH | Where-Object { $_ })) -join ";"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = $Linker
$env:CARGO_BUILD_JOBS = [string]$CargoJobs

$Git = Get-Command git.exe -ErrorAction Stop
$GitExecPath = (& $Git.Source --exec-path | Out-String).Trim()
if ($LASTEXITCODE -eq 0 -and [System.IO.Directory]::Exists($GitExecPath)) {
    $env:GIT_EXEC_PATH = $GitExecPath
}
$ViteCli = Join-Path $Root "node_modules/vite/bin/vite.js"
$TauriCli = Join-Path $Root "node_modules/@tauri-apps/cli/tauri.js"
$BuildConfig = Join-Path $Root "src-tauri/tauri.local-prebuilt.windows.conf.json"
foreach ($Required in @($ViteCli, $TauriCli, $BuildConfig)) {
    if (-not [System.IO.File]::Exists($Required)) {
        throw "WINDOWS_LOCAL_BUILD_DEPENDENCY_MISSING:$Required"
    }
}

$PriorLocation = Get-Location
try {
    Set-Location -LiteralPath $Root
    $LspArguments = @(
        "-NoProfile", "-File", (Join-Path $Root "scripts/build-windows-lsp-runtime.ps1"),
        "-WorkDirectory", (Join-Path $CacheDirectory "lsp")
    )
    if ($ReuseBundleDirectory) {
        $LspArguments += @("-ReuseBundleDirectory", $ReuseBundleDirectory)
    }
    Write-Host "WINDOWS_LOCAL_BUILD: prepare pinned LSP"
    Invoke-External (Join-Path $PSHOME "pwsh.exe") $LspArguments "lsp"
    Write-Host "WINDOWS_LOCAL_BUILD: refresh frontend contracts"
    Invoke-External $Python.Source ($PythonArgs + @(
        "scripts/generate-rust-crosslayer-contracts-dedup.py", "--generate")) "contracts"
    Write-Host "WINDOWS_LOCAL_BUILD: fresh frontend dist ($Minifier)"
    if ($Minifier -eq "esbuild") {
        Write-Warning "esbuild is a local workaround; production Terser parity is not established"
    }
    Invoke-External $Node @($ViteCli, "build", "--minify", $Minifier) "frontend"

    # Persist Cargo cache across fresh checkouts, separate from pinned LSP.
    $TargetDirectory = Join-Path $CacheDirectory "tauri-target"
    [System.IO.Directory]::CreateDirectory($TargetDirectory) | Out-Null
    $env:CARGO_TARGET_DIR = $TargetDirectory
    $Logs = Join-Path $CacheDirectory "logs"
    [System.IO.Directory]::CreateDirectory($Logs) | Out-Null

    for ($Attempt = 1; $Attempt -le $RetryCount; $Attempt++) {
        $LogFile = Join-Path $Logs ("nsis-attempt-" + $Attempt + ".log")
        Write-Host "WINDOWS_LOCAL_BUILD: NSIS attempt $Attempt/$RetryCount; jobs=$CargoJobs"
        $ErrorActionPreference = "Continue"
        $PSNativeCommandUseErrorActionPreference = $false
        & $Node $TauriCli build --bundles nsis --config $BuildConfig 2>&1 |
            Tee-Object -FilePath $LogFile
        $ExitCode = $LASTEXITCODE
        $ErrorActionPreference = "Stop"
        if ($ExitCode -eq 0) {
            Write-Host "WINDOWS_LOCAL_BUILD_NSIS_CREATED: $(Join-Path $TargetDirectory 'release/bundle/nsis')"
            Write-Host "WINDOWS_LOCAL_BUILD_UNVERIFIED: no CI, tests, installation or GUI check ran"
            return
        }
        $Tail = (Get-Content -LiteralPath $LogFile -Tail 160) -join "`n"
        $Transient = $Tail -match "LNK1104|os error 5|Access is denied|拒绝访问|failed to write fingerprint|error writing dependencies"
        if (-not $Transient -or $Attempt -eq $RetryCount) {
            throw "WINDOWS_LOCAL_BUILD_NSIS_FAILED: exit=$ExitCode log=$LogFile"
        }
        Write-Warning "Transient file lock; keeping completed Cargo cache and retrying"
        Start-Sleep -Seconds ([Math]::Min(12, 2 * $Attempt))
    }
} finally {
    Set-Location -LiteralPath $PriorLocation
}
