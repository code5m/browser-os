# Definitions only. Sourcing this file does not build or run anything.
# Every required native selection uses the same execution/coverage rules.
Set-StrictMode -Version Latest

function Get-NativeRustTestBinary {
    param(
        [Parameter(Mandatory = $true)][string]$Manifest,
        [Parameter(Mandatory = $true)][string]$Package
    )
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

function Invoke-NativeRustHarness {
    param(
        [Parameter(Mandatory = $true)][string]$Executable,
        [AllowEmptyString()][string]$Filter = '',
        [ValidateRange(1, 1800)][int]$TimeoutSeconds = 240
    )
    $slug = (($Filter -replace '[^A-Za-z0-9_.-]', '_') + '-' + [guid]::NewGuid().ToString('N'))
    $stdout = Join-Path $env:RUNNER_TEMP "$slug.out.log"
    $stderr = Join-Path $env:RUNNER_TEMP "$slug.err.log"
    $arguments = @('--test-threads=1', '--nocapture')
    if ($Filter) { $arguments = @($Filter) + $arguments }
    $process = $null
    $result = [ordered]@{
        source_sha = $env:BROWSEROS_SOURCE_SHA
        executable = [System.IO.Path]::GetFileName($Executable)
        filter = $Filter
        status = 'NOT_RUN'
        passed = 0
        failed = 0
        ignored = 0
    }
    Write-Host "::group::required native test $Filter"
    try {
        $process = Start-Process -FilePath $Executable -NoNewWindow -PassThru -ArgumentList $arguments `
            -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $result.status = 'TIMEOUT'
            try { $process.Kill($true) } catch {}
            if (-not $process.WaitForExit(5000)) {
                $env:BROWSEROS_NATIVE_CLEANUP_BLOCKED = '1'
                'BROWSEROS_NATIVE_CLEANUP_BLOCKED=1' | Add-Content $env:GITHUB_ENV
                throw "NATIVE_TEST_PROCESS_CLEANUP_TIMEOUT:$Filter"
            }
            throw "NATIVE_TEST_TIMEOUT:$Filter"
        }
        $process.WaitForExit()
        $output = ((Get-Content $stdout -ErrorAction SilentlyContinue) +
            (Get-Content $stderr -ErrorAction SilentlyContinue) | Out-String)
        $output | Write-Host
        if ($process.ExitCode -ne 0) {
            $result.status = 'FAIL'
            throw "NATIVE_TEST_FAILED:$Filter"
        }
        $summaries = [regex]::Matches($output, '(?m)^test result: ok\. (?<passed>\d+) passed; (?<failed>\d+) failed; (?<ignored>\d+) ignored;')
        if ($summaries.Count -ne 1) {
            $result.status = 'BLOCKED'
            throw "NATIVE_TEST_SUMMARY_NOT_UNIQUE:$Filter"
        }
        $result.passed = [int]$summaries[0].Groups['passed'].Value
        $result.failed = [int]$summaries[0].Groups['failed'].Value
        $result.ignored = [int]$summaries[0].Groups['ignored'].Value
        if ($result.passed -lt 1 -or $result.failed -ne 0 -or $result.ignored -ne 0 -or
            $output -match '(?im)^\s*\[?SKIP(?:PED)?\b') {
            $result.status = 'BLOCKED'
            throw "NATIVE_REQUIRED_TESTS_NOT_EXECUTED:$Filter"
        }
        $result.status = 'PASS'
    } catch {
        if ($result.status -eq 'NOT_RUN') { $result.status = 'BLOCKED' }
        throw
    } finally {
        if ($null -ne $process) { $process.Dispose() }
        $result | ConvertTo-Json -Compress |
            Add-Content (Join-Path $env:RUNNER_TEMP 'required-native-test-results.jsonl')
        "Native filter '$Filter': $($result.status); passed=$($result.passed); failed=$($result.failed); ignored=$($result.ignored); source=$($result.source_sha)" |
            Add-Content $env:GITHUB_STEP_SUMMARY
        Write-Host "::endgroup::"
    }
}
