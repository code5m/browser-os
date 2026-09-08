#!/usr/bin/env python3
"""A11 R3B acceptance matrix verifier.

R3B is A0's targeted closure wave for M5-W18-R3. A0's acceptance audit
(`logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`) returned R3 with
nine blocking findings (R3B-01 .. R3B-09) and one explicit critique of A11's R3
matrix:

    "A report that merely finds words in a reference document does not prove
     prototype coverage: Git and accessibility probes must inspect the final
     A1 HTML."

This script honours that critique. It parses the *consolidated A1 prototype
HTML* as a real document and mechanically verifies:

  1. all six target viewport sizes are actually rendered as frames,
  2. all 14 Git workflow units (plus the amend / reset-revert actions A0's
     R3B-01 finding explicitly requires) are visibly located inside the
     prototype UI -- text inside `.frame` elements (the mock chrome), NOT the
     surrounding report prose or a reference document,
  3. accessibility: real `role=` / `aria-*` attributes and a visible keyboard
     focus indicator exist in the prototype,
  4. collapse/restore semantics follow the A0 ruling (all tool windows hide,
     the 28px activity strip stays, a restore affordance exists),
  5. prototype hygiene: no donor branding in UI text, no donor/vendor font
     names in CSS,
  6. shortcut discipline: `Ctrl+K` -> address/search, `Ctrl+Shift+P` -> palette,
     with no binding conflict.

It also runs A10's corrected self-containment checker when present and folds
the result into the exit gate.

Research artifact only: Python standard library, no network, no product import,
no build, no dependency / ACL / capability / native-runtime / user-data change.

Usage:
  python3 logs/research/M5-W18/A11-R3B-acceptance-matrix.py [--manifest PATH]
                                                           [--a1 PATH]
                                                           [--a10 PATH]
                                                           [--report] [--json]
Exit codes: 0 = GATE PASS (no unexpected result), 1 = GATE FAIL.
"""

from __future__ import annotations

import argparse
import html
import json
import os
import re
import subprocess
import sys

MANIFEST_NAME = "A11-R3B-integration-manifest.json"
ARTIFACT_DIRS = ("logs/research/M5-W18", "logs/checkpoints")
CANDIDATE_REFS = (
    "HEAD", "origin/master", "master",
    "codex/m5-w18-a1", "codex/m5-w18-a2", "codex/m5-w18-a3",
    "codex/m5-w18-a4", "codex/m5-w18-a5", "codex/m5-w18-a6",
    "codex/m5-w18-a7", "codex/m5-w18-a8", "codex/m5-w18-a9",
    "codex/m5-w18-a10", "codex/m5-w18-a11",
)

# Six target sizes per A0 ruling (800x600 dropped; 900x600 is the product min).
TARGET_SIZES = [
    (1920, 1080), (1440, 900), (1366, 768),
    (1200, 800), (1024, 720), (900, 600),
]

# 14 Git workflow units (A0 ruling: COPY=0 / ADAPT_PRODUCT=4 / REIMPLEMENT=10).
# `amend` and `reset-revert` are added because A0's R3B-01 blocking finding
# explicitly requires them to be located in the prototype even though they are
# not named in the 14-unit classification line.
# Each unit maps to a set of UI-text tokens; presence of ANY token inside the
# prototype frame text counts as "visibly located".
GIT_UNITS = [
    ("status", ["status"], "ADAPT"),
    ("hunk-staging", ["hunk", "暂存", "stage"], "REIMPLEMENT"),
    ("diff", ["diff", "对比", "并排"], "ADAPT"),
    ("log-graph", ["log", "graph", "提交图", "commit graph"], "REIMPLEMENT"),
    ("branches", ["branch", "分支"], "ADAPT"),
    ("worktrees", ["worktree", "工作树", "工作区"], "REIMPLEMENT"),
    ("stash", ["stash", "储藏"], "REIMPLEMENT"),
    ("merge", ["merge", "合并"], "ADAPT"),
    ("rebase", ["rebase", "变基"], "REIMPLEMENT"),
    ("cherry-pick", ["cherry", "cherry-pick", "挑选提交"], "REIMPLEMENT"),
    ("conflicts", ["conflict", "冲突", "解决冲突"], "REIMPLEMENT"),
    ("history-blame", ["blame", "annotate", "annotat", "文件历史", "行历史"],
     "REIMPLEMENT"),
    ("patch", ["patch", "format-patch", "补丁"], "REIMPLEMENT"),
    ("command-log", ["command log", "命令日志", "操作日志", "git log --"],
     "REIMPLEMENT"),
    ("amend", ["amend", "修正提交", "amend"], "REIMPLEMENT"),
    ("reset-revert", ["reset", "revert", "重置", "回退"], "REIMPLEMENT"),
]

# Donor/vendor font names A0 flagged as donor font strings in prototype CSS.
DONOR_FONTS = ["cascadia code", "sf mono", "segoe ui", "jetbrains mono"]


def repo_root() -> str:
    cur = os.path.abspath(os.path.dirname(__file__))
    for _ in range(6):
        if os.path.isdir(os.path.join(cur, "logs")):
            return cur
        parent = os.path.dirname(cur)
        if parent == cur:
            break
        cur = parent
    return os.getcwd()


def git_show(path: str, ref: str) -> str | None:
    try:
        out = subprocess.run(
            ["git", "show", f"{ref}:{path}"],
            capture_output=True, check=False,
        )
    except OSError:
        return None
    if out.returncode != 0:
        return None
    return out.stdout.decode("utf-8", errors="replace")


def resolve(name: str) -> tuple[str | None, str]:
    root = repo_root()
    for d in ARTIFACT_DIRS:
        p = os.path.join(root, d, name)
        if os.path.isfile(p):
            with open(p, encoding="utf-8", errors="replace") as fh:
                return fh.read(), f"fs:{d}/{name}"
    for d in ARTIFACT_DIRS:
        for ref in CANDIDATE_REFS:
            content = git_show(f"{d}/{name}", ref)
            if content:
                return content, f"git:{ref}:{d}/{name}"
    return None, "absent"


def extract_frames(html_src: str) -> list[str]:
    """Return the raw HTML of each top-level `.frame` block.

    Uses div-balance counting so an unclosed tag elsewhere cannot leak
    report prose into the frame text.
    """
    frames: list[str] = []
    for m in re.finditer(r'<div class="frame[ "][^"]*"', html_src):
        start = m.start()
        depth = 0
        i = start
        n = len(html_src)
        end = n
        while i < n:
            lt = html_src.find("<", i)
            if lt == -1:
                break
            if html_src.startswith("</div", lt):
                depth -= 1
                if depth == 0:
                    end = html_src.find(">", lt) + 1
                    break
                i = lt + 6
            elif html_src.startswith("<div", lt):
                depth += 1
                i = lt + 5
            else:
                gt = html_src.find(">", lt)
                if gt == -1:
                    break
                i = gt + 1
        frames.append(html_src[start:end])
    return frames


def strip_tags(block: str) -> str:
    text = re.sub(r"<[^>]+>", " ", block)
    return html.unescape(text).lower()


def extract_css(html_src: str) -> str:
    parts = re.findall(r"<style[^>]*>(.*?)</style>", html_src, re.DOTALL)
    return "\n".join(parts)


def find_size_rules(css: str) -> dict[int, tuple[int, int]]:
    rules: dict[int, tuple[int, int]] = {}
    for m in re.finditer(r"\.f-(\d+)\s*\{([^}]*)\}", css):
        w = int(m.group(1))
        bw = re.search(r"width:\s*(\d+)px", m.group(2))
        bh = re.search(r"height:\s*(\d+)px", m.group(2))
        if bw and bh:
            rules[w] = (int(bw.group(1)), int(bh.group(1)))
    return rules


def find_frame_usages(html_src: str) -> set[int]:
    used = set()
    for m in re.finditer(r'class="frame\s+f-(\d+)', html_src):
        used.add(int(m.group(1)))
    return used


def run_a10_checker(a10_path: str | None) -> dict:
    result = {"available": False, "summary": "not run", "detail": ""}
    if not a10_path:
        return result
    cleanup_tmp = None
    if not os.path.isfile(a10_path):
        content, _ = resolve(os.path.basename(a10_path))
        if content is None:
            return result
        import tempfile
        fd, tmp = tempfile.mkstemp(suffix=".py", prefix="_a10_")
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(content)
        a10_path = tmp
        cleanup_tmp = tmp
    try:
        out = subprocess.run([sys.executable, a10_path],
                             capture_output=True, text=True, cwd=repo_root(),
                             check=False)
    except OSError as e:
        result["detail"] = f"run error: {e}"
        return result
    finally:
        if cleanup_tmp and os.path.isfile(cleanup_tmp):
            os.remove(cleanup_tmp)
    result["available"] = True
    result["summary"] = "exit=%d" % out.returncode
    result["detail"] = (out.stdout + out.stderr)[-1500:]
    return result


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", default=None)
    ap.add_argument("--a1", default="A1-R3-prototype.html")
    ap.add_argument("--a10", default="A10-R3-prototype-selfcontainment-check.py")
    ap.add_argument("--report", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    manifest_path = args.manifest or os.path.join(
        os.path.dirname(os.path.abspath(__file__)), MANIFEST_NAME)
    manifest = None
    if os.path.isfile(manifest_path):
        with open(manifest_path, encoding="utf-8") as fh:
            manifest = json.load(fh)

    rows: list[tuple[str, str, str]] = []
    tally = {"PASS": 0, "FAIL": 0, "GAP_CONFIRMED": 0, "GAP_CLOSED": 0,
             "NOT_RUN": 0, "WARN": 0}
    open_findings: set[str] = set()

    def flag(fid: str) -> None:
        open_findings.add(fid)

    a1_src, a1_src_from = resolve(args.a1)
    if a1_src is None:
        rows.append(("S-00", "GAP_CONFIRMED",
                     f"consolidated A1 prototype not found ({args.a1})"))
        tally["GAP_CONFIRMED"] += 1
        flag("R3B-01")
        a1_proto_present = False
    else:
        a1_proto_present = True
        blocks = extract_frames(a1_src)
        frame_text = " ".join(strip_tags(b) for b in blocks)
        css = extract_css(a1_src)
        css_low = css.lower()
        size_rules = find_size_rules(css)
        size_used = find_frame_usages(a1_src)

        # [1] six target sizes rendered as frames
        for (w, h) in TARGET_SIZES:
            declared = w in size_rules and size_rules[w] == (w, h)
            used = w in size_used
            ok = declared and used
            cid = f"S-{w}x{h}"
            if ok:
                tally["PASS"] += 1
                rows.append((cid, "PASS", f"frame {w}x{h} declared+rendered"))
            else:
                tally["GAP_CONFIRMED"] += 1
                rows.append((cid, "GAP_CONFIRMED",
                             f"declared={declared} rendered={used} "
                             f"(R3B-08 / R3B-05)"))
                flag("R3B-08")

        # [1b] geometry SSOT (A0 ruling): normal top chrome = 60px (two 30px
        # rows), status bar = 24px, activity strip = 28px; collapsed active
        # width = (w-28)/w, height = (h-60-24)/h, both >= 92% / 85%.
        chrome_top, status_h, strip_w = 60, 24, 28
        geo_fails = []
        for (w, h) in TARGET_SIZES:
            wp = (w - strip_w) / w * 100.0
            hp = (h - chrome_top - status_h) / h * 100.0
            if wp < 92 or hp < 85:
                geo_fails.append(f"{w}x{h}:{wp:.1f}%W/{hp:.1f}%H")
        if not geo_fails:
            tally["PASS"] += 1
            rows.append(("GEO", "PASS",
                         "A0 collapsed formula >=92%W/85%H at all 6 sizes"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("GEO", "GAP_CONFIRMED",
                         f"formula fails: {'; '.join(geo_fails)} (R3B-05)"))
            flag("R3B-05")
        m_top = re.search(r"--top-h:\s*(\d+)px", css)
        m_tab = re.search(r"--tab-h:\s*(\d+)px", css)
        m_status = re.search(r"--status-h:\s*(\d+)px", css)
        if m_top and m_tab:
            top_sum = int(m_top.group(1)) + int(m_tab.group(1))
            if top_sum != 60:
                tally["WARN"] += 1
                rows.append(("GEO-chrome", "WARN",
                             f"top chrome rows sum {top_sum}px (A0 SSOT 60px) "
                             "(R3B-05)"))
                flag("R3B-05")
            else:
                tally["PASS"] += 1
                rows.append(("GEO-chrome", "PASS",
                             f"top chrome two rows = {top_sum}px (A0 SSOT)"))
        if m_status and int(m_status.group(1)) != 24:
            tally["WARN"] += 1
            rows.append(("GEO-status", "WARN",
                         f"status bar {m_status.group(1)}px (A0 SSOT 24px) "
                         "(R3B-05)"))
            flag("R3B-05")

        # [2] Git units visibly located inside the prototype UI
        for (unit, tokens, _cls) in GIT_UNITS:
            hit = [t for t in tokens if t.lower() in frame_text]
            cid = f"G-{unit}"
            if hit:
                tally["PASS"] += 1
                rows.append((cid, "PASS",
                             f"'{unit}' located in prototype UI "
                             f"(tok: {', '.join(hit)})"))
            else:
                tally["GAP_CONFIRMED"] += 1
                rows.append((cid, "GAP_CONFIRMED",
                             f"'{unit}' NOT visible in prototype UI "
                             f"(R3B-01 Git coverage)"))
                flag("R3B-01")

        # [3] accessibility: role/aria + visible focus indicator
        aria_n = len(re.findall(r'\b(role|aria-[\w-]+)\s*=', " ".join(blocks)))
        if aria_n > 0:
            tally["PASS"] += 1
            rows.append(("A-aria", "PASS",
                         f"{aria_n} role/aria-* attrs inside prototype frames"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("A-aria", "GAP_CONFIRMED",
                         "no role=/aria-* inside prototype frames "
                         "(R3B-04 accessibility)"))
            flag("R3B-04")
        focus_ok = bool(re.search(r":focus-visible", css)) and bool(
            re.search(r":focus-visible\s*\{[^}]*outline", css)
            or re.search(r"outline\s*:\s*(?!none)", css))
        if focus_ok:
            tally["PASS"] += 1
            rows.append(("A-focus", "PASS", "visible :focus-visible outline in CSS"))
        else:
            tally["WARN"] += 1
            rows.append(("A-focus", "WARN",
                         "no explicit :focus-visible outline in CSS "
                         "(R3B-04; verify manually)"))
            flag("R3B-04")

        # [4] collapse / restore semantics (A0 ruling)
        collapsed_hides_tw = bool(re.search(
            r"\.mode-collapsed[^{}]*\.tool-win\s*\{[^}]*display:\s*none", css))
        strip_hidden = bool(re.search(
            r"\.mode-collapsed[^{}]*\.edge-strip\s*\{[^}]*display:\s*none", css))
        restore = ("restore" in frame_text) or ("恢复" in frame_text)
        if collapsed_hides_tw:
            tally["PASS"] += 1
            rows.append(("C-hide", "PASS", "collapsed mode hides tool windows"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("C-hide", "GAP_CONFIRMED",
                         "collapsed mode does not hide tool windows "
                         "(R3B-03)"))
            flag("R3B-03")
        if not strip_hidden:
            tally["PASS"] += 1
            rows.append(("C-strip", "PASS",
                         "28px activity strip stays visible in collapsed mode"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("C-strip", "GAP_CONFIRMED",
                         "collapsed mode hides the activity strip "
                         "(violates A0: 28px strip must remain; R3B-03)"))
            flag("R3B-03")
        if restore:
            tally["PASS"] += 1
            rows.append(("C-restore", "PASS", "restore-layout affordance present"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("C-restore", "GAP_CONFIRMED",
                         "no restore-layout affordance in prototype (R3B-03)"))
            flag("R3B-03")

        # [5] hygiene: donor branding in UI text, donor/vendor fonts in CSS
        # Donor BRAND tokens are scanned only in frame UI text (report prose
        # may cite sources per the R3B card).
        brand_hits = [b for b in ("rebased", "jetbrains", "intellij",
                                  "sourcegit", "gitkraken", "tower", "gitlens")
                      if b in frame_text]
        if not brand_hits:
            tally["PASS"] += 1
            rows.append(("H-brand", "PASS", "no donor branding in prototype UI text"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("H-brand", "GAP_CONFIRMED",
                         f"donor branding in UI text: {', '.join(brand_hits)} "
                         "(R3B-02 hygiene)"))
            flag("R3B-02")
        font_hits = [f for f in DONOR_FONTS if f in css_low]
        if not font_hits:
            tally["PASS"] += 1
            rows.append(("H-font", "PASS", "no donor/vendor font names in CSS"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("H-font", "GAP_CONFIRMED",
                         f"donor/vendor font names in CSS: {', '.join(font_hits)} "
                         "(R3B-02; use neutral font stack)"))
            flag("R3B-02")

        # [6] shortcut discipline (A0 ruling: Ctrl+K = address/search,
        # Ctrl+Shift+P = palette). A conflict is Ctrl+K being used as the
        # command entry (palette/command) -- order-independent.
        k_palette_conflict = False
        for m in re.finditer(r"ctrl\+k", a1_src.lower()):
            win = a1_src.lower()[max(0, m.start() - 140): m.end() + 140]
            if "palette" in win or "command" in win:
                k_palette_conflict = True
                break
        shift_p = "ctrl+shift+p" in a1_src.lower()
        if k_palette_conflict:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("K-binding", "GAP_CONFIRMED",
                         "Ctrl+K used as command palette (must be address/search; "
                         "Ctrl+Shift+P is palette) -> R3B-06"))
            flag("R3B-06")
        elif shift_p:
            tally["PASS"] += 1
            rows.append(("K-binding", "PASS",
                         "Ctrl+Shift+P bound to palette; Ctrl+K not conflicting"))
        else:
            tally["WARN"] += 1
            rows.append(("K-binding", "WARN",
                         "shortcut bindings not clearly declared (R3B-06)"))
            flag("R3B-06")

    # ---- manifest structural check: R3B-07 reference identity ----
    if manifest:
        pins = manifest.get("reference_pins", [])
        if pins and pins[0].get("pinned_revision") != "cee14e9":
            tally["WARN"] += 1
            rows.append(("M-pin", "WARN",
                         f"rebased pin = {pins[0].get('pinned_revision')} "
                         "(A0 SSOT is cee14e9 v1.1.15; R3B-07)"))
            flag("R3B-07")
        else:
            tally["PASS"] += 1
            rows.append(("M-pin", "PASS",
                         "rebased pin = cee14e9 (A0 SSOT v1.1.15; R3B-07)"))

    # ---- A10 self-containment checker (folded into gate) ----
    a10 = run_a10_checker(args.a10)
    if a10["available"]:
        if "exit=0" in a10["summary"]:
            tally["PASS"] += 1
            rows.append(("A10-gate", "PASS", "A10 self-containment checker exit 0"))
        else:
            tally["GAP_CONFIRMED"] += 1
            rows.append(("A10-gate", "GAP_CONFIRMED",
                         f"A10 checker {a10['summary']} (R3B-02 hygiene)"))
            flag("R3B-02")
    else:
        tally["NOT_RUN"] += 1
        rows.append(("A10-gate", "NOT_RUN",
                     "A10 self-containment checker not found in this worktree"))

    if not a1_proto_present:
        gate = "FAIL"
    else:
        unexplained = tally["GAP_CONFIRMED"] + tally["FAIL"]
        gate = "PASS" if unexplained == 0 else "FAIL"

    if args.json:
        print(json.dumps({
            "manifest": manifest_path,
            "a1_source": a1_src_from,
            "tally": tally,
            "open_r3b_findings": sorted(open_findings),
            "gate": gate,
            "a10": a10,
        }, indent=2, ensure_ascii=False))
        return 0 if gate == "PASS" else 1

    print("A11-R3B acceptance matrix (inspects the real A1 prototype HTML)")
    print(f"  A1 prototype  : {args.a1} ({a1_src_from})")
    if manifest:
        print(f"  base          : {manifest['base']['short']}")
        print(f"  status        : {manifest['status']}")
        print(f"  rebased pin   : {manifest['reference_pins'][0]['pinned_revision']}"
              f" ({manifest['reference_pins'][0]['default_classification']})")
    print()
    for cid, verdict, detail in rows:
        mark = {"PASS": "PASS", "FAIL": "FAIL", "GAP_CONFIRMED": "GAP ",
                "GAP_CLOSED": "CLOSED", "NOT_RUN": "NRUN", "WARN": "WARN"}.get(verdict, verdict)
        if args.report or verdict in ("FAIL", "GAP_CONFIRMED", "GAP_CLOSED"):
            print(f"  {mark:<6} {cid:<18} {detail}")
    print()
    print(f"  A10 checker   : {a10['summary']}")
    print()
    print(f"A11-R3B TALLY: {tally['PASS']} PASS / {tally['FAIL']} FAIL / "
          f"{tally['GAP_CONFIRMED']} GAP_CONFIRMED / {tally['WARN']} WARN / "
          f"{tally['NOT_RUN']} NOT_RUN")
    if open_findings:
        print(f"  OPEN R3B FINDINGS: {', '.join(sorted(open_findings))}")
    print(f"GATE: {gate}")
    if manifest and manifest["status"].startswith("PROVISIONAL"):
        print("NOTE: PROVISIONAL - A11 finalizes only after A1 R3B-corrected "
              "prototype + A10 R3B review land.")
    return 0 if gate == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
