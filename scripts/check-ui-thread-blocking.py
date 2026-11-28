#!/usr/bin/env python3
"""Reject APIs that can block the Tauri/GTK UI event loop.

This gate rejects blocking APIs on the GUI thread. Two reviewed worker-only
runtime bridges require a fail-closed native thread-identity guard. Broader synchronous command latency (filesystem, subprocess and
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


# This is not a file-level exemption: only the exact guarded call is accepted.
WORKER_BRIDGES = {
    "src-tauri/src/framework/ai/rig_transport.rs",
    "src-tauri/src/native_visibility_review.rs",
}
GUARD_CALL = "crate::shared::blocking::require_off_ui_thread()?;"
RUNTIME_CALL = "tauri::async_runtime::block_on(async {"
GUARD_IMPLEMENTATION = "staticUI_THREAD_ID:OnceLock<ThreadId>=OnceLock::new();pub(crate)fnregister_ui_thread(){UI_THREAD_ID.set(thread::current().id()).expect(\"nativeUIthreadregisteredonce\");}fnensure_off_ui_thread(ui_thread:Option<ThreadId>)->Result<(),String>{matchui_thread{Some(ui)ifui!=thread::current().id()=>Ok(()),_=>Err(\"UI_THREAD_BLOCKING_FORBIDDEN\".into()),}}pub(crate)fnrequire_off_ui_thread()->Result<(),String>{ensure_off_ui_thread(UI_THREAD_ID.get().copied())}"


def worker_guard_registered(root: Path) -> bool:
    source = root / "src-tauri/src"
    try:
        main = (source / "main.rs").read_text(encoding="utf-8")
        boundary = (source / "shared/blocking.rs").read_text(encoding="utf-8")
    except OSError:
        return False
    def code_only(value: str) -> str:
        return re.sub(r"\s+", "", "\n".join(
            line.split("//", 1)[0] for line in value.splitlines()
        ))
    return (
        "fnmain(){shared::blocking::register_ui_thread();" in code_only(main)
        and GUARD_IMPLEMENTATION in code_only(boundary)
    )


def scan(root: Path) -> list[tuple[str, Path, int]]:
    hits: list[tuple[str, Path, int]] = []
    guarded = worker_guard_registered(root)
    for path in rust_files(root):
        lines = path.read_text(encoding="utf-8").splitlines()
        relative = path.relative_to(root)
        for line_no, line in enumerate(lines, 1):
            code = line.split("//", 1)[0]
            for name, pattern in RULES:
                if pattern.search(code):
                    if (
                        guarded
                        and name == "UIBLOCK_RUNTIME_BLOCK_ON"
                        and relative.as_posix() in WORKER_BRIDGES
                        and code.strip() == RUNTIME_CALL
                        and line_no > 1
                        and lines[line_no - 2].strip() == GUARD_CALL
                    ):
                        continue
                    hits.append((name, relative, line_no))
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
        main = source / "main.rs"
        main.write_text("fn main() { shared::blocking::register_ui_thread(); }", encoding="utf-8")
        boundary = source / "shared/blocking.rs"
        boundary.parent.mkdir(parents=True)
        boundary.write_text(GUARD_IMPLEMENTATION, encoding="utf-8")
        worker = root / "src-tauri/src/framework/ai/rig_transport.rs"
        worker.parent.mkdir(parents=True)
        reviewed = GUARD_CALL + "\n" + RUNTIME_CALL + "\n    future.await\n})"
        def worker_hit() -> bool:
            return any(hit == worker.relative_to(root) for _, hit, _ in scan(root))
        worker.write_text(reviewed, encoding="utf-8")
        if worker_hit():
            print("UI_THREAD_BLOCKING_SELF_TEST=FAIL guarded worker rejected", file=sys.stderr)
            return 1
        for broken in (
            RUNTIME_CALL,
            reviewed + "\nrt.block_on(other());",
            GUARD_CALL + "\nrt.block_in_place(work);",
        ):
            worker.write_text(broken, encoding="utf-8")
            if not worker_hit():
                print("UI_THREAD_BLOCKING_SELF_TEST=FAIL unsafe worker accepted", file=sys.stderr)
                return 1
        worker.write_text(reviewed, encoding="utf-8")
        main.write_text("fn main() {}", encoding="utf-8")
        if not worker_hit():
            print("UI_THREAD_BLOCKING_SELF_TEST=FAIL missing registration accepted", file=sys.stderr)
            return 1
        main.write_text("fn main() { shared::blocking::register_ui_thread(); }", encoding="utf-8")
        boundary.write_text(GUARD_IMPLEMENTATION.replace("ui!=", "ui=="), encoding="utf-8")
        if not worker_hit():
            print("UI_THREAD_BLOCKING_SELF_TEST=FAIL inverted guard accepted", file=sys.stderr)
            return 1
    print("UI_THREAD_BLOCKING_SELF_TEST=PASS (original + guarded worker fixtures)")
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
