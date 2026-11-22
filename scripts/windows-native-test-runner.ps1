# Definitions only. Sourcing this file does not compile or execute native code.
# The supervisor and test binaries run only after an explicit harness invocation.
Set-StrictMode -Version Latest

function Get-NativeSelectionManifest {
    $path = Join-Path $PSScriptRoot 'windows-native-required-tests.json'
    if ((Get-Item -LiteralPath $path).Length -gt 131072) {
        throw 'NATIVE_TEST_MANIFEST_TOO_LARGE'
    }
    $manifest = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json -ErrorAction Stop
    if ($manifest.schema_version -ne 1) { throw 'NATIVE_TEST_MANIFEST_VERSION' }
    $selectionIds = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $selections = @($manifest.selections)
    if ($selections.Count -lt 1 -or $selections.Count -gt 128) {
        throw 'NATIVE_TEST_MANIFEST_SELECTION_COUNT'
    }
    foreach ($selection in $selections) {
        foreach ($property in @('id', 'package', 'filter', 'role', 'required_count', 'required_names', 'allow_additional_tests', 'allow_ignored', 'timeout_seconds')) {
            if ($selection.PSObject.Properties.Name -notcontains $property) {
                throw "NATIVE_TEST_MANIFEST_PROPERTY_MISSING:$property"
            }
        }
        if ($selection.id -notmatch '^[A-Za-z0-9_.-]{1,160}$' -or -not $selectionIds.Add([string]$selection.id)) {
            throw 'NATIVE_TEST_MANIFEST_SELECTION_ID'
        }
        if ($selection.role -notin @('required', 'full-suite') -or
            $selection.timeout_seconds -lt 1 -or $selection.timeout_seconds -gt 1800 -or
            $selection.allow_additional_tests -isnot [bool] -or $selection.allow_ignored -isnot [bool] -or
            ($selection.role -eq 'required' -and $selection.allow_ignored)) {
            throw "NATIVE_TEST_MANIFEST_POLICY:$($selection.id)"
        }
        $required = @($selection.required_names)
        if ($selection.required_count -ne $required.Count -or $required.Count -lt 1 -or $required.Count -gt 1024) {
            throw "NATIVE_TEST_MANIFEST_REQUIRED_COUNT:$($selection.id)"
        }
        $names = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach ($name in $required) {
            if ($name -isnot [string] -or $name -notmatch '^[A-Za-z0-9_:]+$' -or -not $names.Add($name)) {
                throw "NATIVE_TEST_MANIFEST_REQUIRED_NAME:$($selection.id)"
            }
        }
        if ($selection.filter -isnot [string] -or $selection.filter.Contains([char]0) -or
            $selection.filter.Contains([char]10) -or $selection.filter.Contains([char]13)) {
            throw "NATIVE_TEST_MANIFEST_FILTER:$($selection.id)"
        }
    }
    return $manifest
}

function Initialize-NativeSupervisor {
    if (-not ('BrowserOs.NativeTests.Supervisor' -as [type])) {
        Add-Type -Path (Join-Path $PSScriptRoot 'windows-native-test-supervisor.cs') -ErrorAction Stop
    }
}

function Set-NativeCleanupQuarantine {
    $env:BROWSEROS_NATIVE_CLEANUP_BLOCKED = '1'
    if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_ENV)) {
        'BROWSEROS_NATIVE_CLEANUP_BLOCKED=1' | Add-Content -LiteralPath $env:GITHUB_ENV -Encoding utf8
    }
}

function Get-NativeRustTestBinary {
    param(
        [Parameter(Mandatory = $true)][string]$Manifest,
        [Parameter(Mandatory = $true)][string]$Package
    )
    if ($env:BROWSEROS_NATIVE_CLEANUP_BLOCKED -eq '1') {
        throw 'NATIVE_JOB_CLEANUP_UNCONFIRMED: runner is quarantined'
    }
    $output = @(& cargo test --manifest-path $Manifest --locked -p $Package --lib --no-run --message-format=json 2>&1)
    if ($LASTEXITCODE -ne 0) {
        $output | Select-Object -Last 45 | Write-Host
        throw "NATIVE_TEST_COMPILATION_FAILED:$Package"
    }
    $targetName = $Package.Replace('-', '_')
    $artifacts = @($output | ForEach-Object {
        try { $_ | ConvertFrom-Json -ErrorAction Stop } catch { $null }
    } | Where-Object {
        $null -ne $_ -and $_.PSObject.Properties.Name -contains 'reason' -and
        $_.reason -eq 'compiler-artifact' -and
        $_.target.name -eq $targetName -and
        ($_.target.kind -contains 'lib') -and
        $_.profile.test -eq $true -and $_.executable
    })
    if ($artifacts.Count -ne 1 -or -not (Test-Path -LiteralPath $artifacts[0].executable -PathType Leaf)) {
        throw "NATIVE_TEST_BINARY_NOT_UNIQUE:$Package"
    }
    return [string]$artifacts[0].executable
}

function Invoke-SupervisedNativeProcess {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [Parameter(Mandatory = $true)][string]$LogPrefix,
        [ValidateRange(1, 1800)][int]$TimeoutSeconds
    )
    if ($env:BROWSEROS_NATIVE_CLEANUP_BLOCKED -eq '1') {
        throw 'NATIVE_JOB_CLEANUP_UNCONFIRMED: runner is quarantined'
    }
    Initialize-NativeSupervisor
    $stdoutPath = "$LogPrefix.out.log"
    $stderrPath = "$LogPrefix.err.log"
    # Output goes directly to native file handles, never an unbounded .NET pipe
    # buffer. This is a polled observation threshold, not a filesystem quota.
    $maximumOutputBytes = 32L * 1024L * 1024L
    $supervision = [BrowserOs.NativeTests.Supervisor]::Run(
        [System.IO.Path]::GetFullPath($Executable),
        $Arguments,
        [System.IO.Path]::GetFullPath($stdoutPath),
        [System.IO.Path]::GetFullPath($stderrPath),
        (Get-Location).ProviderPath,
        ($TimeoutSeconds * 1000),
        10000,
        $maximumOutputBytes
    )
    if (-not $supervision.CleanupComplete) {
        Set-NativeCleanupQuarantine
    }
    return [pscustomobject]@{
        supervision = $supervision
        stdout_path = $stdoutPath
        stderr_path = $stderrPath
        maximum_output_bytes = $maximumOutputBytes
    }
}

function Read-BoundedNativeOutput {
    param([Parameter(Mandatory = $true)]$Run)
    $totalLength = 0L
    foreach ($path in @($Run.stdout_path, $Run.stderr_path)) {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $totalLength += (Get-Item -LiteralPath $path).Length
        }
    }
    # Length is read after native cleanup, so a successful Job-empty proof also
    # prevents descendants from extending these inherited output handles.
    if ($totalLength -gt $Run.maximum_output_bytes) {
        throw 'NATIVE_TEST_OUTPUT_LIMIT: logs retained without loading them into memory'
    }
    $stdout = if (Test-Path -LiteralPath $Run.stdout_path -PathType Leaf) {
        [System.IO.File]::ReadAllText($Run.stdout_path)
    } else { '' }
    $stderr = if (Test-Path -LiteralPath $Run.stderr_path -PathType Leaf) {
        [System.IO.File]::ReadAllText($Run.stderr_path)
    } else { '' }
    return [pscustomobject]@{ stdout = $stdout; stderr = $stderr; combined = "$stdout$([Environment]::NewLine)$stderr" }
}

function Assert-NativeSupervision {
    param(
        [Parameter(Mandatory = $true)]$Run,
        [Parameter(Mandatory = $true)][string]$Phase
    )
    $proof = $Run.supervision
    if (-not $proof.CleanupComplete -or -not $proof.JobQuerySucceeded -or
        -not $proof.JobEmpty -or $proof.ActiveProcesses -ne 0) {
        Set-NativeCleanupQuarantine
        throw "NATIVE_JOB_CLEANUP_UNCONFIRMED:$Phase"
    }
    if (-not $proof.ProcessCreated -or -not $proof.AtomicJobMembershipVerified -or
        -not $proof.RootExited -or $proof.TimedOut -or $proof.OutputLimitExceeded -or
        $proof.DescendantsOutlivedRoot -or -not [string]::IsNullOrEmpty($proof.FailureCode)) {
        throw "NATIVE_TEST_SUPERVISION_FAILED:$($Phase):$($proof.FailureCode)"
    }
}

function Invoke-NativeRustHarness {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [Parameter(Mandatory = $true)][string]$SelectionId
    )
    if ($env:BROWSEROS_NATIVE_CLEANUP_BLOCKED -eq '1') {
        throw 'NATIVE_JOB_CLEANUP_UNCONFIRMED: refusing another native selection'
    }
    $manifest = Get-NativeSelectionManifest
    $selectionsForId = @($manifest.selections | Where-Object { $_.id -ceq $SelectionId })
    if ($selectionsForId.Count -ne 1) { throw "NATIVE_TEST_SELECTION_NOT_UNIQUE:$SelectionId" }
    $selection = $selectionsForId[0]
    $filter = [string]$selection.filter
    $temporaryDirectory = if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        [System.IO.Path]::GetTempPath()
    } else { $env:RUNNER_TEMP }
    $slug = $SelectionId + '-' + [guid]::NewGuid().ToString('N')
    $logPrefix = Join-Path $temporaryDirectory $slug
    $result = [ordered]@{
        source_sha = $env:BROWSEROS_SOURCE_SHA
        executable = [System.IO.Path]::GetFileName($Executable)
        selection = $SelectionId
        package = $selection.package
        filter = $filter
        role = $selection.role
        status = 'NOT_RUN'
        reason = $null
        passed = 0
        failed = 0
        ignored = 0
        required_count = $selection.required_count
        required_names = @($selection.required_names)
        resolved_required_names = @()
        enumerated_count = 0
        enumerated_names = @()
        additional_tests_allowed = $selection.allow_additional_tests
        additional_tests = @()
        executed_tests = @()
        enumeration_supervision = $null
        execution_supervision = $null
        enumeration_logs = @()
        execution_logs = @()
    }
    $failure = $null
    Write-Host "::group::native selection $SelectionId"
    try {
        $enumerationArguments = @('--list', '--format', 'terse', '--color', 'never')
        if ($filter.Length -gt 0) { $enumerationArguments = @($filter) + $enumerationArguments }
        $enumeration = Invoke-SupervisedNativeProcess -Executable $Executable -Arguments $enumerationArguments -LogPrefix "$logPrefix.list" -TimeoutSeconds 60
        $result.enumeration_supervision = $enumeration.supervision
        $result.enumeration_logs = @($enumeration.stdout_path, $enumeration.stderr_path)
        Assert-NativeSupervision -Run $enumeration -Phase 'enumeration'
        if ($enumeration.supervision.ExitCode -ne 0) { throw 'NATIVE_TEST_ENUMERATION_FAILED' }
        $listing = Read-BoundedNativeOutput -Run $enumeration
        $listed = [regex]::Matches($listing.stdout, '(?m)^(?<name>[^\r\n]+): test\r?$')
        $enumerated = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach ($row in $listed) {
            if (-not $enumerated.Add($row.Groups['name'].Value)) {
                throw 'NATIVE_TEST_ENUMERATION_DUPLICATE'
            }
        }
        if ($enumerated.Count -lt 1) { throw 'NATIVE_TEST_ENUMERATION_EMPTY' }
        $result.enumerated_names = @($enumerated | Sort-Object -CaseSensitive)
        $result.enumerated_count = $enumerated.Count
        $resolved = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach ($requiredName in @($selection.required_names)) {
            # A short name must resolve to exactly one complete Rust module path.
            # Adding a second same-named case requires an explicit manifest path.
            $candidates = @($enumerated | Where-Object {
                $_.Equals($requiredName, [StringComparison]::Ordinal) -or
                $_.EndsWith("::$requiredName", [StringComparison]::Ordinal)
            })
            if ($candidates.Count -ne 1 -or -not $resolved.Add([string]$candidates[0])) {
                throw "NATIVE_REQUIRED_NAME_MISSING_OR_AMBIGUOUS:$requiredName"
            }
        }
        $result.resolved_required_names = @($resolved | Sort-Object -CaseSensitive)
        $result.additional_tests = @($enumerated | Where-Object { -not $resolved.Contains($_) } | Sort-Object -CaseSensitive)
        if (-not $selection.allow_additional_tests -and $result.additional_tests.Count -ne 0) {
            throw 'NATIVE_TEST_ENUMERATION_UNREVIEWED_EXTRA'
        }
        Write-Host "Required names: $($resolved.Count); enumerated: $($enumerated.Count); allowed additional cases: $($result.additional_tests.Count)"

        # Capture ordinary Rust test output until its result row has been printed.
        # --show-output still exposes successful helpers' SKIP diagnostics without
        # --nocapture interleaving those messages into 'test NAME ... ok' rows.
        $executionArguments = @('--test-threads=1', '--show-output', '--format', 'pretty', '--color', 'never')
        if ($filter.Length -gt 0) { $executionArguments = @($filter) + $executionArguments }
        $execution = Invoke-SupervisedNativeProcess -Executable $Executable -Arguments $executionArguments -LogPrefix "$logPrefix.run" -TimeoutSeconds $selection.timeout_seconds
        $result.execution_supervision = $execution.supervision
        $result.execution_logs = @($execution.stdout_path, $execution.stderr_path)
        Assert-NativeSupervision -Run $execution -Phase 'execution'
        $output = Read-BoundedNativeOutput -Run $execution
        $output.combined -split '\r?\n' | Select-Object -Last 160 | Write-Host
        # libtest's pretty formatter appends a mode after the discovery name.
        # Keep that annotation outside the identity captured for exact matching.
        $rows = [regex]::Matches($output.stdout, '(?m)^test (?<name>[^\r\n]+?)(?: - (?:should panic|compile fail|compile))? \.\.\. (?<status>ok|FAILED|ignored)(?:, [^\r\n]*)?\r?$')
        $executed = [System.Collections.Generic.Dictionary[string, string]]::new([StringComparer]::Ordinal)
        foreach ($row in $rows) {
            $name = $row.Groups['name'].Value
            if (-not $enumerated.Contains($name) -or $executed.ContainsKey($name)) {
                throw "NATIVE_TEST_RESULT_UNEXPECTED_OR_DUPLICATE:$name"
            }
            $executed.Add($name, $row.Groups['status'].Value)
        }
        $result.executed_tests = @($executed.GetEnumerator() | Sort-Object Key -CaseSensitive | ForEach-Object {
            [pscustomobject]@{ name = $_.Key; status = $_.Value }
        })
        if ($executed.Count -ne $enumerated.Count) {
            throw "NATIVE_TEST_ENUMERATION_EXECUTION_MISMATCH:listed=$($enumerated.Count),reported=$($executed.Count)"
        }
        $summaries = [regex]::Matches($output.stdout, '(?m)^test result: (?<result>ok|FAILED)\. (?<passed>\d+) passed; (?<failed>\d+) failed; (?<ignored>\d+) ignored;')
        if ($summaries.Count -ne 1) { throw 'NATIVE_TEST_SUMMARY_NOT_UNIQUE' }
        $result.passed = [int]$summaries[0].Groups['passed'].Value
        $result.failed = [int]$summaries[0].Groups['failed'].Value
        $result.ignored = [int]$summaries[0].Groups['ignored'].Value
        $actualPassed = @($executed.Values | Where-Object { $_ -ceq 'ok' }).Count
        $actualFailed = @($executed.Values | Where-Object { $_ -ceq 'FAILED' }).Count
        $actualIgnored = @($executed.Values | Where-Object { $_ -ceq 'ignored' }).Count
        if ($result.passed -ne $actualPassed -or $result.failed -ne $actualFailed -or
            $result.ignored -ne $actualIgnored) {
            throw 'NATIVE_TEST_RESULT_COUNT_MISMATCH'
        }
        if ($execution.supervision.ExitCode -ne 0 -or $result.failed -ne 0 -or
            $summaries[0].Groups['result'].Value -cne 'ok') {
            $result.status = 'FAIL'
        }
        foreach ($name in $resolved) {
            if (-not $executed.ContainsKey($name) -or $executed[$name] -cne 'ok') {
                throw "NATIVE_REQUIRED_TEST_DID_NOT_PASS:$name"
            }
        }
        if ($result.status -eq 'FAIL') { throw 'NATIVE_TEST_FAILED' }
        if ((-not $selection.allow_ignored -and $result.ignored -ne 0) -or
            $output.combined -match '(?im)^\s*\[?SKIP(?:PED)?\b') {
            throw 'NATIVE_REQUIRED_TESTS_NOT_EXECUTED'
        }
        $result.status = if ($result.ignored -eq 0) { 'PASS' } else { 'PASS_WITH_IGNORED' }
    } catch {
        $failure = $_.Exception
        $result.reason = $failure.Message
        if ($result.status -eq 'NOT_RUN') {
            $timedOut = ($null -ne $result.enumeration_supervision -and $result.enumeration_supervision.TimedOut) -or
                ($null -ne $result.execution_supervision -and $result.execution_supervision.TimedOut)
            $result.status = if ($timedOut) { 'TIMEOUT' } else { 'BLOCKED' }
        }
    } finally {
        $result | ConvertTo-Json -Depth 12 -Compress |
            Add-Content -LiteralPath (Join-Path $temporaryDirectory 'required-native-test-results.jsonl') -Encoding utf8
        $summary = "Native selection '$SelectionId': $($result.status); required=$($result.required_count); enumerated=$($result.enumerated_count); passed=$($result.passed); failed=$($result.failed); ignored=$($result.ignored); source=$($result.source_sha)"
        Write-Host $summary
        if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_STEP_SUMMARY)) {
            $summary | Add-Content -LiteralPath $env:GITHUB_STEP_SUMMARY -Encoding utf8
        }
        Write-Host "::endgroup::"
    }
    if ($null -ne $failure) {
        $exception = [InvalidOperationException]::new("NATIVE_SELECTION_FAILED:$($SelectionId):$($result.status):$($result.reason)", $failure)
        $exception.Data['NativeResult'] = [pscustomobject]$result
        throw $exception
    }
    return [pscustomobject]$result
}
