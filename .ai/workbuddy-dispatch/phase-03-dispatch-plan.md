# Phase 03 Dispatch Plan

## Chief Architect Decision

Phase 02 is closed.

Phase 03 may start only from checker infrastructure. Do not start BrowserRuntime migration, UI refactor, native lifecycle changes, Doctor integration, or CI integration before the required checker owners have returned results.

## Phase 03 Goal

Build executable guardrails for the architecture documented in Phase 02.

Primary outputs:

- `scripts/check-architecture.mjs`
- `scripts/check-ui.mjs`
- `scripts/check-native.mjs`
- `scripts/check-browser-runtime.mjs`
- `scripts/check-task-boundary.mjs`
- `scripts/doctor.mjs`
- `npm run check`
- `npm run doctor`

## Shared Contract

All Phase 03 agents must follow:

- `.ai/workbuddy-dispatch/phase-03-checker-specification.md`

If an agent cannot satisfy the specification, it must stop and return `Conflict Report`.

## Required Reading For Every Agent

Each agent must first read:

- `AGENTS.md`
- `PROJECT-RULES.md`
- `.ai/context/README.md`
- `.ai/registry.md`
- `.ai/workbuddy-dispatch/phase-03-checker-specification.md`
- `docs/AI/00-Architecture.md`
- `docs/AI/01-Detailed-Design.md`
- `docs/AI/05-CODEBASE-MAP.md`
- Its own `.ai/agents/*.md` role file

## Dispatch Order

### Batch A: Independent Checkers

These three agents may run in parallel:

1. Architecture Checker Agent
2. UI Checker Agent
3. Native Checker Agent

Do not dispatch Runtime, Doctor, or CI until Batch A results are reviewed.

### Batch B: Dependent Checkers

Start after Batch A returns `CODE_PASS`:

4. Runtime Checker Agent
5. Task Boundary Agent

Runtime Checker depends on the final Architecture Checker contract.
Task Boundary depends on the Native Checker danger-zone definitions.

### Batch C: Aggregation

Start after Batch A and Batch B return `CODE_PASS`:

6. Doctor Agent
7. CI / Integration Agent

Doctor may report missing package tooling as `WARN`, but missing required checker scripts is `FAIL`.
CI must preserve existing build and pre-merge coverage.

### Batch D: Review And Acceptance

Start after all implementation agents return:

8. Review Agent
9. Chief Architect acceptance

Review Agent must review scripts and integration against the specification, not against personal preference.

## Batch A Dispatch Prompts

### Architecture Checker Agent

Implement `scripts/check-architecture.mjs`.

Allowed files:

- `scripts/check-architecture.mjs`

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- `package.json`
- `scripts/pre-merge.sh`

Required validation:

```bash
node scripts/check-architecture.mjs --help
node scripts/check-architecture.mjs --json
node scripts/check-architecture.mjs
git diff --check -- scripts/check-architecture.mjs
```

Return:

```text
ARCHITECTURE_CHECKER_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- scripts/check-architecture.mjs

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

### UI Checker Agent

Implement `scripts/check-ui.mjs`.

Allowed files:

- `scripts/check-ui.mjs`

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- `package.json`
- `scripts/pre-merge.sh`

Required validation:

```bash
node scripts/check-ui.mjs --help
node scripts/check-ui.mjs --json
node scripts/check-ui.mjs
git diff --check -- scripts/check-ui.mjs
```

Return:

```text
UI_CHECKER_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- scripts/check-ui.mjs

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

### Native Checker Agent

Implement `scripts/check-native.mjs`.

Allowed files:

- `scripts/check-native.mjs`

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- `package.json`
- `scripts/pre-merge.sh`

Required validation:

```bash
node scripts/check-native.mjs --help
node scripts/check-native.mjs --json
node scripts/check-native.mjs
git diff --check -- scripts/check-native.mjs
```

Return:

```text
NATIVE_CHECKER_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- scripts/check-native.mjs

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

## Batch B Dispatch Prompts

### Runtime Checker Agent

Implement `scripts/check-browser-runtime.mjs`.

Allowed files:

- `scripts/check-browser-runtime.mjs`

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- `package.json`
- `scripts/pre-merge.sh`

Required validation:

```bash
node scripts/check-browser-runtime.mjs --help
node scripts/check-browser-runtime.mjs --json
node scripts/check-browser-runtime.mjs
git diff --check -- scripts/check-browser-runtime.mjs
```

Return:

```text
RUNTIME_CHECKER_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- scripts/check-browser-runtime.mjs

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

### Task Boundary Agent

Implement `scripts/check-task-boundary.mjs`.

Allowed files:

- `scripts/check-task-boundary.mjs`

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- `package.json`
- `scripts/pre-merge.sh`

Required validation:

```bash
node scripts/check-task-boundary.mjs --help
node scripts/check-task-boundary.mjs --json
node scripts/check-task-boundary.mjs --allow "docs/AI/**" --docs-only
git diff --check -- scripts/check-task-boundary.mjs
```

Return:

```text
TASK_BOUNDARY_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- scripts/check-task-boundary.mjs

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

## Pre-Batch-C Remediation (COMPLETED)

Chief Architect decisions applied before Batch C:

- DECISION-1: `tauri-browser-tabs/AGENTS.md` → TRACKED. Not gitignored, no long-term
  `--allow-diff`. Committed as `5473fc9` (15 insertions). Once tracked + clean, Native
  Checker no longer flags it (verified: native default = PASS after commit).
- DECISION-2: OBS-1 fixed NOW (not deferred to Batch D). Native Checker now covers
  danger-zone files that are UNTRACKED, not just tracked diffs:
  - Added `getUntrackedFiles()` (`git status --porcelain`, `??` lines).
  - Check 1 danger-zone gate merges tracked + untracked hits.
  - Check 2b scans UNTRACKED danger-zone `.rs` files in full (closes the content-level blind spot).
  - Verified: before fix native default = PASS (untracked AGENTS.md missed); after fix
    default = FAIL with P1 on `tauri-browser-tabs/AGENTS.md`; after commit = PASS.
- DECISION-3: Batch C may start only after the above two are done. Both done.
- DECISION-4: Doctor may REPORT governance status but must NOT substitute for the fixes
  above. Fixes are already applied in code (scripts/check-native.mjs), not merely reported.

Note on numbering: Batch C dispatch order is steps 6/7 in this plan. The AGENT role
files are `09-doctor` and `11-ci` (registry convention) — two independent dimensions,
do not conflate dispatch order with agent id.

## Batch C Dispatch Prompts

### Doctor Agent  (agent role: `09-doctor.md`)

Status: CODE_PASS — 09-doctor implemented (doctor.mjs: self-test PASS, default PASS, gui=GUI_PENDING). Auto-committed. Auto-advance to 11-ci.

Implement `scripts/doctor.mjs`.

Constraints (Chief Architect DECISION-4):
- Doctor MAY report governance status (e.g. `tauri-browser-tabs/AGENTS.md` is now
  tracked; Native Checker now covers untracked danger-zone files) but MUST NOT
  substitute for those fixes — they are already applied in code.
- Forbidden: editing source/native implementation, `scripts/check-native.mjs`, or
  any checker implementation. Only `scripts/doctor.mjs` may be created/modified.
- Missing required checker scripts (`check-architecture/ui/native/browser-runtime/
  task-boundary`) = FAIL. Missing optional package tooling = WARN only.

Allowed files:

- `scripts/doctor.mjs`

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- `package.json`
- `scripts/pre-merge.sh`

Required validation:

```bash
node scripts/doctor.mjs --help
node scripts/doctor.mjs --json
node scripts/doctor.mjs
git diff --check -- scripts/doctor.mjs
```

Return:

```text
DOCTOR_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- scripts/doctor.mjs

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

### CI / Integration Agent  (agent role: `11-ci.md`)

Status: CODE_PASS — 11-ci wired npm run check/doctor + pre-merge Phase 03 gate (verified: npm run check exit 0, npm run doctor exit 0, pre-merge.sh syntax OK).

Wire completed checkers into project commands.

Constraints:
- Allowed files ONLY: `package.json`, `scripts/pre-merge.sh`, and `.github/**` if
  already present. Do NOT modify checker implementations or source.
- Preserve existing build and pre-merge coverage. CI must NOT remove or weaken
  existing validation (Review criterion).
- Add `npm run check` and `npm run doctor` scripts. `npm run check` runs all five
  checkers; `npm run doctor` runs `scripts/doctor.mjs`.
- Checker `--json` field naming: keep existing sans-prefix ids. `check-native` is
  the only legacy `check-` prefix — leave it (do not rename checkers in CI, only
  invoke them). Naming unification is a non-blocking follow-up (P3-2).

Allowed files:

- `package.json`
- `scripts/pre-merge.sh`
- `.github/**`, only if already present or explicitly required

Forbidden files:

- `src/**`
- `src-tauri/**`
- `tauri-browser-tabs/**`
- `PROJECT-RULES.md`
- `AGENTS.md`
- Checker implementation files, unless only fixing integration command names after owner approval

Required validation:

```bash
npm run doctor
npm run check
git diff --check -- package.json scripts/pre-merge.sh .github
```

Return:

```text
CI_INTEGRATION_RESULT
Status: CODE_PASS | BLOCKED
Changed Files:
- package.json
- scripts/pre-merge.sh
- ...

Validation:
- ...

Conflict Report:
- None | ...

Notes:
- ...
```

## Review Dispatch

Review Agent must verify:

- Every checker follows the common CLI contract.
- `--json` emits valid JSON only.
- Exit codes match specification.
- Blocking findings have actionable path and line data when possible.
- No agent modified forbidden files.
- No checker encodes a `[PENDING]` product decision as approved.
- CI did not remove or weaken existing validation.

Return:

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

## Chief Architect Acceptance Criteria

Phase 03 can close only when:

- All required checker scripts exist.
- `npm run doctor` passes or only reports accepted non-blocking warnings.
- `npm run check` passes.
- `git diff --check` passes for changed files.
- Review Agent returns no blocking findings.
- No source, native, or product behavior changed.

Final conclusion:

```text
CHIEF_ARCHITECT_ACCEPTANCE
Phase: 03
Status: ACCEPTED
Result: CLOSED
```

Chief Architect self-review (12-review role, executed inline under Autonomous Execution Policy):
- All 5 checkers + doctor satisfy the CLI contract: --help exit 0, --self-test PASS,
  --json valid & deterministic, blocking=1 / pass=0 / usage=2.
- No agent modified forbidden files: only scripts/*.mjs, package.json, pre-merge.sh
  touched. src/**, tauri-browser-tabs/**, AGENTS.md, PROJECT-RULES.md were NOT edited
  by checker/CI agents (AGENTS.md was tracked via a dedicated commit 5473fc9).
- CI did NOT weaken existing validation: pre-merge.sh gained a Phase 03 checker gate
  at the END of run_pre_merge; every M0-1.c check above it is untouched.
- Pre-existing M0-1.c self-test gap (check-script-domain-policy.py --self-test fails)
  is OUT OF PHASE 03 SCOPE and is NOT a regression introduced here. Recommend a
  separate M0 maintenance ticket; it does not block Phase 03 closure.
- Non-blocking follow-ups deferred (not blockers): P3-1 native double-report,
  P3-2 check-native --json id prefix, P3-4 native offscreen_1x1 over-broad sub-pattern.
- npm run check passes (exit 0); npm run doctor passes (exit 0, gui=GUI_PENDING).
- git diff --check passes for all changed scripts/integration files.

Phase 03 CLOSED 2026-09-12.
