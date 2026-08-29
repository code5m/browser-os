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
#   6. git diff --check（工作树无空白错误）
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
  scripts/pre-merge.sh              正式门禁（bash -n + help + self-test + invalid + schema + diff --check）
  scripts/pre-merge.sh --help       显示本帮助
  scripts/pre-merge.sh --self-test  自检门禁自身（工具存在 + 各检查命令可调用）

检查项:
  bash -n                       scripts/baseline-check.sh scripts/verify-resources.sh
  --help 退出 0                 两脚本各一次
  --self-test ALL_PASS          两脚本各一次（fixture，不生成正式证据）
  非法参数退出 2                 两脚本各一次（--definitely-invalid）
  schema 自检                   scripts/validate-summary.py --self-test
  git diff --check              工作树空白错误检查

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

  pm_log "--help 退出 0…"
  bash "$SCRIPT_DIR/baseline-check.sh" --help >/dev/null 2>&1 || pm_fail "baseline-check.sh --help"
  bash "$SCRIPT_DIR/verify-resources.sh" --help >/dev/null 2>&1 || pm_fail "verify-resources.sh --help"

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

  pm_log "schema 校验器自检…"
  python3 "$SCRIPT_DIR/validate-summary.py" --self-test >/dev/null 2>&1 || pm_fail "validate-summary.py --self-test"

  pm_log "git diff --check…"
  if [ "$ROOT" != "$SCRIPT_DIR/.." ] && [ -d "$ROOT/.git" ]; then
    (cd "$ROOT" && git diff --check) || pm_fail "git diff --check"
  else
    git -C "$ROOT" diff --check || pm_fail "git diff --check"
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
  for t in bash python3 git sha256sum; do
    if ! command -v "$t" >/dev/null 2>&1; then
      echo "FAIL: required tool missing: $t"; rc=1
    fi
  done
  [ -f "$SCRIPT_DIR/baseline-check.sh" ] || { echo "FAIL: baseline-check.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/verify-resources.sh" ] || { echo "FAIL: verify-resources.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/validate-summary.py" ] || { echo "FAIL: validate-summary.py missing"; rc=1; }
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
