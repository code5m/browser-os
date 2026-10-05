#!/usr/bin/env bash
# Refresh the GitHub source of truth without overwriting local work.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

say() {
  printf 'sync-repo: %s\n' "$*"
}

say "repository: $repo_root"
git status --short --branch

if ! git fetch origin --prune --tags; then
  printf 'sync-repo: unable to refresh origin. Check network and GitHub credentials; the working tree was not modified.\n' >&2
  exit 1
fi

branch="$(git symbolic-ref --quiet --short HEAD || true)"
if [[ -z "$branch" ]]; then
  printf 'sync-repo: detached HEAD detected. Remote refs were refreshed; no working-tree update was attempted.\n' >&2
  exit 2
fi

upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true)"
if [[ -n "$upstream" ]]; then
  say "current branch: $branch (upstream: $upstream)"
else
  say "current branch: $branch (no upstream configured)"
fi

remote_ref="refs/heads/$branch"
remote_line="$(git ls-remote --heads origin "$remote_ref" | head -n 1 || true)"
if [[ -z "$remote_line" ]]; then
  say "GitHub has no branch '$branch'. No update was attempted; a later normal push may create it."
  exit 0
fi

if [[ -n "$(git status --porcelain)" ]]; then
  printf 'sync-repo: working tree contains local changes. Remote refs were refreshed, but the working tree was not modified.\n' >&2
  exit 2
fi

remote_tracking="refs/remotes/origin/$branch"
if ! git show-ref --verify --quiet "$remote_tracking"; then
  printf 'sync-repo: origin/%s was not available locally after fetch. No update was attempted.\n' "$branch" >&2
  exit 1
fi

read -r ahead behind < <(git rev-list --left-right --count "HEAD...$remote_tracking")
if [[ "$behind" == 0 && "$ahead" == 0 ]]; then
  say "Already up to date."
  exit 0
fi
if [[ "$behind" -gt 0 && "$ahead" == 0 ]]; then
  say "GitHub is ahead by $behind commit(s); applying fast-forward only."
  git merge --ff-only "$remote_tracking"
  say "Fast-forward update completed."
  exit 0
fi
if [[ "$behind" == 0 && "$ahead" -gt 0 ]]; then
  printf 'sync-repo: local branch is ahead of GitHub by %s commit(s). No automatic pull was performed.\n' "$ahead" >&2
  exit 2
fi

printf 'sync-repo: local and GitHub histories have diverged (local ahead %s, GitHub ahead %s). Automatic reset, rebase, and merge were intentionally not performed.\n' "$ahead" "$behind" >&2
exit 2
