#!/usr/bin/env python3
"""Aggregate staged M0 quality/resource runs into an M0-0.b or M0-0.c verdict."""

import argparse
import glob
import hashlib
import json
import statistics
import sys
from pathlib import Path


def load_json(path):
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def one(pattern):
    matches = sorted(glob.glob(pattern))
    if len(matches) != 1:
        raise ValueError(f"expected exactly one match for {pattern!r}, got {len(matches)}")
    return Path(matches[0])


def nested(data, *keys):
    value = data
    for key in keys:
        value = value[key]
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"metric {'.'.join(keys)} is not numeric: {value!r}")
    return value


def cycle_metrics(run_dir, kind):
    data = load_json(run_dir / "measurements" / f"{kind}_cycle.json")
    orphan_files = sorted((run_dir / "measurements").glob(f"orphan_{kind}_c*.json"))
    if not orphan_files:
        raise ValueError(f"no orphan samples for {kind}")
    orphan_counts = [nested(load_json(path), "data", "orphan_count") for path in orphan_files]
    return {
        f"{kind}_cycle_rss_slope_kib": nested(data, "stats", "ols_slope"),
        f"{kind}_resource_cycle_fd_delta": nested(data, "fd_delta"),
        f"{kind}_orphan_process_count_max": max(orphan_counts),
    }


def extract_metrics(quality, resource, resource_dir):
    results = quality["results"]
    startup = load_json(resource_dir / "measurements" / "startup_ready.json")
    idle = load_json(resource_dir / "measurements" / "idle_process_tree.json")
    terminal = load_json(resource_dir / "measurements" / "terminal_throughput.json")
    metrics = {
        "clippy_main_unique_warnings": nested(results, "clippy_main_unique_warnings", "count"),
        "clippy_plugin_unique_warnings": nested(results, "clippy_plugin_unique_warnings", "count"),
        "frontend_build_ms": nested(results, "frontend_build_ms", "median_ms"),
        "dist_total_bytes": nested(results, "dist_total_bytes", "bytes"),
        "largest_js_bytes": nested(results, "largest_js_bytes", "bytes"),
        "largest_js_gzip_bytes": nested(results, "largest_js_gzip_bytes", "bytes"),
        "release_binary_bytes": nested(results, "release_binary_bytes", "bytes"),
        "startup_ready_ms": nested(startup, "stats", "median"),
        "idle_process_tree_rss_kib": nested(idle, "rss_kib", "stats", "median"),
        "idle_process_tree_fd_count": nested(idle, "fd_count", "stats", "median"),
        "terminal_10mib_elapsed_ms": nested(terminal, "stats", "median"),
        "terminal_frame_gap_p95_ms": nested(terminal, "frame_gap_p95_ms"),
        "terminal_frame_gap_max_ms": nested(terminal, "frame_gap_max_ms"),
    }
    for kind in ("tab", "grid", "terminal"):
        metrics.update(cycle_metrics(resource_dir, kind))
    return metrics


def summarize_values(values):
    median = statistics.median(values)
    minimum = min(values)
    maximum = max(values)
    if median == 0:
        volatility = None
        unstable = maximum != minimum
    else:
        volatility = abs((maximum - minimum) / median * 100.0)
        unstable = volatility > 10.0
    return {
        "values": values,
        "median": median,
        "min": minimum,
        "max": maximum,
        "volatility_pct": volatility,
        "absolute_range_when_median_zero": maximum - minimum if median == 0 else None,
        "status": "UNSTABLE" if unstable else "PASS",
    }


def hash_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def aggregate(stage_root, batches, mode, output_dir):
    stage_root = Path(stage_root)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    expected_status = "PASS" if mode == "formal" else "EXPLORATORY"
    errors = []
    incomparable = []
    records = []
    metric_values = {}
    reference_environment = None
    reference_source = None
    reference_commit = None

    for number in range(1, batches + 1):
        batch_dir = stage_root / f"batch-{number:02d}"
        quality_summary_path = one(str(batch_dir / "quality" / "*" / "summary.json"))
        resource_summary_path = one(str(batch_dir / "resources" / "*" / "summary.json"))
        quality_dir = quality_summary_path.parent
        resource_dir = resource_summary_path.parent
        quality = load_json(quality_summary_path)
        resource = load_json(resource_summary_path)
        quality_environment = load_json(quality_dir / "environment.json")
        resource_environment = load_json(resource_dir / "environment.json")

        if quality.get("status") != expected_status:
            errors.append(f"batch {number}: quality status={quality.get('status')}, expected {expected_status}")
        if resource.get("status") != expected_status:
            errors.append(f"batch {number}: resource status={resource.get('status')}, expected {expected_status}")
        quality_commit = quality.get("git", {}).get("commit_sha")
        resource_commit = resource.get("git", {}).get("commit_sha")
        if quality_commit != resource_commit:
            incomparable.append(f"batch {number}: quality/resource commits differ")
        quality_hash = quality.get("artifact", {}).get("binary_sha256")
        resource_hash = resource.get("artifact", {}).get("binary_sha256")
        if quality_hash != resource_hash:
            incomparable.append(f"batch {number}: quality/resource binary hashes differ")
        if resource.get("artifact", {}).get("matches_expected") is not True:
            errors.append(f"batch {number}: resource binary did not match quality binary")
        if quality_environment.get("environment_fingerprint") != resource_environment.get("environment_fingerprint"):
            incomparable.append(f"batch {number}: quality/resource environment fingerprints differ")

        source = quality_environment.get("source_fingerprint")
        environment = quality_environment.get("environment_fingerprint")
        if reference_commit is None:
            reference_commit = quality_commit
            reference_source = source
            reference_environment = environment
        else:
            if quality_commit != reference_commit:
                incomparable.append(f"batch {number}: commit differs from batch 1")
            if source != reference_source:
                incomparable.append(f"batch {number}: source fingerprint differs from batch 1")
            if environment != reference_environment:
                incomparable.append(f"batch {number}: environment fingerprint differs from batch 1")

        metrics = extract_metrics(quality, resource, resource_dir)
        for metric, value in metrics.items():
            metric_values.setdefault(metric, []).append(value)
        records.append({
            "batch": number,
            "quality_run_id": quality.get("run_id"),
            "resource_run_id": resource.get("run_id"),
            "commit_sha": quality_commit,
            "binary_sha256": quality_hash,
            "quality_summary_sha256": hash_file(quality_summary_path),
            "resource_summary_sha256": hash_file(resource_summary_path),
            "metrics": metrics,
        })

    metric_summaries = {name: summarize_values(values) for name, values in sorted(metric_values.items())}
    unstable_metrics = [name for name, item in metric_summaries.items() if item["status"] == "UNSTABLE"]
    if errors:
        status = "FAIL"
    elif incomparable:
        status = "INCOMPARABLE"
    elif mode == "smoke":
        status = "EXPLORATORY"
    elif batches == 3 and unstable_metrics:
        status = "UNSTABLE"
    else:
        status = "PASS"

    checkpoint = "M0-0.c" if batches == 3 else "M0-0.b"
    result = {
        "checkpoint": checkpoint,
        "status": status,
        "run_mode": mode,
        "batches_expected": batches,
        "batches_completed": len(records),
        "commit_sha": reference_commit,
        "source_fingerprint": reference_source,
        "environment_fingerprint": reference_environment,
        "records": records,
        "metrics": metric_summaries,
        "unstable_metrics": unstable_metrics,
        "incomparable_reasons": incomparable,
        "errors": errors,
    }
    with open(output_dir / "summary.json", "w", encoding="utf-8") as handle:
        json.dump(result, handle, ensure_ascii=False, indent=2)
        handle.write("\n")
    with open(output_dir / "summary.md", "w", encoding="utf-8") as handle:
        handle.write(f"# {checkpoint} baseline aggregate\n\n")
        handle.write(f"- Status: **{status}**\n")
        handle.write(f"- Mode: `{mode}`\n")
        handle.write(f"- Commit: `{reference_commit}`\n")
        handle.write(f"- Batches: {len(records)}/{batches}\n")
        handle.write(f"- Unstable metrics: {', '.join(unstable_metrics) if unstable_metrics else 'none'}\n")
        handle.write(f"- Incomparable reasons: {'; '.join(incomparable) if incomparable else 'none'}\n")
        handle.write(f"- Errors: {'; '.join(errors) if errors else 'none'}\n")
    return status


def self_test():
    stable = summarize_values([100, 105, 95])
    unstable = summarize_values([100, 120, 80])
    zero_stable = summarize_values([0, 0, 0])
    zero_unstable = summarize_values([0, 1, 0])
    if stable["status"] != "PASS" or round(stable["volatility_pct"], 1) != 10.0:
        print("FAIL: 10% boundary should pass")
        return 1
    if unstable["status"] != "UNSTABLE" or unstable["volatility_pct"] <= 10:
        print("FAIL: >10% should be unstable")
        return 1
    if zero_stable["status"] != "PASS" or zero_unstable["status"] != "UNSTABLE":
        print("FAIL: zero-median absolute range handling")
        return 1
    print("SELF_TEST_RESULT=ALL_PASS")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--stage-root")
    parser.add_argument("--batches", type=int, choices=(1, 3))
    parser.add_argument("--mode", choices=("formal", "smoke"))
    parser.add_argument("--output-dir")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if not all((args.stage_root, args.batches, args.mode, args.output_dir)):
        parser.error("--stage-root, --batches, --mode and --output-dir are required")
    try:
        status = aggregate(args.stage_root, args.batches, args.mode, args.output_dir)
    except (OSError, KeyError, ValueError, json.JSONDecodeError) as exc:
        print(f"error: aggregate failed: {exc}", file=sys.stderr)
        return 1
    print(f"M0_AGGREGATE_STATUS={status}")
    return 0 if status in ("PASS", "EXPLORATORY") else 1


if __name__ == "__main__":
    sys.exit(main())
