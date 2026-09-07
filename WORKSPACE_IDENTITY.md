# Workspace Identity: BACKV3_MAIN

This directory is the canonical V3 main working copy. Agents should use this file as a self-rescue marker when they are unsure which checkout they are editing.

- Path: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
- Expected branch: `master`
- Role: final integration, verification, commits, and pushes
- Do not confuse this directory with `/home/ainfinit/.codex/worktrees/*/mvp-browser-os-v3`

### Active W18 lane exception

The only current exception is the controller-provisioned M5-W18 lane set:

- Path pattern: `/home/ainfinit/.codex/worktrees/m5-w18-aN/mvp-browser-os-v3`
- Branch pattern: `codex/m5-w18-aN`
- Valid lane ids: `N=1..11`, with path, branch, and assigned `Lane AN` required to match exactly.
- Role: W18-R isolated research/design only. A lane may commit its own research reports but must not edit product code or push.
- Source of truth: the latest `M5-W18-R Research and Replication Blueprint Dispatch` in `PARALLEL_COMMAND_BOARD.md`.

This exception does not make arbitrary `.codex/worktrees/*` valid. Any other worktree remains a hard stop unless the user or A0 explicitly assigns it.

## Start Here

Use this directory when the task says any of the following:

- canonical main directory
- BackV3 / V3 main
- final integration
- commit and push to remote
- sync or unify worktrees into the real repo

Do not use this directory for isolated experiments unless the task explicitly says to work in the main copy.

## Required Startup Check

```bash
cat .workspace-identity
pwd
git status --short --branch
git log --oneline -12
```

## Hard Stop Rules

- Stop if `pwd` is neither the canonical path nor the exact active W18 lane path assigned in `PARALLEL_COMMAND_BOARD.md`.
- Stop if the canonical branch is not `master`, or a W18 lane branch/path/id do not match `codex/m5-w18-aN` / `m5-w18-aN` / `Lane AN` exactly.
- Stop if the worktree is dirty and the dirty files do not clearly belong to your assigned task.
- Stop before pushing if there are unreviewed changes from another agent in the same files.

## If You Are Lost

1. Read this file and `.workspace-identity`.
2. Compare `pwd` with the task's `WORKDIR`.
3. If the task names `/home/ainfinit/.codex/worktrees/*`, leave this directory and use that exact worktree.
4. If the task asks for final integration or remote push, return to this directory.
5. Report the mismatch instead of continuing from the wrong checkout.

## Desktop Runtime Source Gate

This project has two intentionally different main-window origins. Treat this as a release-safety contract, not as a local setup detail:

- Debug main window: `http://localhost:1421/`, created programmatically in `src-tauri/src/main.rs`.
- Release main window: bundled `tauri://localhost` assets.
- `src-tauri/tauri.conf.json` must not regain a global `build.devUrl`. In this project, raw release builds previously inherited Tauri's dev configuration and produced a client that depended on the Vite server.
- Debug IPC access is granted only by `src-tauri/dev-capabilities/main.json`, registered under `#[cfg(debug_assertions)]`. The file deliberately lives outside the auto-scanned `src-tauri/capabilities/` directory and must not enter release capabilities.
- Never fix a debug `not allowed ... URL: http://localhost:1421/` error by adding broad `remote.urls` to the default capability. First verify the caller origin and the debug-only capability registration.
- A browser tab showing `localhost:1421` is only the frontend preview. Native-client acceptance must launch the Tauri window and verify at least one IPC command.

Every new Tauri command is one atomic delivery: Rust implementation, `check_invocation_source`, `generate_handler!` registration, ACL permission, typed `src/bridge.ts`/`src/types.ts` exposure, and policy/tests. Before claiming it works, run:

```bash
python3 scripts/check-command-set-consistency.py
bash scripts/check-dev-startup.sh
bash scripts/pre-merge.sh
```
