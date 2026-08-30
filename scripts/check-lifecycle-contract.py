#!/usr/bin/env python3
"""Expose the known M0-2 lifecycle gaps as a reproducible source fixture."""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path


EXPECTED_GAPS = (
    "NO_UNIFIED_SHUTDOWN_CORE",
    "WINDOW_CLOSE_BYPASSES_UNIFIED_CORE",
    "SYSTEM_EXIT_HOOK_MISSING",
    "GRID_PARTIAL_CREATE_ROLLBACK_MISSING",
    "TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP",
    "TERMINAL_KILL_NOT_WAITED",
    "BACKGROUND_WORKERS_NOT_CANCELLABLE",
)


def function_body(source: str, name: str) -> str:
    """Return a Rust function body using a small brace-balanced extractor."""
    match = re.search(rf"\bfn\s+{re.escape(name)}\s*\(", source)
    if not match:
        return ""
    start = source.find("{", match.end())
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


def detect_gaps(main_source: str, bridge_source: str, grid_source: str) -> list[str]:
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

    create_grid = function_body(bridge_source, "create_grid")
    direct_spawn = "mgr.get_or_spawn(index)?" in create_grid
    direct_request = bool(re.search(r"mgr\.request\([\s\S]*?\)\?;", create_grid))
    if direct_spawn and direct_request:
        gaps.append("GRID_PARTIAL_CREATE_ROLLBACK_MISSING")

    close_tab = function_body(bridge_source, "close_tab")
    close_call = close_tab.find(".close_tab")
    first_short_circuit = close_tab.find("?;", close_call)
    metadata_cleanup = close_tab.find("child_layouts", close_call)
    if close_call >= 0 and 0 <= first_short_circuit < metadata_cleanup:
        gaps.append("TAB_CLOSE_FAILURE_SHORT_CIRCUITS_CLEANUP")

    term_kill = function_body(bridge_source, "term_kill")
    if ".child.kill()" in term_kill and ".wait(" not in term_kill:
        gaps.append("TERMINAL_KILL_NOT_WAITED")

    worker_names = (
        "start_layout_enforcer",
        "start_resource_scanner",
        "start_hibernation_sweeper",
    )
    worker_bodies = [function_body(bridge_source, name) for name in worker_names]
    legacy_loops = all("thread::spawn(move || loop" in body for body in worker_bodies)
    cancellation_words = re.compile(r"shutdown_requested|stop_token|cancel_token|is_cancelled")
    if legacy_loops and not any(cancellation_words.search(body) for body in worker_bodies):
        gaps.append("BACKGROUND_WORKERS_NOT_CANCELLABLE")

    return gaps


def scan_repository(root: Path) -> list[str]:
    source_root = root / "src-tauri" / "src"
    return detect_gaps(
        (source_root / "main.rs").read_text(encoding="utf-8"),
        (source_root / "bridge.rs").read_text(encoding="utf-8"),
        (source_root / "grid_process.rs").read_text(encoding="utf-8"),
    )


def print_gaps(gaps: list[str]) -> None:
    for gap in gaps:
        print(f"GAP={gap}")


def run_self_test() -> int:
    legacy_main = """
fn main() {
    match event {
        WindowEvent::CloseRequested { .. } => { state.grid_manager.shutdown_all(); }
        _ => {}
    }
}
"""
    legacy_bridge = """
fn create_grid() {
    for i in 0..n {
        let index = i as u32;
        mgr.get_or_spawn(index)?;
        mgr.request(index, command)?;
    }
}
fn close_tab() {
    manager.close_tab(&id).map_err(convert)?;
    state.child_layouts.remove(id);
}
fn term_kill() { let _ = session.child.kill(); }
fn start_layout_enforcer() { std::thread::spawn(move || loop {}); }
fn start_resource_scanner() { std::thread::spawn(move || loop {}); }
fn start_hibernation_sweeper() { std::thread::spawn(move || loop {}); }
"""
    detected = detect_gaps(legacy_main, legacy_bridge, "")
    if tuple(detected) != EXPECTED_GAPS:
        print(f"self-test: legacy mismatch: {detected}", file=sys.stderr)
        return 1

    resolved_main = """
struct ShutdownCoordinator;
fn main() {
    if let RunEvent::ExitRequested { .. } = event { coordinator.shutdown(); }
    if let WindowEvent::CloseRequested { .. } = event { coordinator.shutdown(); }
}
"""
    resolved_bridge = """
fn create_grid() { coordinator.create_grid_transactionally(); }
fn close_tab() { coordinator.close_tab_best_effort(); }
fn term_kill() { session.child.kill(); session.child.wait(); }
fn start_layout_enforcer() { while !shutdown_requested {} }
fn start_resource_scanner() { while !shutdown_requested {} }
fn start_hibernation_sweeper() { while !shutdown_requested {} }
"""
    if detect_gaps(resolved_main, resolved_bridge, ""):
        print("self-test: resolved fixture still reports gaps", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory) / "src-tauri" / "src"
        root.mkdir(parents=True)
        (root / "main.rs").write_text(legacy_main, encoding="utf-8")
        (root / "bridge.rs").write_text(legacy_bridge, encoding="utf-8")
        (root / "grid_process.rs").write_text("", encoding="utf-8")
        if tuple(scan_repository(root.parents[1])) != EXPECTED_GAPS:
            print("self-test: repository scan mismatch", file=sys.stderr)
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
        help="pass only while the M0-2.a documented gap set remains reproducible",
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
