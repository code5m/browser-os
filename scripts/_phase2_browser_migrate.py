#!/usr/bin/env python3
"""PHASE 2 (Browser create-tab prerequisite) migration helper.
Extracts the Browser-owned create_tab cluster from bridge.rs into
capabilities/browser/commands.rs. Pure text surgery; no behavior change.
Run from repo root: python3 scripts/_phase2_browser_migrate.py
"""
import io, os, re, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BRIDGE = os.path.join(ROOT, "src-tauri", "src", "bridge.rs")
COMMANDS = os.path.join(ROOT, "src-tauri", "src", "capabilities", "browser", "commands.rs")
MODRS = os.path.join(ROOT, "src-tauri", "src", "capabilities", "browser", "mod.rs")

# (marker prefix, include preceding /// doc comments?)
ITEMS = [
    ("pub struct TabInfo", True),
    ("fn normalize_url", True),
    ("fn spawn_child_window", True),
    ("fn hide_bounds", True),
    ("fn remember_layout", True),
    ("fn create_tab", True),
]

def read_lines(path):
    with io.open(path, "r", encoding="utf-8") as f:
        return f.read().split("\n")

def brace_end(lines, start):
    """Return 0-based index of the matching '}' for the block starting at `start` (top-level fn/struct)."""
    depth = 0
    opened = False
    for i in range(start, len(lines)):
        for ch in lines[i]:
            if ch == "{":
                depth += 1
                opened = True
            elif ch == "}":
                depth -= 1
                if opened and depth == 0:
                    return i
    raise RuntimeError("unbalanced braces starting at line %d" % (start + 1))

def block_start(lines, idx):
    """Extend upward to include immediately-preceding /// doc-comment lines."""
    s = idx
    while s - 1 >= 0 and lines[s - 1].lstrip().startswith("///"):
        s -= 1
    return s

def main():
    lines = read_lines(BRIDGE)
    # locate item start indices
    starts = []
    for marker, _ in ITEMS:
        found = None
        for i, ln in enumerate(lines):
            if ln.startswith(marker):
                found = i
                break
        if found is None:
            raise RuntimeError("marker not found: %s" % marker)
        starts.append((found, marker))

    # build extracted texts (in file order) and deletion ranges
    extracted = []  # (order_in_file, text)
    deletions = []  # (start, end) inclusive 0-based
    for s, marker in starts:
        bs = block_start(lines, s)
        be = brace_end(lines, s)
        text = "\n".join(lines[bs:be + 1])
        extracted.append((bs, marker, text))
        deletions.append((bs, be))

    # remove from bridge (reverse order so indices stay valid)
    deletions.sort(reverse=True)
    bridge_out = list(lines)
    for (bs, be) in deletions:
        # drop the block plus one trailing blank line if present
        end = be + 1
        if end < len(bridge_out) and bridge_out[end].strip() == "":
            end += 1
        del bridge_out[bs:end]

    # fix extracted texts for commands.rs
    def fix(text, marker):
        # remove start_resource_scanner call from create_tab
        text = text.replace("    start_resource_scanner(app.clone());\n", "")
        # crate-root AppState path
        text = text.replace("app.state::<AppState>()", "app.state::<crate::AppState>()")
        text = text.replace("state0 = app.state::<AppState>()", "state0 = app.state::<crate::AppState>()")
        # include_str depth: commands.rs is 3 levels below src-tauri
        text = text.replace('include_str!("../injected/collect.js")',
                             'include_str!("../../../injected/collect.js")')
        return text

    ordered = sorted(extracted, key=lambda x: x[0])
    cmd_bodies = []
    for _, marker, text in ordered:
        cmd_bodies.append(fix(text, marker))

    header = (
        "//!\n"
        "//! Browser 能力：页签 / WebView 的创建与基础原语。\n"
        "//! PHASE 2 从 bridge.rs 物理迁移（native-physical-batch-browser-create-tab）。\n"
        "//! 仅做模块归属，不改任何行为；WebView/Grid 生命周期、active tab 权威、\n"
        "//! session persistence 派生（从 Browser 权威 tabs 表）等冻结语义均不变。\n"
        "use tauri::{AppHandle, Manager};\n"
        "use url::Url;\n"
        "use crate::domain::*;\n"
        "\n"
    )
    commands_content = header + "\n".join(cmd_bodies) + "\n"

    os.makedirs(os.path.dirname(COMMANDS), exist_ok=True)
    with io.open(COMMANDS, "w", encoding="utf-8") as f:
        f.write(commands_content)
    mod_content = (
        "//!\n"
        "//! Browser 能力根模块：拥有页签 / WebView 创建与基础原语。\n"
        "//! 详见 commands.rs。\n"
        "pub mod commands;\n"
    )
    with io.open(MODRS, "w", encoding="utf-8") as f:
        f.write(mod_content)

    # write back bridge.rs without the extracted blocks
    with io.open(BRIDGE, "w", encoding="utf-8") as f:
        f.write("\n".join(bridge_out))

    print("extracted %d items into commands.rs" % len(cmd_bodies))
    for _, marker, _ in ordered:
        print("  -", marker)

if __name__ == "__main__":
    main()
