#!/usr/bin/env bash
# Install repository-local Git safeguards. Safe to run repeatedly.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

[[ -x .githooks/pre-push ]] || {
  printf 'install-git-hooks: .githooks/pre-push is missing or not executable.\n' >&2
  exit 1
}

git config --local core.hooksPath .githooks
git config --local pull.ff only
git config --local fetch.prune true

printf 'Git safeguards installed for: %s\n' "$repo_root"
printf '  core.hooksPath = %s\n' "$(git config --local --get core.hooksPath)"
printf '  pull.ff        = %s\n' "$(git config --local --get pull.ff)"
printf '  fetch.prune    = %s\n' "$(git config --local --get fetch.prune)"
