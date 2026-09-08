#!/usr/bin/env python3
"""A2 / M5-W18-R3B :: canonical geometry evidence driver.

Runs two self-contained fixtures under headless Chrome and checks them against the
A0 geometry SSOT (logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md):

    geometry uses the application INNER viewport (W x H), OS titlebar excluded
    normal top chrome   = 60px (two 30px rows)
    status bar          = 24px
    collapsed rail      = 28px
    collapsed height    = (H - 60 - 24) / H   >= 85%
    collapsed width     = (W - 28)     / W    >= 92%
    sizes               = 1920x1080 1440x900 1366x768 1200x800 1024x720 900x600

Fixtures (both live in this directory, both synthetic and self-contained):
    A2-R3-density-replica.html        BASELINE - current shipping shell, CSS constants
                                      transcribed from the product (file:line cited in
                                      A2-R3-shell-density-audit.md)
    A2-R3B-canonical-replica.html     REVISED  - the A0 geometry contract encoded in CSS

Both fixtures size their own root wrapper, so the numbers do not depend on the real
headless viewport (headless Chrome reports a smaller viewport than --window-size).

Usage:
    python3 A2-R3B-measure.py [--sizes WxH,...] [--out PATH] [--strict]

Exit code: 0 = canonical gates PASS, 1 = at least one canonical gate FAIL.
Baseline findings are reported but never gate (they are the "before" numbers).
"""

from __future__ import annotations

import argparse
import html
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
BASELINE_HTML = HERE / "A2-R3-density-replica.html"
CANONICAL_HTML = HERE / "A2-R3B-canonical-replica.html"
DEFAULT_OUT = HERE / "A2-R3B-run-20260908.out"

DEFAULT_SIZES = ["1920x1080", "1440x900", "1366x768", "1200x800", "1024x720", "900x600"]

TOP_CHROME = 60
STATUS = 24
RAIL = 28
MIN_H = 85.0
MIN_W = 92.0

CANON_STATES = ["collapsed", "left", "right", "bottom", "leftright", "focus"]
BASE_STATES = ["calm", "omni", "more", "gridai", "worst", "dock", "sidestack", "compact", "module"]

BASE_LABEL = {
    "calm": "平静（无扩展行/无侧栏）",
    "omni": "地址栏聚焦→最近/常用自动展开",
    "more": "☰ 功能菜单展开",
    "gridai": "宫格 AI 模式（群发行常驻+设置行）",
    "worst": "最坏：AI 群发+宫格设置+资源+网址",
    "dock": "右侧 Dock（文件/终端）",
    "sidestack": "左 AI 导航+收藏夹+右 Dock 三面同开",
    "compact": "精简模式（隐藏活动条+页签条）",
    "module": "模块视图（文件树 260px）",
}
CANON_LABEL = {
    "collapsed": "折叠全部（A0 验收态，28px 条保留）",
    "left": "左工具窗口 min(260,0.25W)",
    "right": "右工具窗口 min(320,0.28W)",
    "bottom": "底工具窗口 min(240,0.30H)",
    "leftright": "左右同开",
    "focus": "焦点模式（A0 未冻结，仅供参考）",
}


def run_fixture(path: pathlib.Path, sizes: list) -> list:
    uri = path.as_uri() + "?sizes=" + ",".join(sizes)
    cmd = [
        "google-chrome", "--headless", "--no-sandbox", "--disable-gpu",
        "--window-size=1920,1200", "--dump-dom", uri,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    if proc.returncode != 0:
        sys.exit("chrome failed on %s: rc=%s\n%s" % (path.name, proc.returncode, proc.stderr[-2000:]))
    m = re.search(r'<pre id="out"[^>]*>(.*?)</pre>', proc.stdout, re.S)
    if not m:
        sys.exit("no measurement output in %s; fixture script did not run" % path.name)
    return json.loads(html.unescape(m.group(1)))


def derived(size: str) -> dict:
    w, h = (int(x) for x in size.split("x"))
    return {
        "size": size,
        "contentH": h - TOP_CHROME - STATUS,
        "contentW": w - RAIL,
        "pctH": (h - TOP_CHROME - STATUS) / h * 100.0,
        "pctW": (w - RAIL) / w * 100.0,
    }


def pct(x: float) -> str:
    return "%.2f%%" % x


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--sizes", default=",".join(DEFAULT_SIZES))
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    ap.add_argument("--strict", action="store_true",
                    help="also fail the gate on baseline findings (default: baseline is reported only)")
    args = ap.parse_args()
    sizes = [s.strip() for s in args.sizes.split(",") if s.strip()]

    base_rows = run_fixture(BASELINE_HTML, sizes)
    canon_rows = run_fixture(CANONICAL_HTML, sizes)

    by = {}
    for r in canon_rows:
        by[(r["size"], r["state"])] = r
    bby = {}
    for r in base_rows:
        bby[(r["size"], r["state"])] = r

    L = []
    L.append("A2 R3B canonical geometry run — headless Chrome %s" % subprocess.run(
        ["google-chrome", "--version"], capture_output=True, text=True).stdout.strip())
    L.append("sizes: %s" % " ".join(sizes))
    L.append("A0 SSOT: top chrome 60px (2x30) / status 24px / collapsed rail 28px / "
             "collapsed >=85%% H and >=92%% W / inner viewport, OS titlebar excluded")
    L.append("")

    # ---------- A. derived ----------
    L.append("## A. 派生值（A0 公式，纯算术，未经过布局引擎）")
    L.append("")
    L.append("| 尺寸 | 内容高 px | 内容宽 px | 高占比 | 宽占比 | ≥85% 高 | ≥92% 宽 |")
    L.append("|" + "---|" * 7)
    for s in sizes:
        d = derived(s)
        L.append("| %s | %d | %d | %s | %s | %s | %s |" % (
            s, d["contentH"], d["contentW"], pct(d["pctH"]), pct(d["pctW"]),
            "PASS" if d["pctH"] >= MIN_H else "FAIL",
            "PASS" if d["pctW"] >= MIN_W else "FAIL"))
    L.append("")

    # ---------- B. canonical measured ----------
    L.append("## B. 修订复刻实测 vs 派生（measured vs derived）")
    L.append("")
    L.append("| 状态 | " + " | ".join(sizes) + " |")
    L.append("|" + "---|" * (len(sizes) + 1))
    for st in CANON_STATES:
        cells = []
        for s in sizes:
            r = by.get((s, st))
            cells.append("—" if r is None else "%s / %s" % (pct(r["pctH"]), pct(r["pctW"])))
        L.append("| %s | %s |" % (CANON_LABEL[st], " | ".join(cells)))
    L.append("")
    L.append("折叠态 measured − derived：")
    L.append("")
    L.append("| 尺寸 | 高 measured | 高 derived | Δpp | 宽 measured | 宽 derived | Δpp |")
    L.append("|" + "---|" * 7)
    for s in sizes:
        r, d = by[(s, "collapsed")], derived(s)
        L.append("| %s | %s | %s | %+.2f | %s | %s | %+.2f |" % (
            s, pct(r["pctH"]), pct(d["pctH"]), r["pctH"] - d["pctH"],
            pct(r["pctW"]), pct(d["pctW"]), r["pctW"] - d["pctW"]))
    L.append("")

    # ---------- C. baseline measured ----------
    L.append("## C. 当前外壳实测（baseline replica，同一 A0 口径：内容占 inner viewport）")
    L.append("")
    L.append("| 状态 | " + " | ".join(sizes) + " |")
    L.append("|" + "---|" * (len(sizes) + 1))
    for st in BASE_STATES:
        cells = []
        for s in sizes:
            r = bby.get((s, st))
            cells.append("—" if r is None else "%s / %s" % (pct(r["pctH"]), pct(r["pctW"])))
        L.append("| %s | %s |" % (BASE_LABEL[st], " | ".join(cells)))
    L.append("")

    # ---------- D. gates ----------
    gates = []

    def gate(name, ok, detail):
        gates.append((name, ok, detail))

    top_bad = [r for r in canon_rows if not r["state"] == "focus" and abs(r["topH"] - TOP_CHROME) > 0.01]
    gate("G1 修订顶栏 = 60px", not top_bad,
         "all states/sizes 60px" if not top_bad else "%d deviation(s), e.g. %s" % (
             len(top_bad), top_bad[0]["size"] + "/" + top_bad[0]["state"] + "=" + str(top_bad[0]["topH"])))

    st_bad = [r for r in canon_rows if r["state"] != "focus" and abs(r["statusH"] - STATUS) > 0.01]
    gate("G2 状态栏 = 24px", not st_bad,
         "all 24px" if not st_bad else "%d deviation(s)" % len(st_bad))

    rail_bad = [r for r in canon_rows if r["state"] != "focus" and abs(r["railW"] - RAIL) > 0.01]
    gate("G3 折叠条 = 28px", not rail_bad,
         "all 28px" if not rail_bad else "%d deviation(s)" % len(rail_bad))

    h_bad = [s for s in sizes if by[(s, "collapsed")]["pctH"] < MIN_H]
    gate("G4 折叠态高 ≥ 85%（六尺寸）", not h_bad,
         "min %s" % pct(min(by[(s, "collapsed")]["pctH"] for s in sizes)) if not h_bad
         else "fails at %s" % ", ".join(h_bad))

    w_bad = [s for s in sizes if by[(s, "collapsed")]["pctW"] < MIN_W]
    gate("G5 折叠态宽 ≥ 92%（六尺寸）", not w_bad,
         "min %s" % pct(min(by[(s, "collapsed")]["pctW"] for s in sizes)) if not w_bad
         else "fails at %s" % ", ".join(w_bad))

    zero = [r for r in canon_rows if r["contentH"] <= 0 or r["contentW"] <= 0]
    gate("G6 任何状态内容面不为 0", not zero,
         "all > 0 (min %gx%g)" % (min(r["contentW"] for r in canon_rows),
                                  min(r["contentH"] for r in canon_rows)) if not zero
         else "%d zero-surface row(s)" % len(zero))

    clip = [r for r in canon_rows if r["rowAClip"] > 0 or r["rowBClip"] > 0]
    gate("G7 30px 行无因溢出而增高（溢出走裁剪）", not [r for r in canon_rows if r["state"] != "focus" and abs(r["topH"] - TOP_CHROME) > 0.01],
         "无裁剪" if not clip else "%d 行发生裁剪（高度仍锁 60px）：%s" % (
             len(clip), ", ".join(sorted({r["size"] for r in clip}))))

    # baseline findings (reported, not gating unless --strict)
    findings = []

    def find(name, ok, detail):
        findings.append((name, ok, detail))

    btop = {s: round(bby[(s, "calm")]["activityH"] + bby[(s, "calm")]["expandH"] + bby[(s, "calm")]["tabbarH"], 1)
            for s in sizes}
    find("B1 基线顶栏 = 60px", all(abs(v - TOP_CHROME) < 0.01 for v in btop.values()),
         "实测 %s（活动条+扩展行+页签条，平静态）" % "/".join("%g" % v for v in btop.values()))
    find("B2 基线状态栏 = 24px", all(abs(bby[(s, "calm")]["statusH"] - STATUS) < 0.01 for s in sizes),
         "实测 %gpx" % bby[(sizes[0], "calm")]["statusH"])
    calm_h = [s for s in sizes if bby[(s, "calm")]["pctH"] < MIN_H]
    find("B3 基线平静态高 ≥ 85%", not calm_h,
         "min %s%s" % (pct(min(bby[(s, "calm")]["pctH"] for s in sizes)),
                       "" if not calm_h else "（低于阈值：" + ", ".join(calm_h) + "）"))
    calm_w = [s for s in sizes if bby[(s, "calm")]["pctW"] < MIN_W]
    find("B4 基线平静态宽 ≥ 92%", not calm_w,
         "min %s%s" % (pct(min(bby[(s, "calm")]["pctW"] for s in sizes)),
                       "" if not calm_w else "（低于阈值：" + ", ".join(calm_w) + "）"))
    wh = [s for s in sizes if bby[(s, "worst")]["pctH"] < MIN_H]
    find("B5 基线最坏态高 ≥ 85%", not wh,
         "min %s%s" % (pct(min(bby[(s, "worst")]["pctH"] for s in sizes)),
                       "" if not wh else "（低于阈值：" + ", ".join(wh) + "）"))
    bzero = [r for r in base_rows if r["contentW"] <= 0]
    find("B6 基线无 0 宽内容面", not bzero,
         "全部 > 0" if not bzero else "; ".join("%s/%s=%gpx" % (r["size"], r["state"], r["contentW"]) for r in bzero))

    # 提案阈值（A2 DESIGN_DECISION，A0 未冻结，不入门禁，仅暴露窄窗风险）
    U_MIN_W, U_MIN_H = 480, 300
    u1 = [r for r in canon_rows if r["state"] != "focus" and r["contentW"] < U_MIN_W]
    find("U1 提案：工具窗口打开后内容宽 ≥ %dpx" % U_MIN_W, not u1,
         "min %gpx" % min(r["contentW"] for r in canon_rows if r["state"] != "focus") if not u1
         else "; ".join(sorted({"%s/%s=%gpx" % (r["size"], r["state"], r["contentW"]) for r in u1})))
    u2 = [r for r in canon_rows if r["state"] != "focus" and r["contentH"] < U_MIN_H]
    find("U2 提案：工具窗口打开后内容高 ≥ %dpx" % U_MIN_H, not u2,
         "min %gpx" % min(r["contentH"] for r in canon_rows if r["state"] != "focus") if not u2
         else "; ".join(sorted({"%s/%s=%gpx" % (r["size"], r["state"], r["contentH"]) for r in u2})))

    L.append("## D. 门禁断言")
    L.append("")
    L.append("| # | 断言 | 结果 | 说明 |")
    L.append("|" + "---|" * 4)
    for name, ok, detail in gates:
        L.append("| | %s | %s | %s |" % (name, "PASS" if ok else "FAIL", detail))
    L.append("")
    L.append("基线发现（BEFORE，默认不入门禁；`--strict` 时并入）：")
    L.append("")
    L.append("| # | 断言 | 结果 | 说明 |")
    L.append("|" + "---|" * 4)
    for name, ok, detail in findings:
        L.append("| | %s | %s | %s |" % (name, "PASS" if ok else "FAIL", detail))
    L.append("")

    # ---------- E. detail ----------
    L.append("## E. 明细（修订复刻，px）")
    L.append("")
    L.append("| size | state | topH | rowA | rowB | status | rail | left | right | bottom | contentH | contentW | clipA | clipB |")
    L.append("|" + "---|" * 14)
    for r in canon_rows:
        L.append("| {size} | {state} | {topH} | {rowAH} | {rowBH} | {statusH} | {railW} | {leftW} | "
                 "{rightW} | {bottomH} | {contentH} | {contentW} | {rowAClip} | {rowBClip} |".format(**r))
    L.append("")
    L.append("## F. 明细（当前基线，px）")
    L.append("")
    L.append("| size | state | activity | expand | tabbar | status | topChrome | contentH | contentW | btnsTop | btnsTab |")
    L.append("|" + "---|" * 11)
    for r in base_rows:
        top = round(r["activityH"] + r["expandH"] + r["tabbarH"], 1)
        L.append("| {size} | {state} | {activityH} | {expandH} | {tabbarH} | {statusH} | {top} | "
                 "{contentH} | {contentW} | {btnsTop} | {btnsTab} |".format(top=top, **r))
    L.append("")

    L.append("## G. 口径说明（measured vs derived）")
    L.append("")
    L.append("- derived：按 A0 公式直接算术，输入只有 W/H 与 60/24/28 三个常量。")
    L.append("- measured：无头 Chrome 对上述两个复刻页真实布局后取 getBoundingClientRect()。")
    L.append("- 修订复刻的 Δ 预期为 0：该页就是把公式写成 CSS，用来验证**编码一致**并抓溢出/换行，")
    L.append("  它不构成对物理客户端的独立证据（native GUI 未跑，记 NOT_RUN）。")
    L.append("- 基线复刻是把产品 CSS 常量逐条抄录的产物，误差来自：字体度量、桌面滚动条占位、")
    L.append("  扩展行换行、模块视图内部分栏未建模、OS 标题栏（A0 口径已排除）。")

    ok = all(o for _, o, _ in gates)
    if args.strict:
        ok = ok and all(o for _, o, _ in findings)
    L.append("")
    L.append("GATE: %s" % ("PASS" if ok else "FAIL"))

    text = "\n".join(L) + "\n"
    pathlib.Path(args.out).write_text(text, encoding="utf-8")
    print(text)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
