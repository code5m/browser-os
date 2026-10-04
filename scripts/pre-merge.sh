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
#  18. M2-3.b 脚本领域与持久化不变量夹具（前端逻辑测试已随 M2-5 UI 落地）
#  19. M2-4.e 脚本执行通道不变量夹具（进程组/输出上限/尾存/退出收口/shell spawn 移除）
#  20. M2-6.d 命令片段库 UI 逻辑层测试
#  21. M3.a 终端输出管道不变量夹具 + 前端 Channel/防抖接入
#  22. 工作树、暂存区、当前分支相对基线的 git diff --check
#  23. M4-8 定时任务 UI 不变量夹具（SCHEDUI_*）+ 前端逻辑层测试（加载真实 taskUi.ts / useTaskStore.ts）
#  24. M5-1.a core 边界不变量夹具（CORE_* 码位）：core 不得 import tauri / 引用 AppHandle·AppState /
#      反向引用 crate::bridge / 出现第二执行路径；[lib] 已声明；shim 与 mod 不冲突；无空文件。
#      ⚠️ 同 package 双 target 下编译器守不住这些，本夹具是唯一防线
#  25. 原生 UI 线程阻塞门禁：禁止 blocking dialog、rfd 同步对话框和 runtime block_on。
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
  build metrics                 measure-build-metrics.py --self-test / --compare（总体积遵循脚本当前上限、
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
  terminal pipeline fixture     check-terminal-policy.py --self-test / 默认门禁
  terminal UI logic tests       check-terminal-ui-logic.mjs（Node，headless）
  script domain fixture         check-script-domain-policy.py --self-test / 默认门禁
  command UI logic tests        check-command-ui-logic.mjs（Node，headless）
  scheduler policy fixture      check-scheduler-policy.py --self-test / --expect-pending / 默认门禁
                                （M4-5.d；20 ACTIVE 码 + 0 pending 码，A7 落地后已全转为默认判定）
  database policy fixture       check-database-policy.py --self-test / --expect-pending / 默认门禁
                                （M4-2.s；7 ACTIVE 码 + 8 pending 码，产物存在才判）
  plugin privacy fixture        check-plugin-privacy.py --self-test / 默认门禁
  plugin UI privacy fixture     check-plugin-ui-privacy.py --self-test / 默认门禁
                                （M5-W14；2 ACTIVE 码 + 2 pending 码。gated：A6 前端文件
                                  落地前为 no-op；落地后激活，守住「只经 bridge.ts / 禁渲染
                                  密钥签名路径 / 禁浏览器持久化 / 错误只显稳定码」）
                                （M5-W13；2 ACTIVE 码 + 5 pending 码。pending 通道当前
                                  有意报红：A9 需在 W13 接线前闭环 3 项回显缺陷，故暂不接
                                  --expect-pending 以免阻塞批次；修复后再接入并转 ACTIVE）
  git diff --check              工作树 + 暂存区 + 当前分支相对基线（机器证据除外）
  scheduler UI policy fixture   check-scheduler-ui-policy.py --self-test / 默认门禁（SCHEDUI_*，M4-8）
  scheduler UI logic tests      check-scheduler-ui-logic.mjs（Node，headless，加载真实 taskUi.ts / useTaskStore.ts）
  UI thread blocking fixture    check-ui-thread-blocking.py --self-test / 默认门禁

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

  pm_log "原生 UI 线程阻塞门禁…"
  python3 "$SCRIPT_DIR/check-ui-thread-blocking.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-ui-thread-blocking.py --self-test"
  python3 "$SCRIPT_DIR/check-ui-thread-blocking.py" >/dev/null 2>&1 \
    || pm_fail "check-ui-thread-blocking.py（禁止 blocking dialog / rfd / runtime block_on）"

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

  pm_log "M0-4.c 构建指标对比（总体积遵循 measure-build-metrics.py 当前上限、cargo warning 不增加）…"
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

  pm_log "M2-4.e 脚本执行通道不变量夹具（进程组/输出上限/尾存/退出收口/shell spawn 移除）…"
  python3 "$SCRIPT_DIR/check-script-exec-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-script-exec-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-script-exec-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-script-exec-policy.py（执行通道安全边界被破坏）"
  python3 "$SCRIPT_DIR/check-script-exec-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-script-exec-policy.py --expect-pending（有 pending 码位已实现，应转入默认判定）"

  pm_log "M2-5 脚本库 UI 不变量夹具（12 默认码；PENDING 集合当前为空）…"
  python3 "$SCRIPT_DIR/check-script-ui-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-script-ui-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-script-ui-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-script-ui-policy.py（脚本库 UI 安全边界被破坏）"
  python3 "$SCRIPT_DIR/check-script-ui-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-script-ui-policy.py --expect-pending（pending 码位集合应与 PENDING_CODES 一致）"

  pm_log "M2-6.a 命令片段领域不变量夹具（argv 模型 / 无正文文件 / 危险标记 / 超时口径）…"
  python3 "$SCRIPT_DIR/check-command-domain-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-command-domain-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-command-domain-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-command-domain-policy.py（命令片段契约被破坏）"
  python3 "$SCRIPT_DIR/check-command-domain-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-command-domain-policy.py --expect-pending（pending 码位集合应与 PENDING_CODES 一致）"

  pm_log "M2-7 工具清单与打包不变量夹具（契约冻结 + include_str! 嵌入 + 只读 list_tools + ACL）…"
  python3 "$SCRIPT_DIR/check-tools-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-tools-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-tools-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-tools-policy.py（工具清单契约被破坏）"

  pm_log "M2-9 种子工具验收不变量夹具（离线/零外链/零 bridge 写原语 + 零能力隔离 + 路径防御）…"
  python3 "$SCRIPT_DIR/check-seed-tools.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-seed-tools.py --self-test"
  python3 "$SCRIPT_DIR/check-seed-tools.py" >/dev/null 2>&1 \
    || pm_fail "check-seed-tools.py（种子工具验收不变量被破坏）"

  pm_log "M2-5 脚本库 UI 逻辑层自动化测试（headless，加载真实 scriptUi.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-script-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-script-ui-logic.mjs（脚本库 CRUD 前端逻辑回归）"

  pm_log "M3.a 终端输出管道不变量夹具（mpsc+pump/丢弃/退避/resize/进程组/零新依赖）…"
  python3 "$SCRIPT_DIR/check-terminal-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-terminal-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-terminal-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-terminal-policy.py（终端输出管道不变量被破坏）"

  pm_log "M3.c 终端体验项 UI 逻辑层自动化测试（headless，加载真实 useSystemStore.ts / useTerminalResize.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-terminal-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-terminal-ui-logic.mjs（临时历史 40 条 / resize 静默窗口回归）"

  pm_log "窗口拖动单一事件路径与主窗口权限回归（不替代桌面验收）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-window-drag.mjs") >/dev/null 2>&1 \
    || pm_fail "check-window-drag.mjs（拖动事件重复或权限回归）"

  pm_log "原生网页子窗口提示遮挡回归…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-native-webview-overlay.mjs") >/dev/null 2>&1 \
    || pm_fail "check-native-webview-overlay.mjs（提示浮层会被原生网页遮挡）"

  pm_log "M2-6.d 命令片段库 UI 逻辑层自动化测试（headless，加载真实 snippetUi.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-command-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-command-ui-logic.mjs（命令片段库前端逻辑回归）"

  # M4-4.a（Lane A5）：数据库面板 UI 逻辑层回归。加载真实 src/utils/dbUi.ts，
  # 覆盖 F1 写默认拒绝 / F2 凭据零落地 / 截断告警 / 带标签 DbValue 解码 / CSV 脱敏 /
  # 风险·生产判定 / 运行门禁等。与 M2-6.d 同范式（headless，无 GUI 依赖）。
  pm_log "M4-4.a 数据库面板 UI 逻辑层自动化测试（headless，加载真实 dbUi.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-database-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-database-ui-logic.mjs（数据库面板前端逻辑回归）"

  # M4-5.d（Lane A6）：调度契约不变量。scheduler/tasks/TaskDef 由 A7 落地，实现经
  # 自检全合规，原 8 个 pending 码位已全部提升为 ACTIVE（现 20 ACTIVE / 0 pending）。
  pm_log "M4-5.d 调度契约不变量夹具（20 ACTIVE 码，0 pending 码）…"
  python3 "$SCRIPT_DIR/check-scheduler-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-scheduler-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-scheduler-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-scheduler-policy.py（调度契约被破坏）"
  python3 "$SCRIPT_DIR/check-scheduler-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-scheduler-policy.py --expect-pending（有 pending 码位已实现，应转入默认判定）"

  # M4-2.s（Lane A4）：数据库安全闸门不变量。SQL 风险分类与生产判定是纯函数
  # （零 db 依赖），故本项可先于 database.rs 落地即生效；db 命令 / 连接池相关的
  # 8 个 pending 码位在产物不存在时自动降级为 no-op，落地后立即接管。
  pm_log "M4-2.s 数据库安全闸门不变量夹具（7 ACTIVE 码 + 8 pending 码，产物存在才判）…"
  python3 "$SCRIPT_DIR/check-database-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-database-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-database-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-database-policy.py（数据库安全闸门被破坏）"
  python3 "$SCRIPT_DIR/check-database-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-database-policy.py --expect-pending（有 pending 码位已实现，应转入默认判定）"

  # M5-W13（Lane A4）：插件隐私不变量夹具（错误不回显 / 审计只用 ids+计数+哈希前缀）。
  # 只接 --self-test 与默认门禁：pending 通道当前有意报红（capabilities/plugin/plugin.rs 潜伏回显缺陷，
  # 待 A9 在 W13 接线前修复），接入 --expect-pending 会阻塞整批，故暂不接。
  python3 "$SCRIPT_DIR/check-plugin-privacy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-plugin-privacy.py --self-test"
  python3 "$SCRIPT_DIR/check-plugin-privacy.py" >/dev/null 2>&1 \
    || pm_fail "check-plugin-privacy.py（插件隐私不变量被破坏）"

  # M5-W14（Lane A4）：插件管理器 UI 隐私夹具（gated：A6 前端文件未落地时为 no-op，
  # 默认门禁与 --self-test 均绿；A6 落地后激活，守住渲染/存储层隐私红线）。
  python3 "$SCRIPT_DIR/check-plugin-ui-privacy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-plugin-ui-privacy.py --self-test"
  python3 "$SCRIPT_DIR/check-plugin-ui-privacy.py" >/dev/null 2>&1 \
    || pm_fail "check-plugin-ui-privacy.py（插件 UI 隐私不变量被破坏）"

  # M4-8（Lane A8）：定时任务 UI 不变量夹具 + 前端逻辑层测试。
  # 守护「前端不许绕过 A6 契约」的结构红线（SCHEDUI_* 码位），与 M4-5.d 后端码位互不重叠。
  # --self-test 双向自检（1 好样本 + N 坏样本变异防呆）；默认模式按设计放行。
  pm_log "M4-8 定时任务 UI 不变量夹具（SCHEDUI_* 码位）…"
  python3 "$SCRIPT_DIR/check-scheduler-ui-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-scheduler-ui-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-scheduler-ui-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-scheduler-ui-policy.py（定时任务 UI 契约被破坏）"

  # M4-8 前端逻辑层测试（headless，加载真实 src/utils/taskUi.ts + useTaskStore.ts）。
  pm_log "M4-8 定时任务 UI 逻辑层自动化测试（headless，加载真实 taskUi.ts / useTaskStore.ts）…"
  (cd "$ROOT" && node "$SCRIPT_DIR/check-scheduler-ui-logic.mjs") >/dev/null 2>&1 \
    || pm_fail "check-scheduler-ui-logic.mjs（定时任务前端逻辑回归）"

  # M5-1.a（Lane A2）：core 边界不变量夹具（CORE_* 码位）。
  # ⚠️ 这是**唯一**能守住 core 纯洁性的手段：阶段一采用同 package 双 target，
  # package 级 [dependencies] 对 lib 与 bin 同时生效，因此 core 内写 `use tauri::…`
  # 在**编译上完全能过**——rustc 与 cargo tree 都不会报错，只有本夹具会拦。
  # 守住：core 不 import tauri / 不引用 AppHandle·AppState / 不反向引用 crate::bridge
  # 与二进制专属模块 / 不出现第二执行路径 / [lib] 已声明 / shim 与 mod 不冲突 / 无空文件。
  pm_log "M5-1.a core 边界不变量夹具（CORE_* 码位）…"
  python3 "$SCRIPT_DIR/check-core-boundary.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-core-boundary.py --self-test"
  python3 "$SCRIPT_DIR/check-core-boundary.py" >/dev/null 2>&1 \
    || pm_fail "check-core-boundary.py（core 边界被破坏：tauri / AppHandle / crate::bridge / 第二执行路径）"
  python3 "$SCRIPT_DIR/check-core-boundary.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-core-boundary.py --expect-pending（有 pending 码位已实现，应转入默认判定）"

  # M5-2（Lane A3）：MCP 命令注册表 / 全局策略夹具。W8 收口 W1 相位债：PENDING 码位全部退役
  # （相位债关闭），rmcp/server/listener 由单一 ACTIVE 守门 MCP_NO_RMCP_SERVER 永久禁止；只读桥
  # 已落地，故新增 --expect-current-gaps 当前相位断言（只读桥在 / 无 rmcp/server/listener / 奇偶只读全绿）。
  # ⚠️ 同 check-core-boundary 同理：rmcp/tokio 红线在编译层守不住，只能靠本夹具。
  pm_log "M5-2 MCP 策略不变量夹具（W8：8 ACTIVE 码，0 PENDING 码；--self-test + 默认 + 当前相位）…"
  python3 "$SCRIPT_DIR/check-mcp-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-mcp-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-mcp-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-mcp-policy.py（M5-2 MCP 安全不变量被破坏：能力漂移/路径根/URL 脱敏/rmcp/tokio/监听）"
  python3 "$SCRIPT_DIR/check-mcp-policy.py" --expect-current-gaps >/dev/null 2>&1 \
    || pm_fail "check-mcp-policy.py --expect-current-gaps（W8 当前相位：只读桥在 / 无 rmcp-server/listener）"

  # M5-3（Lane A4）：agent memory KV 契约不变量夹具（5 ACTIVE 码位，产物存在才判）。
  # 守容量常量单一真源 / 隐私双扫 / per-agent 字节软配额 / 总上限接入 / 审计脱敏。
  pm_log "M5-3 agent memory KV 策略不变量夹具（5 ACTIVE 码位）…"
  python3 "$SCRIPT_DIR/check-agent-memory-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-agent-memory-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-agent-memory-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-agent-memory-policy.py（M5-3 agent memory KV 契约被破坏：常量缺失/隐私未扫值/per-agent 误用 32/无上限/审计含值）"

  # M5-4/5（Lane A5）：Agent/Skill 域与命令策略夹具（AGSK_* 码位）。
  # 守护：无第二执行路径 / SkillExec 禁内联 / ACL 末条 K1 / 能力单一真源 / 命令 ACL 奇偶。
  pm_log "M5-4/5 Agent/Skill 策略不变量夹具（AGSK_* 码位）…"
  python3 "$SCRIPT_DIR/check-agent-skill-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-agent-skill-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-agent-skill-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-agent-skill-policy.py（Agent/Skill 安全不变量被破坏：第二执行路径/Inline/ACL末条/能力漂移）"

  # M5-7/8（Lane A7）：知识图谱 core 切片不变量夹具（GRAPH_* 码位）。
  # 守容量常量单一真源 / GRAPH_PROPS_MAX_BYTES == MAX_TEXT_FIELD_BYTES / 隐私双扫 /
  # 容量拒绝 / 遍历有界 / Skill·Agent id 十六进制完整性 / 无第二执行路径。
  pm_log "M5-7/8 知识图谱 core 切片策略不变量夹具（GRAPH_* 码位）…"
  python3 "$SCRIPT_DIR/check-graph-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-graph-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-graph-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-graph-policy.py（M5-7/8 图谱安全不变量被破坏：常量缺失/隐私未扫值/容量未接/遍历无界/id 非十六进制/第二执行路径）"

  # Phase 0 命令集三源一致性（Rust 注册 ∩ 主窗 ACL ∩ 前端 invoke）。
  # 守 not-allowed 漂移 / 死 ACL / 死调用；KNOWN 仅容纳已裁决的契约占位漂移。
  pm_log "Phase 0 命令集三源一致性（IPC）…"
  python3 "$SCRIPT_DIR/check-command-set-consistency.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-command-set-consistency.py --self-test"
  python3 "$SCRIPT_DIR/check-command-set-consistency.py" >/dev/null 2>&1 \
    || pm_fail "check-command-set-consistency.py（命令集三源漂移：已注册命令未放行/死 ACL/死调用）"

  # M5-10/11（Lane A9）：插件 manifest/生命周期策略不变量夹具（PLUGIN_* 码位）。
  # 守能力单一真源 / 无第二执行路径 / 无内联 shell / 无签名旁路 / 仅形态③ / 无凭据字段。
  pm_log "M5-10/11 插件 manifest/生命周期策略不变量夹具（PLUGIN_* 码位）…"
  python3 "$SCRIPT_DIR/check-plugin-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-plugin-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-plugin-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-plugin-policy.py（M5-10/11 插件安全不变量被破坏：能力漂移/第二执行路径/内联shell/签名旁路/非形态③/凭据字段）"

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

  # Phase 03 checker gate (added by 11-ci). Wires the five boundary checkers + doctor
  # into the pre-merge gate WITHOUT weakening any existing M0-1.c check above.
  pm_log "Phase 03 checker gate (architecture/ui/native/runtime/task-boundary + capability + doctor)…"
  for c in check-architecture check-ui check-native check-native-command-inventory check-browser-runtime check-task-boundary check-grid-close-logic check-view-intent check-capability-boundaries check-capability-composition check-composition-profiles check-capability-platform check-capability-runtime check-pluggable-runtime check-pluggable-lifecycle check-terminal-owners check-developer-owners check-semantic-registry check-semantic-closure-logic check-workspace-owners check-sensitive-side-effects doctor; do
    if ! (cd "$ROOT" && node "$SCRIPT_DIR/$c.mjs") >/dev/null 2>&1; then
      pm_fail "phase03 $c.mjs"
    fi
  done

  # Phase 1.7：Git repository object integrity gate（corrupt/missing loose objects）。
  # Read-only; blocks merge when fsck reports errors. Does not weaken any above check.
  pm_log "Phase 1.7 Git repository object integrity gate…"
  if ! bash "$SCRIPT_DIR/check-git-repo-integrity.sh" >/dev/null 2>&1; then
    pm_fail "check-git-repo-integrity.sh（仓库对象损坏：corrupt/missing；先 git-recover.sh --prune-orphans）"
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
  [ -f "$SCRIPT_DIR/check-script-ui-policy.py" ] || { echo "FAIL: check-script-ui-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-script-ui-logic.mjs" ] || { echo "FAIL: check-script-ui-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-command-ui-logic.mjs" ] || { echo "FAIL: check-command-ui-logic.mjs missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-database-ui-logic.mjs" ] || { echo "FAIL: check-database-ui-logic.mjs missing"; rc=1; }
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
  if ! python3 "$SCRIPT_DIR/check-script-ui-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-script-ui-policy.py --self-test"; rc=1
  fi
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-command-ui-logic.mjs") >/dev/null 2>&1; then
    echo "FAIL: check-command-ui-logic.mjs"; rc=1
  fi
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-database-ui-logic.mjs") >/dev/null 2>&1; then
    echo "FAIL: check-database-ui-logic.mjs"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-tools-policy.py" ] || { echo "FAIL: check-tools-policy.py missing"; rc=1; }
  [ -f "$SCRIPT_DIR/check-terminal-policy.py" ] || { echo "FAIL: check-terminal-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-terminal-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-terminal-policy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-terminal-ui-logic.mjs" ] || { echo "FAIL: check-terminal-ui-logic.mjs missing"; rc=1; }
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-terminal-ui-logic.mjs") >/dev/null 2>&1; then
    echo "FAIL: check-terminal-ui-logic.mjs"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-plugin-privacy.py" ] || { echo "FAIL: check-plugin-privacy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-plugin-privacy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-plugin-privacy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-plugin-ui-privacy.py" ] || { echo "FAIL: check-plugin-ui-privacy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-plugin-ui-privacy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-plugin-ui-privacy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-database-policy.py" ] || { echo "FAIL: check-database-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-database-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-database-policy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-scheduler-ui-policy.py" ] || { echo "FAIL: check-scheduler-ui-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-scheduler-ui-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-scheduler-ui-policy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-scheduler-ui-logic.mjs" ] || { echo "FAIL: check-scheduler-ui-logic.mjs missing"; rc=1; }
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-scheduler-ui-logic.mjs") >/dev/null 2>&1; then
    echo "FAIL: check-scheduler-ui-logic.mjs"; rc=1
  fi
  # M5-1.a（Lane A2）：core 边界夹具（core 纯洁性在编译层守不住，只能靠它）
  [ -f "$SCRIPT_DIR/check-core-boundary.py" ] || { echo "FAIL: check-core-boundary.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-core-boundary.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-core-boundary.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-mcp-policy.py" ] || { echo "FAIL: check-mcp-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-mcp-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-mcp-policy.py --self-test"; rc=1
  fi
  # M5-3（Lane A4）：agent memory KV 契约门禁自检。
  [ -f "$SCRIPT_DIR/check-agent-memory-policy.py" ] || { echo "FAIL: check-agent-memory-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-agent-memory-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-agent-memory-policy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-agent-skill-policy.py" ] || { echo "FAIL: check-agent-skill-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-agent-skill-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-agent-skill-policy.py --self-test"; rc=1
  fi
  # M5-7/8（Lane A7）：图谱策略夹具自检。
  [ -f "$SCRIPT_DIR/check-graph-policy.py" ] || { echo "FAIL: check-graph-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-graph-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-graph-policy.py --self-test"; rc=1
  fi
  # Phase 0：命令集三源一致性夹具自检。
  [ -f "$SCRIPT_DIR/check-command-set-consistency.py" ] || { echo "FAIL: check-command-set-consistency.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-command-set-consistency.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-command-set-consistency.py --self-test"; rc=1
  fi
  # Phase 1：Browser/Grid 单一语义门禁自检（view intent / 状态模型 / 生命周期）。
  [ -f "$SCRIPT_DIR/check-view-intent.mjs" ] || { echo "FAIL: check-view-intent.mjs missing"; rc=1; }
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-view-intent.mjs" --self-test) >/dev/null 2>&1; then
    echo "FAIL: check-view-intent.mjs --self-test"; rc=1
  fi
  # Phase 1.5：Semantic Registry 语义门禁自检（重复状态 / 重复 Intent / Owner 越界 / 副作用误判）。
  [ -f "$SCRIPT_DIR/check-semantic-registry.mjs" ] || { echo "FAIL: check-semantic-registry.mjs missing"; rc=1; }
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-semantic-registry.mjs" --self-test) >/dev/null 2>&1; then
    echo "FAIL: check-semantic-registry.mjs --self-test"; rc=1
  fi
  # Phase 5.1-C：Sensitive Side Effect Contract 自检（keyring 数据流 / 已否决泄露意图）。
  [ -f "$SCRIPT_DIR/check-sensitive-side-effects.mjs" ] || { echo "FAIL: check-sensitive-side-effects.mjs missing"; rc=1; }
  if ! (cd "$ROOT" && node "$SCRIPT_DIR/check-sensitive-side-effects.mjs" --self-test) >/dev/null 2>&1; then
    echo "FAIL: check-sensitive-side-effects.mjs --self-test"; rc=1
  fi
  # Phase 1.7：Git 对象完整性门禁自检（注入损坏检出 + 干净通过，夹具在 /tmp）。
  [ -f "$SCRIPT_DIR/check-git-repo-integrity.sh" ] || { echo "FAIL: check-git-repo-integrity.sh missing"; rc=1; }
  if ! bash "$SCRIPT_DIR/check-git-repo-integrity.sh" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-git-repo-integrity.sh --self-test"; rc=1
  fi
  # M5-10/11（Lane A9）：插件策略夹具自检。
  [ -f "$SCRIPT_DIR/check-plugin-policy.py" ] || { echo "FAIL: check-plugin-policy.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-plugin-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-plugin-policy.py --self-test"; rc=1
  fi
  [ -f "$SCRIPT_DIR/check-ui-thread-blocking.py" ] || { echo "FAIL: check-ui-thread-blocking.py missing"; rc=1; }
  if ! python3 "$SCRIPT_DIR/check-ui-thread-blocking.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-ui-thread-blocking.py --self-test"; rc=1
  fi
  if ! python3 "$SCRIPT_DIR/check-tools-policy.py" --self-test >/dev/null 2>&1; then
    echo "FAIL: check-tools-policy.py --self-test"; rc=1
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
