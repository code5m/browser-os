#!/usr/bin/env python3
"""Expose the M2-5.a script-library CRUD UI invariants as a reproducible fixture.

契约来源：
- `logs/checkpoints/M2-5-20260905-1301.md`（§5 测试矩阵 U1~U12 / §6 违规码）
- `logs/checkpoints/M2-3.a-20260903-1604.md`（ScriptMeta / ScriptParam 口径）
- `logs/checkpoints/M2-4.a-20260903-2233.md`（RunStatus / 事件 / 审计）

本夹具守住脚本库 UI 的不变量（a 卡 + b 卡职责合并）：

  a 卡（纯前端 CRUD UI）七条红线：
  1. 输出/正文渲染禁用 `v-html`（XSS）
  2. 组件不得绕过 `bridge.ts` 直接 `invoke`
  3. 前端不得参与脚本正文路径构造
  4. secret 参数不得明文回显
  5. 删除必须二次确认
  6. 不得新增 npm 依赖
  7. `mainView='scripts'` 必须注册进 `MainView` 枚举 + `MOD_META` + ActivityBar 入口

  b 卡（执行面板）五条不变量：
  8. 运行取消入口必须调用 `bridge.cancelScript`
  9. 事件订阅必须在组件卸载时 `unlisten`
  10. 输出流必须用 `requestAnimationFrame` 合并（D14 前端层）
  11. 输出渲染禁止 HTML 注入（XSS / sandbox escape）
  12. 后端须 emit `script.validate.reject` 审计（D15）

模式：
  默认            判定 ACTIVE 码（a 卡职责）；零命中 → EXIT 0
  --expect-pending 验证 PENDING 码位集合（b/c 职责）确实仍未实现 → EXIT 0
  --self-test     好坏样本双向自检（含**变异防呆**：坏样本必须真的改动内容）

关于「一次性定义码位」：沿用 M0-3.a 模式，13 个码位一次定义，
a/b/c 卡职责的 12 个 ACTIVE 码默认判定；PENDING 集合当前为空
（待 M2-6 等新卡回归时按需扩展）。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS_DIR = ROOT / "src" / "components" / "workspace"
SCRIPT_PANEL = SCRIPTS_DIR / "ScriptPanel.vue"
SCRIPT_PARAM_FORM = SCRIPTS_DIR / "ScriptParamForm.vue"
LAYOUT_STORE = ROOT / "src" / "stores" / "useLayoutStore.ts"
ACTIVITY_BAR = ROOT / "src" / "components" / "layout" / "ActivityBar.vue"
BRIDGE_TS = ROOT / "src" / "bridge.ts"
PACKAGE_JSON = ROOT / "package.json"
PACKAGE_LOCK = ROOT / "package-lock.json"
SCRIPT_RUN_DIALOG = SCRIPTS_DIR / "ScriptRunDialog.vue"
BRIDGE_RS = ROOT / "src-tauri" / "src" / "bridge.rs"
# M2-6-fix1（复核 F-2）：命令片段库前端此前**完全不在扫描范围**——
# CommandSnippetPanel.vue（188 行）与 src/utils/snippetUi.ts（179 行）对
# v-html / 直接 invoke / secret 回显 / 删除确认等 13 个码位零覆盖。
COMMAND_PANEL = SCRIPTS_DIR / "CommandSnippetPanel.vue"
SNIPPET_UI = ROOT / "src" / "utils" / "snippetUi.ts"

# ----------------------------- 码位定义 -----------------------------

# a 卡职责（已落地，默认判定）
ACTIVE_CODES = (
    "SCRIPTUI_VHTML_OUTPUT",
    "SCRIPTUI_DIRECT_INVOKE",
    "SCRIPTUI_PARAM_PATH_CONSTRUCT",
    "SCRIPTUI_SECRET_ECHO",
    "SCRIPTUI_NO_CONFIRM_DELETE",
    "SCRIPTUI_NEW_NPM_DEP",
    "SCRIPTUI_MAINVIEW_UNREGISTERED",
    # b 卡职责（执行面板，已落地）
    "SCRIPTUI_NO_CANCEL_BUTTON",
    "SCRIPTUI_NO_EVENT_UNSUBSCRIBE",
    "SCRIPTUI_OUTPUT_NO_THROTTLE",
    "SCRIPTUI_SANDBOX_ESCAPE",
    "SCRIPTUI_AUDIT_REJECT_MISSING",
    # c 卡职责（运行历史，已落地：script_runs_list 命令 + UI）
    "SCRIPTUI_RUNS_LIST_MISSING",
    # M2-6.d 命令片段库前端（fix1 补齐扫描范围，见 F-2）
    "SCRIPTUI_CMD_DANGEROUS_NO_CONFIRM",
    "SCRIPTUI_CMD_ARGV_SHELL_PARSE",
    "SCRIPTUI_CMD_MAINVIEW_UNREGISTERED",
)

# 当前无 pending 码位（a/b/c 全 active）；保留元组结构便于后续 M2-6 等回归
PENDING_CODES: tuple[str, ...] = ()

# ----------------------------- 检测规则 -----------------------------

VHTML_PATTERN = re.compile(r"v-html\s*=")
DIRECT_INVOKE_PATTERN = re.compile(r"\binvoke\s*\(")
BRIDGE_PROXY = re.compile(r"\bbridge\.[a-zA-Z_]+\s*\(")
# 前端参与脚本正文路径构造：出现 scripts/<id>.<ext> 形态的拼接
PATH_CONSTRUCT_PATTERN = re.compile(r"scripts/\$\{")
# secret 明文回显：以文本插值 {{ ...default }} 形式明文显示 secret 参数值
SECRET_ECHO_PATTERN = re.compile(r"{{\s*[^}]*default[^}]*}}")
# 删除二次确认：存在 confirm( 或 .modal-mask 结构
CONFIRM_PRESENT_PATTERN = re.compile(r"\bconfirm\s*\(|\.modal-mask\b")
# mainView 注册：枚举分支 `| "scripts"`（非 MOD_META / ActivityBar 的值字符串）
MAINVIEW_ENUM_PATTERN = re.compile(r'\|\s*"scripts"')
MAINVIEW_META_PATTERN = re.compile(r"scripts\s*:\s*\{")

# ---- b 卡职责（已落地，默认判定）----
# 运行取消入口：必须调用 bridge.cancelScript
CANCEL_PATTERN = re.compile(r"cancelScript")
# 事件订阅卸载清理：必须 unlisten（否则并发泄漏）
UNLISTEN_PATTERN = re.compile(r"\bunlisten\b")
# 输出流 rAF 合并（D14 前端层）
RAF_PATTERN = re.compile(r"requestAnimationFrame")
# 输出渲染安全：禁止 HTML 注入（XSS / sandbox escape）
SANDBOX_PATTERN = re.compile(r"v-html|innerHTML|outerHTML|insertAdjacentHTML")

# ---- M2-6.d 命令片段库前端（fix1 新增）----
# dangerous 片段运行前必须二次确认：`m.dangerous && !confirm(` 同窗口内出现
DANGEROUS_CONFIRM_PATTERN = re.compile(r"dangerous[\s\S]{0,120}confirm\(")
# argv 编辑不得引入 shell 词法解析：禁 eval / new Function / 按空格切分
ARGV_SHELL_PARSE_PATTERN = re.compile(r"\beval\s*\(|new\s+Function\s*\(|split\(\s*[\"']\s[\"']\s*\)")
# argv 必须按「一行一个元素」解析（与后端 argv 数组一一对应）
ARGV_LINE_SPLIT_PATTERN = re.compile(r"split\(/\\r\?\\n/\)")
# commands 模块入口注册：枚举 + MOD_META + ActivityBar
CMD_MAINVIEW_ENUM_PATTERN = re.compile(r'\|\s*"commands"')
CMD_MAINVIEW_META_PATTERN = re.compile(r"commands\s*:\s*\{")


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8") if path.is_file() else ""


def count_deps(pkg_json: str, pkg_lock: str) -> int:
    n = 0
    try:
        d = json.loads(pkg_json)
        n += len(d.get("dependencies", {}))
        n += len(d.get("devDependencies", {}))
    except Exception:
        pass
    try:
        l = json.loads(pkg_lock)
        pkgs = l.get("packages")
        if pkgs is not None:
            n += len(pkgs)
        else:
            n += len(l.get("dependencies", {}))
    except Exception:
        pass
    return n


def detect(ctx: dict[str, str]) -> set[str]:
    hits: set[str] = set()
    panel = ctx["panel"]
    paramform = ctx["paramform"]
    rundialog = ctx["rundialog"]
    cmdsnippet = ctx["cmdsnippet"]
    snippetui = ctx["snippetui"]
    # 命令片段库前端纳入共享红线（v-html / 直接 invoke / secret 回显 / sandbox escape 等）
    ui_sources = panel + "\n" + paramform + "\n" + rundialog + "\n" + cmdsnippet + "\n" + snippetui
    bridge_rs = ctx["bridge_rs"]

    # 1) v-html 禁用（XSS 红线）
    if VHTML_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_VHTML_OUTPUT")

    # 2) 绕过 bridge 直接 invoke（bridge.ts 自身的封装允许，UI 组件不允许）
    for m in DIRECT_INVOKE_PATTERN.finditer(ui_sources):
        window = ui_sources[max(0, m.start() - 40) : m.start() + 80]
        if not BRIDGE_PROXY.search(window):
            hits.add("SCRIPTUI_DIRECT_INVOKE")
            break

    # 3) 前端构造脚本正文路径
    if PATH_CONSTRUCT_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_PARAM_PATH_CONSTRUCT")

    # 4) secret 明文回显
    if SECRET_ECHO_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_SECRET_ECHO")

    # 5) 删除无二次确认
    if not CONFIRM_PRESENT_PATTERN.search(panel):
        hits.add("SCRIPTUI_NO_CONFIRM_DELETE")

    # 6) 新增 npm 依赖：package.json / package-lock.json 依赖条目数变化
    base = ctx["base_deps"]
    cur = count_deps(ctx["package_json"], ctx["package_lock"])
    if cur != base:
        hits.add("SCRIPTUI_NEW_NPM_DEP")

    # 7) mainView 注册完整性：枚举 + MOD_META + ActivityBar 入口
    layout = ctx["layout"]
    activity = ctx["activity"]
    enum_ok = bool(MAINVIEW_ENUM_PATTERN.search(layout))
    meta_ok = bool(MAINVIEW_META_PATTERN.search(layout))
    activity_ok = '"scripts"' in activity
    if not (enum_ok and meta_ok and activity_ok):
        hits.add("SCRIPTUI_MAINVIEW_UNREGISTERED")

    # 8) 运行取消入口：必须有 bridge.cancelScript（SCRIPTUI_NO_CANCEL_BUTTON）
    if not CANCEL_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_NO_CANCEL_BUTTON")

    # 9) 事件订阅卸载清理：必须 unlisten（SCRIPTUI_NO_EVENT_UNSUBSCRIBE）
    if not UNLISTEN_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_NO_EVENT_UNSUBSCRIBE")

    # 10) 输出流 rAF 合并（D14 前端层，SCRIPTUI_OUTPUT_NO_THROTTLE）
    if not RAF_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_OUTPUT_NO_THROTTLE")

    # 11) 输出渲染安全：禁止 HTML 注入（SCRIPTUI_SANDBOX_ESCAPE）
    if SANDBOX_PATTERN.search(ui_sources):
        hits.add("SCRIPTUI_SANDBOX_ESCAPE")

    # 12) D15 审计：后端须 emit script.validate.reject（SCRIPTUI_AUDIT_REJECT_MISSING）
    if "script.validate.reject" not in bridge_rs:
        hits.add("SCRIPTUI_AUDIT_REJECT_MISSING")

    # 13) M2-5.c 运行历史命令：bridge.rs 须含 `script_runs_list` 函数或字符串（SCRIPTUI_RUNS_LIST_MISSING）。
    # 同时检查「fn script_runs_list」与裸字符串，避免「删函数但留注释」漏检。
    if (
        "fn script_runs_list" not in bridge_rs
        and "script_runs_list" not in bridge_rs
    ):
        hits.add("SCRIPTUI_RUNS_LIST_MISSING")

    # 14) 命令片段库：dangerous 运行前必须二次确认（SCRIPTUI_CMD_DANGEROUS_NO_CONFIRM）
    if cmdsnippet and not DANGEROUS_CONFIRM_PATTERN.search(cmdsnippet):
        hits.add("SCRIPTUI_CMD_DANGEROUS_NO_CONFIRM")

    # 15) 命令片段库：argv 必须按行解析，不得引入 shell 词法解析（SCRIPTUI_CMD_ARGV_SHELL_PARSE）
    if snippetui:
        if ARGV_SHELL_PARSE_PATTERN.search(snippetui) or not ARGV_LINE_SPLIT_PATTERN.search(snippetui):
            hits.add("SCRIPTUI_CMD_ARGV_SHELL_PARSE")

    # 16) 命令片段库：mainView='commands' 注册完整性（SCRIPTUI_CMD_MAINVIEW_UNREGISTERED）
    if cmdsnippet:
        cmd_enum_ok = bool(CMD_MAINVIEW_ENUM_PATTERN.search(layout))
        cmd_meta_ok = bool(CMD_MAINVIEW_META_PATTERN.search(layout))
        cmd_activity_ok = '"commands"' in activity
        if not (cmd_enum_ok and cmd_meta_ok and cmd_activity_ok):
            hits.add("SCRIPTUI_CMD_MAINVIEW_UNREGISTERED")

    return hits


def read_ctx() -> dict[str, str]:
    pkg = read(PACKAGE_JSON)
    lock = read(PACKAGE_LOCK)
    return {
        "panel": read(SCRIPT_PANEL),
        "paramform": read(SCRIPT_PARAM_FORM),
        "rundialog": read(SCRIPT_RUN_DIALOG),
        "cmdsnippet": read(COMMAND_PANEL),
        "snippetui": read(SNIPPET_UI),
        "layout": read(LAYOUT_STORE),
        "activity": read(ACTIVITY_BAR),
        "bridge_rs": read(BRIDGE_RS),
        "package_json": pkg,
        "package_lock": lock,
        "base_deps": count_deps(pkg, lock),
    }


# ----------------------------- 自检（含变异防呆） -----------------------------

# (期望命中的码, 原文片段, 替换为, 替换次数, 目标文件键)
MUTATIONS: tuple[tuple[str, str, str, int, str], ...] = (
    ("SCRIPTUI_VHTML_OUTPUT", '<span class="name">{{ s.name }}</span>', '<span class="name" v-html="s.name"></span>', 1, "panel"),
    ("SCRIPTUI_DIRECT_INVOKE", "<!-- 后端调用统一经 bridge.ts，组件不直接 invoke -->", 'invoke("script_add", p)', 1, "panel"),
    ("SCRIPTUI_PARAM_PATH_CONSTRUCT", "<!-- 前端只传正文，路径由后端按 <id>.<ext> 生成 -->", '<div>{{ `scripts/${s.id}.sh` }}</div>', 1, "panel"),
    ("SCRIPTUI_SECRET_ECHO", "<!-- secret 默认值仅以 password 输入框回填，绝不明文插值显示 -->", '<span class="leak">{{ p.default }}</span>', 1, "paramform"),
    ("SCRIPTUI_NO_CONFIRM_DELETE", "if (!confirm(", "if (false)", 1, "panel"),
    ("SCRIPTUI_NEW_NPM_DEP", '"dependencies": {', '"dependencies": {\n    "left-pad": "^1.3.0",', 1, "package_json"),
    ("SCRIPTUI_MAINVIEW_UNREGISTERED", '  | "scripts"\n', "", 1, "layout"),
    # ---- b 卡职责坏样本（5 个转 active 的码）----
    ("SCRIPTUI_NO_CANCEL_BUTTON", "await bridge.cancelScript(runId.value)", "/* cancel removed */", 1, "rundialog"),
    ("SCRIPTUI_NO_EVENT_UNSUBSCRIBE", "// 在组件卸载时 unlisten 这两个订阅，否则多次运行会泄漏监听器", "", 1, "rundialog"),
    ("SCRIPTUI_OUTPUT_NO_THROTTLE", "rafId = requestAnimationFrame(flushOutput);", "flushOutput();", 1, "rundialog"),
    ("SCRIPTUI_AUDIT_REJECT_MISSING", '"script.validate.reject"', '"script.run.start"', 1, "bridge_rs"),
    ("SCRIPTUI_SANDBOX_ESCAPE", '<pre class="out">{{ output }}</pre>', '<pre class="out" v-html="output"></pre>', 1, "rundialog"),
    # ---- c 卡职责坏样本（运行历史命令）----
    # 模拟"开发者误删 script_runs_list 命令"：把全部 `script_runs_list` 字眼替换为
    # `dummy_history_removed`（bridge.rs 中共 2 处：pub fn 行 + 内部 check_invocation_source
    # 命令名）。detector 要求 `fn script_runs_list` 与 `script_runs_list` 字符串皆不存在，
    # 因此替换后 detect 必然命中。
    (
        "SCRIPTUI_RUNS_LIST_MISSING",
        "script_runs_list",
        "dummy_history_removed",
        2,
        "bridge_rs",
    ),
    # ---- M2-6.d 命令片段库坏样本（fix1 新增扫描范围）----
    # 摘掉 dangerous 前置判定 → 高风险命令可一键直跑
    (
        "SCRIPTUI_CMD_DANGEROUS_NO_CONFIRM",
        "if (m.dangerous && !confirm(",
        "if (!confirm(",
        1,
        "cmdsnippet",
    ),
    # 把「一行一个元素」改成按空格切分 → 重新引入 shell 词法解析
    (
        "SCRIPTUI_CMD_ARGV_SHELL_PARSE",
        ".split(/\\r?\\n/)",
        '.split(" ")',
        1,
        "snippetui",
    ),
    # 从 MainView 枚举里删掉 commands → 模块入口注册残缺
    (
        "SCRIPTUI_CMD_MAINVIEW_UNREGISTERED",
        '  | "commands"\n',
        "",
        1,
        "layout",
    ),
)


def self_test() -> int:
    ctx = read_ctx()
    failures: list[str] = []

    # 1) 好样本：真实源码在默认模式下必须零命中
    good_hits = {c for c in detect(ctx) if c in ACTIVE_CODES}
    if good_hits:
        failures.append(f"好样本本应零命中，实际命中：{sorted(good_hits)}")

    # 2) 坏样本：每个变异必须命中对应码
    for code, old, new, count, key in MUTATIONS:
        # 防呆：变异若未真正改动内容（原文片段不存在），按漏检测计
        if old not in ctx[key]:
            failures.append(f"[{code}] 变异原文片段不存在，夹具已失效（必须同步更新 MUTATIONS）：{old!r}")
            continue
        mutated = dict(ctx)
        mutated[key] = ctx[key].replace(old, new, count)
        if mutated[key] == ctx[key]:
            failures.append(f"[{code}] 变异未改变内容（str.replace 空转），按漏检测计")
            continue
        hits = detect(mutated)
        if code not in hits:
            failures.append(f"[{code}] 坏样本未被检出（实际命中：{sorted(hits & set(ACTIVE_CODES))}）")

    # 3) pending 码位完整性：默认模式不得误判 pending 为 active
    overlap = set(ACTIVE_CODES) & set(PENDING_CODES)
    if overlap:
        failures.append(f"码位重复定义：{sorted(overlap)}")

    if failures:
        print("SELF_TEST_RESULT=FAIL")
        for f in failures:
            print(f"FAIL: {f}")
        return 1
    print(f"SELF_TEST_RESULT=ALL_PASS（{len(MUTATIONS)} 个坏样本 + 1 个好样本 + 码位完整性）")
    return 0


# ----------------------------- 主流程 -----------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description="M2-5 脚本库 UI 安全不变量夹具（a/b/c 卡合并）")
    parser.add_argument("--self-test", action="store_true", help="好坏样本双向自检")
    parser.add_argument(
        "--expect-pending",
        action="store_true",
        help="验证 PENDING_CODES 中码位确实仍未落地（a/b/c 后应为空）",
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    ctx = read_ctx()
    hits = detect(ctx)

    if args.expect_pending:
        pending_hits = [c for c in PENDING_CODES if c in hits]
        if pending_hits:
            print("PENDING_RESULT=FAIL")
            for code in pending_hits:
                print(f"  {code}")
            return 1
        print(f"PENDING_RESULT=NONE（{len(PENDING_CODES)} 个 pending 码位）")
        return 0

    active_hits = sorted(c for c in hits if c in ACTIVE_CODES)
    if active_hits:
        print("SCRIPTUI_POLICY_RESULT=FAIL")
        for code in active_hits:
            print(f"  {code}")
        return 1
    print(f"SCRIPTUI_POLICY_RESULT=PASS（{len(ACTIVE_CODES)} 个默认码位零命中）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
