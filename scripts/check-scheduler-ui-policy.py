#!/usr/bin/env python3
"""M4-8 定时任务 UI 静态不变量夹具（SCHEDUI_* 码位）。

守护的是「前端不许绕过 A6 契约」这一类**结构上**可判的红线，与
`scripts/check-scheduler-policy.py`（A6/A7 的 SCHED_* 后端码位）互不重叠：

- SCHEDUI_DIRECT_INVOKE      组件直接 invoke（必须经 bridge.ts）
- SCHEDUI_ENABLED_DEFAULT_TRUE 新建任务默认不是「未启用」（A6 裁定 R-A6-1）
- SCHEDUI_ENABLED_MUTATED    store 在新建/加载后偷偷把 enabled 置 true
- SCHEDUI_CRON_FIELD_COUNT   cron 段数常量不是 5，或段数判定未用该常量（A6 §4.4 / O-A6-4）
- SCHEDUI_CRON_SIX_FIELD_MSG 6 段拒绝信息未点明需要 5 段（用户复制 6 位表达式需可自查）
- SCHEDUI_INTERVAL_BOUND     间隔下界/上界常量缺失或未被校验使用（A6 F-A6-6）
- SCHEDUI_RETRY_UNCLAMPED    重试延迟未 clamp 到 max_delay_secs（A6 §5.3）
- SCHEDUI_STORE_UNGUARDED    store 调 bridge.task* 前没有 backendReady/guard 拦截
                             （后端命令未落地时不得发出 invoke）
- SCHEDUI_SECRET_EDITABLE    表单允许编辑/保存 secret 参数值（A6 §3.3 R-3）
- SCHEDUI_CATCHUP_NOT_GRAYED 补跑上限未按策略置灰（A6 §3.2，防 no-op 字段）
- SCHEDUI_NO_SKIP_REASON     历史面板不展示跳过原因（A6 O-A6-9）
- SCHEDUI_CRED_PERSISTED     定时任务参数/凭据落前端存储（M4 红线）
- SCHEDUI_UNKNOWN_COMMAND    bridge.ts 出现 A1/A6 未冻结的 task_* 命令名
- SCHEDUI_NO_DISABLED_SHELL  面板未对「后端未就绪」呈现禁用壳
- SCHEDUI_TASK_ADD_ARGS_CAMEL  task_add 入参非 camelCase 平铺（Tauri 默认转换，多词键必须 camelCase）
- SCHEDUI_TASK_UPDATE_WRAPPED  task_update 入参未包成 { task } 单结构体（serde snake_case）
- SCHEDUI_PANEL_LAZY           定时任务面板未懒加载（首屏 JS 体积闸门 IF-2）

沿用既有夹具三约定：
1. `--self-test` **双向自检**：1 个好样本零违规 + N 个坏样本各自命中对应码；
2. 坏样本**变异防呆**：变异字符串若未真正改动内容，按**漏检**计；
3. `PENDING_CODES` 与 `--expect-pending` 双向一致（本卡无 pending 码位，集合为空）。

用法:
    python3 scripts/check-scheduler-policy-ui.py --self-test
    python3 scripts/check-scheduler-policy-ui.py
    python3 scripts/check-scheduler-policy-ui.py --expect-pending
"""

from __future__ import annotations

import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

TASK_UI = "src/utils/taskUi.ts"
TASK_STORE = "src/capabilities/task/state/useTaskStore.ts"
TASK_PANEL = "src/capabilities/task/ui/TaskPanel.vue"
TASK_DIALOG = "src/capabilities/task/ui/TaskEditDialog.vue"
TASK_INDEX = "src/capabilities/task/index.ts"
BRIDGE = "src/bridge.ts"
MAIN_AREA = "src/components/layout/MainArea.vue"

# A1 展开卡 + A6 §6 冻结的 5 条命令，前端不得自行发明第 6 条。
FROZEN_COMMANDS = {
    "task_list",
    "task_add",
    "task_update",
    "task_remove",
    "task_run_now",
}

# 本卡无 pending 码位（M4-8.a/b/c 一次交付），保留集合以便后续卡转默认。
PENDING_CODES: set[str] = set()


def read_repo() -> dict[str, str]:
    files = [TASK_UI, TASK_STORE, TASK_PANEL, TASK_DIALOG, TASK_INDEX, BRIDGE, MAIN_AREA]
    out: dict[str, str] = {}
    for rel in files:
        path = os.path.join(ROOT, rel)
        try:
            with open(path, "r", encoding="utf-8") as fh:
                out[rel] = fh.read()
        except FileNotFoundError:
            out[rel] = ""
    return out


def check(files: dict[str, str]) -> list[str]:
    """返回违规码列表（默认模式只判默认码；pending 码位不参与）。"""
    violations: list[str] = []

    ui = files.get(TASK_UI, "")
    store = files.get(TASK_STORE, "")
    panel = files.get(TASK_PANEL, "")
    dialog = files.get(TASK_DIALOG, "")
    bridge = files.get(BRIDGE, "")
    mainarea = files.get(MAIN_AREA, "")

    # --- SCHEDUI_DIRECT_INVOKE：组件不得直接 invoke ---
    for name, text in ((TASK_PANEL, panel), (TASK_DIALOG, dialog)):
        if "invoke(" in text or "@tauri-apps/api" in text:
            violations.append("SCHEDUI_DIRECT_INVOKE")
            _ = name
            break

    # --- SCHEDUI_ENABLED_DEFAULT_TRUE（R-A6-1）---
    if "export const DEFAULT_TASK_ENABLED = false;" not in ui:
        violations.append("SCHEDUI_ENABLED_DEFAULT_TRUE")
    if "enabled: DEFAULT_TASK_ENABLED," not in ui:
        violations.append("SCHEDUI_ENABLED_DEFAULT_TRUE")

    # --- SCHEDUI_ENABLED_MUTATED：store 不得偷偷启用 ---
    if "draft.value.enabled = true" in store or "enabled: true" in store:
        violations.append("SCHEDUI_ENABLED_MUTATED")

    # --- SCHEDUI_CRON_FIELD_COUNT（A6 §4.4 / O-A6-4）---
    if "export const CRON_FIELD_COUNT = 5;" not in ui:
        violations.append("SCHEDUI_CRON_FIELD_COUNT")
    if "parts.length !== CRON_FIELD_COUNT" not in ui:
        violations.append("SCHEDUI_CRON_FIELD_COUNT")

    # --- SCHEDUI_CRON_SIX_FIELD_MSG：6 段拒绝信息必须点明「含秒不被支持」（O-A6-4）---
    if "含秒的 6 位表达式不被支持" not in ui:
        violations.append("SCHEDUI_CRON_SIX_FIELD_MSG")

    # --- SCHEDUI_INTERVAL_BOUND（F-A6-6）---
    if "export const MIN_INTERVAL_SECS = 60;" not in ui:
        violations.append("SCHEDUI_INTERVAL_BOUND")
    if "export const MAX_INTERVAL_SECS = 2592000;" not in ui:
        violations.append("SCHEDUI_INTERVAL_BOUND")
    if "secs < MIN_INTERVAL_SECS" not in ui or "secs > MAX_INTERVAL_SECS" not in ui:
        violations.append("SCHEDUI_INTERVAL_BOUND")

    # --- SCHEDUI_RETRY_UNCLAMPED（§5.3）---
    if "return Math.min(Math.trunc(raw), max);" not in ui:
        violations.append("SCHEDUI_RETRY_UNCLAMPED")

    # --- SCHEDUI_STORE_UNGUARDED：含 bridge.task* 调用的函数必须有 guard/backendReady 拦截 ---
    # 按函数块切分（避免固定窗口长度带来的漏判与误判）。
    import re

    for segment in re.split(r"\n(?=  (?:async )?function )", store):
        if "bridge.task" in segment and "guard(" not in segment and "backendReady" not in segment:
            violations.append("SCHEDUI_STORE_UNGUARDED")
            break

    # --- SCHEDUI_SECRET_EDITABLE（R-3）---
    if 'v-if="!p.secret"' not in dialog:
        violations.append("SCHEDUI_SECRET_EDITABLE")

    # --- SCHEDUI_CATCHUP_NOT_GRAYED（§3.2）---
    if ':disabled="!catchUpEditable"' not in dialog or "isCatchUpLimitEditable" not in dialog:
        violations.append("SCHEDUI_CATCHUP_NOT_GRAYED")

    # --- SCHEDUI_NO_SKIP_REASON（O-A6-9）：历史必须渲染跳过原因，不能只导入不用 ---
    if "skipReasonLabel(r." not in panel:
        violations.append("SCHEDUI_NO_SKIP_REASON")

    # --- SCHEDUI_CRED_PERSISTED ---
    for text in (ui, store, panel, dialog):
        if "localStorage.setItem" in text or "sessionStorage.setItem" in text:
            violations.append("SCHEDUI_CRED_PERSISTED")
            break

    # --- SCHEDUI_UNKNOWN_COMMAND：命令名必须来自冻结集合 ---
    import re

    for name in re.findall(r'"(task_[a-z_]+)"', bridge):
        if name not in FROZEN_COMMANDS:
            violations.append("SCHEDUI_UNKNOWN_COMMAND")
            break

    # --- SCHEDUI_NO_DISABLED_SHELL：未就绪时必须显式呈现禁用壳（不是静默不可用）---
    if 'v-if="!tasks.backendReady"' not in panel:
        violations.append("SCHEDUI_NO_DISABLED_SHELL")

    # --- SCHEDUI_TASK_ADD_ARGS_CAMEL：task_add 必须是 camelCase 平铺入参 ---
    # Tauri 默认把 Rust 命令参数从 snake_case 转 camelCase；多词键写成 snake_case 会被
    # 转换后丢失（后端收不到 target_id）。必须同时存在：taskUi 的序列化器 + bridge 整包透传。
    if "export function serializeTaskAddArgs" not in ui:
        violations.append("SCHEDUI_TASK_ADD_ARGS_CAMEL")
    if 'invoke<TaskDef>("task_add", p)' not in bridge:
        violations.append("SCHEDUI_TASK_ADD_ARGS_CAMEL")

    # --- SCHEDUI_TASK_UPDATE_WRAPPED：task_update 入参必须是 { task: TaskDef } 单结构体 ---
    if 'invoke<TaskDef>("task_update", { task: p })' not in bridge:
        violations.append("SCHEDUI_TASK_UPDATE_WRAPPED")

    # --- SCHEDUI_PANEL_LAZY：定时任务面板必须懒加载（defineAsyncComponent），不进主 chunk ---
    # STAGE G：面板已升格 capabilities/task/，懒加载迁至 task/index.ts（贡献驱动）；
    # MainArea 与 task 适配器均不得静态 import TaskPanel。
    task_index = files.get(TASK_INDEX, "")
    if "import TaskPanel from" in mainarea or "import TaskPanel from" in task_index:
        violations.append("SCHEDUI_PANEL_LAZY")
    lazy_loader_ok = 'load: () => import("./ui/TaskPanel.vue")' in task_index
    if not lazy_loader_ok:
        violations.append("SCHEDUI_PANEL_LAZY")

    # 去重并保持顺序稳定，便于断言
    seen: set[str] = set()
    ordered = []
    for code in violations:
        if code not in seen:
            seen.add(code)
            ordered.append(code)
    return ordered


# ---------------------------------------------------------------------------
# self-test：1 好 + N 坏（变异防呆）
# ---------------------------------------------------------------------------

BAD_SAMPLES: list[tuple[str, str, str, str]] = [
    # (码位, 文件, 原文, 变异文)
    (
        "SCHEDUI_DIRECT_INVOKE",
        TASK_PANEL,
        "const tasks = useTaskStore();",
        "const tasks = useTaskStore();\nimport { invoke } from '@tauri-apps/api/core';\nvoid invoke;",
    ),
    (
        "SCHEDUI_ENABLED_DEFAULT_TRUE",
        TASK_UI,
        "export const DEFAULT_TASK_ENABLED = false;",
        "export const DEFAULT_TASK_ENABLED = true;",
    ),
    (
        "SCHEDUI_ENABLED_MUTATED",
        TASK_STORE,
        "    draft.value = emptyTaskDraft();",
        "    draft.value = emptyTaskDraft();\n    draft.value.enabled = true;",
    ),
    (
        "SCHEDUI_CRON_FIELD_COUNT",
        TASK_UI,
        "export const CRON_FIELD_COUNT = 5;",
        "export const CRON_FIELD_COUNT = 6;",
    ),
    (
        "SCHEDUI_CRON_SIX_FIELD_MSG",
        TASK_UI,
        "含秒的 6 位表达式不被支持",
        "表达式不合法",
    ),
    (
        "SCHEDUI_INTERVAL_BOUND",
        TASK_UI,
        "export const MIN_INTERVAL_SECS = 60;",
        "export const MIN_INTERVAL_SECS = 1;",
    ),
    (
        "SCHEDUI_RETRY_UNCLAMPED",
        TASK_UI,
        "return Math.min(Math.trunc(raw), max);",
        "return Math.trunc(raw);",
    ),
    (
        "SCHEDUI_STORE_UNGUARDED",
        TASK_STORE,
        "    if (!guard()) return false;\n    if (!validate()) return false;",
        "    if (!validate()) return false;",
    ),
    (
        "SCHEDUI_SECRET_EDITABLE",
        TASK_DIALOG,
        'v-if="!p.secret"',
        'v-if="true"',
    ),
    (
        "SCHEDUI_CATCHUP_NOT_GRAYED",
        TASK_DIALOG,
        ':disabled="!catchUpEditable"',
        ':disabled="false"',
    ),
    (
        "SCHEDUI_NO_SKIP_REASON",
        TASK_PANEL,
        "skipReasonLabel(r.error_code)",
        "r.error_code",
    ),
    (
        "SCHEDUI_CRED_PERSISTED",
        TASK_DIALOG,
        "const tasks = useTaskStore();",
        "const tasks = useTaskStore();\nlocalStorage.setItem('task-token', 'x');",
    ),
    (
        "SCHEDUI_UNKNOWN_COMMAND",
        BRIDGE,
        'invoke<TaskDef[]>("task_list")',
        'invoke<TaskDef[]>("task_history")',
    ),
    (
        "SCHEDUI_NO_DISABLED_SHELL",
        TASK_PANEL,
        'v-if="!tasks.backendReady"',
        'v-if="false"',
    ),
    (
        "SCHEDUI_TASK_ADD_ARGS_CAMEL",
        BRIDGE,
        'invoke<TaskDef>("task_add", p)',
        'invoke<TaskDef>("task_add", { ...p, target_id: p.targetId })',
    ),
    (
        "SCHEDUI_TASK_UPDATE_WRAPPED",
        BRIDGE,
        'invoke<TaskDef>("task_update", { task: p })',
        'invoke<TaskDef>("task_update", p)',
    ),
    (
        "SCHEDUI_PANEL_LAZY",
        TASK_INDEX,
        'load: () => import("./ui/TaskPanel.vue")',
        'import TaskPanel from "./ui/TaskPanel.vue";',
    ),
]


def run_self_test() -> int:
    failures: list[str] = []

    def expect(cond: bool, label: str) -> None:
        if not cond:
            failures.append(label)
            print(f"  ✗ {label}")

    good = read_repo()
    missing = [name for name, text in good.items() if text == ""]
    if missing:
        print(f"FAIL: 缺少被扫描文件 {missing}")
        return 1

    good_violations = check(good)
    if good_violations:
        expect(False, f"好样本应零违规，实得 {good_violations}")
    else:
        print("  ✓ 好样本零违规")

    for code, rel, old, new in BAD_SAMPLES:
        files = dict(good)
        original = files[rel]
        if old not in original:
            expect(False, f"{code}: 变异锚点在 {rel} 中不存在（夹具已过时）")
            continue
        mutated = original.replace(old, new, 1)
        # 变异防呆：替换未真正改动内容时按漏检计。
        if mutated == original:
            expect(False, f"{code}: 变异未改动内容（锚点失效）")
            continue
        files[rel] = mutated
        found = check(files)
        if code not in found:
            expect(False, f"{code}: 坏样本未命中（实得 {found}）")
        else:
            print(f"  ✓ {code} 坏样本命中")

    # pending 集合与 --expect-pending 双向一致
    declared = sorted(PENDING_CODES)
    print(f"  · PENDING_CODES={declared}")

    if failures:
        print(f"\nSELF_TEST_RESULT=FAIL ({len(failures)} 项)")
        return 1
    print(f"\nSELF_TEST_RESULT=ALL_PASS（1 好样本 + {len(BAD_SAMPLES)} 坏样本）")
    return 0


def run_default() -> int:
    files = read_repo()
    violations = check(files)
    if violations:
        for code in violations:
            print(f"FAIL: {code}")
        print(f"\nCHECK_SCHEDULER_UI_POLICY_RESULT=FAIL ({len(violations)} 项)")
        return 1
    print(f"CHECK_SCHEDULER_UI_POLICY_RESULT=PASS（0 违规，{len(BAD_SAMPLES)} 个码位守护中）")
    return 0


def run_expect_pending() -> int:
    print(f"PENDING_CODES={','.join(sorted(PENDING_CODES))}")
    print("EXPECT_PENDING_RESULT=PASS")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if "--self-test" in args:
        return run_self_test()
    if "--expect-pending" in args:
        return run_expect_pending()
    if args:
        print(f"error: unknown argument {args}", file=sys.stderr)
        return 2
    return run_default()


if __name__ == "__main__":
    sys.exit(main())
