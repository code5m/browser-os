#!/usr/bin/env python3
# A10 / M5-W18-R3B research asset (pure stdlib, read-only, zero product impact).
#
# Purpose: make the R3B exit gate MECHANICALLY checkable instead of asserted, so that
# A1-A9 can self-check before committing and A10/A11 can re-run the identical audit.
#
# It encodes A0's R3 acceptance-audit rulings (logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md)
# as nine probes:
#
#   P1 SELF_CONTAINMENT     delegate to A10-R3-prototype-selfcontainment-check.py (crash => FAIL)
#   P2 GEOMETRY_CANON       canonical inner-viewport arithmetic + stale-convention scan
#   P3 SIZE_COVERAGE        six acceptance sizes present, 800x600 dropped
#   P4 GIT_UNITS            A1 prototype visibly locates the 14 matrix units + amend + reset/revert
#   P5 ACCESSIBILITY        role / aria-* / tabindex / focus-visible present in prototypes
#   P6 SHORTCUTS            Ctrl+K = address/search, Ctrl+Shift+P = palette, no collision
#   P7 REFERENCE_PINS       cee14e9 is the SSOT pin, 2896562e only as a demoted snapshot, COPY=0
#   P8 CLASSIFICATION       A7 tally equals its own 14-row table (ADAPT=4 / REIMPLEMENT=10 / COPY=0)
#   P9 COLLAPSE_SEMANTICS   Collapse All hides every tool window, pinned included
#
# Rules of engagement: this script never edits anything, never runs the product, and never
# reaches the network. Every finding carries file:line evidence.
#
# Usage:
#   python3 A10-R3B-closure-audit.py                # audit logs/research/M5-W18 next to this file
#   python3 A10-R3B-closure-audit.py --base DIR     # audit another artifact directory
#   python3 A10-R3B-closure-audit.py --self-test    # prove every probe detects its violation class
#
# Exit code: 0 = every probe PASS, 1 = at least one FAIL (a crashing probe is a FAIL).

import argparse
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))

# ---------------------------------------------------------------- canonical constants (A0 ruling)
TOP_ROWS = 2
TOP_ROW_PX = 30
TOP_CHROME_PX = TOP_ROWS * TOP_ROW_PX          # exactly 60px, OS titlebar excluded
STATUS_PX = 24
SIDE_STRIP_PX = 28
HEIGHT_SHARE = 0.85
WIDTH_SHARE = 0.92
SIZES = [(1920, 1080), (1440, 900), (1366, 768), (1200, 800), (1024, 720), (900, 600)]
DROPPED_SIZE = (800, 600)

# product facts that anchor the ruling (src-tauri/src/main.rs)
PRODUCT_MIN_INNER = (900, 600)
PRODUCT_DEFAULT_INNER = (1200, 800)
PRODUCT_STATUS_PX_TODAY = 26                    # src/components/layout/StatusBar.vue

# ---------------------------------------------------------------- artifacts
A1_PROTO = "A1-R3-prototype.html"
A1_REPORT = "A1-R3-browser-workbench.md"
A2_REPORT = "A2-R3-shell-density-audit.md"
A2_PROTO = "A2-R3-density-replica.html"
A3_REPORT = "A3-R3-toolwindow-disclosure-20260908.md"
A3_PROTO = "A3-R3-toolwindow-prototype.html"
A3_SM = "A3-R3-toolwindow-state-machine.mjs"
A4_REPORT = "A4-R3-shell-state-persistence-contract.md"
A4_SM = "A4-R3-shell-state-prototype.mjs"
A5_PROTO = "A5-R3-prototype.html"
A6_REGISTRY = "A6-R3-context-menu-command-registry.md"
A7_MAP = "A7-R3-git-workflow-reference-map.md"
A8_FRAMES = "A8-R3-visual-density-frames.html"
SELFCONTAINMENT = "A10-R3-prototype-selfcontainment-check.py"

PROTOTYPES = [A1_PROTO, A2_PROTO, A3_PROTO, A5_PROTO, A8_FRAMES]

# 14 units of the A7 matrix + the two units the R3B exit gate names separately.
GIT_UNITS = [
    ("status", [r"\bstatus\b", r"状态"]),
    ("hunk staging", [r"\bhunk\b", r"块暂存", r"分块暂存"]),
    ("diff", [r"\bdiff\b", r"差异"]),
    ("log graph", [r"log\s*graph", r"提交图", r"\bgraph\b"]),
    ("branches", [r"\bbranch", r"分支"]),
    ("worktrees", [r"worktree", r"工作树"]),
    ("stash", [r"\bstash\b", r"贮藏", r"暂存区快照"]),
    ("merge", [r"\bmerge\b", r"合并"]),
    ("rebase / interactive", [r"\brebase\b", r"变基"]),
    ("cherry-pick", [r"cherry[- ]?pick", r"拣选"]),
    ("conflicts", [r"conflict", r"冲突"]),
    ("history / blame", [r"\bblame\b", r"追溯", r"文件历史"]),
    ("patch", [r"\bpatch\b", r"补丁"]),
    ("command log", [r"command\s*log", r"命令日志", r"命令历史"]),
    ("amend", [r"\bamend\b", r"修补提交", r"追加提交"]),
    ("reset / revert", [r"\brevert\b", r"\breset\b", r"回退", r"撤销提交"]),
]

# conventions superseded by the A0 ruling; their appearance as a normative number is a divergence
STALE_CHROME_RX = re.compile(r"(?<![0-9])(68|65|89|94)\s*px", re.I)
STALE_CHROME_CONTEXT = re.compile(r"chrome|顶部|top\s|status|状态栏|合计|total", re.I)


class Finding:
    def __init__(self, severity, probe, message, evidence=""):
        self.severity = severity
        self.probe = probe
        self.message = message
        self.evidence = evidence

    def __repr__(self):
        return f"[{self.severity}] {self.probe}: {self.message}"


SEV_ORDER = {"HIGH": 0, "MEDIUM": 1, "LOW": 2, "INFO": 3}


def read(base, name):
    path = os.path.join(base, name)
    if not os.path.exists(path):
        return None
    return open(path, encoding="utf-8", errors="replace").read()


def lines_of(text, rx):
    """Return [(lineno, stripped_line)] for every line matching rx."""
    out = []
    for i, line in enumerate(text.splitlines(), 1):
        if rx.search(line):
            out.append((i, line.strip()[:120]))
    return out


def has(text, pattern):
    return bool(re.search(pattern, text, re.I))


def count_sizes(text):
    """Return {(w,h): hits} using a separator-tolerant match (x, X, *, U+00D7)."""
    found = {}
    for w, h in SIZES + [DROPPED_SIZE]:
        found[(w, h)] = len(re.findall(rf"{w}\s*[x×X*]\s*{h}", text))
    return found


# ---------------------------------------------------------------------------- P1
def probe_self_containment(base):
    findings = []
    script = os.path.join(base, SELFCONTAINMENT)
    if not os.path.exists(script):
        return [Finding("HIGH", "P1", f"{SELFCONTAINMENT} is missing; the R3B gate cannot run")]
    globbed = sorted(
        os.path.join(base, f) for f in os.listdir(base) if f.endswith(".html")
    )
    if not globbed:
        return [Finding("MEDIUM", "P1", "no prototype .html found to check")]
    try:
        proc = subprocess.run(
            [sys.executable, script] + globbed,
            capture_output=True, text=True, timeout=120,
        )
    except Exception as exc:
        return [Finding("HIGH", "P1", f"self-containment gate crashed: {type(exc).__name__}: {exc}")]
    out = proc.stdout + proc.stderr
    if "Traceback (most recent call last)" in out:
        return [Finding("HIGH", "P1", "self-containment gate raised a traceback (crash counts as FAIL)",
                        out.strip().splitlines()[-1] if out.strip() else "")]
    if not re.search(r"^RESULT: (PASS|FAIL)", out, re.M):
        return [Finding("HIGH", "P1", "self-containment gate produced no RESULT line", out[-200:])]
    for block in re.finditer(r"^\[FAIL\] (\S+)\n((?:\s{8}.*\n)*)", out, re.M):
        path = os.path.relpath(block.group(1), base)
        detail = " / ".join(l.strip() for l in block.group(2).splitlines()[:3])
        findings.append(Finding("HIGH", "P1", f"{path} is not self-contained", detail))
    st = subprocess.run([sys.executable, script, "--self-test"], capture_output=True, text=True, timeout=120)
    if "SELF_TEST: PASS" not in st.stdout:
        findings.append(Finding("HIGH", "P1", "self-containment gate --self-test does not pass",
                                (st.stdout + st.stderr).strip()[-200:]))
    return findings


# ---------------------------------------------------------------------------- P2
def canonical_table():
    rows = []
    for w, h in SIZES:
        hs = (h - TOP_CHROME_PX - STATUS_PX) / h
        ws = (w - SIDE_STRIP_PX) / w
        rows.append((w, h, hs, ws, hs >= HEIGHT_SHARE, ws >= WIDTH_SHARE))
    return rows


def probe_geometry(base):
    findings = []
    for w, h, hs, ws, hok, wok in canonical_table():
        if not hok:
            findings.append(Finding("HIGH", "P2",
                                    f"collapsed height share fails at {w}x{h}: {hs*100:.2f}% < {HEIGHT_SHARE*100:.0f}%"))
        if not wok:
            findings.append(Finding("HIGH", "P2",
                                    f"collapsed width share fails at {w}x{h}: {ws*100:.2f}% < {WIDTH_SHARE*100:.0f}%"))
    for name in (A1_REPORT, A2_REPORT, A1_PROTO):
        text = read(base, name)
        if text is None:
            findings.append(Finding("HIGH", "P2", f"{name} is missing"))
            continue
        for ln, body in lines_of(text, STALE_CHROME_RX):
            if STALE_CHROME_CONTEXT.search(body):
                findings.append(Finding("HIGH", "P2",
                                        f"{name} still states a superseded chrome number "
                                        f"(A0 froze 60px top + 24px status)", f"L{ln}: {body}"))
    return findings


# ---------------------------------------------------------------------------- P3
# A size counts as covered if ANY file of the lane's evidence group carries it, because some
# lanes prove geometry with a measurement script + run log instead of a rendered frame.
SIZE_EVIDENCE = [
    ("A1", [A1_PROTO], SIZES),
    ("A2", [A2_REPORT, "A2-R3-measure.py", "A2-R3-run-20260908.out"], SIZES),
    ("A3", [A3_PROTO, A3_REPORT], SIZES),
    ("A8", [A8_FRAMES, "A8-R3-visual-density-design.md"], SIZES),
    ("A5", [A5_PROTO, "A5-R3-database-shell.md"], [(1366, 768), (900, 600)]),
]
# prototypes whose own frames must not advertise the dropped size any more
DROPPED_SIZE_SCAN = [A1_PROTO, A2_PROTO, A3_PROTO, A5_PROTO, A8_FRAMES, A1_REPORT, A2_REPORT]


def probe_size_coverage(base):
    findings = []
    for lane, group, required in SIZE_EVIDENCE:
        present = set()
        seen_any = False
        for name in group:
            text = read(base, name)
            if text is None:
                continue
            seen_any = True
            hits = count_sizes(text)
            present |= {k for k, v in hits.items() if v}
        if not seen_any:
            findings.append(Finding("HIGH", "P3", f"{lane} has no size-evidence file", ", ".join(group)))
            continue
        missing = [f"{w}x{h}" for (w, h) in required if (w, h) not in present]
        if missing:
            findings.append(Finding("HIGH", "P3",
                                    f"{lane} evidence does not cover acceptance size(s)",
                                    ", ".join(missing) + f"  [group: {', '.join(group)}]"))
    for name in DROPPED_SIZE_SCAN:
        text = read(base, name)
        if text is None:
            continue
        for ln, body in lines_of(text, re.compile(r"800\s*[x×X*]\s*600")):
            findings.append(Finding("MEDIUM", "P3",
                                    f"{name} still references the dropped {DROPPED_SIZE[0]}x{DROPPED_SIZE[1]} size",
                                    f"L{ln}: {body}"))
    return findings


# ---------------------------------------------------------------------------- P4
def probe_git_units(base):
    findings = []
    text = read(base, A1_PROTO)
    if text is None:
        return [Finding("HIGH", "P4", f"{A1_PROTO} is missing")]
    visible = re.sub(r"<!--.*?-->", " ", text, flags=re.S)
    missing = []
    for unit, patterns in GIT_UNITS:
        if not any(re.search(p, visible, re.I) for p in patterns):
            missing.append(unit)
    if missing:
        findings.append(Finding("HIGH", "P4",
                                f"{A1_PROTO} does not visibly locate {len(missing)} workflow unit(s)",
                                "; ".join(missing)))
    return findings


# ---------------------------------------------------------------------------- P5
A11Y_MIN = {A1_PROTO: (8, 8, 1)}
A11Y_DEFAULT = (1, 1, 0)


def probe_accessibility(base):
    findings = []
    for name in PROTOTYPES:
        text = read(base, name)
        if text is None:
            findings.append(Finding("HIGH", "P5", f"{name} is missing"))
            continue
        role = len(re.findall(r"\brole=", text))
        aria = len(re.findall(r"\baria-", text))
        tab = len(re.findall(r"tabindex", text))
        need_role, need_aria, need_tab = A11Y_MIN.get(name, A11Y_DEFAULT)
        if role < need_role or aria < need_aria or tab < need_tab:
            sev = "HIGH" if name == A1_PROTO else "MEDIUM"
            findings.append(Finding(sev, "P5",
                                    f"{name} accessibility below the R3B floor",
                                    f"role={role}/{need_role} aria-*={aria}/{need_aria} tabindex={tab}/{need_tab}"))
        if name == A1_PROTO and not has(text, r"focus-visible"):
            findings.append(Finding("MEDIUM", "P5", f"{name} has no :focus-visible styling"))
    return findings


# ---------------------------------------------------------------------------- P6
PALETTE_RX = r"palette|命令面板|command\s*palette"


def probe_shortcuts(base):
    findings = []
    for name in (A6_REGISTRY, A1_PROTO):
        text = read(base, name)
        if text is None:
            findings.append(Finding("HIGH", "P6", f"{name} is missing"))
            continue
        if not has(text, r"Ctrl\+Shift\+P"):
            findings.append(Finding("HIGH", "P6", f"{name} does not freeze the palette shortcut (Ctrl+Shift+P)"))
        if not has(text, r"Ctrl\+K"):
            findings.append(Finding("HIGH", "P6", f"{name} does not freeze the address/search shortcut (Ctrl+K)"))
        collision = re.compile(rf"Ctrl\+K(?!\+).*(?:{PALETTE_RX})|(?:{PALETTE_RX}).*Ctrl\+K(?!\+)", re.I)
        for ln, body in lines_of(text, collision):
            if re.search(r"Ctrl\+Shift\+P", body, re.I):
                continue  # the same line also names the frozen palette binding -> not a collision
            findings.append(Finding("HIGH", "P6", f"{name} binds Ctrl+K to the command palette (A0 froze Ctrl+Shift+P)",
                                    f"L{ln}: {body}"))
    return findings


# ---------------------------------------------------------------------------- P7
PIN_SSOT = "cee14e9"
PIN_SNAPSHOT = "2896562"
DEMOTION_RX = re.compile(r"snapshot|快照|non-SSOT|非\s*SSOT|superseded|已降级|moving ref|master\s*HEAD", re.I)


def probe_reference_pins(base):
    findings = []
    a7 = read(base, A7_MAP)
    if a7 is None:
        findings.append(Finding("HIGH", "P7", f"{A7_MAP} is missing"))
    else:
        if PIN_SSOT not in a7:
            findings.append(Finding("HIGH", "P7", f"{A7_MAP} does not carry the SSOT pin {PIN_SSOT}"))
        if re.search(r"\bv1\.1\.15\b", a7) and "refs/tags/1.1.15" not in a7:
            findings.append(Finding("MEDIUM", "P7",
                                    f"{A7_MAP} writes the tag as v1.1.15; the upstream ref is refs/tags/1.1.15 "
                                    f"(no v prefix) -> record the exact ref name"))
        for ln, body in lines_of(a7, re.compile(r"\bCOPY\b")):
            if re.search(r"\|\s*COPY\s*\|", body):
                findings.append(Finding("HIGH", "P7", f"{A7_MAP} classifies a unit as COPY (A0 ruled COPY=0)",
                                        f"L{ln}: {body}"))
    for name in sorted(f for f in os.listdir(base) if f.endswith(".md")):
        text = read(base, name)
        if text is None or PIN_SNAPSHOT not in text:
            continue
        if not DEMOTION_RX.search(text):
            hit = lines_of(text, re.compile(PIN_SNAPSHOT))
            findings.append(Finding("MEDIUM", "P7",
                                    f"{name} cites {PIN_SNAPSHOT}e without marking it a demoted research snapshot",
                                    f"L{hit[0][0]}: {hit[0][1]}" if hit else ""))
    return findings


# ---------------------------------------------------------------------------- P8
EXPECTED_TALLY = {"ADAPT": 4, "REIMPLEMENT": 10, "COPY": 0}


def parse_unit_table(text):
    """Return {class: count} for the numbered 14-unit matrix rows."""
    tally = {}
    for line in text.splitlines():
        m = re.match(r"^\|\s*(\d{1,2})\s*\|.*\|\s*([A-Z_]+)\s*\|\s*$", line.strip())
        if m and 1 <= int(m.group(1)) <= 20:
            cls = m.group(2).split("_")[0]
            tally[cls] = tally.get(cls, 0) + 1
    return tally


def probe_classification(base):
    findings = []
    text = read(base, A7_MAP)
    if text is None:
        return [Finding("HIGH", "P8", f"{A7_MAP} is missing")]
    tally = parse_unit_table(text)
    total = sum(tally.values())
    if total != 14:
        findings.append(Finding("HIGH", "P8", f"{A7_MAP} unit table has {total} classified rows, expected 14",
                                str(tally)))
    for cls, want in EXPECTED_TALLY.items():
        got = tally.get(cls, 0)
        if got != want:
            findings.append(Finding("HIGH", "P8",
                                    f"{A7_MAP} table has {cls}={got}, A0 ruled {cls}={want}", str(tally)))
    # the prose tally must equal the table, otherwise the ruling is only half-applied
    for ln, body in lines_of(text, re.compile(r"(ADAPT|REIMPLEMENT)\w*\s*=?\s*\**\s*(\d+)", re.I)):
        for cls, num in re.findall(r"(ADAPT|REIMPLEMENT)\w*[^0-9]{0,12}(\d+)", body, re.I):
            key = cls.upper()
            if key in EXPECTED_TALLY and int(num) != EXPECTED_TALLY[key]:
                findings.append(Finding("HIGH", "P8",
                                        f"{A7_MAP} prose tally states {key}={num}, table/A0 ruling says "
                                        f"{EXPECTED_TALLY[key]}", f"L{ln}: {body}"))
    return findings


# ---------------------------------------------------------------------------- P9
PINNED_SURVIVES_RX = re.compile(
    r"(?:collapse\s*all|collapseAll|全部折叠)[^\n]{0,160}?(?:pinned\s*(?:survive|preserved|stay|remain)|"
    r"non-?`?pinned`?|pinned\s*保留|保留\s*pinned)"
    r"|(?:pinned\s*(?:survive|preserved|stay|remain)|non-?`?pinned`?)[^\n]{0,160}?(?:collapse\s*all|collapseAll|全部折叠)",
    re.I)
PINNED_HIDDEN_RX = re.compile(
    r"(?:collapse\s*all|collapseAll|全部折叠)[^\n]{0,200}?(?:including\s+pinned|pinned\s+included|"
    r"pinned\s*(?:also|too)?\s*(?:hidden|hide)|含\s*pinned|包括\s*pinned|pinned\s*一并|pinned\s*也(?:被)?隐藏)",
    re.I)


def probe_collapse_semantics(base):
    findings = []
    for name in (A4_REPORT, A4_SM, A3_REPORT, A3_SM, A1_REPORT):
        text = read(base, name)
        if text is None:
            findings.append(Finding("MEDIUM", "P9", f"{name} is missing"))
            continue
        for ln, body in lines_of(text, PINNED_SURVIVES_RX):
            findings.append(Finding("HIGH", "P9",
                                    f"{name} keeps pinned tool windows during Collapse All "
                                    f"(A0 ruled Collapse All hides all, pinned included)", f"L{ln}: {body}"))
    a4 = read(base, A4_REPORT)
    if a4 is not None and not PINNED_HIDDEN_RX.search(a4):
        findings.append(Finding("MEDIUM", "P9",
                                f"{A4_REPORT} never states explicitly that Collapse All hides pinned windows too"))
    return findings


PROBES = [
    ("P1 SELF_CONTAINMENT", probe_self_containment),
    ("P2 GEOMETRY_CANON", probe_geometry),
    ("P3 SIZE_COVERAGE", probe_size_coverage),
    ("P4 GIT_UNITS", probe_git_units),
    ("P5 ACCESSIBILITY", probe_accessibility),
    ("P6 SHORTCUTS", probe_shortcuts),
    ("P7 REFERENCE_PINS", probe_reference_pins),
    ("P8 CLASSIFICATION", probe_classification),
    ("P9 COLLAPSE_SEMANTICS", probe_collapse_semantics),
]


def run_audit(base, verbose=True):
    all_findings = []
    statuses = []
    if verbose:
        print("A10-R3B closure audit — A0 R3 acceptance rulings as machine checks")
        print(f"base: {base}\n")
        print("== canonical geometry (inner viewport, OS titlebar excluded) ==")
        print(f"   top chrome = {TOP_ROWS} x {TOP_ROW_PX}px = {TOP_CHROME_PX}px | status = {STATUS_PX}px | "
              f"side strip = {SIDE_STRIP_PX}px")
        for w, h, hs, ws, hok, wok in canonical_table():
            print(f"   {w:>4}x{h:<4}  height {hs*100:6.2f}% {'OK ' if hok else 'BAD'}"
                  f"  width {ws*100:6.2f}% {'OK ' if wok else 'BAD'}"
                  f"  vertical slack {int(h*(1-HEIGHT_SHARE)) - (TOP_CHROME_PX + STATUS_PX):>4}px")
        print()
    for label, fn in PROBES:
        try:
            findings = fn(base)
        except Exception as exc:  # a crashing probe is a FAIL, never a pass
            findings = [Finding("HIGH", label.split()[0],
                                f"probe crashed: {type(exc).__name__}: {exc}")]
        status = "FAIL" if findings else "PASS"
        statuses.append((label, status, len(findings)))
        all_findings.extend(findings)
        if verbose:
            print(f"[{status}] {label} ({len(findings)} finding(s))")
            for f in sorted(findings, key=lambda x: SEV_ORDER[x.severity]):
                print(f"        {f.severity:<6} {f.message}")
                if f.evidence:
                    print(f"               {f.evidence}")
    if verbose:
        high = sum(1 for f in all_findings if f.severity == "HIGH")
        med = sum(1 for f in all_findings if f.severity == "MEDIUM")
        low = sum(1 for f in all_findings if f.severity not in ("HIGH", "MEDIUM"))
        passed = sum(1 for _, s, _ in statuses if s == "PASS")
        print(f"\nPROBES: {passed}/{len(statuses)} PASS")
        print(f"FINDINGS: HIGH={high} MEDIUM={med} LOW/INFO={low}")
        print(f"GATE: {'PASS' if not all_findings else 'FAIL'}")
    return all_findings


# ---------------------------------------------------------------------------- self-test
CLEAN_PROTO_HEAD = """<!doctype html><html lang="zh-CN"><head><meta charset="utf-8">
<style>:focus-visible{outline:2px solid #58a6ff}</style></head><body>
"""


def _clean_frames(sizes):
    body = []
    for w, h in sizes:
        body.append(f'<section role="group" aria-label="frame {w}x{h}" tabindex="0"><h2>{w}×{h}</h2></section>')
    return CLEAN_PROTO_HEAD + "\n".join(body) + "</body></html>\n"


def _a1_clean():
    units = ("status hunk diff log graph branch worktree stash merge rebase cherry-pick "
             "conflict blame patch command log amend revert reset")
    extra = "".join(
        f'<div role="tab" aria-label="unit {i}" aria-selected="false" tabindex="0">u{i}</div>' for i in range(9)
    )
    return _a1_wrap(_clean_frames(SIZES), units, extra)


def _a1_wrap(base_html, units, extra):
    inject = (f'<nav role="navigation" aria-label="git units" tabindex="0">{units}</nav>\n{extra}\n'
              '<div role="search" aria-label="address">Ctrl+K</div>\n'
              '<div role="dialog" aria-label="palette">Ctrl+Shift+P command palette</div>\n')
    return base_html.replace("</body>", inject + "</body>")


A7_CLEAN = """# A7 map
| Reference | Pinned revision |
|---|---|
| Rebased | 1.1.15 @ `cee14e9` (`refs/tags/1.1.15`) |

| # | Unit | Product status | Rebased status | Source identity | Class |
|---|---|---|---|---|---|
""" + "".join(
    f"| {i} | u{i} | x | y | z | {'ADAPT' if i in (1, 3, 5, 8) else 'REIMPLEMENT'} |\n" for i in range(1, 15)
) + """
Tally: COPY = 0, ADAPT = 4, REIMPLEMENT = 10.
"""

A6_CLEAN = "# A6 registry\n- Ctrl+K -> address/search focus\n- Ctrl+Shift+P -> command palette\n"
A4_CLEAN = ("# A4 contract\n### 7.5 collapseAll\n- `collapseAll` hides every tool window, including pinned "
            "windows (per A0 ruling).\n\n| # | Function | Assertion |\n| T14 | collapseAll | all hidden, "
            "pinned included |\n")
A3_CLEAN = "# A3 disclosure\n- collapseAll hides all tool windows, pinned included.\n"
A3_SM_CLEAN = "export function collapseAll(state){ for (const id of Object.keys(state.tools)) state.tools[id].open=false; return state; }\n"
A4_SM_CLEAN = "export function collapseAll(list){ return list.map(w => ({...w, state:'hidden'})); }\n"
A1_REPORT_CLEAN = ("# A1 workbench\n- top chrome 60px (2 x 30px), status bar 24px, collapsed strip 28px.\n"
                   "- Ctrl+K address, Ctrl+Shift+P palette.\n- collapseAll hides all tool windows, including pinned.\n")
A2_REPORT_CLEAN = "# A2 density\n- measured on inner viewport: 60px top chrome, 24px status bar.\n" + \
                  "".join(f"- {w}x{h} verified\n" for w, h in SIZES)


def _write_clean_base(base):
    def w(name, text):
        open(os.path.join(base, name), "w", encoding="utf-8").write(text)

    w(A1_PROTO, _a1_clean())
    w(A2_PROTO, _clean_frames(SIZES))
    w(A3_PROTO, _clean_frames(SIZES))
    w(A8_FRAMES, _clean_frames(SIZES))
    w(A5_PROTO, _clean_frames([(1366, 768), (900, 600)]))
    w(A1_REPORT, A1_REPORT_CLEAN)
    w(A2_REPORT, A2_REPORT_CLEAN)
    w(A3_REPORT, A3_CLEAN)
    w(A3_SM, A3_SM_CLEAN)
    w(A4_REPORT, A4_CLEAN)
    w(A4_SM, A4_SM_CLEAN)
    w(A6_REGISTRY, A6_CLEAN)
    w(A7_MAP, A7_CLEAN)
    src = os.path.join(HERE, SELFCONTAINMENT)
    if os.path.exists(src):
        open(os.path.join(base, SELFCONTAINMENT), "w", encoding="utf-8").write(
            open(src, encoding="utf-8").read())


MUTATIONS = {
    "P1": (A5_PROTO, lambda t: t.replace("</body>", '<div>JetBrains Mono</div></body>')),
    "P2": (A1_REPORT, lambda t: t + "- top chrome total 68px\n"),
    "P3": (A8_FRAMES, lambda t: t.replace("1200×800", "800×600")),
    "P4": (A1_PROTO, lambda t: t.replace("worktree", "").replace("amend", "")),
    "P5": (A1_PROTO, lambda t: re.sub(r'\brole="[^"]*"', "", t)),
    "P6": (A6_REGISTRY, lambda t: t.replace("Ctrl+Shift+P -> command palette", "Ctrl+K -> command palette")),
    "P7": (A7_MAP, lambda t: t.replace("`cee14e9` (`refs/tags/1.1.15`)", "`2896562e`")),
    "P8": (A7_MAP, lambda t: t.replace("ADAPT = 4", "ADAPT = 5")),
    "P9": (A4_REPORT, lambda t: t.replace("hides every tool window, including pinned windows",
                                          "sets every non-`pinned` window hidden (pinned survive)")),
}


def self_test():
    ok = True
    checks = 0

    def expect(cond, label):
        nonlocal ok, checks
        checks += 1
        print(("  PASS  " if cond else "  FAIL  ") + label)
        if not cond:
            ok = False

    print("A10-R3B closure audit --self-test\n")
    with tempfile.TemporaryDirectory() as td:
        _write_clean_base(td)
        findings = run_audit(td, verbose=False)
        expect(not findings, f"clean synthetic artifact set passes all probes ({[str(f) for f in findings][:3]})")

        for probe, (fname, mutate) in MUTATIONS.items():
            with tempfile.TemporaryDirectory() as td2:
                _write_clean_base(td2)
                path = os.path.join(td2, fname)
                text = open(path, encoding="utf-8").read()
                open(path, "w", encoding="utf-8").write(mutate(text))
                got = run_audit(td2, verbose=False)
                probes_hit = {f.probe.split()[0] for f in got}
                expect(probe in probes_hit, f"{probe} detects its violation ({sorted(probes_hit)})")

        # a crashing probe must be reported as FAIL, not swallowed
        saved = PROBES[:]
        try:
            PROBES.append(("PX CRASH", lambda base: (_ for _ in ()).throw(RuntimeError("boom"))))
            with tempfile.TemporaryDirectory() as td3:
                _write_clean_base(td3)
                got = run_audit(td3, verbose=False)
            expect(any("probe crashed" in f.message for f in got), "crashing probe is reported as a FAIL finding")
        finally:
            PROBES[:] = saved

        with tempfile.TemporaryDirectory() as td4:
            got = run_audit(td4, verbose=False)
            expect(len(got) >= len(PROBES) - 1, "empty artifact directory fails loudly (missing files)")

    expect(all(sum(1 for _, p in GIT_UNITS if _ == u) == 1 for u, _ in GIT_UNITS), "git unit list has no duplicate")
    expect(len(GIT_UNITS) == 16, "git unit list = 14 matrix units + amend + reset/revert")
    print(f"\nSELF_TEST: {'PASS' if ok else 'FAIL'} ({checks} checks, PROBES={len(PROBES)})")
    return 0 if ok else 1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default=HERE)
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    if args.self_test:
        return self_test()
    findings = run_audit(os.path.abspath(args.base))
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
