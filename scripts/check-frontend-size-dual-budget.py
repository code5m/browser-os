#!/usr/bin/env python3
"""Fail-closed current frontend size gates: approved absolute budgets,
exact production asset verification and a same-run master increment limit.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EDITOR_POLICY = "docs/engineering/coding-editor-budget.json"
METRICS = "scripts/measure-build-metrics.py"
# A small, blocking incremental performance review budget.
# The separately approved editor/core/total absolute caps remain mandatory.
MASTER_GROWTH_LIMIT_PCT = 1.0


def metrics_module():
    spec = importlib.util.spec_from_file_location("browseros_metrics", ROOT / METRICS)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load build metrics implementation")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def run(args: list[str], cwd: Path = ROOT) -> str:
    result = subprocess.run(args, cwd=cwd, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, errors="replace")
    if result.returncode:
        raise RuntimeError(f"{' '.join(args)} failed ({result.returncode}):\n"
                           + result.stdout[-6000:])
    return result.stdout


def git(*args: str) -> str:
    return run(["git", *args]).strip()


def make_snapshot(mod, dist: Path) -> dict:
    if not dist.is_dir() or not (dist / "index.html").is_file():
        raise RuntimeError(f"missing production dist: {dist}")
    result = mod.collect_dist(dist)
    if result["total_bytes"] <= 0 or result["file_count"] < 2:
        raise RuntimeError(f"invalid production dist: {dist}")
    return result


def delta(base_bytes: int, current_bytes: int, limit_pct: float = MASTER_GROWTH_LIMIT_PCT) -> dict:
    if type(base_bytes) is not int or base_bytes <= 0 or type(current_bytes) is not int or current_bytes < 0:
        raise ValueError("invalid measured build sizes")
    change = current_bytes - base_bytes
    pct = change / base_bytes * 100
    return {"delta_bytes": change, "delta_pct": round(pct, 5),
            "limit_pct": limit_pct, "status": "PASS" if pct <= limit_pct else "FAIL"}


def same_blob(head: str, base: str, path: str) -> bool:
    return git("rev-parse", f"{head}:{path}") == git("rev-parse", f"{base}:{path}")


def approved_policy_unchanged(base: str) -> bool:
    """Only drop obsolete reporting metadata; never silently revise caps or deps."""
    base_policy = json.loads(git("show", f"{base}:{EDITOR_POLICY}"))
    current_policy = json.loads((ROOT / EDITOR_POLICY).read_text(encoding="utf-8"))
    base_policy.pop("historicalBaseline", None)
    base_policy.pop("historicalGrowthLimitPct", None)
    return current_policy == base_policy


def build_master(ref: str, mod) -> tuple[str, dict]:
    sha = git("rev-parse", "--verify", f"{ref}^{{commit}}")
    if not re.fullmatch(r"[0-9a-f]{40}", sha):
        raise RuntimeError("master reference does not resolve to a commit SHA")
    # An archive is a clean independent source copy, not a git worktree.
    # Build current (already produced by CI) and master on the SAME Linux runner,
    # Node version, npm commands and dist byte counter.
    with tempfile.TemporaryDirectory(prefix="browseros-master-", dir=os.getenv("RUNNER_TEMP")) as temp:
        source = Path(temp) / "source"
        source.mkdir()
        archive = Path(temp) / "master.tar"
        run(["git", "archive", "--format=tar", f"--output={archive}", sha])
        run(["tar", "-xf", str(archive), "-C", str(source)])
        print("MASTER_BUILD_REF=" + sha, flush=True)
        run(["npm", "ci"], source)
        output = run(["npm", "run", "build"], source)
        print("MASTER_PRODUCTION_BUILD=PASS " + next(
            (line.strip() for line in output.splitlines() if "built in " in line), ""), flush=True)
        return sha, make_snapshot(mod, source / "dist")


def audited_editor(output: Path) -> dict:
    run(["node", "scripts/coding-editor-budget.mjs", "--self-test"])
    cmd = ["node", "scripts/report-coding-editor-budget.mjs", "--verify-dist", "--enforce-budget"]
    result = subprocess.run(cmd, cwd=ROOT, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True, errors="replace")
    if result.stdout.strip():
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(result.stdout, encoding="utf-8")
    if result.returncode:
        raise RuntimeError("approved editor audit failed:\n"
                           + (result.stderr + "\n" + result.stdout)[-6000:])
    audit = json.loads(result.stdout)
    if audit.get("approved_budget", {}).get("status") != "PASS" or audit.get("dist_verification", {}).get("status") != "PASS":
        raise RuntimeError("editor budget or exact dist hashes not verified")
    return {"status": "PASS",
            "exclusive_editor_bytes": audit["exclusive_editor_bytes"],
            "core_and_shared_bytes": audit["core_and_shared_bytes"],
            "total_bytes": audit["total_bytes"],
            "files_sha256_verified": audit["dist_verification"]["files"],
            "budget": audit["approved_budget"]}


def check(ref: str, out: Path, editor_out: Path) -> int:
    report: dict = {"schema": "browseros-frontend-size-gate-v2",
                    "generated_at": datetime.now(timezone.utc).isoformat(),
                    "status": "BLOCKED", "errors": []}
    errors = report["errors"]
    try:
        mod = metrics_module()
        current = make_snapshot(mod, ROOT / "dist")
        report["candidate"] = {"sha": git("rev-parse", "HEAD"), "dist": current}
        base_sha = git("rev-parse", "--verify", f"{ref}^{{commit}}")
        report["master_ref"] = base_sha
        immutable = [METRICS]
        same = {name: same_blob("HEAD", base_sha, name) for name in immutable}
        same[EDITOR_POLICY] = approved_policy_unchanged(base_sha)
        report["immutable_inputs"] = same
        if not all(same.values()):
            errors.append("metrics implementation or approved editor policy differs from master")
        report["build_provenance"] = {
            "node": run(["node", "--version"]).strip(),
            "npm": run(["npm", "--version"]).strip(),
            "same_package_json": same_blob("HEAD", base_sha, "package.json"),
            "same_package_lock": same_blob("HEAD", base_sha, "package-lock.json"),
            "same_vite_config": same_blob("HEAD", base_sha, "vite.config.ts"),
            "command": "npm ci && npm run build",
            "runner": os.getenv("RUNNER_OS", "unknown")}
        try:
            report["approved_editor"] = audited_editor(editor_out)
            print("APPROVED_EDITOR_BUDGET=PASS with exact dist SHA256 verification", flush=True)
        except Exception as exc:
            report["approved_editor"] = {"status": "FAIL"}
            errors.append(f"approved editor budget: {exc}")
        try:
            master_sha, base = build_master(base_sha, mod)
            report["master"] = {"sha": master_sha, "dist": base}
            comparison = delta(base["total_bytes"], current["total_bytes"])
            report["master_increment"] = comparison
            print(f"MASTER_INCREMENT={comparison['status']}: {comparison['delta_bytes']} bytes"
                  f" ({comparison['delta_pct']}%; limit {MASTER_GROWTH_LIMIT_PCT}%)", flush=True)
            if comparison["status"] != "PASS":
                errors.append("same-run master increment exceeds 1% performance review budget")
        except Exception as exc:
            errors.append(f"master same-run build/comparison: {exc}")
        if not errors:
            report["status"] = "PASS"
    except Exception as exc:
        errors.append(f"size evidence preparation: {exc}")
    finally:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print("FRONTEND_SIZE_GATE_RESULT=" + report["status"], flush=True)
        for failure in errors:
            print("::error title=Frontend size gate::" + failure[-1600:].replace("\n", " | "), flush=True)
    return 0 if report["status"].startswith("PASS") else 1


def self_test() -> int:
    assert delta(10000, 10100)["status"] == "PASS"
    assert delta(10000, 10101)["status"] == "FAIL"
    assert delta(10000, 9000)["status"] == "PASS"
    assert delta(10000, 10000)["status"] == "PASS"
    for invalid in [(0, 1), (-1, 100), (100, -1), (True, 2)]:
        try:
            delta(*invalid)
            raise AssertionError(f"expected invalid build sizes {invalid}")
        except ValueError:
            pass
    assert MASTER_GROWTH_LIMIT_PCT == 1.0
    print("FRONTEND_SIZE_GATE_SELF_TEST=PASS threshold/boundary/negative/invalid")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--master-ref", default="origin/master")
    parser.add_argument("--out", type=Path, default=Path("/tmp/browseros-frontend-size-budget.json"))
    parser.add_argument("--editor-out", type=Path, default=Path("/tmp/browseros-editor-budget-audit.json"))
    args = parser.parse_args()
    return self_test() if args.self_test else check(args.master_ref, args.out, args.editor_out)


if __name__ == "__main__":
    sys.exit(main())
