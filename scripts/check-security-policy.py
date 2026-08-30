#!/usr/bin/env python3
"""Expose the known M0-3 security boundary gaps as a reproducible source fixture.

M0-3.a 只盘点与定契约（威胁矩阵 logs/m0-security-threat-matrix-v1.md），
收口分属 M0-3.b/c/d，因此默认模式当前预期 EXIT=1。
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path

EXPECTED_GAPS = (
    "LAUNCH_APP_ARBITRARY_SHELL",
    "FILE_COMMANDS_WITHOUT_PATH_POLICY",
    "CAPABILITY_STALE_BROWSER_LABEL",
    "REMOTE_WILDCARD_IPC",
    "EVAL_WITHOUT_SOURCE_CHECK",
)

# 路径类命令：这些函数直接把调用方传入的 path 交给 std::fs，没有策略层。
FILE_COMMAND_MARKERS = (
    "std::fs::write(",
    "std::fs::remove_dir_all(",
    "std::fs::remove_file(",
    "std::fs::create_dir_all(",
)


def detect_gaps(bridge_source: str, capability_sources: str) -> list[str]:
    gaps: list[str] = []

    # SEC-01/SEC-07：launch_app 用 sh -c 执行任意命令。
    if re.search(r'Command::new\(\s*"sh"\s*\)\s*[\s\S]{0,120}?\.arg\(\s*"-c"\s*\)', bridge_source):
        gaps.append("LAUNCH_APP_ARBITRARY_SHELL")

    # SEC-02：文件命令存在，且没有 canonicalize / 允许根目录校验参与。
    has_file_commands = any(marker in bridge_source for marker in FILE_COMMAND_MARKERS)
    has_path_policy = "canonicalize(" in bridge_source and "check_path_within_roots" in bridge_source
    if has_file_commands and not has_path_policy:
        gaps.append("FILE_COMMANDS_WITHOUT_PATH_POLICY")

    # SEC-03：capability 里残留已不存在的 browser label。
    if re.search(r'"(windows|webviews)"\s*:\s*\[[^\]]*"browser"', capability_sources):
        gaps.append("CAPABILITY_STALE_BROWSER_LABEL")

    # SEC-04：remote urls 全通配。
    if re.search(r'"remote"\s*:\s*\{[\s\S]{0,200}?"https?://\*"', capability_sources):
        gaps.append("REMOTE_WILDCARD_IPC")

    # SEC-05：向子 webview 注入 JS，且没有来源/意图校验。
    has_eval = re.search(r"\.eval\(\s*&", bridge_source)
    has_source_check = "check_webview_label" in bridge_source and "user_intent" in bridge_source
    if has_eval and not has_source_check:
        gaps.append("EVAL_WITHOUT_SOURCE_CHECK")

    return gaps


def load_capability_sources(root: Path) -> str:
    capability_dir = root / "src-tauri" / "capabilities"
    if not capability_dir.is_dir():
        return ""
    parts = []
    for path in sorted(capability_dir.glob("*.json")):
        parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def scan_repository(root: Path) -> list[str]:
    bridge_source = (root / "src-tauri" / "src" / "bridge.rs").read_text(encoding="utf-8")
    return detect_gaps(bridge_source, load_capability_sources(root))


def run_self_test() -> int:
    legacy_bridge = """
pub fn launch_app(exec: String) -> Result<(), String> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(exec)
        .spawn()?;
    Ok(())
}
pub fn write_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| e.to_string())
}
pub fn delete_path(path: String) -> Result<(), String> {
    std::fs::remove_dir_all(&path).map_err(|e| e.to_string())
}
pub fn eval_in_tab(id: String, js: String) {
    let _ = win.eval(&format!("{}", js));
}
"""
    legacy_capabilities = """
{
  "windows": ["main", "browser"],
  "webviews": ["browser", "tab-*"],
  "remote": {"urls": ["https://*", "http://*"]}
}
"""
    detected = detect_gaps(legacy_bridge, legacy_capabilities)
    if tuple(detected) != EXPECTED_GAPS:
        print(f"self-test: legacy mismatch: {detected}", file=sys.stderr)
        return 1

    resolved_bridge = """
pub fn launch_app(entry: AppEntry) -> Result<(), String> {
    std::process::Command::new(entry.program).args(entry.args).spawn()?;
    Ok(())
}
pub fn write_file(path: String, content: String) -> Result<(), String> {
    let canonical = std::fs::canonicalize(&path)?;
    security_policy::check_path_within_roots(&canonical, allowed_roots())?;
    std::fs::write(&canonical, &content).map_err(|e| e.to_string())
}
pub fn eval_in_tab(id: String, js: String) {
    if !user_intent_confirmed() { return; }
    security_policy::check_webview_label(&id)?;
    let _ = win.eval(&format!("{}", js));
}
"""
    resolved_capabilities = """
{
  "windows": ["main"],
  "webviews": ["tab-*", "grid-*"],
  "remote": {"urls": ["https://trusted.example"]}
}
"""
    if detect_gaps(resolved_bridge, resolved_capabilities):
        print("self-test: resolved fixture still reports gaps", file=sys.stderr)
        return 1

    # 端到端：临时仓库扫描必须与 fixture 判定一致。
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        (root / "src-tauri" / "src").mkdir(parents=True)
        (root / "src-tauri" / "capabilities").mkdir(parents=True)
        (root / "src-tauri" / "src" / "bridge.rs").write_text(legacy_bridge, encoding="utf-8")
        (root / "src-tauri" / "capabilities" / "default.json").write_text(
            legacy_capabilities, encoding="utf-8"
        )
        if tuple(scan_repository(root)) != EXPECTED_GAPS:
            print("self-test: repository scan mismatch", file=sys.stderr)
            return 1

    # capability JSON 必须可被解析，避免夹具只做字符串匹配而放过坏 JSON。
    try:
        json.loads(legacy_capabilities)
    except json.JSONDecodeError as error:
        print(f"self-test: capability fixture is not valid JSON: {error}", file=sys.stderr)
        return 1

    print("SELF_TEST_RESULT=ALL_PASS")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Report whether the M0-3 security boundary contract is enforced."
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--self-test", action="store_true", help="run built-in fixtures")
    mode.add_argument(
        "--expect-current-gaps",
        action="store_true",
        help="pass only while the documented M0-3.a gap set remains reproducible",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="repository root (defaults to this script's repository)",
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()

    try:
        gaps = scan_repository(args.root.resolve())
    except (OSError, UnicodeError) as error:
        print(f"security-policy: scan error: {error}", file=sys.stderr)
        return 1

    if args.expect_current_gaps:
        missing = sorted(set(EXPECTED_GAPS) - set(gaps))
        unexpected = sorted(set(gaps) - set(EXPECTED_GAPS))
        if missing or unexpected:
            print("security-current-gap-fixture: FAIL")
            for gap in missing:
                print(f"MISSING_EXPECTED_GAP={gap}")
            for gap in unexpected:
                print(f"UNEXPECTED_GAP={gap}")
            return 1
        for gap in gaps:
            print(f"GAP={gap}")
        print("CURRENT_GAP_FIXTURE_RESULT=PASS")
        return 0

    if gaps:
        print(f"security-policy: FAIL ({len(gaps)} known gap(s))")
        for gap in gaps:
            print(f"GAP={gap}")
        print("SECURITY_POLICY_RESULT=FAIL")
        return 1

    print("SECURITY_POLICY_RESULT=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
