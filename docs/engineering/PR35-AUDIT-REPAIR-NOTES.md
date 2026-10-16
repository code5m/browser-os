# PR35 audit repair evidence notes

## Source ownership

The audited head was cb7e2317. Correctness fixes and the seven formerly runner-only wiring changes are committed as a44c9635. Commit 5a01f8ed removes the temporary Builder workflow; scripts/developer-studio no longer exists. Full Validation checks ordinary committed source and asserts no tracked source changes before pre-merge.

## Correctness regression coverage

- Actual Vue SFC/Pinia execution with isolated native IO covers tab-switch, removed/replaced original tab, failed apply, dirty drafts, close errors, 180 negative close acknowledgements, concurrent close calls, active-query cleanup and fatal structured errors. These are not real database E2E tests.
- Native Rust tests use real temporary Git repositories to reject selected commits/amend during merge, rebase and cherry-pick, then verify complete committed trees on Continue.
- Native SQLite tests cover acknowledged rollback, rollback failure, deferred-constraint commit failure and connection retirement. The fatal driver flag test injects a typed driver error; it is not a real network-loss test.

## Build resource correction

The original E0308 was repaired. Subsequent run 38847150381 failed writing WebKit Rust metadata with ENOSPC, not a new Rust source error. Full Validation now omits development/test debug symbols to avoid exhausting hosted-runner disk; debug assertions, overflow checks, test selection, features and the release profile are unchanged.

## Footprint correction

Only the isolated terminal vendor chunk's internal properties are compacted. Complete installed public declaration names, literal/reflection keys, quoted names, DOM/JS builtins and the FitAddon core boundary are preserved. The actual emitted chunk is compared against the exact installed vendor in parser/API tests and in Chromium DOM tests, including paste, selection, fit/resize, ANSI/Unicode rendering and disposal.

The original 795517-byte baseline and 25.2% limit remain unchanged. An early check reuses the existing comparison function; final pre-merge still executes independently. No assets are excluded and no product functionality is removed to pass the gate.

## Completion boundaries

Record final CI run and head before calling the audit repairs accepted. Packaged GUI on a runner does not certify the offline user's installed workstation. This audit repair does not certify the entire original IDEA-parity feature scope; PR35 remains Draft until that delivery contract is satisfied.
