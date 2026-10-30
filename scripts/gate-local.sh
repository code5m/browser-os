#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
case "${1:-}" in
  quick)
    node tests/check-local-gate.mjs
    npm run check:engineering-governance
    npm run check:ai-config
    npm run check:coding-workbench
    git diff --check
    git diff --cached --check
    echo 'QUICK_CHECKS=PASS (subset only; not merge or GUI acceptance)'
    ;;
  coding)
    npm run check:ai-config
    npm run check:coding-workbench
    node scripts/check-coding-ui-managed.mjs
    echo 'CODING_CHECKS=PASS (simulated native transport; real provider, MCP and GUI not verified)'
    ;;
  merge)
    node tests/check-local-gate.mjs
    npm run check
    cargo test --manifest-path src-tauri/Cargo.toml --locked
    node scripts/check-coding-ui-managed.mjs
    bash scripts/pre-merge.sh
    echo 'LOCAL_MERGE_CHECKS=PASS (automated checks only)'
    echo 'REAL_PROVIDER_MCP=NOT_VERIFIED PACKAGE=NOT_VERIFIED GUI=GUI_PENDING'
    echo 'Remote CI, clean worktree, approvals and release acceptance require separate verification.'
    ;;
  --help|-h)
    echo 'Usage: bash scripts/gate-local.sh quick|coding|merge'
    echo 'quick: local subset; coding: focused workbench checks; merge: comprehensive local automated checks.'
    echo 'No mode installs the app or establishes real provider, package or GUI acceptance.'
    ;;
  *)
    echo 'Usage: bash scripts/gate-local.sh quick|coding|merge' >&2
    exit 2
    ;;
esac
