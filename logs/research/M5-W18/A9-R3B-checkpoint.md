# A9 — Lane Checkpoint (M5-W18-R3B Git Interaction-Safety Matrix)

```text
LANE=A9
STATUS=PASS
MODE=RESEARCH_AND_PROTOTYPE
BASE=200f0f1   (origin/master HEAD; branch already rebased onto origin/master at prior R3 commit)
HEAD=<set by commit>
DISPATCH=M5-W18-R3B   (correction wave; supersedes R2B per WORKSPACE_IDENTITY.md + PARALLEL_COMMAND_BOARD.md)
WORKDIR=/home/ainfinit/.codex/worktrees/m5-w18-a9/mvp-browser-os-v3
BRANCH=codex/m5-w18-a9
REFERENCE=M5-W18-R3B-CORRECTION-TASKS-20260908.md, M5-W18-R2B-TASKS-20260908.md (historical, context), WORKSPACE_IDENTITY.md, PARALLEL_COMMAND_BOARD.md, A7-R3-git-workflow-reference-map.md (pinned cee14e9), A9-R3-interaction-safety.md, src-tauri/src/bridge.rs, src-tauri/src/domain.rs, src-tauri/src/sync.rs
GIT_REFERENCE=DetachHead/rebased v1.1.15 @cee14e9 (git4idea Apache-2.0) via A7 reference map
FILES=logs/research/M5-W18/A9-R3B-git-safety-matrix.md, logs/research/M5-W18/A9-R3B-checkpoint.md
DELIVERABLES=14-unit Git safety matrix; 6 frozen boundaries (dirty-tree / protected-branch / confirmation-tier / undo-abort / audit-redaction / credential); context-menu policy verification; Git threat model G1-G12
PRODUCT_CODE_CHANGED=no
ACL_CAPABILITY_CHANGED=no
USER_VAULT_CHANGED=no
PUSHED=no (A0 integrates)
```

## A9 R3B summary

R3B correction for Lane A9: **extend the R3 interaction-safety review (A9-R3-interaction-safety.md) across the canonical 14 Git workflow units** (per A7's reference map, pinned `DetachHead/rebased` v1.1.15 `@cee14e9`, git4idea Apache-2.0) and **freeze six boundaries** demanded by R3B §A9.

**Measured product safety substrate (frozen assets, cited file:line):**
- Two-phase confirmable-job gate `request_git_write`/`confirm_git_write` at the IPC layer, both calling `check_invocation_source` (`bridge.rs:2377`, `2486`) → source-agnostic.
- Op whitelist `GitWriteOp` = {Stage, Unstage, Discard, Commit, CreateBranch, CheckoutBranch, Push} (`domain.rs:234-266`); all history-rewriting ops → `Err("操作禁止")` (`bridge.rs:2378-2379`) → fail-closed.
- Dangerous double-confirm for `Discard|Push` (`domain.rs:274`; `bridge.rs:2189`).
- Push is non-force + single-remote by construction: `GIT_PUSH_REMOTE="origin"` (`sync.rs:919`), `build_push_refspec` forbids `+`(force) and leading `:`(remote delete) (`sync.rs:921-937`).
- Audit redaction by construction: `git_write_audit_detail` excludes credentials/paths/diff (`bridge.rs:2214`).
- Credentials read from OS keyring only at push (`sync.rs:15-16`).

**Six frozen boundaries (§2):** dirty-tree (read status precheck, bounded path list, read-only units exempt); protected-branch (mainline protected, force-push structurally impossible, T3/T4 for destructive-on-protected); confirmation-tier (T0–T4 map for all 14 units + the "especially" ops amend/reset/revert/rebase-continue-abort/cherry-continue-abort/conflict/patch/command-log); undo/abort (prominent T1 Abort for in-progress rebase/cherry/merge, reflog recovery for irreversible); audit-redaction (fail-closed, new ops must reuse `git_write_audit_detail`); credential (keyring-only, origin-only, URL-strip).

**Context-menu policy verification (§4):** PASS — the gate is at the Rust IPC layer irrespective of trigger source; the op set is server-frozen; tiers/dangerous flags are server-enforced; registry parity makes source irrelevant. Dynamic GUI click-through marked **NOT_RUN** (reserved for A0/user; requires W19 Git slices first).

**Threat model:** Git-specific G1–G12 added (extends R3 T1–T12).

**Alignment:** ADAPT=5 / REIMPLEMENT=9 / COPY=0 from A7 preserved; adopts A0 ruling #9 (keyring-only credentials); depends on A4 (dirty/protected DTO), A6 (command-registry parity + machine-checkable audit), A7 (whitelist extension must preserve S1–S8), A10 (verify §4 against real wiring); hands off to A1 (prototype Git-mode tiers).

**Constraints honored:** no product source / ACL / capability / manifest / dependency / user-vault change; design evidence only under the R3B research boundary; STATUS=PASS with dynamic GUI NOT_RUN per board (`W19=CLOSED` for autonomous opening).

## Verification

- Evidence = static/structural reads of `bridge.rs`/`domain.rs`/`sync.rs` at HEAD `200f0f1` → **PASS**.
- `git diff --check` on new files: run at commit (expected clean).
- Files confined to `logs/research/M5-W18/` (A9's assigned research path); no overlap with A1/A4/A6/A7 files (referenced, not edited).

## NEXT

W19 stays `CLOSED`. If A0 opens the Git slices (S-… Git mode), §2.3 tier map + §3 matrix feed: (1) A6's command-registry safety classes + disabled reasons, (2) A4's dirty/protected DTO, (3) A7's whitelist extension (preserving S1–S8), (4) Git history-rewrite / conflict / patch dialogs, (5) command-log credential redaction. A10 verifies the §4 claim against real frontend wiring before A1 finalizes.
