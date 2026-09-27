#!/usr/bin/env python3
"""PHASE 3 (Session command) migration helper.
Extracts the Session-owned command cluster from bridge.rs into
capabilities/session/commands.rs. Pure text surgery; no behavior change.
Run from repo root: python3 scripts/_phase3_session_migrate.py
"""
import io, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BRIDGE = os.path.join(ROOT, "src-tauri", "src", "bridge.rs")
COMMANDS = os.path.join(ROOT, "src-tauri", "src", "capabilities", "session", "commands.rs")
MODRS = os.path.join(ROOT, "src-tauri", "src", "capabilities", "session", "mod.rs")

# Session-owned functions to extract (in file order they are contiguous).
ITEMS = [
    "fn drop_session_draft",
    "fn build_session_for_tab",
    "fn persist_session",
    "pub fn session_save",
    "pub fn session_discard",
    "pub fn session_list",
    "pub fn session_get",
    "pub fn session_delete",
    "pub fn session_export",
    "pub fn session_restore",
    "pub fn flush_sessions",
    "pub fn flush_sessions_inner",
    "pub fn get_session_policy",
    "pub fn set_session_policy",
]

# string/comment-aware brace matching (so format!("{...}") does not desync)
def brace_end(lines, start):
    depth = 0
    opened = False
    i = start
    while i < len(lines):
        line = lines[i]
        j = 0
        in_str = False
        in_char = False
        in_lc = False
        while j < len(line):
            c = line[j]
            nxt = line[j + 1] if j + 1 < len(line) else ""
            if in_lc:
                j += 1
                continue
            if in_str:
                if c == "\\":
                    j += 2
                    continue
                if c == '"':
                    in_str = False
                j += 1
                continue
            if in_char:
                if c == "\\":
                    j += 2
                    continue
                if c == "'":
                    in_char = False
                j += 1
                continue
            if c == "/" and nxt == "/":
                in_lc = True
                j += 2
                continue
            if c == '"':
                in_str = True
                j += 1
                continue
            if c == "'":
                in_char = True
                j += 1
                continue
            if c == "{":
                depth += 1
                opened = True
            elif c == "}":
                depth -= 1
                if opened and depth == 0:
                    return i
            j += 1
        i += 1
    raise RuntimeError("unbalanced braces starting at line %d" % (start + 1))

def block_start(lines, idx):
    """Extend upward over /// doc lines, #[ attributes, and blank lines."""
    s = idx
    while s - 1 >= 0:
        t = lines[s - 1].lstrip()
        if t.startswith("///") or t.startswith("#[") or t == "":
            s -= 1
        else:
            break
    return s

def main():
    lines = io.open(BRIDGE, "r", encoding="utf-8").read().split("\n")
    starts = []
    for marker in ITEMS:
        found = None
        for i, ln in enumerate(lines):
            if ln.startswith(marker):
                found = i
                break
        if found is None:
            raise RuntimeError("marker not found: %s" % marker)
        starts.append(found)

    extracted = []
    first_bs = None
    last_be = None
    for s in starts:
        bs = block_start(lines, s)
        be = brace_end(lines, s)
        text = "\n".join(lines[bs:be + 1])
        extracted.append((bs, text))
        first_bs = bs if first_bs is None else min(first_bs, bs)
        last_be = be if last_be is None else max(last_be, be)

    # find the M1-9 '// ---' section separator immediately preceding the cluster
    sep_start = first_bs
    while sep_start - 1 >= 0 and not re.match(r"^// -{3,}", lines[sep_start - 1]):
        sep_start -= 1

    # delete [sep_start, last_be + 1) from bridge
    bridge_out = list(lines)
    end = last_be + 1
    if end < len(bridge_out) and bridge_out[end].strip() == "":
        end += 1
    del bridge_out[sep_start:end]

    # fix extracted texts for commands.rs
    def fix(text):
        # crate-root AppState path (unambiguous, no dependency on domain re-export)
        text = text.replace("<AppState>", "<crate::AppState>")
        return text

    ordered = sorted(extracted, key=lambda x: x[0])
    cmd_bodies = [fix(t) for _, t in ordered]

    header = (
        "//!\n"
        "//! Session 能力：会话快照的持久化、列举、恢复与关闭策略。\n"
        "//! PHASE 3 从 bridge.rs 物理迁移（native-physical-batch-session）。\n"
        "//! 仅做模块归属，不改任何冻结语义：\n"
        "//!   - `build_session` 始终对 URL 脱敏、预览 512B 截断（见 crate::session）。\n"
        "//!   - `SessionDraft` 仅内存、绝不自动落盘（字段保留于 AppState）。\n"
        "//!   - `flush_sessions` 命令与 ShutdownCoordinator 任务共用 `flush_sessions_inner`，\n"
        "//!     两路径行为一致、可审计。\n"
        "//!   - Session→Browser 仅经窄契约 `crate::capabilities::browser::commands::create_tab`\n"
        "//!     （session_restore 用已脱敏 URL 建 tab），绝不反向依赖 bridge。\n"
        "use std::sync::atomic::Ordering;\n"
        "use tauri::{AppHandle, Manager, Webview};\n"
        "use crate::domain::*;\n"
        "use crate::session;\n"
        "use crate::workspace;\n"
        "use crate::images::check_id;\n"
        "use crate::shared::invocation::{check_invocation_source, check_tab_id};\n"
        "use crate::capabilities::browser::commands::create_tab;\n"
        "\n"
    )
    commands_content = header + "\n".join(cmd_bodies) + "\n"

    os.makedirs(os.path.dirname(COMMANDS), exist_ok=True)
    io.open(COMMANDS, "w", encoding="utf-8").write(commands_content)
    mod_content = (
        "//!\n"
        "//! Session 能力根模块：拥有会话快照的持久化 / 列举 / 恢复 / 关闭策略。\n"
        "//! 详见 commands.rs。\n"
        "pub mod commands;\n"
    )
    io.open(MODRS, "w", encoding="utf-8").write(mod_content)

    io.open(BRIDGE, "w", encoding="utf-8").write("\n".join(bridge_out))

    print("extracted %d session items into commands.rs" % len(cmd_bodies))
    for marker in ITEMS:
        print("  -", marker)

if __name__ == "__main__":
    main()
