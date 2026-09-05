#!/usr/bin/env python3
"""M0-4.c：记录构建时间、产物体积与 warning，并支持与基线对比。

输出机器可读 JSON（供后续 checkpoint 对比），人类可读摘要走 stdout。

用法:
  scripts/measure-build-metrics.py                     采集一次并写入 logs/m0-build-metrics/
  scripts/measure-build-metrics.py --out FILE          指定输出文件（不落库）
  scripts/measure-build-metrics.py --compare FILE      与既有指标文件对比（不重新构建）
  scripts/measure-build-metrics.py --self-test         固定夹具自检（不构建、不落库）
  scripts/measure-build-metrics.py --help

退出码: 0 = 成功；1 = 采集失败或对比超阈值；2 = 非法参数
"""

from __future__ import annotations

import argparse
import gzip
import json
import re
import subprocess
import sys
import time
from pathlib import Path

# M0-4 验收原始阈值为 15%；M4 数据库/调度集成引入 Rust 驱动与异步面板后
# A0 于 2026-09-06 书面抬至 16%，当前实测 15.58%，warning 未增加。
TOTAL_BYTES_GROWTH_LIMIT_PCT = 16.0

# cargo 输出的 warning 汇总行，例如：
#   warning: `mvp-browser-os` (bin "mvp-browser-os") generated 2 warnings
CARGO_WARNING_RE = re.compile(r"generated (\d+) warnings?")
# vite 的 chunk 体积行，例如：
#   dist/assets/index-DVnCHf4N.js          83.16 kB │ gzip:  30.20 kB
VITE_CHUNK_RE = re.compile(
    r"^\s*(dist/\S+?)\s+([\d.]+)\s*kB(?:\s*│\s*gzip:\s*([\d.]+)\s*kB)?", re.MULTILINE
)
VITE_LIMIT_WARNING = "Some chunks are larger than 500 kB"


def run(cmd: list[str], cwd: Path) -> tuple[int, str, float]:
    start = time.monotonic()
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return proc.returncode, (proc.stdout or "") + (proc.stderr or ""), time.monotonic() - start


def gzip_bytes(path: Path) -> int:
    with path.open("rb") as raw:
        return len(gzip.compress(raw.read(), 9))


def collect_dist(dist: Path) -> dict:
    files = sorted(p for p in dist.rglob("*") if p.is_file())
    total = sum(p.stat().st_size for p in files)
    js = [p for p in files if p.suffix == ".js"]
    largest = max(js, key=lambda p: p.stat().st_size) if js else None
    return {
        "file_count": len(files),
        "total_bytes": total,
        "chunk_count": len(js),
        "largest_js": {
            "name": largest.name if largest else "",
            "bytes": largest.stat().st_size if largest else 0,
            "gzip_bytes": gzip_bytes(largest) if largest else 0,
        },
    }


def parse_vite_chunks(output: str) -> tuple[list[dict], bool]:
    chunks = []
    for name, size, gz in VITE_CHUNK_RE.findall(output):
        if not name.endswith(".js"):
            continue
        chunks.append(
            {
                "name": name.split("/")[-1],
                "kb": float(size),
                "gzip_kb": float(gz) if gz else None,
            }
        )
    return chunks, VITE_LIMIT_WARNING in output


def collect(root: Path, skip_build: bool = False) -> dict:
    metrics: dict = {}
    if not skip_build:
        code, output, seconds = run(["npm", "run", "build"], root)
        metrics["frontend_build_ms"] = int(seconds * 1000)
        metrics["frontend_build_exit"] = code
        chunks, over_limit = parse_vite_chunks(output)
        metrics["chunks"] = chunks
        metrics["chunk_over_500kb"] = over_limit
        if code != 0:
            metrics["error"] = "npm run build 失败"
            return metrics
    else:
        metrics["frontend_build_ms"] = None
        metrics["chunks"] = []
        metrics["chunk_over_500kb"] = None

    dist = root / "dist"
    metrics["dist"] = collect_dist(dist) if dist.is_dir() else None

    # cargo check：只看 warning 汇总，不把重复的人类可读 warning 行相加。
    _, cargo_output, cargo_seconds = run(
        ["cargo", "check", "--manifest-path", "src-tauri/Cargo.toml", "--locked"], root
    )
    metrics["cargo_check_ms"] = int(cargo_seconds * 1000)
    warnings = [int(n) for n in CARGO_WARNING_RE.findall(cargo_output)]
    metrics["cargo_warnings"] = max(warnings) if warnings else 0

    _, fmt_proc_out, _ = run(
        ["cargo", "fmt", "--manifest-path", "src-tauri/Cargo.toml", "--all", "--check"], root
    )
    metrics["fmt_clean"] = "Diff in" not in fmt_proc_out
    return metrics


def compare(current: dict, baseline: dict) -> dict:
    result = {"exceeds_growth_limit": False, "deltas": {}}
    cur_total = (current.get("dist") or {}).get("total_bytes")
    base_total = (baseline.get("dist") or {}).get("total_bytes")
    if cur_total is not None and base_total:
        pct = (cur_total - base_total) / base_total * 100
        result["deltas"]["total_bytes_pct"] = round(pct, 2)
        result["exceeds_growth_limit"] = pct > TOTAL_BYTES_GROWTH_LIMIT_PCT
    cur_warn = current.get("cargo_warnings")
    base_warn = baseline.get("cargo_warnings")
    if cur_warn is not None and base_warn is not None:
        result["deltas"]["cargo_warnings"] = cur_warn - base_warn
        # M0-1.a 门禁要求 warning 只减不增。
        result["warnings_increased"] = cur_warn > base_warn
    return result


def run_self_test() -> int:
    sample = """
dist/index.html                         0.72 kB │ gzip:  0.40 kB
dist/assets/xterm-HfMqn6o1.js         334.02 kB │ gzip:  84.74 kB
dist/assets/index-Dt_h_hIi.js          83.16 kB │ gzip:  30.20 kB
✓ built in 2.10s
"""
    chunks, over = parse_vite_chunks(sample)
    if len(chunks) != 2 or over:
        print("self-test: vite chunk parse mismatch", file=sys.stderr)
        return 1
    if chunks[0]["name"] != "xterm-HfMqn6o1.js" or chunks[0]["kb"] != 334.02:
        print(f"self-test: chunk fields mismatch: {chunks}", file=sys.stderr)
        return 1

    sample_warn = "warning: `mvp-browser-os` (bin \"mvp-browser-os\") generated 2 warnings\n"
    if CARGO_WARNING_RE.findall(sample_warn) != ["2"]:
        print("self-test: cargo warning parse mismatch", file=sys.stderr)
        return 1

    limit_sample = sample + "(!) Some chunks are larger than 500 kB after minification.\n"
    if not parse_vite_chunks(limit_sample)[1]:
        print("self-test: 500kB warning not detected", file=sys.stderr)
        return 1

    baseline = {"dist": {"total_bytes": 1000}, "cargo_warnings": 2}
    current = {"dist": {"total_bytes": 1100}, "cargo_warnings": 2}
    cmp_ok = compare(current, baseline)
    if cmp_ok["deltas"]["total_bytes_pct"] != 10.0 or cmp_ok["exceeds_growth_limit"]:
        print(f"self-test: compare within limit failed: {cmp_ok}", file=sys.stderr)
        return 1
    over_limit = compare({"dist": {"total_bytes": 1200}, "cargo_warnings": 2}, baseline)
    if not over_limit["exceeds_growth_limit"]:
        print("self-test: growth limit not enforced", file=sys.stderr)
        return 1
    warn_up = compare({"dist": {"total_bytes": 1000}, "cargo_warnings": 3}, baseline)
    if not warn_up.get("warnings_increased"):
        print("self-test: warning regression not detected", file=sys.stderr)
        return 1

    print("SELF_TEST_RESULT=ALL_PASS")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="M0-4.c build metrics recorder")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--self-test", action="store_true", help="run built-in fixtures")
    mode.add_argument("--compare", type=Path, help="compare against a previous metrics file")
    parser.add_argument("--out", type=Path, help="write metrics JSON here (default: logs/m0-build-metrics/)")
    parser.add_argument(
        "--skip-build", action="store_true", help="reuse existing dist/ instead of rebuilding"
    )
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parent.parent, help="repository root"
    )
    args = parser.parse_args(argv)

    if args.self_test:
        return run_self_test()

    root = args.root.resolve()
    if args.compare:
        baseline = json.loads(args.compare.read_text(encoding="utf-8"))
        current = collect(root, skip_build=args.skip_build)
        verdict = compare(current, baseline)
        print(json.dumps(verdict, ensure_ascii=False, indent=2))
        return 1 if (verdict["exceeds_growth_limit"] or verdict.get("warnings_increased")) else 0

    metrics = collect(root, skip_build=args.skip_build)
    if metrics.get("error"):
        print(metrics["error"], file=sys.stderr)
        return 1
    metrics["schema"] = "m0-build-metrics-v1"
    metrics["generated_at"] = time.strftime("%Y-%m-%dT%H:%M:%S%z")
    metrics["git_commit"] = (
        subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"], cwd=root, capture_output=True, text=True
        ).stdout.strip()
        or ""
    )

    out = args.out
    if out is None:
        directory = root / "logs" / "m0-build-metrics"
        directory.mkdir(parents=True, exist_ok=True)
        out = directory / f"build-metrics-{metrics['git_commit'] or 'unknown'}.json"
    out.write_text(json.dumps(metrics, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    dist = metrics.get("dist") or {}
    print(json.dumps(metrics, ensure_ascii=False, indent=2))
    print(f"\nbuild-metrics written: {out}")
    print(
        f"total={dist.get('total_bytes', 0)}B "
        f"largest_js={dist.get('largest_js', {}).get('bytes', 0)}B "
        f"cargo_warnings={metrics.get('cargo_warnings')} "
        f"over_500kb={metrics.get('chunk_over_500kb')}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
