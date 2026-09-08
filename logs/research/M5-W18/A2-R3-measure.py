#!/usr/bin/env python3
"""A2 / M5-W18-R3 :: run the current-shell density replica under headless Chrome.

Reads `A2-R3-density-replica.html`, renders it once with `google-chrome --dump-dom`,
parses the JSON the page writes into `#out`, and prints markdown tables.

The replica sizes its own root wrapper explicitly, so the numbers do not depend on
the real viewport of the headless browser (headless Chrome reports an 87px-smaller
viewport than --window-size, which is exactly why the wrapper is used).

Usage:  python3 A2-R3-measure.py [--html PATH] [--out PATH]
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
DEFAULT_HTML = HERE / "A2-R3-density-replica.html"
DEFAULT_OUT = HERE / "A2-R3-run-20260908.out"

STATE_LABEL = {
    "calm": "默认平静（无扩展行/无侧栏）",
    "omni": "地址栏聚焦 → 最近/常用自动展开",
    "more": "☰ 功能菜单展开",
    "gridai": "宫格 AI 模式（群发行常驻 + 宫格设置）",
    "worst": "最坏：AI 群发+宫格设置+资源+网址 四行同开",
    "dock": "浏览器右侧 Dock（文件/终端）",
    "sidestack": "左 AI 导航 + 收藏夹 + 右 Dock 三面同开",
    "compact": "精简模式（隐藏活动条与页签条）",
    "module": "模块视图（文件 IDE 固定 260px 树）",
}
STATE_ORDER = ["calm", "omni", "more", "gridai", "worst", "dock", "sidestack", "compact", "module"]
PRIMARY_SIZES = ["1920x1080", "1440x900", "1366x768", "1024x720"]


def run_chrome(html_path: pathlib.Path) -> str:
    cmd = [
        "google-chrome",
        "--headless",
        "--no-sandbox",
        "--disable-gpu",
        "--window-size=1920,1200",
        "--dump-dom",
        html_path.as_uri(),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
    if proc.returncode != 0:
        sys.exit("chrome failed: rc=%s\n%s" % (proc.returncode, proc.stderr[-2000:]))
    return proc.stdout


def extract_json(dom: str) -> list:
    m = re.search(r'<pre id="out"[^>]*>(.*?)</pre>', dom, re.S)
    if not m:
        sys.exit("measurement output not found; the replica script did not run")
    return json.loads(html.unescape(m.group(1)))


def row_table(rows: list, sizes: list, title: str) -> list:
    lines = ["", "### " + title, ""]
    header = "| 状态 | " + " | ".join(sizes) + " |"
    lines.append(header)
    lines.append("|" + "---|" * (len(sizes) + 1))
    for st in STATE_ORDER:
        cells = []
        for s in sizes:
            r = next((x for x in rows if x["size"] == s and x["state"] == st), None)
            cells.append("—" if r is None else "%.1f%% / %.1f%%" % (r["pctH"], r["pctW"]))
        lines.append("| %s | %s |" % (STATE_LABEL[st], " | ".join(cells)))
    return lines


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--html", default=str(DEFAULT_HTML))
    ap.add_argument("--out", default=str(DEFAULT_OUT))
    args = ap.parse_args()

    rows = extract_json(run_chrome(pathlib.Path(args.html)))
    lines = [
        "A2 R3 density run — current-shell replica, headless Chrome",
        "columns: content height %% of viewport / content width %% of viewport",
        "",
    ]
    lines += row_table(rows, PRIMARY_SIZES, "主尺寸（客户区 = 目标尺寸）")
    lines += row_table(
        rows,
        ["1920x1043", "1440x863", "1366x731", "1024x683"],
        "敏感度：窗口最大化且存在约 37px 原生标题栏时的客户区",
    )

    lines += ["", "### 明细（chrome 分解，px）", ""]
    lines.append("| size | state | density | activity | expand | tabbar | status | chrome | contentH | contentW | btns(top) | btns(tab) |")
    lines.append("|" + "---|" * 12)
    for r in rows:
        lines.append(
            "| {size} | {state} | {density} | {activityH} | {expandH} | {tabbarH} | {statusH} | "
            "{chromeH} | {contentH} | {contentW} | {btnsTop} | {btnsTab} |".format(**r)
        )

    text = "\n".join(lines) + "\n"
    pathlib.Path(args.out).write_text(text, encoding="utf-8")
    print(text)


if __name__ == "__main__":
    main()
