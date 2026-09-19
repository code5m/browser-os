#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# check-git-repo-integrity.sh — Git repository object integrity gate (Phase 1.7)
#
# Purpose: detect corrupt / missing git objects in the repository object
#          database so that a broken repo is caught at pre-merge time instead of
#          silently propagating (see incident: HANDOFF commit 2ff24aa object loss).
#
# READ-ONLY by default. Never mutates git state.
#
# Usage:
#   scripts/check-git-repo-integrity.sh [--repo <path>]   formal gate (exit 0/1)
#   scripts/check-git-repo-integrity.sh --self-test        fixture self-test in /tmp
#   scripts/check-git-repo-integrity.sh --help
#
# Exit codes:
#   0  PASS   no corrupt/missing objects reachable from refs
#   1  FAIL   one or more corrupt/missing objects (repo is broken)
#   2  USAGE  invalid arguments
#
# Classification:
#   FATAL (-> FAIL): any line from `git fsck --full` starting with "error:"
#                    (corrupt object / missing object / broken ref / mmap / empty file)
#   WARN  (non-fatal): "dangling commit" / "悬空 commit" — may indicate lost work,
#                    reported but does NOT block (human judgement).
#   INFO  (ignored): "dangling blob" / "dangling tree" / "悬空 blob|tree" — normal gc.
# ---------------------------------------------------------------------------
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/..")"
REPO="$ROOT"

usage() {
  cat <<'EOF'
check-git-repo-integrity.sh — Git repository object integrity gate (Phase 1.7)

Usage:
  scripts/check-git-repo-integrity.sh [--repo <path>]   formal gate (exit 0/1)
  scripts/check-git-repo-integrity.sh --self-test        fixture self-test in /tmp
  scripts/check-git-repo-integrity.sh --help

Exit codes: 0 PASS / 1 FAIL / 2 USAGE
EOF
}

# Inspect a single repository; echoes a one-line verdict and returns 0/1.
# Does NOT exit (so callers control flow).
inspect_repo() {
  local repo="$1"
  if ! git -C "$repo" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "GIT_INTEGRITY: FAIL (not a git repo: $repo)"
    return 1
  fi
  local out
  out="$(git -C "$repo" fsck --full 2>&1)"
  local fatal
  fatal="$(printf '%s\n' "$out" | grep -E '^error:' || true)"
  local dangling_commit
  dangling_commit="$(printf '%s\n' "$out" | grep -E 'dangling commit|悬空 commit' | wc -l | tr -d ' ')"
  if [ -n "$fatal" ]; then
    echo "GIT_INTEGRITY: FAIL (corrupt/missing objects detected in $repo)"
    printf '%s\n' "$fatal" | sed 's/^/  /'
    return 1
  fi
  if [ "${dangling_commit:-0}" != "0" ]; then
    echo "GIT_INTEGRITY: PASS (with ${dangling_commit} dangling commit(s) — review for lost work, non-blocking)"
    return 0
  fi
  echo "GIT_INTEGRITY: PASS (no corrupt/missing objects)"
  return 0
}

# ---- argument parsing ----
MODE="check"
while [ $# -gt 0 ]; do
  case "$1" in
    --repo) MODE="check"; REPO="${2:-}"; [ -z "$REPO" ] && { echo "error: --repo requires a path" >&2; exit 2; }; shift 2 ;;
    --self-test) MODE="selftest"; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "error: unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

if [ "$MODE" = "selftest" ]; then
  errors=0
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT

  # --- clean repo must PASS ---
  git init -q "$TMP/clean" >/dev/null 2>&1
  ( cd "$TMP/clean" && echo "hello" > a.txt && git add a.txt && git -c user.email=t@t -c user.name=t commit -qm init >/dev/null 2>&1 )
  if inspect_repo "$TMP/clean" >/dev/null 2>&1; then
    echo "self-test[clean-repo]: PASS (detected clean as PASS)"
  else
    echo "self-test[clean-repo]: FAIL (clean repo wrongly reported FAIL)"; errors=$((errors+1))
  fi

  # --- repo with a corrupted loose object must FAIL ---
  git init -q "$TMP/dirty" >/dev/null 2>&1
  ( cd "$TMP/dirty" && echo "data" > b.txt && git add b.txt && git -c user.email=t@t -c user.name=t commit -qm init >/dev/null 2>&1 )
  # create a VALID object, then empty its loose file to simulate corruption.
  # git stores loose objects read-only; make it writable before truncating.
  good_oid="$(cd "$TMP/dirty" && git hash-object -w --stdin <<< "corrupt-me" 2>/dev/null)"
  if [ -n "$good_oid" ]; then
    chmod u+w "$TMP/dirty/.git/objects/${good_oid:0:2}/${good_oid:2}" 2>/dev/null
    : > "$TMP/dirty/.git/objects/${good_oid:0:2}/${good_oid:2}"   # empty the object -> "对象文件 ... 为空"
  fi
  if ! inspect_repo "$TMP/dirty" >/dev/null 2>&1; then
    echo "self-test[corrupt-object]: PASS (detected corruption as FAIL)"
  else
    echo "self-test[corrupt-object]: FAIL (corruption not detected)"; errors=$((errors+1))
  fi

  if [ "$errors" -eq 0 ]; then
    echo "SELF_TEST: PASS (git-repo-integrity internal consistency)"
    exit 0
  else
    echo "SELF_TEST: FAIL"
    exit 1
  fi
fi

# ---- formal gate ----
inspect_repo "$REPO"
exit $?
