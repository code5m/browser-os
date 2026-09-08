#!/usr/bin/env python3
"""Reject APIs that can block the Tauri/GTK UI event loop.

This gate intentionally covers APIs that are never acceptable in this desktop
application. Broader synchronous command latency (filesystem, subprocess and
IPC waits) is audited separately because those calls require call-path context.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path


RULES: tuple[tuple[str, re.Pattern[str]], ...] = (
    (
        "UIBLOCK_BLOCKING_DIALOG",
        re.compile(
            r"\bblocking_(?:pick_file|pick_files|pick_folder|save_file|show|message|confirm)\s*\("
        ),
    ),
    (
        "UIBLOCK_RFD_SYNC_DIALOG",
        re.compile(r"\brfd\s*::\s*FileDialog\b"),
    ),
    (
        "UIBLOCK_RUNTIME_BLOCK_ON",
        re.compile(r"\b(?:block_on|block_in_place)\s*\("),
    ),
)

SCAN_DIRS = ("src-tauri/src", "tauri-browser-tabs/crates")


def rust_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for relative in SCAN_DIRS:
        base = root / relative
        if base.exists():
            files.extend(path for path in base.rglob("*.rs") if "target" not in path.parts)
    return sorted(files)


def scan(root: Path) -> list[tuple[str, Path, int]]:
    hits: list[tuple[str, Path, int]] = []
    for path in rust_files(root):
        text = path.read_text(encoding="utf-8")
        for line_no, line in enumerate(text.splitlines(), 1):
            code = line.split("//", 1)[0]
            for name, pattern in RULES:
                if pattern.search(code):
                    hits.append((name, path.relative_to(root), line_no))
    return hits


def self_test() -> int:
    with tempfile.TemporaryDirectory(prefix="ui-thread-blocking-") as temp:
        root = Path(temp)
        source = root / "src-tauri/src"
        source.mkdir(parents=True)
        good = """
#[tauri::command]
pub async fn choose(app: AppHandle) -> Result<(), String> {
    app.dialog().file().pick_folder(|_| {});
    tauri::async_runtime::spawn_blocking(|| expensive_io()).await?;
    Ok(())
}
"""
        (source / "good.rs").write_text(good, encoding="utf-8")
        if scan(root):
            print("UI_THREAD_BLOCKING_SELF_TEST=FAIL good fixture rejected", file=sys.stderr)
            return 1

        bad_samples = {
            "blocking-dialog.rs": "fn x(d: Dialog) { d.blocking_pick_folder(); }",
            "rfd-dialog.rs": "fn x() { let _ = rfd::FileDialog::new().pick_folder(); }",
            "runtime-block.rs": "fn x(rt: Runtime) { rt.block_on(run()); }",
        }
        expected = {
            "blocking-dialog.rs": "UIBLOCK_BLOCKING_DIALOG",
            "rfd-dialog.rs": "UIBLOCK_RFD_SYNC_DIALOG",
            "runtime-block.rs": "UIBLOCK_RUNTIME_BLOCK_ON",
        }
        for name, body in bad_samples.items():
            path = source / name
            path.write_text(body, encoding="utf-8")
            codes = {code for code, hit_path, _ in scan(root) if hit_path.name == name}
            if expected[name] not in codes:
                print(f"UI_THREAD_BLOCKING_SELF_TEST=FAIL missed {name}", file=sys.stderr)
                return 1
    print("UI_THREAD_BLOCKING_SELF_TEST=PASS (1 good + 3 bad fixtures)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="check native UI thread blocking APIs")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    hits = scan(args.root.resolve())
    if hits:
        for code, path, line in hits:
            print(f"{code}: {path}:{line}", file=sys.stderr)
        print("UI_THREAD_BLOCKING_POLICY=FAIL", file=sys.stderr)
        return 1
    print("UI_THREAD_BLOCKING_POLICY=PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
