#!/usr/bin/env python3
"""Expose the M2-9 seed-tool acceptance invariants as a reproducible fixture.

契约来源：logs/checkpoints/M2-9-20260905-1950.md（a 卡冻结 F1~F9）。

M2-9 是 M2-7/M2-8 的**验收收口**：五个内置种子工具（json / base64 / timestamp /
regex / cron）须满足「可枚举 / 离线可开 / 零外链 / 零 bridge 写原语 / 单数真源 /
零能力隔离 / 打开负向 / 路径防御 / 错误页零外链」。

两层验收并进：
  - Rust `#[cfg(test)]` 单测直验 `builtin_tool_html(id)` 嵌入字节（零外链/零 invoke）。
  - 本夹具在门禁层复验种子文件 + capability 零能力隔离 + 结构复验（与 Rust 单测同口径）。

默认模式：全部不变量成立 → EXIT 0；任一被破坏 → 打印违规码并 EXIT 1。
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

SEED_HTML = (
    "json-tool.html",
    "base64-tool.html",
    "timestamp-tool.html",
    "regex-tool.html",
    "cron-tool.html",
)


def strip_comments(source: str) -> str:
    without_blocks = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
    return "\n".join(line.split("//", 1)[0] for line in without_blocks.splitlines())


def rust_fn_body(source: str, name: str) -> str:
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    depth = 0
    index = match.end() - 1
    while index < len(source):
        if source[index] == "(":
            depth += 1
        elif source[index] == ")":
            depth -= 1
            if depth == 0:
                break
        index += 1
    start = source.find("{", index)
    if start < 0:
        return ""
    depth = 0
    for i in range(start, len(source)):
        if source[i] == "{":
            depth += 1
        elif source[i] == "}":
            depth -= 1
            if depth == 0:
                return source[start : i + 1]
    return source[start:]


def has_invoke(html: str) -> bool:
    return (
        "invoke(" in html
        or "__TAURI__" in html
        or "@tauri-apps" in html
        or "__TAURI_INVOKE__" in html
        or "window.__TAURI__" in html
    )


def external_asset(html: str) -> bool:
    l = html.lower()
    return (
        '<script src="http' in l
        or ("<link" in l and 'href="http' in l)
        or ("@import" in l and "url(http" in l)
    )


def detect_violations(files: dict) -> list[str]:
    v: list[str] = []
    tools = files.get("tools_rs", "")
    seeds: dict = files.get("seeds", {})
    caps = files.get("caps", "")

    # ---- F1：5 个种子文件齐全 + BUILTIN_TOOLS 覆盖 + 嵌入臂一一对应 ----
    for name in SEED_HTML:
        if name not in seeds:
            v.append(f"SEED_MISSING:种子文件缺失 {name}")
    m = re.search(r"BUILTIN_TOOLS[^\n]*= &\[(.*?)\];", tools, re.S)
    if not m:
        v.append("SEED_TABLE_MISSING:tools.rs 缺 BUILTIN_TOOLS")
    else:
        table = m.group(1)
        for name in SEED_HTML:
            sid = name.replace("-tool.html", "")
            if f'"{sid}"' not in table:
                v.append(f"SEED_TABLE_MISMATCH:BUILTIN_TOOLS 缺 {sid}")
            if f'"{sid}" => Some(include_str!("{name}"))' not in tools:
                v.append(f"SEED_EMBED_MISMATCH:builtin_tool_html 缺 {name} 嵌入臂")

    # ---- F3/F4：每份种子零外链、零 bridge 写原语 ----
    for name, content in seeds.items():
        if "http://" in content or "https://" in content:
            v.append(f"SEED_EXTERNAL_LINK:{name} 含外链 http(s)://")
        if external_asset(content):
            v.append(f"SEED_EXTERNAL_ASSET:{name} 含外部 script/link/@import 外链")
        if has_invoke(content):
            v.append(f"SEED_INVOKE_PRESENT:{name} 含 bridge 写原语")

    # ---- F6：隔离——工具窗口（label tool-*）不得出现在任何 capability ----
    if "tool-" in caps:
        v.append("SEED_CAPABILITY_LEAK:capabilities 出现 tool-* 窗口（工具窗口须零能力隔离）")

    # ---- F7：open_tool 拒非法 id（结构复验）----
    if "非法工具 id" not in tools:
        v.append("OPEN_ID_CHECK_MISSING:open_tool 缺少非法 id 拒绝分支")

    # ---- F8：路径越权防御（结构复验）----
    if "validate_user_tool_path" not in tools or "canonicalize" not in tools or "starts_with" not in tools:
        v.append("PATH_DEFENSE_MISSING:tools.rs 缺 validate_user_tool_path/canonicalize/starts_with")

    # ---- F9：错误页零外链（结构复验）----
    ep = rust_fn_body(tools, "error_page")
    if "http://" in ep or "https://" in ep:
        v.append("ERROR_PAGE_EXTERNAL:error_page 含外链")

    return v


def read_repo(root: Path) -> dict:
    tools_rs = root / "src-tauri/src/capabilities/tools/tools.rs"
    out = {
        "tools_rs": tools_rs.read_text(encoding="utf-8") if tools_rs.exists() else "",
        "seeds": {},
    }
    seeds_dir = root / "src-tauri/src/capabilities/tools"
    if seeds_dir.is_dir():
        for f in sorted(seeds_dir.iterdir()):
            if f.suffix == ".html":
                try:
                    out["seeds"][f.name] = f.read_text(encoding="utf-8")
                except OSError:
                    pass
    caps_text = ""
    caps_dir = root / "src-tauri/capabilities"
    if caps_dir.is_dir():
        for f in sorted(caps_dir.iterdir()):
            if f.suffix == ".json":
                try:
                    caps_text += f.read_text(encoding="utf-8") + "\n"
                except OSError:
                    pass
    out["caps"] = caps_text
    return out


def run_self_test(root: Path) -> int:
    good = read_repo(root)
    if detect_violations(good):
        print("self-test FAIL: 当前仓库自身存在违规（应为空）")
        for x in detect_violations(good):
            print(f"  x {x}")
        return 1

    samples: list[tuple[str, dict, str]] = []

    def mutate(**kw) -> dict:
        d = {k: v for k, v in good.items()}
        for k, val in kw.items():
            d[k] = val
        return d

    def add(desc: str, mutated: dict, key_before: str, expect_code: str) -> None:
        before = good.get(key_before)
        after = mutated.get(key_before)
        if after == before:
            print(f"self-test FAIL: 坏样本「{desc}」未改动任何内容（变异失配，按漏检计）")
            sys.exit(1)
        samples.append((desc, mutated, expect_code))

    # 1. 种子文件缺失
    bad_seeds = {k: v for k, v in good["seeds"].items() if k != "json-tool.html"}
    add("种子文件 json-tool.html 被删除", mutate(seeds=bad_seeds), "seeds", "SEED_MISSING")

    # 2. 种子注入外链
    injected = dict(good["seeds"])
    injected["base64-tool.html"] = (
        good["seeds"]["base64-tool.html"]
        + '<script src="https://cdn.example.com/x.js"></script>'
    )
    add("种子注入外部 <script src>", mutate(seeds=injected), "seeds", "SEED_EXTERNAL_LINK")

    # 3. 种子注入 bridge 写原语
    injected2 = dict(good["seeds"])
    injected2["timestamp-tool.html"] = (
        good["seeds"]["timestamp-tool.html"]
        + 'window.__TAURI__.core.invoke("list_tools")'
    )
    add("种子注入 invoke 写原语", mutate(seeds=injected2), "seeds", "SEED_INVOKE_PRESENT")

    # 4. capability 泄露 tool-* 窗口
    add(
        "隔离被破坏：capability 出现 tool-* 窗口",
        mutate(
            caps='permissions = [{ "identifier": "default-commands", "windows": ["main", "tool-json"] }]'
        ),
        "caps",
        "SEED_CAPABILITY_LEAK",
    )

    # 5. open_tool 移除非法 id 拒绝分支
    add(
        "open_tool 移除非法 id 拒绝分支",
        mutate(tools_rs=good["tools_rs"].replace('"非法工具 id"', '"ok"', 1)),
        "tools_rs",
        "OPEN_ID_CHECK_MISSING",
    )

    # 6. 移除路径越权防御
    add(
        "移除路径越权防御",
        mutate(
            tools_rs=good["tools_rs"]
            .replace("validate_user_tool_path", "zz_path_guard_zz")
            .replace("canonicalize", "zz_canonical_zz")
            .replace("starts_with", "zz_starts_zz")
        ),
        "tools_rs",
        "PATH_DEFENSE_MISSING",
    )

    # 7. 错误页注入外链
    add(
        "错误页注入外链",
        mutate(
            tools_rs=good["tools_rs"].replace(
                'fn error_page(msg: &str) -> String {',
                'fn error_page(msg: &str) -> String { let _ = "https://evil.example.com";',
                1,
            )
        ),
        "tools_rs",
        "ERROR_PAGE_EXTERNAL",
    )

    failures = 0
    for desc, mutated, expect in samples:
        got = detect_violations(mutated)
        if not any(x.startswith(expect) for x in got):
            print(f"self-test FAIL: 坏样本「{desc}」未检出 {expect}（实得 {got}）")
            failures += 1

    if failures:
        return 1

    print(
        f"self-test OK: 好样本零违规 + {len(samples)} 个坏样本全部检出（含变异防呆）"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--self-test", action="store_true", help="跑好样本 + 坏样本自检（含变异防呆）")
    args = ap.parse_args()

    root = Path(__file__).resolve().parents[1]

    if args.self_test:
        return run_self_test(root)

    violations = detect_violations(read_repo(root))
    if violations:
        for x in violations:
            print(x)
        return 1
    print("seed-tool acceptance: all invariants hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
