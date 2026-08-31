#!/usr/bin/env python3
"""Summarize M0 resource-cycle evidence into an ownership/plateau matrix."""

from __future__ import annotations

import argparse
import json
import math
import re
from collections import defaultdict
from pathlib import Path
from statistics import median
from typing import Any


KINDS = ("tab", "grid", "terminal")
LATE_WINDOW = 20
PLATEAU_SLOPE_KIB_PER_CYCLE = 50.0


def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def ols_slope(values: list[float]) -> float | None:
    if not values:
        return None
    if len(values) == 1:
        return 0.0
    xs = list(range(len(values)))
    mean_x = sum(xs) / len(xs)
    mean_y = sum(values) / len(values)
    den = sum((x - mean_x) ** 2 for x in xs)
    if den == 0:
        return 0.0
    return sum((xs[i] - mean_x) * (values[i] - mean_y) for i in range(len(values))) / den


def stats(values: list[float]) -> dict[str, Any]:
    if not values:
        return {
            "count": 0,
            "first": None,
            "last": None,
            "min": None,
            "max": None,
            "median": None,
            "last_minus_first": None,
            "pct": None,
            "ols_slope": None,
        }
    first = values[0]
    last = values[-1]
    return {
        "count": len(values),
        "first": first,
        "last": last,
        "min": min(values),
        "max": max(values),
        "median": median(values),
        "last_minus_first": last - first,
        "pct": ((last - first) / first * 100.0) if first else 0.0,
        "ols_slope": ols_slope(values),
    }


def cycle_no(path: Path) -> int:
    match = re.search(r"_c(\d+)_", path.name)
    if not match:
        raise ValueError(f"cannot parse cycle number from {path}")
    return int(match.group(1))


def process_role(member: dict[str, Any], root_pid: int) -> str:
    if member.get("pid") == root_pid:
        return "root:mvp-browser-os"
    comm = (member.get("comm") or "").strip()
    cmdline = (member.get("cmdline") or "").strip()
    if comm:
        return f"child:{comm}"
    if cmdline:
        return f"child:{Path(cmdline.split()[0]).name}"
    return "child:unknown"


def summarize_members(raw_dir: Path, kind: str) -> dict[str, Any]:
    closed_paths = sorted(raw_dir.glob(f"{kind}_cycle_c*_closed.json"), key=cycle_no)
    opened_paths = sorted(raw_dir.glob(f"{kind}_cycle_c*_opened.json"), key=cycle_no)
    baseline_paths = sorted(raw_dir.glob(f"{kind}_cycle_c*_baseline.json"), key=cycle_no)
    by_role: dict[str, dict[str, list[float]]] = defaultdict(lambda: {"rss_kib": [], "fd_count": [], "present": []})
    closed_member_counts: list[float] = []
    opened_extra_counts: list[float] = []

    for path in closed_paths:
        snap = load_json(path)
        root_pid = int(snap["root_pid"])
        closed_member_counts.append(float(len(snap.get("members", []))))
        per_role: dict[str, dict[str, float]] = defaultdict(lambda: {"rss_kib": 0.0, "fd_count": 0.0})
        for member in snap.get("members", []):
            role = process_role(member, root_pid)
            if member.get("rss_kib") is not None:
                per_role[role]["rss_kib"] += float(member["rss_kib"])
            if member.get("fd_count") is not None:
                per_role[role]["fd_count"] += float(member["fd_count"])
        for role, values in per_role.items():
            by_role[role]["rss_kib"].append(values["rss_kib"])
            by_role[role]["fd_count"].append(values["fd_count"])
            by_role[role]["present"].append(1.0)

    for baseline_path, opened_path in zip(baseline_paths, opened_paths):
        baseline = load_json(baseline_path)
        opened = load_json(opened_path)
        opened_extra_counts.append(float(len(opened.get("members", [])) - len(baseline.get("members", []))))

    roles = {}
    for role, values in sorted(by_role.items()):
        roles[role] = {
            "presence_cycles": int(sum(values["present"])),
            "rss_kib": stats(values["rss_kib"]),
            "fd_count": stats(values["fd_count"]),
        }

    return {
        "closed_cycles": len(closed_paths),
        "closed_member_count": stats(closed_member_counts),
        "opened_minus_baseline_member_count": stats(opened_extra_counts),
        "roles": roles,
    }


def summarize_kind(run_dir: Path, kind: str) -> dict[str, Any] | None:
    measure_path = run_dir / "measurements" / f"{kind}_cycle.json"
    if not measure_path.exists():
        return None
    measure = load_json(measure_path)
    samples = measure.get("samples", [])
    rss = [float(sample["rss_kib"]) for sample in samples if "rss_kib" in sample]
    fds = [float(sample["fd_count"]) for sample in samples if "fd_count" in sample]
    orphans = [float(sample.get("orphan_count", 0)) for sample in samples]
    candidates = [float(sample.get("candidate_count", 0)) for sample in samples]
    late_rss = rss[-LATE_WINDOW:] if len(rss) >= LATE_WINDOW else []
    early_rss = rss[:LATE_WINDOW] if len(rss) >= LATE_WINDOW else []
    late_slope = ols_slope(late_rss)
    early_slope = ols_slope(early_rss)
    fd_delta = int(measure.get("fd_delta", 0))
    orphan_max = int(max(orphans)) if orphans else 0
    rss_stats = stats(rss)
    plateau = (
        len(late_rss) >= LATE_WINDOW
        and late_slope is not None
        and late_slope <= PLATEAU_SLOPE_KIB_PER_CYCLE
        and orphan_max == 0
        and fd_delta <= 0
    )
    if len(late_rss) < LATE_WINDOW:
        verdict = "INSUFFICIENT_EXTENDED_SAMPLES"
    elif orphan_max > 0:
        verdict = "LEAK_SUSPECT_ORPHAN_PROCESS"
    elif fd_delta > 0:
        verdict = "LEAK_SUSPECT_FD_GROWTH"
    elif late_slope is not None and late_slope > PLATEAU_SLOPE_KIB_PER_CYCLE:
        verdict = "LEAK_SUSPECT_RSS_NOT_PLATEAUED"
    else:
        verdict = "PLATEAU_OR_NO_GROWTH"

    return {
        "kind": kind,
        "formal_samples": int(measure.get("formal", len(samples))),
        "warmup_samples": int(measure.get("warmup", 0)),
        "rss_kib": rss_stats,
        "rss_early_window_slope_kib_per_cycle": early_slope,
        "rss_late_window_slope_kib_per_cycle": late_slope,
        "fd_delta": fd_delta,
        "fd_count": stats(fds),
        "orphan_count": stats(orphans),
        "orphan_max": orphan_max,
        "candidate_count": stats(candidates),
        "plateau_threshold_kib_per_cycle": PLATEAU_SLOPE_KIB_PER_CYCLE,
        "plateau": plateau,
        "verdict": verdict,
        "process_members": summarize_members(run_dir / "raw", kind),
    }


def fmt_num(value: Any, digits: int = 2) -> str:
    if value is None:
        return "n/a"
    if isinstance(value, float):
        if math.isnan(value) or math.isinf(value):
            return "n/a"
        if value.is_integer():
            return str(int(value))
        return f"{value:.{digits}f}"
    return str(value)


def write_markdown(summary: dict[str, Any], output: Path) -> None:
    lines = [
        f"# M0-5.a Resource Cycle Diagnosis - {summary['run_id']}",
        "",
        f"> Run dir: `{summary['run_dir']}`",
        f"> Commit: `{summary.get('commit', 'unknown')}`; mode: `{summary.get('run_mode', 'unknown')}`",
        f"> Plateau rule: late-window ({LATE_WINDOW} samples) OLS slope <= {PLATEAU_SLOPE_KIB_PER_CYCLE:.0f} KiB/cycle, FD delta <= 0, orphan max = 0.",
        "",
        "## Cycle Summary",
        "",
        "| kind | samples | rss first->last KiB | rss pct | total slope | late slope | fd delta | orphan max | verdict |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for kind in KINDS:
        item = summary["kinds"].get(kind)
        if not item:
            continue
        rss = item["rss_kib"]
        lines.append(
            "| {kind} | {samples} | {first}->{last} | {pct}% | {slope} | {late} | {fd} | {orphans} | {verdict} |".format(
                kind=kind,
                samples=item["formal_samples"],
                first=fmt_num(rss["first"], 0),
                last=fmt_num(rss["last"], 0),
                pct=fmt_num(rss["pct"]),
                slope=fmt_num(rss["ols_slope"]),
                late=fmt_num(item["rss_late_window_slope_kib_per_cycle"]),
                fd=item["fd_delta"],
                orphans=item["orphan_max"],
                verdict=item["verdict"],
            )
        )
    lines += ["", "## Process Member Matrix", ""]
    for kind in KINDS:
        item = summary["kinds"].get(kind)
        if not item:
            continue
        lines += [
            f"### {kind}",
            "",
            "| role | presence | rss first->last KiB | rss slope | fd first->last | fd slope |",
            "|---|---:|---:|---:|---:|---:|",
        ]
        roles = item["process_members"]["roles"]
        for role, role_stats in roles.items():
            rr = role_stats["rss_kib"]
            ff = role_stats["fd_count"]
            lines.append(
                "| {role} | {presence} | {rf}->{rl} | {rs} | {ff}->{fl} | {fs} |".format(
                    role=role,
                    presence=role_stats["presence_cycles"],
                    rf=fmt_num(rr["first"], 0),
                    rl=fmt_num(rr["last"], 0),
                    rs=fmt_num(rr["ols_slope"]),
                    ff=fmt_num(ff["first"], 0),
                    fl=fmt_num(ff["last"], 0),
                    fs=fmt_num(ff["ols_slope"]),
                )
            )
        lines.append("")
    output.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("--output-json", type=Path)
    parser.add_argument("--output-md", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        values = [10, 20, 30]
        assert stats(values)["median"] == 20
        assert round(ols_slope(values) or 0, 2) == 10.0
        print("SELF_TEST_RESULT=ALL_PASS")
        return 0

    run_dir = args.run_dir.resolve()
    summary_path = run_dir / "summary.json"
    source_summary = load_json(summary_path) if summary_path.exists() else {}
    out = {
        "run_id": source_summary.get("run_id", run_dir.name),
        "run_dir": str(run_dir),
        "commit": (source_summary.get("git") or {}).get("short_sha"),
        "run_mode": source_summary.get("run_mode"),
        "source_status": source_summary.get("status"),
        "kinds": {},
    }
    for kind in KINDS:
        item = summarize_kind(run_dir, kind)
        if item:
            out["kinds"][kind] = item

    if args.output_json:
        args.output_json.parent.mkdir(parents=True, exist_ok=True)
        args.output_json.write_text(json.dumps(out, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if args.output_md:
        args.output_md.parent.mkdir(parents=True, exist_ok=True)
        write_markdown(out, args.output_md)
    if not args.output_json and not args.output_md:
        print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
