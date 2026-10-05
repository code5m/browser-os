#!/usr/bin/env bash
# Exercise the versioned hooks and helpers in disposable repositories only.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

configure_identity() {
  git -C "$1" config user.name 'Git Safeguard Test'
  git -C "$1" config user.email 'git-safeguard-test@example.invalid'
}

new_clone() {
  local destination="$1"
  git clone "$test_root/origin.git" "$destination" >/dev/null
  configure_identity "$destination"
  (cd "$destination" && ./scripts/install-git-hooks.sh >/dev/null && ./scripts/install-git-hooks.sh >/dev/null)
}

expect_ok() {
  "$@" >/dev/null
}

expect_blocked() {
  if "$@" >/dev/null 2>&1; then
    printf 'expected command to be blocked: %s\n' "$*" >&2
    exit 1
  fi
}

git init --bare "$test_root/origin.git" >/dev/null
git init "$test_root/seed" >/dev/null
configure_identity "$test_root/seed"
mkdir -p "$test_root/seed/.githooks" "$test_root/seed/scripts"
cp "$repo_root/.githooks/pre-push" "$test_root/seed/.githooks/pre-push"
cp "$repo_root/scripts/install-git-hooks.sh" "$repo_root/scripts/sync-repo.sh" "$test_root/seed/scripts/"
chmod +x "$test_root/seed/.githooks/pre-push" "$test_root/seed/scripts/install-git-hooks.sh" "$test_root/seed/scripts/sync-repo.sh"
printf 'base\n' > "$test_root/seed/README.md"
git -C "$test_root/seed" add README.md .githooks scripts
git -C "$test_root/seed" commit -m base >/dev/null
git -C "$test_root/seed" branch -M master
git -C "$test_root/seed" remote add origin "$test_root/origin.git"
git -C "$test_root/seed" push -u origin master >/dev/null

new_clone "$test_root/a"
new_clone "$test_root/b"
expect_ok bash -c "cd '$test_root/a' && ./scripts/sync-repo.sh"

# Remote-ahead: hook blocks the stale clone; sync fast-forwards only when clean.
printf 'remote\n' > "$test_root/b/remote.txt"
git -C "$test_root/b" add remote.txt
git -C "$test_root/b" commit -m remote >/dev/null
git -C "$test_root/b" push origin master >/dev/null
expect_blocked bash -c "cd '$test_root/a' && git push --dry-run origin master"
expect_ok bash -c "cd '$test_root/a' && ./scripts/sync-repo.sh"

# Local-ahead: the fast-forward-safe dry-run push is accepted and preserves the commit.
printf 'local\n' > "$test_root/a/local.txt"
git -C "$test_root/a" add local.txt
git -C "$test_root/a" commit -m local >/dev/null
expect_ok bash -c "cd '$test_root/a' && git push --dry-run origin master"

# Divergence: an independent remote commit makes the local client refuse push and sync.
new_clone "$test_root/c"
new_clone "$test_root/d"
printf 'd\n' > "$test_root/d/d.txt"
git -C "$test_root/d" add d.txt
git -C "$test_root/d" commit -m d >/dev/null
git -C "$test_root/d" push origin master >/dev/null
printf 'c\n' > "$test_root/c/c.txt"
git -C "$test_root/c" add c.txt
git -C "$test_root/c" commit -m c >/dev/null
expect_blocked bash -c "cd '$test_root/c' && git push --dry-run origin master"
expect_blocked bash -c "cd '$test_root/c' && ./scripts/sync-repo.sh"

# Dirty trees refresh refs but never change files.
new_clone "$test_root/dirty"
printf 'uncommitted\n' > "$test_root/dirty/dirty.txt"
expect_blocked bash -c "cd '$test_root/dirty' && ./scripts/sync-repo.sh"
test -f "$test_root/dirty/dirty.txt"

# Tags: create, matching no-op, mismatched tag, and deletion are all gated.
new_clone "$test_root/tags"
git -C "$test_root/tags" tag safeguard-test
expect_ok bash -c "cd '$test_root/tags' && git push --dry-run origin refs/tags/safeguard-test"
git -C "$test_root/tags" push origin refs/tags/safeguard-test >/dev/null
expect_ok bash -c "cd '$test_root/tags' && git push --dry-run origin refs/tags/safeguard-test"
git -C "$test_root/tags" tag -f safeguard-test HEAD~1 >/dev/null
expect_blocked bash -c "cd '$test_root/tags' && git push --dry-run origin refs/tags/safeguard-test"
expect_blocked bash -c "cd '$test_root/tags' && git push --dry-run origin :refs/tags/safeguard-test"

printf 'git safeguards: PASS\n'
