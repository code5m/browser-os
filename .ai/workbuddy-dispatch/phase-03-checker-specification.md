# Phase 03 Checker Specification

## Role

Chief Architect owns this specification. Checker agents implement it, but they must not expand or weaken it without a conflict report.

## Mission

Create a shared contract for Phase 03 checker scripts before implementation begins.

Phase 03 is not a BrowserRuntime migration phase. It only creates guardrail scripts and integration entrypoints that make later architecture work safer.

## Required Reading

Each implementing agent must read these files first:

- `AGENTS.md`
- `PROJECT-RULES.md`
- `.ai/context/README.md`
- `.ai/registry.md`
- `docs/AI/00-Architecture.md`
- `docs/AI/01-Detailed-Design.md`
- `docs/AI/05-CODEBASE-MAP.md`
- Its own `.ai/agents/*.md` role file
- The existing target script, if it already exists

If these files conflict, stop and output `Conflict Report`.

## Global Rules

- Do not modify Vue, TypeScript, Rust, native plugin, or product behavior.
- Do not decide `[PENDING]` product requirements.
- Do not mark GUI validation as passed.
- Do not introduce network access.
- Do not add external dependencies unless Chief Architect explicitly approves.
- Prefer Node.js standard library for `.mjs` scripts.
- Existing approved legacy behavior may be reported as `WARN` before the corresponding migration phase, but newly introduced violations must be `FAIL`.
- Every checker must have deterministic output and stable exit codes.

## Common CLI Contract

Every Phase 03 checker script must support:

```bash
node scripts/<checker>.mjs
node scripts/<checker>.mjs --help
node scripts/<checker>.mjs --json
```

Optional flags:

```bash
node scripts/<checker>.mjs --strict
node scripts/<checker>.mjs --self-test
```

Do not implement `--fix` in Phase 03 unless Chief Architect explicitly approves it.

## Exit Codes

- `0`: PASS. No blocking findings.
- `1`: FAIL. One or more blocking findings.
- `2`: USAGE_ERROR. Invalid CLI arguments or unreadable required input.

Warnings do not change the exit code unless `--strict` explicitly upgrades them.

## Text Output Contract

Human output must use this shape:

```text
CHECK_NAME: PASS
```

or:

```text
CHECK_NAME: FAIL

[P1] path:line message
[P2] path:line message
```

Severity meanings:

- `P1`: architecture or native safety violation that can create unsafe future work.
- `P2`: documented boundary violation or high-risk drift.
- `P3`: consistency, naming, or migration-readiness issue.
- `WARN`: known current gap or non-blocking observation.

Every blocking finding must include a file path and line number when possible.

## JSON Output Contract

`--json` must emit valid JSON only, with this shape:

```json
{
  "check": "check-name",
  "status": "PASS",
  "findings": [],
  "warnings": [],
  "summary": {
    "filesScanned": 0
  }
}
```

Allowed `status` values:

- `PASS`
- `FAIL`
- `USAGE_ERROR`

Each finding:

```json
{
  "severity": "P2",
  "file": "src/example.ts",
  "line": 12,
  "message": "Actionable message",
  "rule": "PROJECT-RULES.md §6"
}
```

## Checker Ownership

### Architecture Checker

Owner file:

- `scripts/check-architecture.mjs`

Must check:

- Vue components do not directly call Tauri `invoke`.
- `src/bridge.ts` remains the current IPC boundary until BrowserRuntime migration lands.
- BrowserRuntime, MockRuntime, BrowserScene, and syncScene must not be claimed as implemented unless matching source files or symbols exist.
- Stores do not import native modules directly.
- Components do not bypass the documented runtime or bridge boundary.

Initial expected status:

- Existing zero direct component `invoke` should PASS.
- Missing BrowserRuntime target should be WARN, not FAIL, until implementation phase starts.

### UI Checker

Owner file:

- `scripts/check-ui.mjs`

Must check:

- No new HTML Toast, Modal, Popover, Tooltip, or Overlay pattern is introduced over the browser region.
- No `position: fixed` browser-overlay workaround is introduced.
- No arbitrary high `z-index` workaround is introduced for native WebView overlap.
- Existing approved independent panels are not treated as failures unless they violate documented layout synchronization rules.

Initial expected status:

- Current approved status-bar and independent-layout patterns should PASS.
- Ambiguous legacy UI should be WARN unless clearly forbidden by `PROJECT-RULES.md`.

### Native Checker

Owner file:

- `scripts/check-native.mjs`

Must check:

- Native danger-zone files are detected in the current diff:
  - `src-tauri/src/bridge.rs`
  - `src-tauri/src/**/linux.rs`
  - `src-tauri/capabilities/**`
  - `tauri-browser-tabs/**`
- Direct changes to native danger-zone files must FAIL unless an explicit allow marker is passed by CI or task boundary tooling.
- Deprecated native patterns must be detected when scanning full source:
  - `set_size_request`
  - `queue_resize`
  - `webview.hide()`
  - offscreen `1x1` resize hiding
  - extreme DPR-based resize workaround

Initial expected status:

- Full-source deprecated native pattern matches should be WARN if they are historical and documented.
- Current diff touching danger-zone files should FAIL by default.

### Runtime Checker

Owner file:

- `scripts/check-browser-runtime.mjs`

Must check:

- BrowserRuntime interface consistency once introduced.
- MockRuntime parity once introduced.
- BrowserScene and syncScene naming consistency once introduced.
- No component imports a future runtime implementation directly when an adapter boundary exists.

Initial expected status:

- Since BrowserRuntime is an approved target and not current source, absence should PASS with a clear `target-not-yet-implemented` summary.
- Partial implementation should FAIL if interface, mock, and call sites are inconsistent.

### Task Boundary Checker

Owner file:

- `scripts/check-task-boundary.mjs`

Must check:

- Given an allowlist and current diff, changed files stay inside task scope.
- Native danger-zone edits fail unless explicitly authorized.
- Documentation-only tasks fail if source or scripts are changed.

Required flags:

```bash
node scripts/check-task-boundary.mjs --allow "src/components/**"
node scripts/check-task-boundary.mjs --allow "docs/AI/**" --docs-only
```

Initial expected status:

- No changed file outside allowlist: PASS.
- Any source/script/native change in docs-only mode: FAIL.

### Doctor

Owner file:

- `scripts/doctor.mjs`

Must report:

- Git status summary.
- Node/package script availability.
- Build command availability.
- Existing checker availability.
- Native toolchain availability when detectable without installing.
- Package command availability.
- Runtime target summary.
- Version summary from `package.json` and Tauri config.

Initial expected status:

- Missing optional tools should WARN.
- Missing required project files should FAIL.
- Doctor must not run expensive build/package commands by default.

### CI / Integration

Owner files:

- `package.json`
- `scripts/pre-merge.sh`
- CI workflow files, if present

Must add or preserve:

```bash
npm run check
npm run doctor
```

`npm run check` should run:

1. `node scripts/check-architecture.mjs`
2. `node scripts/check-ui.mjs`
3. `node scripts/check-native.mjs`
4. `node scripts/check-browser-runtime.mjs`
5. `node scripts/check-task-boundary.mjs` only when a task allowlist is provided, or in a documented no-op/self-test mode

Integration must preserve the existing `npm run build` and `scripts/pre-merge.sh` coverage.

## Validation Requirements

Each script owner must run:

```bash
node scripts/<checker>.mjs --help
node scripts/<checker>.mjs --json
node scripts/<checker>.mjs
git diff --check -- scripts/<checker>.mjs
```

CI Agent must run:

```bash
npm run doctor
npm run check
git diff --check -- package.json scripts/pre-merge.sh .github
```

Run `npm run build` only after checker scripts are integrated, or when an implementation changes files that could affect build output.

## Stop Conditions

Stop and output `Conflict Report` if:

- A checker requires modifying source behavior to pass.
- A checker would need to decide a `[PENDING]` requirement.
- Existing docs and source disagree on a rule to enforce.
- Existing `scripts/pre-merge.sh` behavior would be removed or weakened.
- Native danger-zone changes appear without explicit authorization.

## Phase 03 Dispatch Order

Recommended order:

1. Architecture Checker
2. UI Checker
3. Native Checker
4. Runtime Checker
5. Task Boundary Checker
6. Doctor
7. CI / Integration
8. Review Agent
9. Chief Architect acceptance

Architecture, UI, Native, and Runtime checkers may run in parallel after this specification is accepted.

Doctor should wait until at least two checker scripts exist.

CI / Integration should wait until all checker scripts exist or explicitly support no-op/self-test mode.

## Required Agent Result Format

Each implementing agent must return:

```text
<AGENT_NAME>_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- ...

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

Review Agent must return:

```text
PHASE_03_REVIEW_RESULT
Status: CODE_PASS | BLOCKED
Findings:
- ...

Open Questions:
- ...

Residual Risk:
- ...

Recommended Owner Actions:
- ...
```

Chief Architect closes Phase 03 only after all required checkers and integration pass review.
