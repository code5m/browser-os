# Build the Windows LSP runtime from one pinned official source tree.
# This entry point is shared by CI, Tauri packaging and Windows build.rs.
[CmdletBinding()]
param(
    [string]$WorkDirectory = "",
    [string]$ExecutableDirectory = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if (-not $IsWindows) {
    throw "WINDOWS_LSP_PREPARE_REQUIRES_WINDOWS"
}

$RepositoryRoot = Split-Path -Parent $PSScriptRoot
$Assets = Join-Path $RepositoryRoot "vendor/windows-rust-analyzer"
$Bundle = Join-Path $RepositoryRoot "src-tauri/resources/windows-lsp"
$Utf8 = [System.Text.UTF8Encoding]::new($false, $true)
$LineFeed = [string][char]10
$Source = Get-Content -LiteralPath (Join-Path $Assets "source.json") -Raw | ConvertFrom-Json -Depth 30
$ExpectedFiles = @("rust-analyzer.exe", "runtime.json", "LICENSE-MIT", "LICENSE-APACHE", "Cargo.lock")

function Assert-Hash([string]$Path, [string]$Expected, [string]$Stage) {
    if ($Expected -cnotmatch "^[a-f0-9]{64}$" -or
        -not [System.IO.File]::Exists($Path) -or
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $Expected) {
        throw "WINDOWS_LSP_HASH_MISMATCH:$Stage"
    }
}

function Invoke-Checked([string]$Program, [string[]]$ToolArguments) {
    & $Program @ToolArguments
    if ($LASTEXITCODE -ne 0) {
        throw "WINDOWS_LSP_TOOL_FAILED:$Program exit=$LASTEXITCODE"
    }
}

function Read-CheckedOutput([string]$Program, [string[]]$ToolArguments) {
    $ToolOutput = & $Program @ToolArguments 2>&1
    if ($LASTEXITCODE -ne 0) {
        $ToolOutput | ForEach-Object { Write-Host $_ }
        throw "WINDOWS_LSP_TOOL_FAILED:$Program exit=$LASTEXITCODE"
    }
    return ($ToolOutput -join $LineFeed).Trim()
}

function Exact-Properties($Value, [string[]]$Names) {
    return (((@($Value.PSObject.Properties.Name) | Sort-Object) -join "|") -ceq
        (($Names | Sort-Object) -join "|"))
}

function Valid-Bundle([string]$Directory) {
    try {
        foreach ($FileName in $ExpectedFiles) {
            $FilePath = Join-Path $Directory $FileName
            if (-not [System.IO.File]::Exists($FilePath) -or
                ((Get-Item -LiteralPath $FilePath).Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
                return $false
            }
        }
        $Record = Get-Content -LiteralPath (Join-Path $Directory "runtime.json") -Raw |
            ConvertFrom-Json -Depth 30
        if (-not (Exact-Properties $Record @("schema_version", "source", "artifact", "build")) -or
            -not (Exact-Properties $Record.artifact @("file", "sha256", "size")) -or
            -not (Exact-Properties $Record.build @("rustc_release", "rustc_commit", "rustc_sha256", "target")) -or
            $Record.schema_version -ne 1 -or
            ($Record.source | ConvertTo-Json -Depth 30 -Compress) -cne
                ($Source | ConvertTo-Json -Depth 30 -Compress) -or
            $Record.artifact.file -cne "rust-analyzer.exe" -or
            $Record.artifact.size -le 0 -or $Record.artifact.size -gt 268435456 -or
            $Record.build.rustc_release -cne $Source.rust_release -or
            $Record.build.rustc_commit -cne $Source.source_commit -or
            $Record.build.target -cne $Source.target -or
            $Record.build.rustc_sha256 -cnotmatch "^[a-f0-9]{64}$") {
            return $false
        }
        $Executable = Join-Path $Directory "rust-analyzer.exe"
        if ((Get-Item -LiteralPath $Executable).Length -ne $Record.artifact.size) {
            return $false
        }
        Assert-Hash $Executable $Record.artifact.sha256 "cached executable"
        Assert-Hash (Join-Path $Directory "Cargo.lock") $Source.lock_sha256 "cached lock"
        foreach ($License in $Source.licenses) {
            Assert-Hash (Join-Path $Directory $License.file) $License.sha256 "cached license"
        }
        return $true
    } catch {
        return $false
    }
}

function Assert-PlainDirectory([string]$Directory) {
    $Current = [System.IO.DirectoryInfo]::new([System.IO.Path]::GetFullPath($Directory))
    while ($null -ne $Current) {
        try {
            $Attributes = [System.IO.File]::GetAttributes($Current.FullName)
            if (($Attributes -band [System.IO.FileAttributes]::ReparsePoint) -or
                -not ($Attributes -band [System.IO.FileAttributes]::Directory)) {
                throw "WINDOWS_LSP_OUTPUT_DIRECTORY_UNSAFE"
            }
        } catch [System.IO.FileNotFoundException] {
            # A missing component will be created under the checked parents.
        } catch [System.IO.DirectoryNotFoundException] {
            # A missing component will be created under the checked parents.
        }
        $Current = $Current.Parent
    }
    [System.IO.Directory]::CreateDirectory($Directory) | Out-Null
}

function Assert-PlainOutputFile([string]$Path) {
    try {
        $Attributes = [System.IO.File]::GetAttributes($Path)
        if (($Attributes -band [System.IO.FileAttributes]::ReparsePoint) -or
            ($Attributes -band [System.IO.FileAttributes]::Directory)) {
            throw "WINDOWS_LSP_OUTPUT_FILE_UNSAFE"
        }
    } catch [System.IO.FileNotFoundException] {
        # Only this named generated file may be created.
    } catch [System.IO.DirectoryNotFoundException] {
        # Its parent is created separately by Assert-PlainDirectory.
    }
}

function Publish-ForExecutable {
    if ([string]::IsNullOrWhiteSpace($ExecutableDirectory)) {
        return
    }
    $Destination = [System.IO.Path]::GetFullPath(
        (Join-Path $ExecutableDirectory "windows-lsp"))
    Assert-PlainDirectory $Destination
    foreach ($FileName in $ExpectedFiles) {
        Assert-PlainOutputFile (Join-Path $Destination $FileName)
    }
    if ([string]::Equals(
        $Destination, [System.IO.Path]::GetFullPath($Bundle),
        [System.StringComparison]::OrdinalIgnoreCase)) {
        return
    }
    $ManifestHash = (Get-FileHash -LiteralPath (Join-Path $Bundle "runtime.json") -Algorithm SHA256).Hash.ToLowerInvariant()
    $AlreadyCurrent = $false
    if (Valid-Bundle $Destination) {
        $AlreadyCurrent = (Get-FileHash -LiteralPath (Join-Path $Destination "runtime.json") -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $ManifestHash
    }
    if (-not $AlreadyCurrent) {
        # Copy only the five declared files, and publish the manifest last.
        # Unknown files are neither deleted nor staged as runtime DLL peers.
        foreach ($FileName in @("LICENSE-MIT", "LICENSE-APACHE", "Cargo.lock", "rust-analyzer.exe", "runtime.json")) {
            Copy-Item -LiteralPath (Join-Path $Bundle $FileName) -Destination (Join-Path $Destination $FileName) -Force
        }
    }
    Assert-Hash (Join-Path $Destination "runtime.json") $ManifestHash "profile output manifest"
    if (-not (Valid-Bundle $Destination)) {
        throw "WINDOWS_LSP_PROFILE_OUTPUT_INVALID"
    }
    Write-Host "WINDOWS_LSP_PROFILE_OUTPUT=VERIFIED"
}

if ($Source.schema_version -ne 1 -or
    $Source.runtime_id -cne "browseros-rust-analyzer-1.100.0-stdio-1" -or
    $Source.source_repository -cne "https://github.com/rust-lang/rust" -or
    $Source.source_commit -cne "0ef1bc386059596dada8fb65b50425aab2ea637a" -or
    $Source.source_subdirectory -cne "src/tools/rust-analyzer" -or
    $Source.rust_release -cne "1.100.0" -or
    $Source.target -cne "x86_64-pc-windows-msvc" -or
    $Source.patch_file -cne "metadata-stdin.patch" -or
    $Source.metadata_source_file -cne "src/tools/rust-analyzer/crates/project-model/src/lib.rs" -or
    $Source.lock_file -cne "Cargo.lock" -or
    @($Source.licenses).Count -ne 2 -or
    $Source.licenses[0].file -cne "LICENSE-MIT" -or
    $Source.licenses[1].file -cne "LICENSE-APACHE") {
    throw "WINDOWS_LSP_SOURCE_CONTRACT_INVALID"
}
foreach ($License in $Source.licenses) {
    if ($License.source -cne ($Source.source_subdirectory + "/" + $License.file)) {
        throw "WINDOWS_LSP_LICENSE_SOURCE_INVALID"
    }
}
Assert-Hash (Join-Path $Assets $Source.patch_file) $Source.patch_sha256 "reviewed patch"
Assert-Hash (Join-Path $Assets $Source.lock_file) $Source.lock_sha256 "reviewed lock"

Assert-PlainDirectory $Bundle
foreach ($FileName in $ExpectedFiles) {
    Assert-PlainOutputFile (Join-Path $Bundle $FileName)
}
$BuildLockPath = Join-Path $Bundle ".prepare.lock"
Assert-PlainOutputFile $BuildLockPath
$BuildLock = $null
$Checkout = $null
$SavedEnvironment = @{}
try {
    try {
        $BuildLock = [System.IO.File]::Open(
            $BuildLockPath, [System.IO.FileMode]::OpenOrCreate,
            [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
    } catch {
        throw "WINDOWS_LSP_PREPARE_BUSY: another prepare owns this bundle"
    }
    if (Valid-Bundle $Bundle) {
        Write-Host "WINDOWS_LSP_BUNDLE=VERIFIED_CACHED"
        Publish-ForExecutable
        return
    }

    if ([string]::IsNullOrWhiteSpace($WorkDirectory)) {
        $WorkDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "browseros-windows-lsp"
    }
    $WorkDirectory = [System.IO.Path]::GetFullPath($WorkDirectory)
    Assert-PlainDirectory $WorkDirectory
    $Checkout = Join-Path $WorkDirectory ("source-" + [guid]::NewGuid().ToString("N"))
    $TargetDirectory = Join-Path $WorkDirectory (
        "target-" + $Source.source_commit.Substring(0, 12) + "-" + $Source.patch_sha256.Substring(0, 12) +
        "-" + $Source.lock_sha256.Substring(0, 12))
    Assert-PlainDirectory $TargetDirectory

    # The tag is only a transport selector; the commit and every changed input
    # are checked before applying a patch or invoking Cargo.
    Invoke-Checked "git" @(
        "-c", "core.autocrlf=false", "clone", "--filter=blob:none", "--no-checkout",
        "--depth", "1", "--branch", $Source.rust_release,
        $Source.source_repository, $Checkout)
    Invoke-Checked "git" @("-C", $Checkout, "config", "core.autocrlf", "false")
    Invoke-Checked "git" @("-C", $Checkout, "sparse-checkout", "init", "--cone")
    Invoke-Checked "git" @("-C", $Checkout, "sparse-checkout", "set", $Source.source_subdirectory)
    $ActualCommit = Read-CheckedOutput "git" @("-C", $Checkout, "rev-parse", "HEAD")
    if ($ActualCommit -cne $Source.source_commit) {
        throw "WINDOWS_LSP_SOURCE_COMMIT_MISMATCH"
    }
    Invoke-Checked "git" @("-C", $Checkout, "checkout", "--detach", $Source.source_commit)
    $Project = Join-Path $Checkout $Source.source_subdirectory
    $MetadataSource = Join-Path $Checkout $Source.metadata_source_file
    $BuildLockFile = Join-Path $Project "Cargo.lock"
    Assert-Hash $MetadataSource $Source.upstream_metadata_sha256 "upstream metadata"
    Assert-Hash $BuildLockFile $Source.upstream_lock_sha256 "upstream lock"
    foreach ($License in $Source.licenses) {
        Assert-Hash (Join-Path $Checkout $License.source) $License.sha256 "upstream license"
    }
    $PatchPath = Join-Path $Assets $Source.patch_file
    Invoke-Checked "git" @("-C", $Checkout, "apply", "--check", "--whitespace=error-all", $PatchPath)
    Invoke-Checked "git" @("-C", $Checkout, "apply", "--whitespace=error-all", $PatchPath)
    Copy-Item -LiteralPath (Join-Path $Assets "Cargo.lock") -Destination $BuildLockFile -Force
    Assert-Hash $MetadataSource $Source.patched_metadata_sha256 "patched metadata"
    Assert-Hash $BuildLockFile $Source.lock_sha256 "security lock"
    $Changed = @((
        Read-CheckedOutput "git" @("-C", $Checkout, "diff", "--name-only")
    ) -split "\r?\n" | Where-Object { $_ })
    $ExpectedChanged = @($Source.metadata_source_file, $Source.source_subdirectory + "/Cargo.lock") |
        Sort-Object
    if ((($Changed | Sort-Object) -join "|") -cne ($ExpectedChanged -join "|")) {
        throw "WINDOWS_LSP_UNEXPECTED_SOURCE_CHANGE"
    }

    # build.rs supplies host-Cargo variables. Do not let those select another
    # compiler, linker flags, target directory, wrapper or parent jobserver.
    $ClearNames = @(
        "RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
        "RUSTFLAGS", "RUSTDOCFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_ENCODED_RUSTDOCFLAGS",
        "RUSTC_BOOTSTRAP", "RUSTUP_TOOLCHAIN", "CARGO_HOME",
        "CARGO_BUILD_TARGET", "CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR",
        "CARGO_BUILD_RUSTC", "CARGO_BUILD_RUSTDOC", "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
        "CARGO_MAKEFLAGS", "MAKEFLAGS", "MFLAGS", "CARGO_BUILD_JOBS", "CFG_RELEASE"
    )
    $ClearNames += @(Get-ChildItem Env: | Where-Object {
        $_.Name -match "^CARGO_PROFILE_" -or $_.Name -match "^CARGO_TARGET_.*_(RUSTFLAGS|LINKER)$"
    } | ForEach-Object { $_.Name })
    foreach ($EnvironmentName in ($ClearNames | Sort-Object -Unique)) {
        $SavedEnvironment[$EnvironmentName] = [Environment]::GetEnvironmentVariable($EnvironmentName, "Process")
        [Environment]::SetEnvironmentVariable($EnvironmentName, $null, "Process")
    }
    $BuildToolchain = $Source.rust_release + "-" + $Source.target
    Invoke-Checked "rustup" @(
        "toolchain", "install", $BuildToolchain, "--profile", "minimal", "--no-self-update")
    $Rustc = Read-CheckedOutput "rustup" @("which", "--toolchain", $BuildToolchain, "rustc")
    $Cargo = Read-CheckedOutput "rustup" @("which", "--toolchain", $BuildToolchain, "cargo")
    $Rustdoc = Read-CheckedOutput "rustup" @("which", "--toolchain", $BuildToolchain, "rustdoc")
    $CompilerHash = (Get-FileHash -LiteralPath $Rustc -Algorithm SHA256).Hash.ToLowerInvariant()
    $VersionText = Read-CheckedOutput $Rustc @("--version", "--verbose")
    $VersionFields = @{}
    foreach ($VersionLine in ($VersionText -split "\r?\n")) {
        $Pair = $VersionLine -split ": ", 2
        if ($Pair.Count -eq 2) { $VersionFields[$Pair[0]] = $Pair[1] }
    }
    if ($VersionFields["release"] -cne $Source.rust_release -or
        $VersionFields["commit-hash"] -cne $Source.source_commit -or
        $VersionFields["host"] -cne $Source.target) {
        throw "WINDOWS_LSP_COMPILER_IDENTITY_MISMATCH"
    }
    # A dedicated Cargo home avoids inheriting user/parent wrappers or config.
    # Only this fixed-source build uses its cache; the host project is unchanged.
    $env:CARGO_HOME = Join-Path $WorkDirectory "cargo-home"
    Assert-PlainDirectory $env:CARGO_HOME
    $env:RUSTC = $Rustc
    $env:CARGO_BUILD_RUSTC = $Rustc
    $env:RUSTDOC = $Rustdoc
    $env:RUSTC_WRAPPER = ""
    $env:RUSTC_WORKSPACE_WRAPPER = ""
    $env:CARGO_BUILD_RUSTC_WRAPPER = ""
    $env:CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER = ""
    $env:RUSTFLAGS = ""
    $env:CARGO_ENCODED_RUSTFLAGS = ""
    $env:CFG_RELEASE = "1.100.0-browseros-stdio-1"
    # Cargo discovers config from the working directory, not --manifest-path.
    # The fresh source checkout is independent of the BrowserOS workspace.
    Push-Location -LiteralPath $Project
    try {
        Invoke-Checked $Cargo @(
            "build", "--manifest-path", (Join-Path $Project "Cargo.toml"), "--locked",
            "--release", "--target", $Source.target, "--target-dir", $TargetDirectory,
            "--jobs", "2", "-p", "rust-analyzer", "--bin", "rust-analyzer")
    } finally {
        Pop-Location
    }
    Assert-Hash $Rustc $CompilerHash "compiler after build"
    Assert-Hash $MetadataSource $Source.patched_metadata_sha256 "metadata after build"
    Assert-Hash $BuildLockFile $Source.lock_sha256 "lock after build"

    $Executable = Join-Path $TargetDirectory ($Source.target + "/release/rust-analyzer.exe")
    $ExecutableInfo = Get-Item -LiteralPath $Executable
    if ($ExecutableInfo.Length -le 0 -or $ExecutableInfo.Length -gt 268435456) {
        throw "WINDOWS_LSP_EXECUTABLE_SIZE_INVALID"
    }
    $Candidate = Join-Path $Checkout "browseros-runtime-bundle"
    [System.IO.Directory]::CreateDirectory($Candidate) | Out-Null
    Copy-Item -LiteralPath $Executable -Destination (Join-Path $Candidate "rust-analyzer.exe")
    Copy-Item -LiteralPath $BuildLockFile -Destination (Join-Path $Candidate "Cargo.lock")
    foreach ($License in $Source.licenses) {
        Copy-Item -LiteralPath (Join-Path $Checkout $License.source) -Destination (Join-Path $Candidate $License.file)
    }
    $Record = [ordered]@{
        schema_version = 1
        source = $Source
        artifact = [ordered]@{
            file = "rust-analyzer.exe"
            sha256 = (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash.ToLowerInvariant()
            size = $ExecutableInfo.Length
        }
        build = [ordered]@{
            rustc_release = $Source.rust_release
            rustc_commit = $Source.source_commit
            rustc_sha256 = $CompilerHash
            target = $Source.target
        }
    }
    $RecordText = ($Record | ConvertTo-Json -Depth 30).Replace(
        ([string][char]13 + [char]10), $LineFeed) + $LineFeed
    [System.IO.File]::WriteAllText((Join-Path $Candidate "runtime.json"), $RecordText, $Utf8)
    if (-not (Valid-Bundle $Candidate)) {
        throw "WINDOWS_LSP_GENERATED_BUNDLE_INVALID"
    }
    # Publish the manifest last. An interrupted replacement can never validate
    # against the old compiled manifest and must be prepared again.
    foreach ($FileName in @("LICENSE-MIT", "LICENSE-APACHE", "Cargo.lock", "rust-analyzer.exe", "runtime.json")) {
        Copy-Item -LiteralPath (Join-Path $Candidate $FileName) -Destination (Join-Path $Bundle $FileName) -Force
    }
    if (-not (Valid-Bundle $Bundle)) {
        throw "WINDOWS_LSP_PUBLISHED_BUNDLE_INVALID"
    }
    Publish-ForExecutable
    Write-Host ("WINDOWS_LSP_BUNDLE=BUILT source=" + $Source.source_commit +
        " binary_sha256=" + $Record.artifact.sha256 + " lock_sha256=" + $Source.lock_sha256)
} finally {
    try {
        foreach ($EnvironmentName in $SavedEnvironment.Keys) {
            [Environment]::SetEnvironmentVariable($EnvironmentName, $SavedEnvironment[$EnvironmentName], "Process")
        }
        if ($null -ne $Checkout -and [System.IO.Directory]::Exists($Checkout)) {
            Remove-Item -LiteralPath $Checkout -Recurse -Force
        }
    } finally {
        if ($null -ne $BuildLock) {
            $BuildLock.Dispose()
            Remove-Item -LiteralPath $BuildLockPath -Force -ErrorAction SilentlyContinue
        }
    }
}
