# Workspace Identity: BACKV3_MAIN

This directory is the canonical V3 main working copy. Agents should use this file as a self-rescue marker when they are unsure which checkout they are editing.

- Path: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
- Expected branch: `master`
- Role: final integration, verification, commits, and pushes
- Do not confuse this directory with `/home/ainfinit/.codex/worktrees/*/mvp-browser-os-v3`

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

- Stop if `pwd` is not `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`.
- Stop if the branch is not `master`, unless the user explicitly named another branch.
- Stop if the worktree is dirty and the dirty files do not clearly belong to your assigned task.
- Stop before pushing if there are unreviewed changes from another agent in the same files.

## If You Are Lost

1. Read this file and `.workspace-identity`.
2. Compare `pwd` with the task's `WORKDIR`.
3. If the task names `/home/ainfinit/.codex/worktrees/*`, leave this directory and use that exact worktree.
4. If the task asks for final integration or remote push, return to this directory.
5. Report the mismatch instead of continuing from the wrong checkout.
