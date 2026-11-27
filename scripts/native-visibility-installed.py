#!/usr/bin/env python3
"""Manual-only installed BrowserOS desktop-pixel fixture, Linux X11/Windows.

Run only in a disposable graphical login/VM with an unused BrowserOS profile.
No CI, installation, package build, process killing, or real-account capture.
The native fixture drives real windows; only this separate observer can attest
pixels. This is not physical scanout, general GUI acceptance, or source signing.
Dependencies (operator installed): Python 3.10+, Pillow, psutil.
"""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import http.server
import itertools
import json
import math
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import tempfile
import threading
import time

STAGES = [
    ("baseline", [None, None], []),
    ("tab_a_show", ["a", None], ["a"]),
    ("tab_b_switch", [None, "b"], ["a", "b"]),
    ("tab_b_move", ["b", None], ["a", "b"]),
    ("tab_a_return", [None, "a"], ["a", "b"]),
    ("tabs_hidden", [None, None], ["a", "b"]),
    ("grid_show", ["c", "d"], ["a", "b", "c", "d"]),
    ("grid_hidden", [None, None], ["a", "b", "c", "d"]),
    ("closed", [None, None], []),
]
HEX64 = re.compile(r"^[0-9a-f]{64}$")
HEX40 = re.compile(r"^[0-9a-f]{40}$")
EXE = "mvp-browser-os.exe" if sys.platform == "win32" else "mvp-browser-os"
IDENTIFIER = "com.jizhijiandan.mvp"
MAX_JSON = 64 * 1024
WINDOWS_PAYLOAD = {"mvp-browser-os.exe", "windows-lsp/rust-analyzer.exe",
                   "windows-lsp/runtime.json", "windows-lsp/LICENSE-MIT",
                   "windows-lsp/LICENSE-APACHE", "windows-lsp/Cargo.lock"}


def demand(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as fh:
        for block in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def read_json(path):
    with Path(path).open("rb") as fh:
        raw = fh.read(MAX_JSON + 1)
    demand(len(raw) <= MAX_JSON, "JSON evidence exceeds bound")
    return json.loads(raw.decode("utf-8-sig"))


def command(*args, **kw):
    return subprocess.check_output(args, text=True, timeout=30, **kw).strip()


def verify_install(args, exe):
    # The build record is externally supplied trusted evidence, not a
    # self-issued signature or a guess based on the checkout's HEAD.
    record = read_json(args.build_record)
    source = record.get("source_sha", "")
    expected = record.get("installed_exe_sha256", "")
    demand(HEX40.fullmatch(source) and HEX64.fullmatch(expected),
           "build record must identify source SHA and installed executable SHA256")
    demand(Path(__file__).resolve() == (args.repo.resolve() / "scripts/native-visibility-installed.py").resolve(),
           "observer script must belong to the reviewed source checkout")
    demand(command("git", "rev-parse", "HEAD", cwd=args.repo) == source,
           "checkout does not match source SHA in build record")
    demand(not command("git", "status", "--porcelain", "--untracked-files=no", cwd=args.repo),
           "tracked source tree must be clean")
    demand(exe.name.lower() == EXE.lower() and exe.is_file() and digest(exe) == expected,
           "installed executable does not match reviewed build record")
    result = {"source_sha": source, "installed_exe_sha256": expected,
              "build_record_sha256": digest(args.build_record),
              "observer_sha256": digest(Path(__file__)),
              "source_provenance": "externally supplied build record; not signature verification"}
    if sys.platform == "linux":
        demand(exe == Path("/usr/bin/mvp-browser-os").resolve(), "must use installed /usr/bin executable")
        demand(args.deb is not None and args.windows_payload_report is None,
               "Linux requires the exact reviewed .deb")
        package_hash = record.get("package_sha256", "")
        demand(HEX64.fullmatch(package_hash) and digest(args.deb) == package_hash,
               "package hash mismatch")
        package = command("dpkg-deb", "-f", str(args.deb), "Package")
        version = command("dpkg-deb", "-f", str(args.deb), "Version")
        installed = command("dpkg-query", "-W", "-f=${Status}\t${Version}", package)
        demand(installed == "install ok installed\t" + version, "installed package/version mismatch")
        owner = command("dpkg-query", "-S", "/usr/bin/mvp-browser-os")
        demand(owner.split(": ", 1)[0].split(":")[0] == package, "installed file is not package owned")
        # Extract ONLY the executable to a pipe. No package scripts run.
        # No shell interpolations or archive paths are written to disk.
        tar = subprocess.Popen(["dpkg-deb", "--fsys-tarfile", str(args.deb)], stdout=subprocess.PIPE)
        try:
            import tarfile
            matches = 0
            with tarfile.open(fileobj=tar.stdout, mode="r|") as archive:
                for member in archive:
                    if member.name in ("./usr/bin/mvp-browser-os", "usr/bin/mvp-browser-os"):
                        demand(member.isfile() and 0 < member.size <= 1024**3, "invalid package executable")
                        h = hashlib.sha256()
                        with archive.extractfile(member) as fp:
                            for block in iter(lambda: fp.read(1024 * 1024), b""):
                                h.update(block)
                        demand(h.hexdigest() == expected, "package executable hash mismatch")
                        matches += 1
            demand(tar.wait(timeout=30) == 0 and matches == 1, "cannot prove package executable")
        finally:
            if tar.poll() is None:
                tar.kill()
                tar.wait()
            if tar.stdout is not None:
                tar.stdout.close()
        result.update({"package": package, "package_version": version, "package_sha256": package_hash})
    else:
        demand(args.windows_payload_report is not None and args.deb is None,
               "Windows requires existing NSIS package-payload.json")
        report = read_json(args.windows_payload_report)
        files = report.get("files", {})
        demand(report.get("status") == "PACKAGE_PAYLOAD_PASS_GUI_PENDING"
               and report.get("source_sha") == source and set(files) == WINDOWS_PAYLOAD,
               "NSIS package payload evidence is missing or inconsistent")
        demand(files[EXE].lower() == expected, "NSIS installed executable hash mismatch")
        demand((exe.parent / "uninstall.exe").is_file(), "NSIS installed directory required")
        for name, sha in files.items():
            path = exe.parent / name
            demand(HEX64.fullmatch(sha.lower()) and path.is_file() and not path.is_symlink()
                   and digest(path) == sha.lower(), "NSIS payload changed: " + name)
        result["windows_payload_report_sha256"] = digest(args.windows_payload_report)
    return result


def palettes_for_run():
    # 16 different well-separated, run-local colors. Avoid common UI
    # neutrals and retain enough channel separation for display tolerance.
    pool = [rgb for rgb in itertools.product((24, 96, 168, 240), repeat=3)
            if max(rgb) - min(rgb) >= 144]
    colors = secrets.SystemRandom().sample(pool, 16)
    return {name: colors[i*4:i*4+4] for i, name in enumerate("abcd")}


def fixture_page(colors):
    blocks = "".join('<div style="background:rgb(%d,%d,%d)"></div>' % c for c in colors)
    return ('<!doctype html><meta charset="utf-8"><title>BrowserOS visibility fixture</title>'
            '<style>html,body{width:100%;height:100%;margin:0;overflow:hidden}'
            'body{display:grid;grid-template-columns:1fr 1fr;grid-template-rows:1fr 1fr}</style>'
            + blocks).encode("ascii")


def rect_box(rect):
    demand(set(rect) == {"x", "y", "w", "h"}
           and all(type(v) in (int, float) and math.isfinite(v) for v in rect.values()),
           "invalid native physical rectangle")
    x, y, w, h = (math.trunc(rect[k]) for k in ("x", "y", "w", "h"))
    demand(w >= 160 and h >= 120, "fixture rectangle too small")
    return (x, y, x+w, y+h)


def close_color(a, b):
    return max(abs(a[i]-b[i]) for i in range(3)) <= 18


def score_pixels(image, boxes, markers, palettes, Image):
    """Check all quadrants, and scan the remaining screen for ghost markers.

    Only cropped expected fixture regions are saved. Full desktop pixels
    never leave the observer's memory. The caller requires a disposable login.
    """
    width, height = image.size
    demand(320 <= width <= 7680 and 240 <= height <= 4320, "unsupported screen size")
    for x0, y0, x1, y1 in boxes:
        demand(0 <= x0 < x1 <= width and 0 <= y0 < y1 <= height, "fixture outside screen")
        # The global sample grid must resolve a ghost of an entire quadrant
        # with substantial margin above the absence threshold.
        demand((x1-x0)*(y1-y0)*320*180/(4*width*height) >= 32,
               "desktop sampling density insufficient for this fixture")
    # Absence alone must not accept a blank/failed desktop capture during
    # baseline, hidden, or closed stages. This is a conservative fixture
    # liveness floor, not a general compositor/scanout guarantee.
    overview = image.resize((32, 24), Image.Resampling.NEAREST)
    demand(any(hi-lo > 8 for lo, hi in overview.getextrema()),
           "uniform or near-uniform desktop capture")
    crops = [image.crop(box) for box in boxes]
    metrics = []
    for crop, marker in zip(crops, markers):
        if marker is None:
            metrics.append(None)
            continue
        w, h = crop.size
        ratios = []
        for i, color in enumerate(palettes[marker]):
            # Sample interior of each quadrant, not antialiased borders.
            x, y = i % 2, i // 2
            patch = crop.crop((int(w*(x/2+.12)), int(h*(y/2+.12)),
                               int(w*(x/2+.38)), int(h*(y/2+.38))))
            sample = patch.resize((24, 24), Image.Resampling.NEAREST)
            ratios.append(sum(close_color(p, color) for p in sample.getdata()) / 576)
        metrics.append(ratios)
    # Exclude ONLY the region in which that marker is expected. For a hidden
    # marker scan the whole observed desktop, not merely its old rectangle.
    sample = image.resize((320, 180), Image.Resampling.NEAREST)
    pixels = list(sample.getdata())
    ghosts = {}
    for name, colors in palettes.items():
        occupied = boxes[markers.index(name)] if name in markers else None
        counts = Counter()
        for j, pixel in enumerate(pixels):
            sx = (j % 320 + .5) * width / 320
            sy = (j // 320 + .5) * height / 180
            if occupied and occupied[0]-3 <= sx <= occupied[2]+3 and occupied[1]-3 <= sy <= occupied[3]+3:
                continue
            counts[pixel] += 1
        hits = [sum(n for pixel, n in counts.items() if close_color(pixel, color)) for color in colors]
        ghosts[name] = hits
    # Require 4/4 quadrants and reject a substantial signature (3 colors)
    # anywhere outside the admitted region; also reject a single broad
    # leftover color patch when it greatly exceeds the initial baseline.
    correct = all(row is None or min(row) >= .95 for row in metrics)
    return correct, metrics, ghosts, crops


def frame_matches(correct, ghosts, baseline):
    if not correct:
        return False
    if baseline is None:
        return all(sorted(row)[1] <= 8 and max(row) <= 45 for row in ghosts.values())
    return all(
        sorted(max(0, v-b) for v, b in zip(row, baseline[name]))[1] <= 8
        and max(max(0, v-b) for v, b in zip(row, baseline[name])) <= 45
        for name, row in ghosts.items())


class Observer:
    def __init__(self, nonce, palettes, output, exe, expected_hash, Image, ImageGrab, psutil):
        self.nonce, self.palettes, self.output = nonce, palettes, output
        self.exe, self.expected_hash = exe, expected_hash
        self.Image, self.ImageGrab, self.psutil = Image, ImageGrab, psutil
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.launcher = None
        self.records, self.identities = [], {}
        self.owners = {}
        self.main = None
        self.boxes = None
        self.screen_size = None
        self.baseline = None
        self.finished = None
        self.failure = None

    def verify_process(self, pid):
        demand(type(pid) is int and pid > 0 and self.launcher is not None, "invalid process identity")
        p = self.psutil.Process(pid)
        identity = (pid, p.create_time())
        demand(pid == self.launcher.pid or self.launcher.pid in [q.pid for q in p.parents()],
               "native process is not in this launched application tree")
        demand(Path(p.exe()).resolve() == self.exe, "native process executable path mismatch")
        if pid in self.identities:
            demand(self.identities[pid] == identity, "native PID recycled during review")
        else:
            # Linux hashes the actual running image. Windows checks the
            # process image path and current installed bytes (locked image).
            binary = Path("/proc") / str(pid) / "exe" if sys.platform == "linux" else self.exe
            demand(digest(binary) == self.expected_hash, "running executable hash mismatch")
            self.identities[pid] = identity
        return identity

    def original_process_retired(self, identity):
        pid, created = identity
        try:
            p = self.psutil.Process(pid)
            return p.create_time() != created or not p.is_running()
        except self.psutil.NoSuchProcess:
            return True

    def grab(self):
        if sys.platform == "linux":
            return self.ImageGrab.grab(xdisplay=os.environ["DISPLAY"]).convert("RGB")
        return self.ImageGrab.grab(all_screens=False, include_layered_windows=True).convert("RGB")

    def handle_stage(self, body):
        demand(not self.stop.is_set() and self.finished is None, "observer already terminal")
        n = len(self.records)
        demand(n < len(STAGES), "unexpected extra stage")
        phase, markers, expected_names = STAGES[n]
        demand(type(body.get("sequence")) is int and body["sequence"] == n+1
               and body.get("phase") == phase and body.get("markers") == markers
               and body.get("backend") == "default_isolated_kernel_ready", "wrong stage or backend")
        main = self.verify_process(body["pid"])
        demand(Path(body["executable"]).resolve() == self.exe, "native executable identity mismatch")
        if self.main is None:
            self.main = main
        demand(main == self.main, "main owner changed")
        boxes = [rect_box(r) for r in body["regions"]]
        demand(len(boxes) == 2 and boxes[0][2] <= boxes[1][0], "overlapping test regions")
        if self.boxes is None:
            self.boxes = boxes
        demand(boxes == self.boxes, "main window moved during evidence capture")
        receipts = body["receipts"]
        demand(isinstance(receipts, list) and len(receipts) == len(expected_names), "native receipt count mismatch")
        demand(len({r["pid"] for r in receipts}) == len(receipts)
               and len({r["index"] for r in receipts}) == len(receipts),
               "renderer routes or processes are not independent")
        for name, item in zip(expected_names, receipts):
            self.verify_process(item["pid"])
            key = (item["index"], item["pid"], item["spawn_epoch"], item["generation"])
            revision = item["navigation_revision"]
            demand(all(type(v) is int and v >= 0 for v in key) and all(v > 0 for v in key[1:])
                   and key[0] <= 0xffffffff and key[1] <= 0xffffffff
                   and type(revision) is int and 0 <= revision <= 0xffffffffffffffff,
                   "invalid native owner tuple")
            demand(item["pid"] != self.main[0], "renderer shares main process")
            if name in "ab":
                demand(item["index"] & (1 << 31), "ordinary tab is not an isolated renderer")
            else:
                demand(item["index"] == ("cd".index(name)), "unexpected grid route")
            if name not in self.owners:
                demand(all(other[0] != key[0] for other in self.owners.values()),
                       "native route reused for another fixture")
                self.owners[name] = key
            demand(self.owners[name] == key, "renderer replaced during fixture")
            receipt = item["receipt"]
            visible = name in markers
            demand(receipt["visible"] is visible and
                   receipt["source"] == ("windows_dwm" if sys.platform == "win32" else "x11_server") and
                   receipt["compositor_barrier"] is (sys.platform == "win32"),
                   "wrong native visibility evidence")
            if visible:
                actual = rect_box(receipt["rect"])
                expected = boxes[markers.index(name)]
                demand(all(abs(a-b) <= 4 for a, b in zip(actual, expected)), "native bounds differ")
            else:
                demand(receipt["rect"] is None, "hidden receipt contains visible bounds")
        # Three consecutive desktop samples, not a single lucky frame.
        deadline = time.monotonic() + 22
        samples = []
        while time.monotonic() < deadline and len(samples) < 3:
            demand(not self.stop.is_set(), "observer capture cancelled")
            image = self.grab()
            demand(not self.stop.is_set() and time.monotonic() < deadline,
                   "desktop capture exceeded observation deadline")
            if self.screen_size is None:
                self.screen_size = image.size
            demand(image.size == self.screen_size, "desktop capture geometry changed")
            ok, metrics, ghosts, crops = score_pixels(image, boxes, markers, self.palettes, self.Image)
            if phase == "baseline":
                # Reject an accidental matching palette in every baseline
                # frame, not just relative to a possibly contaminated first.
                ok = frame_matches(ok, ghosts, None)
            else:
                demand(self.baseline is not None, "no baseline")
                ok = frame_matches(ok, ghosts, self.baseline)
            if not ok:
                samples.clear()
            else:
                samples.append((metrics, ghosts, crops))
            if len(samples) < 3:
                time.sleep(.25)
        demand(len(samples) == 3 and not self.stop.is_set() and time.monotonic() < deadline,
               "desktop pixels did not match phase: " + phase)
        # The actual OS process identities must survive the capture, not
        # merely the parent manager's before/after cached renderer routes.
        demand(self.verify_process(body["pid"]) == self.main, "main process changed during capture")
        for item in receipts:
            self.verify_process(item["pid"])
        if phase == "closed":
            for _, pid, _, _ in self.owners.values():
                demand(self.original_process_retired(self.identities[pid]),
                       "original renderer still exists at closed phase: %d" % pid)
        if phase == "baseline":
            # Keep the minimum observed baseline per color: incidental
            # transient colors cannot grant additional ghost tolerance.
            self.baseline = {name: [min(row[1][name][i] for row in samples) for i in range(4)]
                             for name in self.palettes}
        # Persist only bounded fixture crops, not the full desktop image.
        # Evidence stays in the operator's fresh local output directory.
        files = []
        for sample, (_, _, crops) in enumerate(samples, 1):
            for i, crop in enumerate(crops):
                path = self.output / ("%02d-%s-%d-%d.png" % (n+1, phase, sample, i))
                demand(not self.stop.is_set(), "observer capture cancelled")
                with path.open("xb") as fh:
                    crop.save(fh, format="PNG")
                files.append({"file": path.name, "sha256": digest(path)})
        demand(not self.stop.is_set(), "observer capture cancelled")
        self.records.append({"sequence": n+1, "phase": phase, "markers": markers,
                             "native": receipts, "screen_regions": boxes,
                             "desktop_size": self.screen_size,
                             "samples": [{"quadrants": q, "outside_palette_pixels": g}
                                         for q, g, _ in samples], "crops": files})

    def handle_finished(self, body):
        demand(not self.stop.is_set() and self.finished is None, "duplicate terminal result")
        demand(body.get("ok") is True and len(self.records) == len(STAGES)
               and self.main is not None and type(body.get("pid")) is int
               and self.verify_process(body["pid"]) == self.main,
               "native fixture incomplete: " + str(body.get("error", ""))[:240])
        self.finished = body

    def freeze(self):
        # Native capture APIs can stall. A bounded snapshot must not race a
        # daemon handler or wait forever for its lock. Incomplete snapshots
        # cannot be used to publish PASS.
        self.stop.set()
        if not self.lock.acquire(timeout=2):
            return None
        try:
            return {"stages": list(self.records), "palette": self.palettes,
                    "native_terminal": self.finished, "observer_error": self.failure,
                    "identities": tuple(self.identities.values())}
        finally:
            self.lock.release()


def serve(observer):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def do_GET(self):
            name = self.path.removeprefix("/fixture/")
            if self.path == "/fixture/" + name and name in observer.palettes:
                data = fixture_page(observer.palettes[name])
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Cache-Control", "no-store")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)
            else:
                self.send_error(404)

        def do_POST(self):
            if self.headers.get("Authorization") != "Bearer " + observer.nonce:
                self.send_error(403)
                return
            try:
                size = int(self.headers.get("Content-Length", "-1"))
                demand(0 < size <= MAX_JSON, "invalid observer payload")
                self.connection.settimeout(4)
                body = json.loads(self.rfile.read(size))
                with observer.lock:
                    demand(not observer.stop.is_set() and observer.failure is None,
                           "observer already closed or failed")
                    if self.path == "/stage":
                        observer.handle_stage(body)
                    elif self.path == "/finished":
                        observer.handle_finished(body)
                    else:
                        raise RuntimeError("unknown fixture route")
                self.send_response(204)
                self.end_headers()
            except Exception as error:
                with observer.lock:
                    if not observer.stop.is_set() and observer.failure is None:
                        observer.failure = str(error)[:500]
                self.send_error(409, "fixture rejected")
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    return server


def environment(scratch):
    env = dict(os.environ)
    for key in ("BROWSEROS_EXPERIMENTAL_ISOLATED_TABS", "GRID_SELFTEST", "GRID_GUI_REGRESSION",
                "NATIVE_58_GUI_REVIEW", "M0_RUN_ID", "MVP_WORKBENCH_SMOKE"):
        env.pop(key, None)
    if sys.platform == "linux":
        for key, folder in (("HOME", "home"), ("XDG_DATA_HOME", "data"),
                            ("XDG_CONFIG_HOME", "config"), ("XDG_CACHE_HOME", "cache")):
            path = scratch / folder
            path.mkdir(mode=0o700)
            env[key] = str(path)
        target = Path(env["XDG_DATA_HOME"]) / IDENTIFIER
    else:
        # No APPDATA override. Native preflight independently checks Tauri's
        # actual Known Folder resolution before workspace preparation.
        target = Path(env["APPDATA"]) / IDENTIFIER
    demand(target.parent.is_dir() and not target.exists() and not target.is_symlink(),
           "existing BrowserOS profile: use a fresh disposable OS account")
    env["BROWSEROS_VISIBILITY_REVIEW"] = "1"
    env["BROWSEROS_VISIBILITY_REVIEW_DATA_ROOT"] = str(target)
    return env


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--installed-exe", type=Path, required=True)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--build-record", type=Path, required=True)
    parser.add_argument("--deb", type=Path)
    parser.add_argument("--windows-payload-report", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--disposable-desktop", action="store_true",
                        help="attest a dedicated graphical VM/login with no other user content")
    args = parser.parse_args()
    demand(args.disposable_desktop, "explicit disposable desktop consent required")
    demand(sys.platform in ("linux", "win32"), "Linux X11 or Windows only")
    from PIL import Image, ImageGrab, features
    import psutil
    if sys.platform == "linux":
        demand(os.environ.get("DISPLAY") and not os.environ.get("WAYLAND_DISPLAY")
               and os.environ.get("XDG_SESSION_TYPE", "x11").lower() == "x11"
               and features.check_feature("xcb"), "native X11 display and Pillow XCB required")
    else:
        import ctypes
        user32 = ctypes.windll.user32
        demand(user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4)) != 0,
               "cannot establish per-monitor physical pixel coordinates")
        demand(user32.GetSystemMetrics(80) == 1, "single-monitor fixture required")
    demand(not any((p.info.get("name") or "").lower() == EXE.lower()
                   for p in psutil.process_iter(["name"])), "another BrowserOS instance exists")
    exe = args.installed_exe.resolve(strict=True)
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    output = args.output.resolve()
    # Keep a temporary profile on failure; never remove data while a renderer
    # may still be running. The operator owns cleanup of the disposable login.
    scratch = Path(tempfile.mkdtemp(prefix="browseros-visibility-")).resolve()
    evidence = {"status": "GUI_PENDING", "platform": sys.platform, "stages": [],
                "physical_scanout_verified": False, "full_gui_pass": False,
                "disposable_profile_scratch": str(scratch)}
    server = observer = process = server_thread = frozen = None
    try:
        evidence["install"] = verify_install(args, exe)
        env = environment(scratch)
        nonce = secrets.token_hex(16)
        observer = Observer(nonce, palettes_for_run(), output, exe,
                            evidence["install"]["installed_exe_sha256"], Image, ImageGrab, psutil)
        server = serve(observer)
        env["BROWSEROS_VISIBILITY_REVIEW_BASE"] = "http://127.0.0.1:%d/" % server.server_port
        env["BROWSEROS_VISIBILITY_REVIEW_NONCE"] = nonce
        with (output / "native.log").open("xb") as log:
            process = subprocess.Popen([str(exe)], env=env, stdout=log, stderr=subprocess.STDOUT)
            observer.launcher = process
            server_thread = threading.Thread(target=server.serve_forever, daemon=True)
            server_thread.start()
            try:
                evidence["launcher_exit_code"] = process.wait(timeout=360)
            except subprocess.TimeoutExpired:
                raise RuntimeError("native launcher did not exit; close the disposable VM normally")
        frozen = observer.freeze()
        demand(frozen is not None, "observer did not quiesce after native exit")
        demand(frozen["observer_error"] is None, frozen["observer_error"] or "observer failed")
        demand(frozen["native_terminal"] is not None and frozen["native_terminal"].get("ok") is True
               and len(frozen["stages"]) == len(STAGES) and process.returncode == 0,
               "native driver did not finish all pixel stages and exit cleanly")
        for identity in frozen["identities"]:
            demand(observer.original_process_retired(identity),
                   "original native process still exists: %d" % identity[0])
        demand(digest(exe) == observer.expected_hash, "installed executable changed during run")
        evidence["status"] = "DESKTOP_VISIBILITY_FIXTURE_PASS"
    except Exception as error:
        evidence["status"] = "GUI_PENDING"
        evidence["error"] = str(error)[:500]
    finally:
        if server is not None:
            if server_thread is not None and server_thread.is_alive():
                server.shutdown()
            server.server_close()
        if observer is not None:
            if frozen is None:
                frozen = observer.freeze()
            if frozen is None:
                evidence["status"] = "GUI_PENDING"
                evidence["observer_error"] = "observer snapshot unavailable; handler still active"
            else:
                evidence.update({key: value for key, value in frozen.items() if key != "identities"})
        if process is not None and process.poll() is None:
            evidence["cleanup_pending"] = True
        # Retain the disposable profile even on success. A separate manual
        # cleanup can verify no unsampled descendants or broker remain.
        with (output / "summary.json").open("x", encoding="utf-8") as fh:
            json.dump(evidence, fh, ensure_ascii=False, indent=2)
            fh.write("\n")
    print(evidence["status"], output / "summary.json")
    return 0 if evidence["status"] == "DESKTOP_VISIBILITY_FIXTURE_PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
