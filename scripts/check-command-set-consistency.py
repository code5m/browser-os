#!/usr/bin/env python3
"""
Command-set three-way consistency guard (BUG-HUNT A9, hardened in Phase 0).

目标：捕获「Rust 命令注册 ∩ ACL 权限 ∩ 前端 invoke」三源漂移，
避免「新增命令漏加 ACL / 前端 invoke 未注册」这类只在运行时偶然暴露的问题。

三源：
  A        = Rust 注册的命令
             （src-tauri/src/main.rs 中全部 invoke_handler(generate_handler![...]) 块，
               含主窗口、宫格子窗口、图谱/插件等所有 generate_handler! 注册块）
  B_main   = src-tauri/permissions/default-commands.toml 的 commands.allow（主窗口 ACL）
  B_remote = src-tauri/permissions/remote-collect.toml 的 commands.allow（远程 webview ACL）
  C_main   = src/bridge.ts 中 invoke("...") / invoke<Type>("...") 调用的命令（主窗口封装）
  C_remote = src-tauri/injected/collect.js 中 invoke("...") 调用的命令（注入到所有页面，含远程）

Phase 0 修复的两个真实盲区（已验证）：
  1) registered_commands 原先只取第一个 generate_handler! 块，漏掉第 2/3 个块
     （宫格子窗口、图谱/插件命令），导致 A 严重少算、B_main - A 出现大量假阳性
     "stale ACL"。现改为捕获全部 generate_handler! 块。
  2) invoked 正则只匹配 invoke("...")，漏掉 bridge.ts 大量使用的
     invoke<Type>("...") 泛型形态（如 invoke<Artifact>("collect_selection")），
     导致 C_main 少算、契约分类失真。现同时匹配泛型形态。

检查：
  A - B_main          ：已注册但主窗口 ACL 未放行 → 主窗口也会 'not allowed'
  B_main - A          ：ACL 放行但 Rust 未注册 → 死 ACL（应清理）
  C_main - A          ：前端封装了 invoke 但后端未注册 → 死调用（契约占位除外）
  C_remote - B_remote ：远程 webview 调用了远程 ACL 未放行的命令

行为：
  默认（gate 模式）：仅当存在「非已知漂移」的不一致时以非零退出（CI 门禁）。
  --report           ：仅打印完整报告，始终退出 0（人工审计用）。
  --self-test        ：用内置 fixture 验证检查器自身逻辑，不读真实仓库。
  --json             ：机器可读输出。
  --help             ：用法。
不修改任何产品代码；仅做静态文本比对。
"""
import os
import re
import sys
import json

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

MAIN_RS = os.path.join(REPO, "src-tauri", "src", "main.rs")
BRIDGE_TS = os.path.join(REPO, "src", "bridge.ts")
COLLECT_JS = os.path.join(REPO, "src-tauri", "injected", "collect.js")
DEFAULT_TOML = os.path.join(REPO, "src-tauri", "permissions", "default-commands.toml")
REMOTE_TOML = os.path.join(REPO, "src-tauri", "permissions", "remote-collect.toml")

# 已确认的、当前有效且非本 lane 修复范围的漂移。
# 每条必须附证据（frontend caller / Rust implementation / handler registration / ACL），
# 死代码/规划命令不得因为文档写了就加入。
#
# Agent/Skill 运行时契约占位：以下命令在 bridge.ts 中已被 invoke 封装，但 Rust 端
# （src-tauri/src）尚未实现对应 #[tauri::command]，属已知占位漂移，不计入「新增漂移」。
# 证据（bridge.ts 行号）：
#   agent_chat/agent_chat_cancel        ~436/~438
#   confirm_agent_install/confirm_skill_install  ~428/~425
#   agent_install/agent_list/agent_runs_list     ~421/~403/~463
#   skill_install/skill_list/skill_run/skill_runs_list  ~418/~401/~432/~462
# 经 Phase 0 重新验证：以上均为真实 invoke 调用（含泛型形态），Rust 端无对应 fn，
# 属同一占位家族；不视为死代码（死代码不会出现在 C_main 真实调用集中）。
KNOWN = {
    "registered_not_in_default_acl": set(),
    "main_invoke_not_registered": {
        "agent_chat", "agent_chat_cancel",
        "confirm_agent_install", "confirm_skill_install",
        "agent_install", "agent_list", "agent_runs_list",
        "skill_install", "skill_list", "skill_run", "skill_runs_list",
    },
    # collect.js 仍调用这 3 条命令，但 remote-collect.toml 已将它们在 M0-3.b 移除
    # （写盘/开终端副作用命令不向远程开放）。属已知远程漂移。
    "remote_invoke_not_in_remote_acl": {
        "collect_selection", "request_open_terminal", "save_note",
    },
}


def _read(path):
    with open(path, "r", encoding="utf-8") as f:
        return f.read()


def registered_commands(src=None):
    """应用层实际注册的命令名集合（全部 generate_handler! 块）。"""
    if src is None:
        src = _read(MAIN_RS)
    names = set()
    # 捕获每一个 generate_handler![ ... ] 块（不止第一个）。
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


def invoked(path=None, text=None):
    """从前端源码提取所有 invoke 的命令名。

    同时识别两种形态：
      invoke("cmd", ...)           普通
      invoke<Type>("cmd", ...)    泛型（bridge.ts 大量使用）
    为降低误报，先剥离 // 行注释（静态启发式）。
    """
    if text is None:
        text = _read(path)
    text = re.sub(r"//[^\n]*", "", text)  # 剥离单行注释
    plain = set(re.findall(r'invoke\(\s*["\']([^"\']+)["\']', text))
    typed = set(re.findall(r'invoke<[^>]*>\s*\(\s*["\']([^"\']+)["\']', text))
    return plain | typed


def compute_diffs(A, B_main, B_remote, C_main, C_remote):
    return {
        "registered_not_in_default_acl": (A - B_main, KNOWN["registered_not_in_default_acl"]),
        "stale_acl_in_default": (B_main - A, set()),
        "main_invoke_not_registered": (C_main - A, KNOWN["main_invoke_not_registered"]),
        "remote_invoke_not_in_remote_acl": (C_remote - B_remote, KNOWN["remote_invoke_not_in_remote_acl"]),
    }


def classify(A, B_main, B_remote, C_main, C_remote):
    """契约分类：CONSISTENT / DRIFT_RISK / BROKEN / UNREGISTERED / ACL_MISSING /
    FRONTEND_ONLY / BACKEND_ONLY。"""
    out = []
    for cmd in sorted(A & B_main & C_main):
        out.append((cmd, "CONSISTENT", "registered + default-ACL + main-invoke 三源齐备"))
    for cmd in sorted((A & B_main) - C_main):
        out.append((cmd, "BACKEND_ONLY", "已注册+ACL 放行，但 bridge.ts 未封装（可能由其它前端路径或远程调用）"))
    for cmd in sorted((A & C_main) - B_main):
        out.append((cmd, "ACL_MISSING", "已注册+主窗口封装，但 default-ACL 未放行 → 主窗口调用会 not allowed"))
    for cmd in sorted((B_main & C_main) - A):
        out.append((cmd, "BACKEND_ONLY", "ACL+前端封装但 Rust 未注册（疑为插件/动态命令或死 ACL）"))
    for cmd in sorted(C_main - A - B_main):
        out.append((cmd, "UNREGISTERED", "前端封装但 Rust 未注册且 ACL 未放行（死调用 / 契约占位）"))
    for cmd in sorted((B_main - A) - C_main):
        out.append((cmd, "BROKEN", "ACL 放行但 Rust 未注册且无前端封装 → 死 ACL"))
    for cmd in sorted(C_remote - B_remote):
        out.append((cmd, "DRIFT_RISK", "远程 webview 调用了远程 ACL 未放行的命令"))
    return out


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    report_mode = "--report" in argv
    self_test = "--self-test" in argv
    json_mode = "--json" in argv
    strict = "--strict" in argv
    help_mode = "--help" in argv or "-h" in argv
    known = {"--report", "--self-test", "--json", "--strict", "--help", "-h"}
    unknown = [a for a in argv if a not in known]

    if unknown:
        sys.stderr.write("未知参数: %s\n" % " ".join(unknown))
        sys.stderr.write("使用 --help 查看可用参数\n")
        return 2
    if help_mode:
        print(__doc__)
        return 0
    if self_test:
        return run_self_test()

    A = registered_commands()
    B_main = toml_allows(DEFAULT_TOML)
    B_remote = toml_allows(REMOTE_TOML)
    C_main = invoked(BRIDGE_TS)
    C_remote = invoked(COLLECT_JS)

    diffs = compute_diffs(A, B_main, B_remote, C_main, C_remote)

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
        "stale_acl_in_default": "stale ACL (ACL allows but Rust not registered) — should be cleaned up",
        "main_invoke_not_registered": "KNOWN DRIFT (agent/skill contract placeholders, see KNOWN)",
        "remote_invoke_not_in_remote_acl": "KNOWN DRIFT (collect.js 3 cmds, removed from remote ACL in M0-3.b)",
    }

    new_drift = []
    rows = []
    for key, (items, allowed) in diffs.items():
        extra = sorted(items - allowed)
        known_hits = sorted(items & allowed)
        rows.append((key, labels[key], extra, known_hits, known_comments[key]))
        new_drift.extend(extra)

    if json_mode:
        payload = {
            "check": "command-set-consistency",
            "counts": {
                "registered": len(A), "default_acl": len(B_main),
                "remote_acl": len(B_remote), "main_invoke": len(C_main),
                "remote_invoke": len(C_remote),
            },
            "new_drift": new_drift,
            "diffs": {k: {"items": sorted(items), "known": sorted(allowed)}
                      for k, (items, allowed) in diffs.items()},
            "status": "FAIL" if new_drift else "PASS",
        }
        print(json.dumps(payload, indent=2, ensure_ascii=False))
        return 0 if report_mode else (1 if new_drift else 0)

    print("=== Command-set three-way consistency (Phase 0 hardened) ===")
    print("Registered (A, all generate_handler! blocks): %d | Default ACL (B_main): %d | Remote ACL (B_remote): %d"
          % (len(A), len(B_main), len(B_remote)))
    print("Main invocations (C_main/bridge.ts): %d | Remote invocations (C_remote/collect.js): %d"
          % (len(C_main), len(C_remote)))
    print("")

    for key, label, extra, known_hits, kc in rows:
        print("[%s] %s" % (key, label))
        if extra:
            print("    NEW DRIFT: " + ", ".join(extra))
        if known_hits:
            print("    known: " + ", ".join(known_hits) + "  (" + kc + ")")
        if not extra and not known_hits:
            print("    (none)")
        print("")

    # 契约分类（仅在报告模式或显式需要时打印，避免 gate 噪声）
    if report_mode or "--classify" in argv:
        print("=== Contract classification ===")
        for cmd, cls, note in classify(A, B_main, B_remote, C_main, C_remote):
            print("  %-28s %-16s %s" % (cmd, cls, note))
        print("")

    if new_drift:
        print("GATE: FAIL — %d new inconsistency(ies) not in known-drift allowlist:" % len(new_drift))
        for x in new_drift:
            print("  - " + x)
        return 0 if report_mode else 1

    print("GATE: PASS — only known drift present (no new inconsistency).")
    return 0


# ===== 自测：用内置 fixture 验证检查器自身逻辑 =====
def run_self_test():
    failures = []

    # fixture 1: 多块 generate_handler! 必须全部捕获
    main_fixture = """
        .invoke_handler(tauri::generate_handler![
            bridge::open_browser,
            bridge::close_browser,
        ])
        .invoke_handler(tauri::generate_handler![
            bridge::graph_query,
            bridge::graph_node_get,
        ])
        .invoke_handler(tauri::generate_handler![
            tools::open_tool,
        ])
    """
    A = registered_commands(main_fixture)
    if not {"open_browser", "close_browser", "graph_query", "graph_node_get", "open_tool"}.issubset(A):
        failures.append("registered_commands 未捕获全部 generate_handler! 块: %s" % sorted(A))

    # fixture 2: 普通 + 泛型 + 多行 + 注释 的 invoke 提取
    bridge_fixture = """
        // 这是注释 invoke("should_not_count")
        openBrowser: () => invoke("open_browser"),
        listArtifacts: () => invoke<Artifact[]>("list_artifacts"),
        saveNote: (p) => invoke<string>(
          "save_note",
          p
        ),
    """
    C = invoked(text=bridge_fixture)
    if "open_browser" not in C:
        failures.append("invoked 未识别普通 invoke(\"open_browser\")")
    if "list_artifacts" not in C:
        failures.append("invoked 未识别泛型 invoke<Artifact[]>(\"list_artifacts\")")
    if "save_note" not in C:
        failures.append("invoked 未识别多行 invoke(\"save_note\")")
    if "should_not_count" in C:
        failures.append("invoked 误把 // 注释中的 invoke 计入（false positive）")

    # fixture 3: 负面用例——未注册命令
    B_main = {"open_browser", "close_browser"}
    B_remote = {"report_resources"}
    C_main = {"open_browser", "list_artifacts"}     # list_artifacts 未注册
    C_remote = {"collect_selection"}                # 远程未放行
    diffs = compute_diffs(A, B_main, B_remote, C_main, C_remote)

    if "list_artifacts" not in diffs["main_invoke_not_registered"][0]:
        failures.append("diff 未检出 C_main - A（未注册命令）")
    if "collect_selection" not in diffs["remote_invoke_not_in_remote_acl"][0]:
        failures.append("diff 未检出 C_remote - B_remote（远程未放行）")
    if "open_browser" in diffs["stale_acl_in_default"][0]:
        failures.append("diff 误报 B_main - A（open_browser 实际已注册）")
    if "open_tool" in diffs["stale_acl_in_default"][0]:
        failures.append("diff 误报 B_main - A（open_tool 实际已注册，第二块）")

    # fixture 4: KNOWN 漂移应被 suppress，不计入 new_drift
    A2 = {"open_browser"}
    B_main2 = {"open_browser"}
    B_remote2 = {"report_resources"}
    C_main2 = {"open_browser", "agent_chat"}   # agent_chat 在 KNOWN
    C_remote2 = {"collect_selection"}          # 在 KNOWN
    diffs2 = compute_diffs(A2, B_main2, B_remote2, C_main2, C_remote2)
    if diffs2["main_invoke_not_registered"][0] - KNOWN["main_invoke_not_registered"]:
        failures.append("KNOWN main_invoke_not_registered 漂移未被抑制")
    if diffs2["remote_invoke_not_in_remote_acl"][0] - KNOWN["remote_invoke_not_in_remote_acl"]:
        failures.append("KNOWN remote drift 未被抑制")

    # fixture 5: 真实 new drift 必须导致 FAIL（不被抑制）
    A3 = {"open_browser"}
    B_main3 = {"open_browser"}
    C_main3 = {"open_browser", "really_new_cmd"}   # 真正的新漂移，不在 KNOWN
    diffs3 = compute_diffs(A3, B_main3, set(), C_main3, set())
    if not (diffs3["main_invoke_not_registered"][0] - KNOWN["main_invoke_not_registered"]):
        failures.append("真实 new drift 未被检出（false negative）")

    if failures:
        print("COMMAND_SET_SELF_TEST: FAIL")
        for f in failures:
            print("  - " + f)
        return 1
    print("COMMAND_SET_SELF_TEST: ALL_PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
