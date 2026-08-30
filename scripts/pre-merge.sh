#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# pre-merge.sh — M0-1.c 本地 pre-merge 门禁（M0-1 脚本集合的合并前检查入口）
#
# 作用：在合并/提交前固定验证 M0-1 两个门禁脚本自身的可运行性与一致性：
#   1. bash -n 语法检查（baseline-check.sh、verify-resources.sh）
#   2. --help 退出 0
#   3. --self-test 全部用例 PASS（fixture 模式，不生成正式证据、不触碰用户数据）
#   4. 非法参数非零退出（退出码 2）
#   5. summary.json schema 校验器自检（validate-summary.py --self-test）
#   6. M0 总控/聚合器自检
#   7. Rust fmt、前端 production build、Rust cargo check
#   8. 正式证据 schema/SHA256 完整性
#   9. 工作树、暂存区、当前分支相对基线的 git diff --check
#
# 用法:
#   scripts/pre-merge.sh            正式门禁（所有检查必须通过）
#   scripts/pre-merge.sh --help     显示本帮助
#   scripts/pre-merge.sh --self-test 自检门禁自身（工具存在 + 各检查命令可调用）
#
# 退出码: 0 = 全部通过；1 = 任一检查失败；2 = 非法参数
#
# 接入方式（二选一，均不修改 git 配置）:
#   A. 手动运行：提交/合并前执行 scripts/pre-merge.sh。
#   B. hook：复制 .githooks/pre-merge-commit 到 .git/hooks/ 并 chmod +x，
#      或在克隆后一次性设置 core.hooksPath（需自行执行 git config）。
# ---------------------------------------------------------------------------
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR/..")"

usage() {
  cat <<'EOF'
pre-merge.sh — M0-1.c 本地 pre-merge 门禁（M0-1 脚本合并前检查入口）

用法:
  scripts/pre-merge.sh              正式门禁（脚本契约 + fmt/build/check + 全范围 diff --check）
  scripts/pre-merge.sh --help       显示本帮助
  scripts/pre-merge.sh --self-test  自检门禁自身（工具存在 + 各检查命令可调用）

检查项:
  bash -n                       scripts/baseline-check.sh scripts/verify-resources.sh
  --help 退出 0                 两脚本各一次
  --self-test ALL_PASS          两脚本各一次（fixture，不生成正式证据）
  非法参数退出 2                 两脚本各一次（--definitely-invalid）
  schema 自检                   scripts/validate-summary.py --self-test
  M0 总控/聚合自检              collect-m0-baseline.sh / aggregate-m0-baseline.py
  Rust fmt                      主工程 + browser-tabs workspace
  production build / check      npm run build + cargo check --locked
  evidence integrity            正式 run schema + 全部 SHA256SUMS
  git diff --check              工作树 + 暂存区 + 当前分支相对基线（机器证据除外）

退出码: 0 = 全部通过；1 = 任一失败；2 = 非法参数
EOF
}

pm_log() {
  echo "[pre-merge] $*"
}

pm_fail() {
  echo "[pre-merge] FAIL: $*" >&2
  PM_RC=1
}

run_pre_merge() {
  PM_RC=0

  pm_log "syntax check (bash -n)…"
  bash -n "$SCRIPT_DIR/baseline-check.sh" || pm_fail "bash -n baseline-check.sh"
  bash -n "$SCRIPT_DIR/verify-resources.sh" || pm_fail "bash -n verify-resources.sh"
  bash -n "$SCRIPT_DIR/collect-m0-baseline.sh" || pm_fail "bash -n collect-m0-baseline.sh"

  pm_log "--help 退出 0…"
  bash "$SCRIPT_DIR/baseline-check.sh" --help >/dev/null 2>&1 || pm_fail "baseline-check.sh --help"
  bash "$SCRIPT_DIR/verify-resources.sh" --help >/dev/null 2>&1 || pm_fail "verify-resources.sh --help"
  bash "$SCRIPT_DIR/collect-m0-baseline.sh" --help >/dev/null 2>&1 || pm_fail "collect-m0-baseline.sh --help"

  pm_log "--self-test（两脚本，fixture 模式）…"
  if ! bash "$SCRIPT_DIR/baseline-check.sh" --self-test >/dev/null 2>&1; then
    pm_fail "baseline-check.sh --self-test"
  else
    pm_log "baseline-check.sh --self-test ALL_PASS"
  fi
  if ! bash "$SCRIPT_DIR/verify-resources.sh" --self-test >/dev/null 2>&1; then
    pm_fail "verify-resources.sh --self-test"
  else
    pm_log "verify-resources.sh --self-test ALL_PASS"
  fi

  pm_log "非法参数非零退出…"
  if bash "$SCRIPT_DIR/baseline-check.sh" --definitely-invalid >/dev/null 2>&1; then
    pm_fail "baseline-check.sh invalid arg should exit nonzero"
  fi
  if bash "$SCRIPT_DIR/verify-resources.sh" --definitely-invalid >/dev/null 2>&1; then
    pm_fail "verify-resources.sh invalid arg should exit nonzero"
  fi
  if bash "$SCRIPT_DIR/collect-m0-baseline.sh" --definitely-invalid >/dev/null 2>&1; then
    pm_fail "collect-m0-baseline.sh invalid arg should exit nonzero"
  fi

  pm_log "schema 校验器自检…"
  python3 "$SCRIPT_DIR/validate-summary.py" --self-test >/dev/null 2>&1 || pm_fail "validate-summary.py --self-test"

  pm_log "M0 总控/聚合器自检…"
  bash "$SCRIPT_DIR/collect-m0-baseline.sh" --self-test >/dev/null 2>&1 || pm_fail "collect-m0-baseline.sh --self-test"
  python3 "$SCRIPT_DIR/aggregate-m0-baseline.py" --self-test >/dev/null 2>&1 || pm_fail "aggregate-m0-baseline.py --self-test"

  pm_log "Rust fmt（主工程 + browser-tabs workspace）…"
  cargo fmt --manifest-path "$ROOT/src-tauri/Cargo.toml" --all --check || pm_fail "cargo fmt main"
  cargo fmt --manifest-path "$ROOT/tauri-browser-tabs/Cargo.toml" --all --check || pm_fail "cargo fmt plugin"

  pm_log "frontend production build…"
  (cd "$ROOT" && npm run build) || pm_fail "npm run build"

  pm_log "Rust cargo check --locked…"
  cargo check --manifest-path "$ROOT/src-tauri/Cargo.toml" --locked || pm_fail "cargo check --locked"

  pm_log "正式证据 schema / SHA256SUMS…"
  local evidence_file evidence_dir summary_file contract_version
  while IFS= read -r evidence_file; do
    evidence_dir="$(dirname "$evidence_file")"
    if ! (cd "$evidence_dir" && sha256sum -c SHA256SUMS >/dev/null 2>&1); then
      pm_fail "evidence checksum: ${evidence_dir#"$ROOT/"}"
    fi
  done < <(find "$ROOT/logs/m0-baseline" -type f -name SHA256SUMS -print | sort)
  while IFS= read -r summary_file; do
    if ! contract_version="$(python3 -c 'import json, sys; print(json.load(open(sys.argv[1], encoding="utf-8")).get("contract_version", ""))' "$summary_file")"; then
      pm_fail "evidence summary JSON: ${summary_file#"$ROOT/"}"
      continue
    fi
    case "$contract_version" in
      V1.1)
        if ! python3 "$SCRIPT_DIR/validate-summary.py" "$SCRIPT_DIR/schema/m0-summary.schema.json" "$summary_file" >/dev/null 2>&1; then
          pm_fail "evidence schema: ${summary_file#"$ROOT/"}"
        fi
        ;;
      V1.0)
        pm_log "skip legacy V1.0 schema: ${summary_file#"$ROOT/"}"
        ;;
      *)
        pm_fail "unsupported evidence contract '$contract_version': ${summary_file#"$ROOT/"}"
        ;;
    esac
  done < <(find "$ROOT/logs/m0-baseline" -mindepth 2 -maxdepth 2 -type f -name summary.json -print | sort)

  pm_log "git diff --check（工作树 + 暂存区）…"
  git -C "$ROOT" diff --check || pm_fail "git diff --check (worktree)"
  git -C "$ROOT" diff --cached --check || pm_fail "git diff --check (staged)"

  local base_ref="${M0_BASE_REF:-}" merge_base=""
  if [ -z "$base_ref" ]; then
    for candidate in origin/master master origin/main main; do
      if git -C "$ROOT" rev-parse --verify --quiet "$candidate^{commit}" >/dev/null; then
        base_ref="$candidate"
        break
      fi
    done
  fi
  if [ -n "$base_ref" ]; then
    merge_base="$(git -C "$ROOT" merge-base HEAD "$base_ref" 2>/dev/null || true)"
  fi
  if [ -n "$merge_base" ]; then
    pm_log "git diff --check（$base_ref merge-base..HEAD）…"
    # raw evidence preserves third-party command output byte-for-byte and is governed by SHA256SUMS.
    git -C "$ROOT" diff --check "$merge_base..HEAD" -- . ':(exclude)logs/m0-baseline/**' || pm_fail "git diff --check (branch range)"
  else
    pm_fail "cannot resolve base ref for branch-range diff check (set M0_BASE_REF)"
  fi

  echo ""
  if [ "$PM_RC" -eq 0 ]; then
    pm_log "PRE_MERGE_RESULT=ALL_PASS"
  else
    pm_log "PRE_MERGE_RESULT=FAIL"
  fi
  return "$PM_RC"
}

run_self_test() {
  local rc=0
  pm_log "[self-test] fixture mode: checking pre-merge prerequisites"
  for t in bash cargo npm python3 git sha256sum; do
    if ! command -v "$t" >/dev/null 2>&1; then
      echo "FAIL: required tool missing: $t"; rc=1
    fi
  done
  [ -f "$SCRIPT_DIR/baseline-check.sh" ] || { echo "FAIL: baseline-check.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/verify-resources.sh" ] || { echo "FAIL: verify-resources.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/validate-summary.py" ] || { echo "FAIL: validate-summary.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/aggregate-m0-baseline.py" ] || { echo "FAIL: aggregate-m0-baseline.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/collect-m0-baseline.sh" ] || { echo "FAIL: collect-m0-baseline.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/schema/m0-summary.schema.json" ] || { echo "FAIL: schema missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/validate-summary.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: validate-summary.py --self-test"; rc=1
  fi
  if [ "$rc" -eq 0 ]; then
    echo ""
    echo "SELF_TEST_RESULT=ALL_PASS"
  else
    echo ""
    echo "SELF_TEST_RESULT=FAIL"
  fi
  return "$rc"
}

case "${1:-}" in
  --help|-h) usage; exit 0 ;;
  --self-test) run_self_test; exit $? ;;
  --) ;;
  -*)
    echo "error: unknown option: $1" >&2
    usage >&2
    exit 2
    ;;
  *)
    if [ $# -gt 0 ]; then
      echo "error: unexpected argument: $1" >&2
      usage >&2
      exit 2
    fi
    ;;
esac

run_pre_merge
