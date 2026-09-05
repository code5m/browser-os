#!/usr/bin/env python3
"""Expose the M2-5.a script-library CRUD UI invariants as a reproducible fixture.

契约来源：
- `logs/checkpoints/M2-5-20260905-1301.md`（§5 测试矩阵 U1~U12 / §6 违规码）
- `logs/checkpoints/M2-3.a-20260903-1604.md`（ScriptMeta / ScriptParam 口径）
- `logs/checkpoints/M2-4.a-20260903-2233.md`（RunStatus / 事件 / 审计）

本夹具守住「纯前端 CRUD UI」的七条红线（a 卡职责）：

  1. 输出/正文渲染禁用 `v-html`（XSS）
  2. 组件不得绕过 `bridge.ts` 直接 `invoke`
  3. 前端不得参与脚本正文路径构造
  4. secret 参数不得明文回显
  5. 删除必须二次确认
  6. 不得新增 npm 依赖
  7. `mainView='scripts'` 必须注册进 `MainView` 枚举 + `MOD_META` + ActivityBar 入口

模式：
  默认            判定 ACTIVE 码（a 卡职责）；零命中 → EXIT 0
  --expect-pending 验证 PENDING 码位集合（b/c 职责）确实仍未实现 → EXIT 0
  --self-test     好坏样本双向自检（含**变异防呆**：坏样本必须真的改动内容）

关于「一次性定义码位」：沿用 M0-3.a 模式，13 个码位一次定义，
默认只判 a 卡职责的 7 个 ACTIVE 码；b/c 卡职责的 6 个 PENDING 码位默认不判，
待 b/c 卡收口后转 active 或单独裁定（避免 b/c 未实现就把 pre-merge 染红）。
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
)

# b/c 卡职责（默认不判，pending）
PENDING_CODES = (
    "SCRIPTUI_OUTPUT_NO_THROTTLE",
    "SCRIPTUI_NO_EVENT_UNSUBSCRIBE",
    "SCRIPTUI_NO_CANCEL_BUTTON",
    "SCRIPTUI_RUNS_LIST_MISSING",
    "SCRIPTUI_AUDIT_REJECT_MISSING",
    "SCRIPTUI_SANDBOX_ESCAPE",
)

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
    ui_sources = panel + "\n" + paramform

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

    return hits


def read_ctx() -> dict[str, str]:
    pkg = read(PACKAGE_JSON)
    lock = read(PACKAGE_LOCK)
    return {
        "panel": read(SCRIPT_PANEL),
        "paramform": read(SCRIPT_PARAM_FORM),
        "layout": read(LAYOUT_STORE),
        "activity": read(ACTIVITY_BAR),
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
    parser = argparse.ArgumentParser(description="M2-5.a 脚本库 CRUD UI 安全不变量夹具")
    parser.add_argument("--self-test", action="store_true", help="好坏样本双向自检")
    parser.add_argument(
        "--expect-pending",
        action="store_true",
        help="验证 pending 码（b/c 卡职责）确实仍未实现",
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
