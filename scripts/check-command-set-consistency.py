#!/usr/bin/env python3
"""
Command-set three-way consistency guard (BUG-HUNT A9).

目标：捕获「Rust 命令注册 ∩ ACL 权限 ∩ 前端 invoke」三源漂移（BUG-HUNT B12-02），
避免「新增命令漏加 ACL / 前端 invoke 未注册」这类只在运行时偶然暴露的问题。

三源：
  A        = Rust 注册的命令
             （src-tauri/src/main.rs 的 generate_handler![...]，
              并联合 src-tauri 全仓 / 插件 crate 的 `#[tauri::command] fn <name>`）
  B_main   = src-tauri/permissions/default-commands.toml 的 commands.allow（主窗口 ACL）
  B_remote = src-tauri/permissions/remote-collect.toml 的 commands.allow（远程 webview ACL）
  C_main   = src/bridge.ts 中 invoke("...") 调用的命令（主窗口封装）
  C_remote = src-tauri/injected/collect.js 中 invoke("...") 调用的命令（注入到所有页面，含远程）

检查：
  A - B_main          ：已注册但主窗口 ACL 未放行 → 主窗口也会 'not allowed'（如 open_tool）
  B_main - A          ：ACL 放行但 Rust 未注册 → 死 ACL（应清理）
  C_main - A          ：前端封装了 invoke 但后端未注册 → 死调用（契约占位除外）
  C_remote - B_remote ：远程 webview 调用了远程 ACL 未放行的命令（如 collect.js 三命令）

行为：
  默认（gate 模式）：仅当存在「非已知漂移」的不一致时以非零退出（CI 门禁）。
  --report           ：仅打印完整报告，始终退出 0（人工审计用）。
不修改任何产品代码；仅做静态文本比对。
"""
import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

MAIN_RS = os.path.join(REPO, "src-tauri", "src", "main.rs")
BRIDGE_TS = os.path.join(REPO, "src", "bridge.ts")
COLLECT_JS = os.path.join(REPO, "src-tauri", "injected", "collect.js")
DEFAULT_TOML = os.path.join(REPO, "src-tauri", "permissions", "default-commands.toml")
REMOTE_TOML = os.path.join(REPO, "src-tauri", "permissions", "remote-collect.toml")

# BUG-HUNT-SUMMARY.md 已记录的、当前已知且非本 lane 修复范围的漂移。
# 本门禁只防「新增」漂移，不阻止已声明的契约占位。
KNOWN = {
    "registered_not_in_default_acl": set(),
    "main_invoke_not_registered": {
        "agent_chat", "agent_chat_cancel",
        "confirm_agent_install", "confirm_skill_install",
    },
    "remote_invoke_not_in_remote_acl": {
        "collect_selection", "request_open_terminal", "save_note",
    },
}


def _read(path):
    with open(path, "r", encoding="utf-8") as f:
        return f.read()


def registered_commands():
    """应用层（主窗口）实际注册的命令名集合。

    仅取 src-tauri/src/main.rs 的 invoke_handler(generate_handler![...])，
    因为它正是 default-commands.toml 所管辖的命令范围。插件 crate（如
    tauri-plugin-browser-tabs）的命令由插件自身权限文件管辖，不在本门禁三源之内。
    """
    names = set()
    src = _read(MAIN_RS)
    for blk in re.finditer(r"generate_handler!\s*\[(.*?)\]", src, re.S):
        for m in re.finditer(r"(\w+)::(\w+)", blk.group(1)):
            names.add(m.group(2))
    return names


def toml_allows(path):
    txt = _read(path)
    m = re.search(r"commands\.allow\s*=\s*\[(.*?)\]", txt, re.S)
    if not m:
        return set()
    return set(re.findall(r'"([^"]+)"', m.group(1)))


def invoked(path):
    return set(re.findall(r'invoke\(\s*["\']([^"\']+)["\']', _read(path)))


def main():
    A = registered_commands()
    B_main = toml_allows(DEFAULT_TOML)
    B_remote = toml_allows(REMOTE_TOML)
    C_main = invoked(BRIDGE_TS)
    C_remote = invoked(COLLECT_JS)

    diffs = {
        "registered_not_in_default_acl": (A - B_main, KNOWN["registered_not_in_default_acl"]),
        "stale_acl_in_default": (B_main - A, set()),
        "main_invoke_not_registered": (C_main - A, KNOWN["main_invoke_not_registered"]),
        "remote_invoke_not_in_remote_acl": (C_remote - B_remote, KNOWN["remote_invoke_not_in_remote_acl"]),
    }

    report_mode = "--report" in sys.argv[1:]

    print("=== Command-set three-way consistency (BUG-HUNT A9) ===")
    print("Registered (A): %d | Default ACL (B_main): %d | Remote ACL (B_remote): %d"
          % (len(A), len(B_main), len(B_remote)))
    print("Main invocations (C_main/bridge.ts): %d | Remote invocations (C_remote/collect.js): %d"
          % (len(C_main), len(C_remote)))
    print("")

    labels = {
        "registered_not_in_default_acl":
            "A - B_main  (registered, not allowed by default ACL → 'not allowed' from main window)",
        "stale_acl_in_default":
            "B_main - A  (ACL allows but Rust not registered → stale ACL)",
        "main_invoke_not_registered":
            "C_main - A  (bridged invoke with no backend registration → dead call)",
        "remote_invoke_not_in_remote_acl":
            "C_remote - B_remote  (remote webview invoke not in remote ACL)",
    }
    known_comments = {
        "registered_not_in_default_acl": "",
        "main_invoke_not_registered": "KNOWN DRIFT (agent/skill contract placeholders)",
        "remote_invoke_not_in_remote_acl": "KNOWN DRIFT (collect.js 3 cmds, B1-2/B9-3/B12-03)",
    }

    new_drift = []
    for key, (items, allowed) in diffs.items():
        extra = sorted(items - allowed)
        known = sorted(items & allowed)
        print("[%s] %s" % (key, labels[key]))
        if extra:
            print("    NEW DRIFT: " + ", ".join(extra))
            new_drift.extend(extra)
        if known:
            print("    known: " + ", ".join(known) + "  (" + known_comments[key] + ")")
        if not items:
            print("    (none)")
        print("")

    if new_drift:
        print("GATE: FAIL — %d new inconsistency(ies) not in known-drift allowlist:" % len(new_drift))
        for x in new_drift:
            print("  - " + x)
        return 0 if report_mode else 1

    print("GATE: PASS — only known drift present (no new inconsistency).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
