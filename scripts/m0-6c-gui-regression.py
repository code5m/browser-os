#!/usr/bin/env python3
"""Run the M0-6.c GUI regression with a local AI-site mock.

The Rust side drives real Tauri windows, grid child processes, normal tabs, and
PTY sessions. This wrapper provides deterministic web content, captures events,
and writes a replayable evidence bundle.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from datetime import datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, quote, urlparse


ROOT = Path(__file__).resolve().parents[1]
TAURI_DIR = ROOT / "src-tauri"
APP_BIN = TAURI_DIR / "target" / "release" / "mvp-browser-os"


class EventStore:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.lock = threading.Lock()
        self.events: list[dict[str, str]] = []

    def add(self, event: dict[str, str]) -> None:
        event["ts"] = f"{time.time():.3f}"
        with self.lock:
            self.events.append(event)
            with self.path.open("a", encoding="utf-8") as fh:
                fh.write(json.dumps(event, ensure_ascii=False, sort_keys=True) + "\n")

    def snapshot(self) -> list[dict[str, str]]:
        with self.lock:
            return list(self.events)


class MockHandler(BaseHTTPRequestHandler):
    store: EventStore

    def log_message(self, fmt: str, *args: object) -> None:
        return

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        query = {key: values[-1] for key, values in parse_qs(parsed.query).items()}
        if parsed.path == "/event":
            event = dict(query)
            event["cookie_header"] = self.headers.get("Cookie", "")
            self.store.add(event)
            self.send_response(204)
            self.end_headers()
            return
        if parsed.path != "/login":
            self.send_error(404)
            return

        case_id = query.get("case", "unknown")
        grid = query.get("grid", "0")
        self.store.add(
            {
                "case": case_id,
                "grid": grid,
                "phase": "server_load",
                "cookie_header": self.headers.get("Cookie", ""),
            }
        )
        body = self._page(case_id, grid).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("Set-Cookie", "m0_session=ok; Path=/; Max-Age=3600; SameSite=Lax")
        self.end_headers()
        self.wfile.write(body)

    def _page(self, case_id: str, grid: str) -> str:
        safe_case = quote(case_id)
        safe_grid = quote(grid)
        return f"""<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <title>M0 mock case {safe_case} grid {safe_grid}</title>
  <style>
    body {{
      margin: 0;
      font: 15px/1.45 system-ui, sans-serif;
      color: #18202a;
      background: #f7f9fb;
    }}
    main {{
      padding: 24px;
    }}
    #stream {{
      min-height: 120px;
      white-space: pre-wrap;
      border: 1px solid #9aa7b2;
      background: white;
      padding: 16px;
    }}
    button {{
      margin-top: 16px;
      padding: 8px 12px;
    }}
  </style>
</head>
<body>
  <main>
    <h1>M0 GUI Regression Mock</h1>
    <p>case={safe_case} grid={safe_grid}</p>
    <div id="stream"></div>
    <button id="send">Send</button>
  </main>
  <script>
    const caseId = "{safe_case}";
    const grid = "{safe_grid}";
    function report(phase) {{
      const params = new URLSearchParams({{
        case: caseId,
        grid,
        phase,
        cookie: document.cookie || "",
        text: document.getElementById("stream").innerText
      }});
      fetch("/event?" + params.toString(), {{ credentials: "include", keepalive: true }}).catch(() => {{}});
    }}
    let chunks = ["AI mock ready", "\\nstream chunk 1", "\\nstream chunk 2", "\\nDONE"];
    let i = 0;
    let timer = setInterval(() => {{
      document.getElementById("stream").innerText += chunks[i] || "";
      i += 1;
      if (i >= chunks.length) {{
        clearInterval(timer);
        report("stream_done");
      }}
    }}, 180);
    window.addEventListener("load", () => setTimeout(() => report("loaded"), 80));
    document.getElementById("send").addEventListener("click", () => report("button_click"));
  </script>
</body>
</html>"""


def parse_events(path: Path) -> list[dict[str, str]]:
    if not path.exists():
        return []
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            events.append(json.loads(line))
    return events


def event_has(
    events: list[dict[str, str]],
    case_id: str,
    phase: str,
    grid: str | None = None,
    require_cookie: bool = False,
    menu: str | None = None,
) -> bool:
    for event in events:
        if event.get("case") != case_id or event.get("phase") != phase:
            continue
        if grid is not None and event.get("grid") != grid:
            continue
        if require_cookie and "m0_session=ok" not in event.get("cookie", ""):
            continue
        if menu is not None and event.get("menu") != menu:
            continue
        return True
    return False


def validate(driver_report: dict[str, object], events: list[dict[str, str]]) -> list[dict[str, object]]:
    checks: list[dict[str, object]] = []

    def check(name: str, ok: bool, detail: str) -> None:
        checks.append({"name": name, "status": "PASS" if ok else "FAIL", "detail": detail})

    check(
        "driver-status",
        driver_report.get("status") == "PASS",
        f"driver status={driver_report.get('status')}",
    )
    check(
        "scenario-1-event",
        event_has(events, "1", "single_broadcast", "0", True),
        "single grid AI mock event with session cookie",
    )
    for grid in range(4):
        check(
            f"scenario-2-grid-{grid}",
            event_has(events, "2", "concurrent_ai", str(grid), True),
            f"concurrent AI mock event for grid-{grid}",
        )
    check(
        "scenario-3-event",
        event_has(events, "3", "after_move_resize", "0", True),
        "main-window move/resize event reached grid-0",
    )
    check(
        "scenario-4-event",
        event_has(events, "4", "after_focus_restore", "0", True),
        "blur/focus recovery event reached grid-0",
    )
    check(
        "scenario-5-event",
        event_has(events, "5", "after_view_switch", "0", True),
        "view-switch event reached grid-0",
    )
    check(
        "scenario-6-event",
        event_has(events, "6", "tabs_closed_grid_survives", "0", True),
        "normal tab workflow left grid-0 usable",
    )
    check(
        "scenario-7-event",
        event_has(events, "7", "after_crash_recovery", "0", True),
        "grid child crash recovery event reached restarted grid-0",
    )
    check(
        "scenario-8-menu",
        event_has(events, "8", "context_menu", "0", True, "true"),
        "injected context menu opened inside grid webview",
    )
    check(
        "scenario-9-before",
        event_has(events, "9", "before_restart", "0", True),
        "login-state cookie present before grid restart",
    )
    check(
        "scenario-9-after",
        event_has(events, "9", "after_restart", "0", True),
        "login-state cookie present after grid restart",
    )
    return checks


def write_summary(evidence_dir: Path, driver_report: dict[str, object], checks: list[dict[str, object]], exit_code: int) -> None:
    status = "PASS" if exit_code == 0 and all(c["status"] == "PASS" for c in checks) else "FAIL"
    summary = {
        "status": status,
        "driver_status": driver_report.get("status"),
        "checks": checks,
        "evidence_dir": str(evidence_dir),
    }
    (evidence_dir / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    lines = [
        f"# M0-6.c GUI Regression Evidence",
        "",
        f"STATUS={status}",
        f"APP_EXIT={exit_code}",
        f"DRIVER_STATUS={driver_report.get('status')}",
        "",
        "## Checks",
        "",
    ]
    for check in checks:
        lines.append(f"- {check['status']} {check['name']}: {check['detail']}")
    lines.extend(
        [
            "",
            "## Files",
            "",
            "- app.log",
            "- driver-report.json",
            "- mock-events.jsonl",
            "- summary.json",
        ]
    )
    (evidence_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_self_test() -> int:
    sample = [
        {"case": "1", "phase": "single_broadcast", "grid": "0", "cookie": "m0_session=ok"},
        {"case": "2", "phase": "concurrent_ai", "grid": "0", "cookie": "m0_session=ok"},
        {"case": "8", "phase": "context_menu", "grid": "0", "cookie": "m0_session=ok", "menu": "true"},
    ]
    assert event_has(sample, "1", "single_broadcast", "0", True)
    assert event_has(sample, "8", "context_menu", "0", True, "true")
    assert not event_has(sample, "8", "context_menu", "0", True, "false")
    print("M0_6C_GUI_REGRESSION_SELF_TEST=PASS")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--evidence-dir", type=Path)
    args = parser.parse_args()

    if args.self_test:
        return run_self_test()

    if not args.no_build:
        subprocess.run(["cargo", "build", "--release", "--locked"], cwd=TAURI_DIR, check=True)
    if not APP_BIN.exists():
        raise SystemExit(f"release binary not found: {APP_BIN}")

    stamp = datetime.now().strftime("%Y%m%d-%H%M")
    evidence_dir = args.evidence_dir or ROOT / "logs" / "m0-6c-gui-evidence" / stamp
    evidence_dir.mkdir(parents=True, exist_ok=True)
    app_log = evidence_dir / "app.log"
    event_path = evidence_dir / "mock-events.jsonl"
    driver_report_path = evidence_dir / "driver-report.json"
    event_path.write_text("", encoding="utf-8")

    store = EventStore(event_path)
    MockHandler.store = store
    server = ThreadingHTTPServer(("127.0.0.1", 0), MockHandler)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()

    xdg_root = Path(tempfile.mkdtemp(prefix="m0-6c-xdg-"))
    env = os.environ.copy()
    env.update(
        {
            "GRID_GUI_REGRESSION": "1",
            "M0_GUI_MOCK_BASE": f"http://127.0.0.1:{server.server_port}",
            "M0_GUI_REGRESSION_REPORT": str(driver_report_path),
            "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
            "GTK_USE_PORTAL": "0",
            "GDK_BACKEND": "x11",
            "XDG_CONFIG_HOME": str(xdg_root / "config"),
            "XDG_CACHE_HOME": str(xdg_root / "cache"),
            "XDG_DATA_HOME": str(xdg_root / "data"),
            "XDG_RUNTIME_DIR": os.environ.get("XDG_RUNTIME_DIR", "/tmp"),
        }
    )
    exit_code = 1
    try:
        with app_log.open("w", encoding="utf-8") as log:
            proc = subprocess.run([str(APP_BIN)], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=220)
            exit_code = proc.returncode
    finally:
        server.shutdown()
        server.server_close()
        shutil.rmtree(xdg_root, ignore_errors=True)

    driver_report: dict[str, object] = {}
    if driver_report_path.exists():
        driver_report = json.loads(driver_report_path.read_text(encoding="utf-8"))
    else:
        driver_report = {"status": "FAIL", "steps": [{"id": "driver-report", "status": "FAIL", "detail": "missing"}]}
    events = parse_events(event_path)
    checks = validate(driver_report, events)
    write_summary(evidence_dir, driver_report, checks, exit_code)
    status = "PASS" if exit_code == 0 and all(c["status"] == "PASS" for c in checks) else "FAIL"
    print(f"M0_6C_GUI_REGRESSION_RESULT={status}")
    print(f"EVIDENCE_DIR={evidence_dir}")
    return 0 if status == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
