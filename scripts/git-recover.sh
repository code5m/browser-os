#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# git-recover.sh — Git integrity recovery procedure (Phase 1.7)
#
# Standardised recovery from corrupt git objects. Principle:
#   diagnose -> record -> recovery point -> continue.  NO blind reset.
#
# READ-ONLY unless --prune-orphans is explicitly given.
#
# Subcommands:
#   --snapshot          capture project state via scripts/snapshot.sh (read-only)
#   --diagnose          list corrupt loose objects + reachability (read-only)
#   --prune-orphans     SAFELY delete corrupt loose objects that are NOT reachable
#                       from any ref/reflog. Guarded; aborts if a corrupt object is
#                       reachable (manual intervention required).
#   --help
#
# Safety guards (--prune-orphans aborts if any violated):
#   1. only loose objects (.git/objects/<2>/<38>) are touched, never packs
#   2. only objects where `git cat-file -t` fails (corrupt / empty)
#   3. object must be absent from `git rev-list --all --reflog --objects` (unreachable)
#   4. snapshot taken before deletion; fsck re-verified after deletion
# ---------------------------------------------------------------------------
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/..")"
cd "$ROOT" || exit 1
GITDIR="$(git rev-parse --git-dir 2>/dev/null || echo "$ROOT/.git")"

usage() {
  cat <<'EOF'
git-recover.sh — Git integrity recovery procedure (Phase 1.7)

Usage:
  scripts/git-recover.sh --snapshot           read-only snapshot via snapshot.sh
  scripts/git-recover.sh --diagnose           list corrupt objects + reachability (read-only)
  scripts/git-recover.sh --prune-orphans      SAFELY delete unreachable corrupt loose objects
  scripts/git-recover.sh --help

Read-only unless --prune-orphans is given. Never does blind reset.
EOF
}

# Find corrupt loose objects. Prints "<oid> <path>" lines, or nothing.
find_corrupt_loose() {
  [ -d "$GITDIR/objects" ] || return 0
  local f prefix suffix oid
  while IFS= read -r f; do
    prefix="$(basename "$(dirname "$f")")"
    suffix="$(basename "$f")"
    # only loose objects: 2-char dir + 38-char file
    case "$prefix$suffix" in
      *[^0-9a-f]*) continue ;;
    esac
    [ "${#prefix}" -eq 2 ] && [ "${#suffix}" -eq 38 ] || continue
    oid="$prefix$suffix"
    if ! git cat-file -t "$oid" >/dev/null 2>&1; then
      echo "$oid $f"
    fi
  done < <(find "$GITDIR/objects" -type f -path '*/??/*' 2>/dev/null)
}

is_reachable() {
  local oid="$1"
  git rev-list --all --reflog --objects 2>/dev/null | grep -qE "^$oid"
}

cmd_snapshot() {
  bash "$SCRIPT_DIR/snapshot.sh" "before-recover"
}

cmd_diagnose() {
  local found=0 reachable=0
  echo "=== git fsck --full (raw) ==="
  git fsck --full 2>&1 | sed 's/^/  /' || true
  echo
  echo "=== corrupt loose objects + reachability ==="
  local line oid path
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    oid="${line%% *}"; path="${line#* }"
    found=$((found+1))
    if is_reachable "$oid"; then
      echo "  [REACHABLE] $oid  <- DO NOT DELETE (manual/clone recovery needed)"
      reachable=$((reachable+1))
    else
      echo "  [ORPHAN]     $oid  <- safe to prune (git-recover.sh --prune-orphans)"
    fi
  done < <(find_corrupt_loose)
  echo
  if [ "$found" -eq 0 ]; then
    echo "DIAGNOSE: no corrupt loose objects found"
  elif [ "$reachable" -gt 0 ]; then
    echo "DIAGNOSE: $found corrupt object(s); $reachable REACHABLE -> ABORT prune, manual recovery"
  else
    echo "DIAGNOSE: $found corrupt orphan object(s); safe to prune"
  fi
}

cmd_prune() {
  cmd_snapshot
  local line oid path deleted=0 aborted=0
  local -a corrupt_list
  while IFS= read -r line; do [ -n "$line" ] && corrupt_list+=("$line"); done < <(find_corrupt_loose)
  if [ "${#corrupt_list[@]}" -eq 0 ]; then
    echo "PRUNE: no corrupt loose objects; nothing to do"
    exit 0
  fi
  # Guard 3: abort if any corrupt object is reachable.
  for line in "${corrupt_list[@]}"; do
    oid="${line%% *}"
    if is_reachable "$oid"; then
      echo "PRUNE: ABORT — corrupt object $oid is reachable; requires manual/clone recovery (do NOT delete)."
      aborted=1
    fi
  done
  [ "$aborted" -eq 1 ] && exit 1

  for line in "${corrupt_list[@]}"; do
    oid="${line%% *}"; path="${line#* }"
    echo "PRUNE: removing orphan corrupt loose object $oid"
    rm -f "$path" && deleted=$((deleted+1)) || echo "PRUNE: FAILED to remove $path"
  done

  echo "=== re-verify git fsck --full ==="
  if git fsck --full 2>&1 | grep -qE '^error:'; then
    echo "PRUNE: WARNING — fsck still reports errors after prune (may be pack corruption; needs git unpack-objects / clone)."
  else
    echo "PRUNE: OK — git fsck --full reports no errors"
  fi
  echo "PRUNE: removed $deleted corrupt orphan object(s)"
  echo "PRUNE: recommended follow-up: git reflog expire --expire=now --all && git gc --prune=now"
}

case "${1:-}" in
  --snapshot) cmd_snapshot ;;
  --diagnose) cmd_diagnose ;;
  --prune-orphans) cmd_prune ;;
  --help|-h) usage; exit 0 ;;
  *) echo "error: missing/invalid subcommand" >&2; usage >&2; exit 2 ;;
esac
