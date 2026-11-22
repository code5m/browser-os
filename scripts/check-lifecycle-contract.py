#!/usr/bin/env python3
"""Expose the known M0-2 lifecycle gaps as a reproducible source fixture."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path
from native_bridge_owner_sources import read_native_bridge_owners


LEGACY_GAPS = (
    "WINDOW_CLOSE_BYPASSES_UNIFIED_CORE",
    "SYSTEM_EXIT_HOOK_MISSING",
    "GRID_PARTIAL_CREATE_ROLLBACK_MISSING",
    "TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP",
    "TERMINAL_KILL_NOT_WAITED",
    "BACKGROUND_WORKERS_NOT_CANCELLABLE",
)

# M0-2.c 迁移统一退出调用方、后台线程取消、tab/PTY 清理与 grid 半初始化回滚后，
# 机器可检生命周期缺口应为 0；后续新增 gap 必须同步本文档与所有权表。
EXPECTED_GAPS = ()


def brace_body(source: str, start: int) -> str:
    if start < 0:
        return ""
    depth = 0
    for index in range(start, len(source)):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                return source[start + 1 : index]
    return ""


def function_body(source: str, name: str) -> str:
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    return brace_body(source, source.find("{", match.end()))


def invoked_owner_body(source: str, name: str, suffix: str) -> str:
    body = function_body(source, name)
    delegate = f"{name}_{suffix}"
    if re.search(rf"\b{re.escape(delegate)}\s*\(", body):
        implementation = function_body(source, delegate)
        if not implementation:
            return ""
        return body + "\n" + implementation
    return body


def shutdown_tab_body(source: str) -> str:
    body = function_body(source, "register_shutdown_tasks")
    match = re.search(r'\.register\s*\(\s*"close-tabs"\s*,\s*move\s*\|\|\s*\{', body)
    return brace_body(body, match.end() - 1) if match else ""


def detect_gaps(
    main_source: str, bridge_source: str, grid_source: str, *, require_owners: bool = False
) -> list[str]:
    gaps: list[str] = []
    all_source = "\n".join((main_source, bridge_source, grid_source))

    if "ShutdownCoordinator" not in all_source:
        gaps.append("NO_UNIFIED_SHUTDOWN_CORE")

    if re.search(
        r"WindowEvent::CloseRequested[\s\S]{0,240}grid_manager\.shutdown_all\(\)",
        main_source,
    ):
        gaps.append("WINDOW_CLOSE_BYPASSES_UNIFIED_CORE")

    if "RunEvent::ExitRequested" not in main_source:
        gaps.append("SYSTEM_EXIT_HOOK_MISSING")

    create_grid = invoked_owner_body(bridge_source, "create_grid", "blocking")
    direct_spawn = bool(re.search(r"(?:mgr|manager)\.get_or_spawn\s*\([^)]*\)\s*\?", create_grid))
    direct_request = bool(re.search(r"(?:mgr|manager)\.request\([\s\S]*?\)\?;", create_grid))
    if (require_owners and not create_grid) or (direct_spawn and direct_request):
        gaps.append("GRID_PARTIAL_CREATE_ROLLBACK_MISSING")

    # Ordinary close is reversible: keep every Owner record/draft if the
    # native WebView refuses to close. A cleanup receipt may follow success
    # only. This is intentionally different from final shutdown aggregation.
    close_tab = function_body(bridge_source, "close_tab")
    close_call = close_tab.find(".close_tab")
    first_short_circuit = close_tab.find("?;", close_call)
    metadata_cleanup = close_tab.find("child_layouts", close_call)
    if require_owners and (close_call < 0 or metadata_cleanup < 0):
        gaps.append("TAB_CLOSE_OWNER_CLEANUP_MISSING")
    elif close_call >= 0 and metadata_cleanup >= 0 and not (
        close_call < first_short_circuit < metadata_cleanup
    ):
        gaps.append("TAB_CLOSE_FAILURE_LOSES_OWNER_STATE")

    # Final shutdown must attempt all native tabs and aggregate failures;
    # propagating a close error before clearing the batch is still a defect.
    shutdown_tabs = shutdown_tab_body(bridge_source)
    shutdown_call = shutdown_tabs.find(".close_tab")
    shutdown_short_circuit = shutdown_tabs.find("?;", shutdown_call)
    shutdown_cleanup = shutdown_tabs.find("child_layouts", shutdown_call)
    if require_owners and (shutdown_call < 0 or shutdown_cleanup < 0):
        gaps.append("SHUTDOWN_TAB_OWNER_MISSING")
    elif shutdown_call >= 0 and 0 <= shutdown_short_circuit < shutdown_cleanup:
        gaps.append("TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP")

    term_kill = invoked_owner_body(bridge_source, "term_kill", "sync")
    if (require_owners and not term_kill) or (
        ".child.kill()" in term_kill and ".wait(" not in term_kill
    ):
        gaps.append("TERMINAL_KILL_NOT_WAITED")

    worker_names = (
        "start_layout_enforcer",
        "start_resource_scanner",
        "start_hibernation_sweeper",
    )
    worker_bodies = [function_body(bridge_source, name) for name in worker_names]
    cancellation_words = re.compile(r"shutdown_requested|stop_token|cancel_token|is_cancelled")
    if any(
        (require_owners and not body)
        or ("thread::spawn(move || loop" in body and not cancellation_words.search(body))
        for body in worker_bodies
    ):
        gaps.append("BACKGROUND_WORKERS_NOT_CANCELLABLE")

    return gaps


def scan_repository(root: Path) -> list[str]:
    source_root = root / "src-tauri" / "src"
    if (source_root / "bridge").is_dir():
        bridge = read_native_bridge_owners(
            root, "grid_runtime", "terminal_commands", "browser_commands",
            "resource_commands", "tab_runtime",
        )
    else:
        bridge = (source_root / "bridge.rs").read_text(encoding="utf-8")
    return detect_gaps(
        (source_root / "main.rs").read_text(encoding="utf-8"),
        bridge,
        (source_root / "grid_process.rs").read_text(encoding="utf-8"),
        require_owners=True,
    )


def print_gaps(gaps: list[str]) -> None:
    for gap in gaps:
        print(f"GAP={gap}")


def run_self_test() -> int:
    clean_main = """
struct ShutdownCoordinator;
fn main() {
    if let RunEvent::ExitRequested { .. } = event { coordinator.shutdown(); }
    if let WindowEvent::CloseRequested { .. } = event { coordinator.shutdown(); }
}
"""
    clean_grid = "fn create_grid() { coordinator.create_grid_transactionally(); }"
    clean_close = """
fn close_tab() {
    let close_result = manager.close_tab(&id).map_err(convert);
    close_result?;
    state.child_layouts.remove(id);
    state.session_drafts.remove(id);
}
"""
    clean_shutdown = """
fn register_shutdown_tasks() {
    coordinator.register("close-tabs", move || {
        let mut errors = Vec::new();
        for id in manager.get_tab_ids() {
            if let Err(error) = manager.close_tab(&id) { errors.push(error); }
        }
        state.child_layouts.clear();
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    })?;
}
"""
    clean_terminal = "fn term_kill() { session.child.kill(); session.child.wait(); }"
    worker_names = ("start_layout_enforcer", "start_resource_scanner", "start_hibernation_sweeper")
    clean_workers = {
        name: f"fn {name}() {{ while !shutdown_requested {{ }} }}"
        for name in worker_names
    }
    clean_bridge = "\n".join(
        (clean_grid, clean_close, clean_shutdown, clean_terminal, *clean_workers.values())
    )
    unsafe_grid = """
fn create_grid() {
    for i in 0..n {
        let index = i as u32;
        mgr.get_or_spawn(index)?;
        mgr.request(index, command)?;
    }
}
"""
    unsafe_shutdown = """
fn register_shutdown_tasks() {
    coordinator.register("close-tabs", move || {
        for id in manager.get_tab_ids() { manager.close_tab(&id).map_err(convert)?; }
        state.child_layouts.clear();
        Ok(())
    })?;
}
"""
    unsafe_terminal = "fn term_kill() { let _ = session.child.kill(); }"
    bypass_main = clean_main.replace(
        "WindowEvent::CloseRequested { .. } = event { coordinator.shutdown(); }",
        "WindowEvent::CloseRequested { .. } = event { state.grid_manager.shutdown_all(); }",
    )
    no_exit_main = clean_main.replace("RunEvent::ExitRequested", "RunEvent::Other")
    legacy_main = bypass_main.replace("RunEvent::ExitRequested", "RunEvent::Other")
    legacy_bridge = clean_bridge.replace(clean_grid, unsafe_grid).replace(
        clean_shutdown, unsafe_shutdown
    ).replace(clean_terminal, unsafe_terminal)
    for name, clean_worker in clean_workers.items():
        legacy_bridge = legacy_bridge.replace(
            clean_worker, f"fn {name}() {{ std::thread::spawn(move || loop {{}}); }}"
        )
    if tuple(detect_gaps(legacy_main, legacy_bridge, "", require_owners=True)) != LEGACY_GAPS:
        print("self-test: legacy lifecycle gaps changed", file=sys.stderr)
        return 1
    if detect_gaps(clean_main, clean_bridge, "", require_owners=True):
        print("self-test: protected close/shutdown fixture rejected", file=sys.stderr)
        return 1

    cases = [
        ("WINDOW_CLOSE_BYPASSES_UNIFIED_CORE", bypass_main, clean_bridge),
        ("SYSTEM_EXIT_HOOK_MISSING", no_exit_main, clean_bridge),
        ("GRID_PARTIAL_CREATE_ROLLBACK_MISSING", clean_main, clean_bridge.replace(clean_grid, unsafe_grid)),
        ("TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP", clean_main, clean_bridge.replace(clean_shutdown, unsafe_shutdown)),
        ("TAB_CLOSE_FAILURE_LOSES_OWNER_STATE", clean_main, clean_bridge.replace("    close_result?;", "    let _ = close_result;")),
        ("TERMINAL_KILL_NOT_WAITED", clean_main, clean_bridge.replace(clean_terminal, unsafe_terminal)),
        ("TAB_CLOSE_OWNER_CLEANUP_MISSING", clean_main, clean_bridge.replace(clean_close, "")),
        ("SHUTDOWN_TAB_OWNER_MISSING", clean_main, clean_bridge.replace(clean_shutdown, "")),
    ]
    for name, clean_worker in clean_workers.items():
        cases.append((
            "BACKGROUND_WORKERS_NOT_CANCELLABLE", clean_main,
            clean_bridge.replace(clean_worker, f"fn {name}() {{ std::thread::spawn(move || loop {{}}); }}"),
        ))
        cases.append(("BACKGROUND_WORKERS_NOT_CANCELLABLE", clean_main, clean_bridge.replace(clean_worker, "")))
    for gap, gap_main, gap_bridge in cases:
        found = detect_gaps(gap_main, gap_bridge, "", require_owners=True)
        if found != [gap]:
            print(f"self-test: single-gap fixture mismatch for {gap}: {found}", file=sys.stderr)
            return 1

    # Migration cannot silently disable the grid/terminal/worker guards.
    with tempfile.TemporaryDirectory() as directory:
        repo = Path(directory)
        root = repo / "src-tauri" / "src"
        root.mkdir(parents=True)
        (root / "main.rs").write_text(clean_main, encoding="utf-8")
        (root / "bridge.rs").write_text(clean_bridge, encoding="utf-8")
        (root / "grid_process.rs").write_text("", encoding="utf-8")
        if scan_repository(repo):
            print("self-test: facade fixture rejected", file=sys.stderr)
            return 1
        owners = root / "bridge"
        owners.mkdir()
        (root / "bridge.rs").write_text(clean_close + clean_shutdown, encoding="utf-8")
        grid = owners / "grid_runtime.rs"
        grid.write_text(
            "pub async fn create_grid() { spawn_blocking(move || create_grid_blocking()).await }\n"
            + clean_grid.replace("fn create_grid()", "fn create_grid_blocking()"),
            encoding="utf-8",
        )
        (owners / "terminal_commands.rs").write_text(
            "pub async fn term_kill() { spawn_blocking(move || term_kill_sync()).await }\n"
            + clean_terminal.replace("fn term_kill()", "fn term_kill_sync()"),
            encoding="utf-8",
        )
        for owner, worker in (
            ("browser_commands", "start_layout_enforcer"),
            ("resource_commands", "start_resource_scanner"),
            ("tab_runtime", "start_hibernation_sweeper"),
        ):
            (owners / f"{owner}.rs").write_text(clean_workers[worker], encoding="utf-8")
        if scan_repository(repo):
            print("self-test: migrated physical owners rejected", file=sys.stderr)
            return 1
        grid.write_text(
            "pub async fn create_grid() { spawn_blocking(move || create_grid_blocking()).await }\n"
            + unsafe_grid.replace("fn create_grid()", "fn create_grid_blocking()"),
            encoding="utf-8",
        )
        if scan_repository(repo) != ["GRID_PARTIAL_CREATE_ROLLBACK_MISSING"]:
            print("self-test: migrated unsafe blocking grid owner escaped", file=sys.stderr)
            return 1
        (owners / "terminal_commands.rs").unlink()
        try:
            scan_repository(repo)
        except FileNotFoundError:
            pass
        else:
            print("self-test: missing physical owner did not fail closed", file=sys.stderr)
            return 1

    print("SELF_TEST_RESULT=ALL_PASS")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Report whether the M0-2 unified lifecycle contract is implemented."
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--self-test", action="store_true", help="run built-in fixtures")
    mode.add_argument(
        "--expect-current-gaps",
        action="store_true",
        help="pass only while the documented current gap set remains reproducible",
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
        print(f"lifecycle-contract: scan error: {error}", file=sys.stderr)
        return 1

    if args.expect_current_gaps:
        missing = sorted(set(EXPECTED_GAPS) - set(gaps))
        unexpected = sorted(set(gaps) - set(EXPECTED_GAPS))
        if missing or unexpected:
            print("lifecycle-current-gap-fixture: FAIL")
            for gap in missing:
                print(f"MISSING_EXPECTED_GAP={gap}")
            for gap in unexpected:
                print(f"UNEXPECTED_GAP={gap}")
            return 1
        print_gaps(gaps)
        print("CURRENT_GAP_FIXTURE_RESULT=PASS")
        return 0

    if gaps:
        print(f"lifecycle-contract: FAIL ({len(gaps)} known gap(s))")
        print_gaps(gaps)
        print("LIFECYCLE_CONTRACT_RESULT=FAIL")
        return 1

    print("LIFECYCLE_CONTRACT_RESULT=PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
