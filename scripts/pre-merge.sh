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
#   7. WBS 模型路由标签自检与 50 项完整性检查
#   8. Rust fmt、前端 production build、Rust cargo check
#   9. 正式证据 schema/SHA256 完整性
#  10. M0-2 生命周期契约夹具（--self-test + --expect-current-gaps + 默认门禁；
#      M0-2.c 后默认模式必须 PASS）
#  11. M0-3.a 安全边界夹具（--self-test + --expect-current-gaps；
#      默认模式按设计 EXIT=1，属现状缺口，不并入本门禁）
#      M0-2.c 后默认模式必须 PASS，防止退出路径回归）
#  12. M0-6.c GUI 回归汇总脚本自检（不启动 GUI）
#  13. M1-6 Git 写安全不变量夹具（白名单/写原语校验/闸门/审计脱敏/ACL）
#  14. M1-8 资源瀑布隐私/容量不变量夹具 + 前端逻辑层测试
#  15. M1-9 会话持久化/关闭协议不变量夹具 + 前端逻辑层测试
#  16. M2-1 图片领域与持久化不变量夹具 + 前端展示逻辑层测试
#  17. M2-2.b 图片预览不变量夹具 + 预览逻辑层测试
#  18. M2-3.b 脚本领域与持久化不变量夹具（前端逻辑测试随 M2-5 UI 落地）
#  19. 工作树、暂存区、当前分支相对基线的 git diff --check
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
  plan routing                  50 个 WBS 的优先级/复杂度/模型标签
  Rust fmt                      主工程 + browser-tabs workspace
  production build / check      npm run build + cargo check --locked
  evidence integrity            正式 run schema + 全部 SHA256SUMS
  lifecycle fixture             check-lifecycle-contract.py --self-test / --expect-current-gaps / 默认门禁
  security fixture              check-security-policy.py --self-test / --expect-current-gaps
                                （默认模式按设计 EXIT=1，见 logs/m0-security-threat-matrix-v1.md）
  build metrics                 measure-build-metrics.py --self-test / --compare（总体积 ≤15%、
                                warning 不增加；指标存 logs/m0-build-metrics/）
  GUI regression harness        m0-6c-gui-regression.py --self-test（不启动 GUI）
  git write policy fixture      check-git-write-policy.py --self-test / 默认门禁
  git UI policy fixture         check-git-ui-policy.py --self-test / 默认门禁
  git UI logic tests            check-git-ui-logic.mjs（Node，headless）
  resource capture fixture      check-resource-capture-policy.py --self-test / 默认门禁
  resource UI logic tests       check-resource-ui-logic.mjs（Node，headless）
  session persistence fixture   check-session-persistence-policy.py --self-test / 默认门禁
  session logic tests           check-session-logic.mjs（Node，headless）
  image policy fixture          check-image-policy.py --self-test / 默认门禁
  image UI logic tests          check-image-ui-logic.mjs（Node，headless）
  image preview fixture         check-image-preview-policy.py --self-test / 默认门禁
  image preview logic tests     check-image-preview-logic.mjs（Node，headless）
  script domain fixture         check-script-domain-policy.py --self-test / 默认门禁
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

  pm_log "WBS 模型路由标签…"
  python3 "$SCRIPT_DIR/check-plan-routing.py" --self-test >/dev/null 2>&1 || pm_fail "check-plan-routing.py --self-test"
  python3 "$SCRIPT_DIR/check-plan-routing.py" || pm_fail "check-plan-routing.py"

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

  pm_log "M0-2 生命周期契约夹具…"
  python3 "$SCRIPT_DIR/check-lifecycle-contract.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-lifecycle-contract.py --self-test"
  if python3 "$SCRIPT_DIR/check-lifecycle-contract.py" --expect-current-gaps >/dev/null 2>&1; then
    pm_log "lifecycle gap 集合与 logs/m0-resource-ownership-v1.md 一致"
  else
    pm_fail "lifecycle gap 集合已变化：关闭缺口或新增缺口后需同步 EXPECTED_GAPS 与所有权表"
  fi
  python3 "$SCRIPT_DIR/check-lifecycle-contract.py" >/dev/null 2>&1 \
    || pm_fail "check-lifecycle-contract.py 默认生命周期门禁"

  pm_log "M0-4.c 构建指标对比（总体积 ≤15% 增长、cargo warning 不增加）…"
  local baseline_file
  baseline_file="$(ls -1 "$ROOT"/logs/m0-build-metrics/build-metrics-*.json 2>/dev/null | sort | head -1 || true)"
  if [ -n "$baseline_file" ]; then
    # 复用上一步 npm run build 的产物，不重复构建。
    if python3 "$SCRIPT_DIR/measure-build-metrics.py" --compare "$baseline_file" --skip-build \
      >/dev/null 2>&1; then
      pm_log "build metrics 未回归（基线 ${baseline_file#"$ROOT/"}）"
    else
      pm_fail "build metrics regression vs ${baseline_file#"$ROOT/"}"
    fi
  else
    pm_log "无构建指标基线，跳过对比"
  fi

  pm_log "M0-3.a 安全边界夹具（现状缺口，默认模式按设计 EXIT=1，不入门禁）…"
  python3 "$SCRIPT_DIR/check-security-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-security-policy.py --self-test"
  if python3 "$SCRIPT_DIR/check-security-policy.py" --expect-current-gaps >/dev/null 2>&1; then
    pm_log "security gap 集合与 logs/m0-security-threat-matrix-v1.md 一致"
  else
    pm_fail "security gap 集合已变化：M0-3.b/c/d 收口或新增缺口后需同步 EXPECTED_GAPS 与威胁矩阵"
  fi

  pm_log "M0-6.c GUI 回归汇总脚本自检（不启动 GUI）…"
  python3 "$SCRIPT_DIR/m0-6c-gui-regression.py" --self-test >/dev/null 2>&1 \
    || pm_fail "m0-6c-gui-regression.py --self-test"

  pm_log "M1-6 Git 写安全不变量夹具…"
  python3 "$SCRIPT_DIR/check-git-write-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-git-write-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-git-write-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-git-write-policy.py（Git 写安全不变量被破坏）"

  pm_log "M1-7 Git UI 安全不变量夹具…"
  python3 "$SCRIPT_DIR/check-git-ui-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-git-ui-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-git-ui-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-git-ui-policy.py（Git UI 安全不变量被破坏）"

  pm_log "M1-7 Git UI 逻辑层自动化测试（headless，mock 仅替换 bridge）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-git-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-git-ui-logic.mjs（Git UI 闸门逻辑回归）"

  pm_log "M1-8 资源瀑布隐私/容量不变量夹具…"
  python3 "$SCRIPT_DIR/check-resource-capture-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-resource-capture-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-resource-capture-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-resource-capture-policy.py（资源瀑布隐私/容量不变量被破坏）"

  pm_log "M1-8 资源瀑布前端逻辑层自动化测试（headless，mock 仅替换 bridge）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-resource-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-resource-ui-logic.mjs（资源瀑布前端逻辑回归）"

  pm_log "M1-9 会话持久化/关闭协议不变量夹具…"
  python3 "$SCRIPT_DIR/check-session-persistence-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-session-persistence-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-session-persistence-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-session-persistence-policy.py（会话持久化/关闭协议不变量被破坏）"

  pm_log "M1-9 会话关闭协议前端逻辑层自动化测试（headless，mock 仅替换 bridge）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-session-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-session-logic.mjs（会话关闭协议前端逻辑回归）"

  pm_log "M2-1 图片领域与持久化不变量夹具…"
  python3 "$SCRIPT_DIR/check-image-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-image-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-image-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-image-policy.py（图片领域/持久化不变量被破坏）"

  pm_log "M2-1 图片展示逻辑前端自动化测试（headless，加载真实 src/utils/image.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-image-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-image-ui-logic.mjs（图片展示/降级逻辑回归）"

  pm_log "M2-2.b 图片预览不变量夹具（通道/边界/命令三处同步/隐私）…"
  python3 "$SCRIPT_DIR/check-image-preview-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-image-preview-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-image-preview-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-image-preview-policy.py（图片预览安全边界被破坏）"

  pm_log "M2-2.b 图片预览逻辑层自动化测试（headless，加载真实 src/utils/imagePreview.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-image-preview-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-image-preview-logic.mjs（画廊/灯箱/缩放逻辑回归）"

  pm_log "M2-3.b 脚本领域与持久化不变量夹具（边界/路径/命令三处同步/审计脱敏）…"
  python3 "$SCRIPT_DIR/check-script-domain-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-script-domain-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-script-domain-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-script-domain-policy.py（脚本领域安全边界被破坏）"

  pm_log "M2-4.b 脚本执行通道不变量夹具（进程组/超时分层/回收/引号/平台桩）…"
  python3 "$SCRIPT_DIR/check-script-exec-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-script-exec-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-script-exec-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-script-exec-policy.py（执行通道安全边界被破坏）"
  python3 "$SCRIPT_DIR/check-script-exec-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-script-exec-policy.py --expect-pending（有 pending 码位已实现，应转入默认判定）"

  pm_log "git diff --check（工作树 + 暂存区，机器证据除外）…"
  # Raw evidence is immutable third-party output; SHA256SUMS, not whitespace rewriting, protects it.
  git -C "$ROOT" diff --check -- . ':(exclude)logs/m0-baseline/**' || pm_fail "git diff --check (worktree)"
  git -C "$ROOT" diff --cached --check -- . ':(exclude)logs/m0-baseline/**' || pm_fail "git diff --check (staged)"

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
  for t in bash cargo npm node python3 git sha256sum; do
    if ! command -v "$t" >/dev/null 2>&1; then
      echo "FAIL: required tool missing: $t"; rc=1
    fi
  done
  [ -f "$SCRIPT_DIR/baseline-check.sh" ] || { echo "FAIL: baseline-check.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/verify-resources.sh" ] || { echo "FAIL: verify-resources.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/validate-summary.py" ] || { echo "FAIL: validate-summary.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/aggregate-m0-baseline.py" ] || { echo "FAIL: aggregate-m0-baseline.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-plan-routing.py" ] || { echo "FAIL: check-plan-routing.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-lifecycle-contract.py" ] || { echo "FAIL: check-lifecycle-contract.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-security-policy.py" ] || { echo "FAIL: check-security-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-git-write-policy.py" ] || { echo "FAIL: check-git-write-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-git-ui-policy.py" ] || { echo "FAIL: check-git-ui-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-git-ui-logic.mjs" ] || { echo "FAIL: check-git-ui-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-resource-capture-policy.py" ] || { echo "FAIL: check-resource-capture-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-resource-ui-logic.mjs" ] || { echo "FAIL: check-resource-ui-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-session-persistence-policy.py" ] || { echo "FAIL: check-session-persistence-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-session-logic.mjs" ] || { echo "FAIL: check-session-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-image-policy.py" ] || { echo "FAIL: check-image-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-image-ui-logic.mjs" ] || { echo "FAIL: check-image-ui-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-image-preview-policy.py" ] || { echo "FAIL: check-image-preview-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-image-preview-logic.mjs" ] || { echo "FAIL: check-image-preview-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-script-domain-policy.py" ] || { echo "FAIL: check-script-domain-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-script-exec-policy.py" ] || { echo "FAIL: check-script-exec-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-image-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-image-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-image-preview-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-image-preview-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-script-domain-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-script-domain-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-script-exec-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-script-exec-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-session-persistence-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-session-persistence-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-resource-capture-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-resource-capture-policy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/measure-build-metrics.py" ] || { echo "FAIL: measure-build-metrics.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-git-write-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-git-write-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-git-ui-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-git-ui-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/measure-build-metrics.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: measure-build-metrics.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-security-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-security-policy.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-lifecycle-contract.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-lifecycle-contract.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/collect-m0-baseline.sh" ] || { echo "FAIL: collect-m0-baseline.sh missing"; rc=1; }
  [ -f "$SCRIPT_DIR/schema/m0-summary.schema.json" ] || { echo "FAIL: schema missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/validate-summary.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: validate-summary.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-plan-routing.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-plan-routing.py --self-test"; rc=1
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
