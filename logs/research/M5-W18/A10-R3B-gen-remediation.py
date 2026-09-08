#!/usr/bin/env python3
# A10-R3B helper: turn the closure-audit JSON into a per-lane remediation guide
# (markdown) for A0 to dispatch. Zero product impact; reads only the audit JSON.
#
# Usage:
#   python3 A10-R3B-closure-audit.py --json > A10-R3B-findings.json
#   python3 A10-R3B-gen-remediation.py A10-R3B-findings.json A10-R3B-remediation-guide.md
#
# Exit 0 always (it only renders; gating lives in the audit script).

import json
import re
import sys

# probe -> owning lanes for findings that are NOT tied to a specific file.
PROBE_OWNERS = {
    "P1": ["A1", "A3", "A5"],
    "P2": ["A1", "A2"],
    "P4": ["A1"],
    "P5": ["A1", "A2", "A8"],
    "P6": ["A1", "A6"],
    "P8": ["A7"],
    "P9": ["A4"],
}

# file -> owning lane, used to pin file-specific findings to the right lane(s).
FILE_LANE = {
    "A1-R3-prototype.html": "A1",
    "A1-R3-browser-workbench.md": "A1",
    "A2-R3-density-replica.html": "A2",
    "A2-R3-shell-density-audit.md": "A2",
    "A3-R3-toolwindow-prototype.html": "A3",
    "A3-R3-toolwindow-disclosure-20260908.md": "A3",
    "A5-R3-prototype.html": "A5",
    "A5-R3-database-shell.md": "A5",
    "A8-R3-visual-density-frames.html": "A8",
    "A8-R3-visual-density-design.md": "A8",
    "A7-R3-git-workflow-reference-map.md": "A7",
    "A10-R3-acceptance-preflight-20260908.md": "A10",
    "A10-R3-reference-and-feasibility-review-20260908.md": "A10",
    "A10-checkpoint-R3-20260908.md": "A10",
    "A11-R3-checkpoint.md": "A11",
    "A11-R3-progress.md": "A11",
    "A6-R3-context-menu-command-registry.md": "A6",
    "A4-R3-shell-state-persistence-contract.md": "A4",
}


def owners_for(it):
    """Resolve the concrete owning lane(s) for one finding."""
    probe = it["probe"]
    ev = it.get("evidence") or ""
    if probe == "P3":
        # size-coverage finding names the lane in its message: "<LANE> evidence ..."
        m = re.match(r"([A-Za-z0-9]+) evidence", it["message"])
        if m:
            return [m.group(1)]
        # dropped-size scan: pin to the file named in the evidence
        for fn, lane in FILE_LANE.items():
            if fn in ev:
                return [lane]
        return ["A1", "A2", "A3", "A5", "A8"]
    if probe == "P7":
        lanes = []
        for fn, lane in FILE_LANE.items():
            if fn in ev or fn.replace(".md", "").replace(".html", "") in ev:
                if lane not in lanes:
                    lanes.append(lane)
        return lanes or ["A7", "A10", "A11"]
    return PROBE_OWNERS.get(probe, [])

# concrete fix instruction per probe (severity-agnostic; text explains the ruling)
PROBE_FIX = {
    "P1": "原型 UI 文本/芯片不得出现 donor 品牌名或字体名（Rebased / IntelliJ / JetBrains / "
          "JetBrains Mono 等）；报告里可作署名引用，但 UI 须用中性表述（如「日志/图谱作中央文档区」）。",
    "P2": "几何口径统一为 A0 冻结值：顶部 chrome = 2×30px = 60px、状态栏 24px、折叠竖条 28px。"
          "删除一切 68/65/89/94/108px 等旧 chrome 数字。",
    "P3": "验收证据须覆盖 A0 四强制尺寸（1920×1080/1440×900/1366×768/1024×720）；"
          "若 A0 改判六尺寸全强制（--strict-sizes），还需补 1200×800 与 900×600 帧。"
          "800×600 仅保留「剔除」说明，不得作为可达帧。",
    "P4": "A1 终稿原型须以可见 mock 面板/列表呈现 14 个 Git 单元 + amend + reset/revert；"
          "当前缺 diff/worktrees/conflicts/patch/命令日志/amend/reset|revert。",
    "P5": "无障碍达 R3B 地板：A1 至少 8 个 role= + 8 个 aria-* + 1 个 tabindex + :focus-visible；"
          "其余原型至少各 1 个 role= + 1 个 aria-*。",
    "P6": "快捷键定版：Ctrl+Shift+P = 命令面板，Ctrl+K = 地址/搜索，二者零冲突。"
          "A6 注册表须显式冻结这两条；A1 原型不得把 Ctrl+K 绑到命令面板。",
    "P7": "参照钉版：cee14e9 为 SSOT；2896562e 仅作降级研究快照（引用处须标注「降级、非 SSOT、终裁前不得 W19 采用」）。"
          "COPY 分类维度删除（A0 裁定 COPY=0，并入 REJECT/forbidden）；tag 精确记为 refs/tags/1.1.15（无 v 前缀）。",
    "P8": "A7 分类计数须等于其 14 行表：ADAPT=4 / REIMPLEMENT=10 / COPY=0；散文与表保持一致。",
    "P9": "Collapse All 隐藏所有工具窗口（含 pinned）；不得保留 pinned。",
}

LANE_ORDER = ["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A10", "A11"]


def main():
    if len(sys.argv) != 3:
        print("usage: A10-R3B-gen-remediation.py <findings.json> <out.md>", file=sys.stderr)
        return 2
    data = json.load(open(sys.argv[1], encoding="utf-8"))
    items = data["findings"]["items"]

    # lane -> list of (probe, severity, message, evidence, line)
    by_lane = {l: [] for l in LANE_ORDER}
    for it in items:
        owners = owners_for(it)
        for l in owners:
            by_lane.setdefault(l, []).append(it)

    lines = []
    lines.append("# A10 — M5-W18-R3B 逐 Lane 修复指引（机器生成）\n")
    lines.append(f"- 来源：`A10-R3B-closure-audit.py --json`（strict_sizes={data['strict_sizes']}）")
    lines.append(f"- 门禁：`GATE={data['gate']}`，HIGH={data['findings']['high']} / MEDIUM={data['findings']['medium']}")
    lines.append(f"- 探针失败：{data['probes']['failed']}/{data['probes']['total']}")
    lines.append("")
    lines.append("> 本指引按「谁负责修」聚合发现。**A10 不自改他 lane 资产**（W18-R 硬约束）；")
    lines.append("> 各 lane 修复后由 A0/本 lane 复跑 `bash A10-R3B-rerun.sh` 确认 `GATE: PASS`。\n")

    for lane in LANE_ORDER:
        rows = by_lane.get(lane) or []
        if not rows:
            continue
        lines.append(f"## Lane {lane}（{len(rows)} 项）\n")
        # group by probe
        seen = {}
        for it in rows:
            seen.setdefault(it["probe"], []).append(it)
        for probe, its in seen.items():
            highs = [x for x in its if x["severity"] == "HIGH"]
            meds = [x for x in its if x["severity"] == "MEDIUM"]
            tag = f"HIGH×{len(highs)}" + (f" + MED×{len(meds)}" if meds else "")
            lines.append(f"### {probe} [{tag}]\n")
            lines.append(f"**裁定修复**：{PROBE_FIX.get(probe, '')}\n")
            for x in sorted(its, key=lambda z: z["severity"]):
                loc = f" L{x['line']}" if x.get("line") else ""
                ev = f" — `{x['evidence']}`" if x.get("evidence") else ""
                lines.append(f"- `{x['severity']}` {x['message']}{loc}{ev}")
            lines.append("")
    open(sys.argv[2], "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print(f"wrote {sys.argv[2]} ({sum(len(v) for v in by_lane.values())} findings mapped)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
