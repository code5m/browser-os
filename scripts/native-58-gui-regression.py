#!/usr/bin/env python3
"""Run the Native 58 side-effect review against the installed BrowserOS GUI.

The runner uses a temporary HOME/XDG tree and an xdg-settings recorder. The
application is the exact installed release binary; commands are invoked by the
real main WebView IPC, and remote-only telemetry is invoked by a separate HTTP
child WebView. Evidence is written as JSON/Markdown plus app and remote logs.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import http.server
import json
import os
import re
import shutil
import socket
import subprocess
import tempfile
import threading
import time
from pathlib import Path
from urllib.parse import parse_qs, urlparse

PIXEL = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jXioAAAAASUVORK5CYII="
)

class EvidenceState:
    def __init__(self):
        self.remote_results = []
        self.events = []
        self.lock = threading.Lock()

class Handler(http.server.BaseHTTPRequestHandler):
    state: EvidenceState
    def log_message(self, *_args):
        return
    def _send(self, code, content_type, body):
        data = body if isinstance(body, bytes) else body.encode()
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)
    def do_GET(self):
        parsed = urlparse(self.path)
        if parsed.path == "/remote-results":
            with self.state.lock:
                body = json.dumps(self.state.remote_results)
            return self._send(200, "application/json", body)
        if parsed.path == "/events":
            phase = parse_qs(parsed.query).get("phase", [""])[0]
            with self.state.lock:
                body = json.dumps([x for x in self.state.events if not phase or x.get("phase") == phase])
            return self._send(200, "application/json", body)
        if parsed.path == "/event":
            with self.state.lock:
                self.state.events.append({"phase": parse_qs(parsed.query).get("phase", [""])[0], "time": time.time()})
            return self._send(200, "text/plain", "ok")
        if parsed.path == "/pixel.png":
            return self._send(200, "image/png", PIXEL)
        if parsed.path == "/style.css":
            return self._send(200, "text/css", "body{font-family:sans-serif;background:#fff} img{width:1px;height:1px}")
        if parsed.path == "/resource.js":
            return self._send(200, "application/javascript", "fetch('/event?phase=resource-script').catch(()=>{});")
        if parsed.path == "/remote":
            page = r'''<!doctype html><html><head><title>Native58 Remote Tab</title><link rel="stylesheet" href="/style.css"></head><body>
<img src="/pixel.png"><script src="/resource.js"></script><main>Native 58 remote-origin test</main>
<script>
(async()=>{
 const api=window.__TAURI__?.core?.invoke;
 const result=[];
 async function attempt(command,args,expected){try{await api(command,args);result.push({command,status:expected==='DENIED'?'UNEXPECTED_ALLOW':'PASS'});}catch(e){const message=String(e);const authorizationDenied=/not allowed|permission denied|NATIVE_LEGACY_SOURCE_DENIED|not permitted|Forbidden/i.test(message);result.push({command,status:expected==='DENIED'&&authorizationDenied?'DENIED':'FAIL',error:message});}}
 if(!api){result.push({command:'remote-api',status:'FAIL',error:'window.__TAURI__.core.invoke missing'});}
 else {
  await attempt('report_title',{title:'Native58 Remote Tab'},'PASS');
  await attempt('report_resources',{pageUrl:location.href,items:[]},'PASS');
  await attempt('clipboard_write',{text:'REMOTE_MUST_NOT_WRITE_CLIPBOARD'},'DENIED');
 }
 try{await fetch('/remote-result',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(result)});}catch(e){}
})();
</script></body></html>'''
            return self._send(200, "text/html; charset=utf-8", page)
        if parsed.path == "/page":
            name = parse_qs(parsed.query).get("name", ["page"])[0]
            page = f'''<!doctype html><html><head><title>{name}</title><link rel="stylesheet" href="/style.css"></head><body><img src="/pixel.png"><script src="/resource.js"></script><main>{name}</main><script>fetch('/event?phase=page-{name}').catch(()=>{{}});</script></body></html>'''
            return self._send(200, "text/html; charset=utf-8", page)
        return self._send(404, "text/plain", "not found")
    def do_POST(self):
        if urlparse(self.path).path != "/remote-result":
            return self._send(404, "text/plain", "not found")
        length = int(self.headers.get("Content-Length", "0"))
        try:
            value = json.loads(self.rfile.read(length) or b"[]")
            if not isinstance(value, list):
                raise ValueError("remote result must be a list")
        except Exception as error:
            value = [{"command":"remote-result","status":"FAIL","error":str(error)}]
        with self.state.lock:
            self.state.remote_results.extend(value)
        return self._send(200, "application/json", "{}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path, default=Path("/tmp/browseros-native-58"))
    parser.add_argument("--binary", type=Path, default=Path("/usr/bin/mvp-browser-os"))
    parser.add_argument("--timeout", type=int, default=165)
    args = parser.parse_args()
    evidence = args.evidence_dir.resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve()
    if not binary.is_file():
        print(f"NATIVE_58_GUI_RESULT=FAIL missing installed binary: {binary}")
        return 2
    running = subprocess.run(["pgrep", "-x", "mvp-browser-os"], capture_output=True, text=True)
    if running.returncode == 0:
        print("NATIVE_58_GUI_RESULT=FAIL another BrowserOS process is already running; refusing to disturb it")
        return 2

    display = os.environ.get("DISPLAY", "")
    processes = subprocess.run(["ps", "-eo", "args"], capture_output=True, text=True, check=True).stdout.splitlines()
    xvfb_pattern = re.compile(r"(?:^|\s)(?:\S*/)?Xvfb\s+" + re.escape(display) + r"(?:\s|$)")
    if not display or not any(xvfb_pattern.search(row) for row in processes):
        print("NATIVE_58_GUI_RESULT=FAIL a real isolated Xvfb display is required; refusing access to the user's clipboard")
        return 2
    state = EvidenceState()
    handler = type("Native58Handler", (Handler,), {"state": state})
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    server.daemon_threads = True
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    base_url = f"http://127.0.0.1:{server.server_address[1]}"

    with tempfile.TemporaryDirectory(prefix="browseros-native58-") as temp:
        temp_root = Path(temp)
        home = temp_root / "home"
        data = temp_root / "data"
        config = temp_root / "config"
        cache = temp_root / "cache"
        bin_dir = temp_root / "bin"
        for directory in (home, data, config, cache, bin_dir):
            directory.mkdir(parents=True, exist_ok=True)
        xdg_log = evidence / "xdg-settings.log"
        xdg_current = temp_root / "default-browser.txt"
        stub = bin_dir / "xdg-settings"
        stub.write_text("""#!/bin/sh
printf '%s\\n' \"$*\" >> \"$NATIVE_58_XDG_LOG\"
if [ \"$1 $2\" = \"get default-web-browser\" ]; then
  if [ -f \"$NATIVE_58_XDG_CURRENT\" ]; then cat \"$NATIVE_58_XDG_CURRENT\"; else printf 'firefox.desktop\\n'; fi
  exit 0
fi
if [ \"$1 $2\" = \"set default-web-browser\" ]; then printf '%s\\n' \"$3\" > \"$NATIVE_58_XDG_CURRENT\"; exit 0; fi
exit 2
""", encoding="utf-8")
        stub.chmod(0o755)
        report_path = evidence / "native-58-report.json"
        app_log = evidence / "app.log"
        env = os.environ.copy()
        env.update({
            "HOME": str(home),
            "XDG_DATA_HOME": str(data),
            "XDG_CONFIG_HOME": str(config),
            "XDG_CACHE_HOME": str(cache),
            "PATH": str(bin_dir) + os.pathsep + env.get("PATH", ""),
            "NATIVE_58_GUI_REVIEW": "1",
            "NATIVE_58_REVIEW_BASE_URL": base_url,
            "NATIVE_58_REVIEW_REPORT": str(report_path),
            "NATIVE_58_XDG_LOG": str(xdg_log),
            "NATIVE_58_XDG_CURRENT": str(xdg_current),
            "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
            "GDK_BACKEND": "x11",
            "GTK_USE_PORTAL": "0",
        })
        screenshot_target = evidence / "installed-gui.png"
        screenshot_result = {"path": None}
        def capture_live_gui():
            time.sleep(7)
            if process.poll() is not None:
                return
            if shutil.which("import"):
                result = subprocess.run(["import", "-window", "root", str(screenshot_target)], capture_output=True, text=True, timeout=8)
                if result.returncode == 0 and screenshot_target.exists():
                    screenshot_result["path"] = str(screenshot_target)
        with app_log.open("w", encoding="utf-8") as output:
            process = subprocess.Popen([str(binary)], stdout=output, stderr=subprocess.STDOUT, env=env)
            screenshot_thread = threading.Thread(target=capture_live_gui, daemon=True)
            screenshot_thread.start()
            try:
                code = process.wait(timeout=args.timeout)
            except subprocess.TimeoutExpired:
                process.terminate()
                try:
                    process.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
                code = 124
            screenshot_thread.join(timeout=1)

    server.shutdown()
    server.server_close()
    (evidence / "remote-results.json").write_text(json.dumps(state.remote_results, indent=2), encoding="utf-8")
    (evidence / "remote-events.json").write_text(json.dumps(state.events, indent=2), encoding="utf-8")
    report = {}
    if report_path.exists():
        try:
            report = json.loads(report_path.read_text(encoding="utf-8"))
        except Exception as error:
            report = {"status":"FAIL","error":f"invalid report: {error}"}
    progress_path = evidence / "native-58-progress.jsonl"
    progress_lines = progress_path.read_text(encoding="utf-8").splitlines() if progress_path.is_file() else []
    try:
        progress_records = [json.loads(row) for row in progress_lines]
    except (json.JSONDecodeError, ValueError):
        progress_records = [{"phase": "INVALID_PROGRESS_EVIDENCE"}]
    report["progressRecordCount"] = len(progress_records)
    report["lastNativeProgress"] = progress_records[-1] if progress_records else None
    remote_by_name = {item.get("command"): item for item in state.remote_results}
    remote_ok = (
        remote_by_name.get("report_title", {}).get("status") == "PASS"
        and remote_by_name.get("report_resources", {}).get("status") == "PASS"
        and remote_by_name.get("clipboard_write", {}).get("status") == "DENIED"
    )
    xdg_ok = xdg_log.is_file() and "get default-web-browser" in xdg_log.read_text(encoding="utf-8") and "set default-web-browser" in xdg_log.read_text(encoding="utf-8")
    eval_event = any(item.get("phase") == "eval-in-tab" for item in state.events)
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=Path(__file__).resolve().parent.parent, capture_output=True, text=True)
    source_sha = commit.stdout.strip() if commit.returncode == 0 else "UNVERIFIED"
    report["sourceSha"] = source_sha
    report["installedBinarySha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
    report["pythonEvidence"] = {"remote_origin_allow_and_deny": "PASS" if remote_ok else "FAIL", "child_webview_eval_side_effect": "PASS" if eval_event else "FAIL", "isolated_xdg_settings": "PASS" if xdg_ok else "FAIL", "isolated_xvfb": "PASS", "process_exit_code": code}
    report["status"] = "PASS" if report.get("status") == "PASS" and remote_ok and eval_event and xdg_ok and code == 0 else "FAIL"
    (evidence / "summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    lines = ["# BrowserOS Native 58 installed-GUI evidence", "", f"- Result: **{report['status']}**", f"- Installed binary: `{binary}`", f"- Process exit: `{code}`", f"- Command coverage: {len(report.get('commands', []))}/58", f"- Last native progress: `{report['lastNativeProgress']}`", "", "| Command | Result | Evidence |", "|---|---|---|"]
    for item in report.get("commands", []):
        lines.append(f"| `{item.get('command','')}` | {item.get('status','FAIL')} | {str(item.get('detail','')).replace('|','/')} |")
    lines += ["", "## Native postconditions", "", "```json", json.dumps(report.get("nativePostconditions", {}), ensure_ascii=False, indent=2), "```", "", "## Remote-origin evidence", "", "```json", json.dumps(state.remote_results, ensure_ascii=False, indent=2), "```", ""]
    (evidence / "summary.md").write_text("\n".join(lines), encoding="utf-8")

    # Screenshot was captured while the actual installed client was visible.
    report["screenshot"] = screenshot_result.get("path")
    (evidence / "summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"NATIVE_58_GUI_RESULT={report['status']} commands={len(report.get('commands', []))}/58 remote={remote_ok} xdg={xdg_ok} exit={code}")
    if report["status"] != "PASS":
        print((evidence / "summary.md").read_text(encoding="utf-8"))
        print("APP_LOG_TAIL:")
        print("\n".join(app_log.read_text(encoding="utf-8", errors="replace").splitlines()[-100:]))
        return 1
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
