#!/usr/bin/env python3
"""Strict production size evidence, advisory growth/byte thresholds.

Build/asset integrity, dependency attribution and measurement provenance fail closed;
normal increases above approved size envelopes never block a merge.
"""
from __future__ import annotations

import argparse
import ast
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
# Historical review thresholds retained for reporting, never as merge blockers.
# Production asset hashes, attribution and dependency integrity remain mandatory.
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
            "limit_pct": limit_pct, "status": "PASS" if pct <= limit_pct else "WARN", "advisory_only": True}


def same_blob(head: str, base: str, path: str) -> bool:
    return git("rev-parse", f"{head}:{path}") == git("rev-parse", f"{base}:{path}")


def metrics_collector_unchanged(base: str) -> bool:
    """Permit removal of obsolete M0 verdicts, but not measurement drift.

    The only imported size functions are collect_dist and gzip_bytes. Compare
    their exact normalized ASTs to the base commit (comments/CLI may evolve).
    """
    original = ast.parse(git("show", f"{base}:{METRICS}"))
    current = ast.parse((ROOT / METRICS).read_text(encoding="utf-8"))
    def selected(tree):
        return {node.name: ast.dump(node, include_attributes=False)
                for node in tree.body if isinstance(node, ast.FunctionDef)
                and node.name in {"collect_dist", "gzip_bytes"}}
    original_functions = selected(original)
    return len(original_functions) == 2 and selected(current) == original_functions


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
    cmd = ["node", "scripts/report-coding-editor-budget.mjs", "--verify-dist", "--audit-budget"]
    result = subprocess.run(cmd, cwd=ROOT, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True, errors="replace")
    if result.stdout.strip():
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(result.stdout, encoding="utf-8")
    if result.returncode:
        raise RuntimeError("approved editor audit failed:\n"
                           + (result.stderr + "\n" + result.stdout)[-6000:])
    audit = json.loads(result.stdout)
    if audit.get("approved_budget", {}).get("status") not in ("PASS", "WARN") or audit.get("approved_budget", {}).get("advisory_only") is not True or audit.get("dist_verification", {}).get("status") != "PASS":
        raise RuntimeError("editor ownership/metric assertions or exact dist hashes not verified")
    return {"status": "PASS",
            "exclusive_editor_bytes": audit["exclusive_editor_bytes"],
            "core_and_shared_bytes": audit["core_and_shared_bytes"],
            "total_bytes": audit["total_bytes"],
            "files_sha256_verified": audit["dist_verification"]["files"],
            "budget_status": audit["approved_budget"]["status"],
            "budget": audit["approved_budget"]}


def check(ref: str, out: Path, editor_out: Path) -> int:
    report: dict = {"schema": "browseros-frontend-size-gate-v2",
                    "generated_at": datetime.now(timezone.utc).isoformat(),
                    "status": "BLOCKED", "errors": [], "warnings": []}
    errors = report["errors"]
    warnings = report["warnings"]
    try:
        mod = metrics_module()
        current = make_snapshot(mod, ROOT / "dist")
        report["candidate"] = {"sha": git("rev-parse", "HEAD"), "dist": current}
        base_sha = git("rev-parse", "--verify", f"{ref}^{{commit}}")
        report["master_ref"] = base_sha
        same = {METRICS: metrics_collector_unchanged(base_sha),
                EDITOR_POLICY: approved_policy_unchanged(base_sha)}
        report["immutable_inputs"] = same
        if not all(same.values()):
            errors.append("production size collector or approved editor caps/dependencies differ from master")
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
            budget = report["approved_editor"]["budget"]
            print("APPROVED_EDITOR_SIZE=" + budget["status"] + " with exact dist SHA256 verification", flush=True)
            if budget["status"] == "WARN":
                warnings.append("absolute size budgets exceeded: " + ", ".join(budget["failures"]))
        except Exception as exc:
            report["approved_editor"] = {"status": "FAIL"}
            errors.append(f"production asset integrity/ownership audit: {exc}")
        try:
            master_sha, base = build_master(base_sha, mod)
            report["master"] = {"sha": master_sha, "dist": base}
            comparison = delta(base["total_bytes"], current["total_bytes"])
            report["master_increment"] = comparison
            print(f"MASTER_INCREMENT={comparison['status']}: {comparison['delta_bytes']} bytes"
                  f" ({comparison['delta_pct']}%; limit {MASTER_GROWTH_LIMIT_PCT}%)", flush=True)
            if comparison["status"] == "WARN":
                warnings.append("same-run master increment above 1% advisory threshold")
        except Exception as exc:
            report["master_increment"] = {"status": "UNAVAILABLE", "advisory_only": True}
            warnings.append(f"same-run master comparison unavailable: {exc}")
        if not errors:
            report["status"] = "PASS"
    except Exception as exc:
        errors.append(f"size evidence preparation: {exc}")
    finally:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print("FRONTEND_SIZE_GATE_RESULT=" + report["status"], flush=True)
        for warning in warnings:
            print("::warning title=Frontend size advisory::" + warning[-1600:].replace("\n", " | "), flush=True)
        for failure in errors:
            print("::error title=Frontend size evidence::" + failure[-1600:].replace("\n", " | "), flush=True)
    return 0 if report["status"].startswith("PASS") else 1



def self_test_advisory_and_integrity() -> None:
    """Exercise the real gate result when both thresholds are exceeded.

    Synthetic samples avoid a second production build for policy-negative tests.
    A size overrun must never cause a nonzero exit; a hash/owner failure must.
    """
    from contextlib import redirect_stdout
    from io import StringIO
    from unittest.mock import patch

    def raise_error(*_args):
        raise RuntimeError("fixture production hashes differ")

    with tempfile.TemporaryDirectory(prefix="browseros-size-fixture-") as temp:
        path = Path(temp) / "result.json"
        editor = Path(temp) / "editor.json"
        fake = {
            "metrics_module": lambda: object(),
            "make_snapshot": lambda _mod, _path: {"total_bytes": 20000, "file_count": 3},
            "git": lambda *_args: "a" * 40,
            "metrics_collector_unchanged": lambda _base: True,
            "approved_policy_unchanged": lambda _base: True,
            "same_blob": lambda *_args: True,
            "run": lambda _args: "v22.0.0",
            "audited_editor": lambda _out: {"status": "PASS", "budget": {"status": "WARN", "failures": ["total_bytes"], "advisory_only": True}},
            "build_master": lambda _ref, _mod: ("b" * 40, {"total_bytes": 10000}),
        }
        with patch.dict(globals(), fake), redirect_stdout(StringIO()):
            assert check("origin/master", path, editor) == 0
            report = json.loads(path.read_text(encoding="utf-8"))
            assert report["status"] == "PASS" and not report["errors"]
            assert report["master_increment"]["status"] == "WARN"
            assert report["approved_editor"]["budget"]["status"] == "WARN"
            assert len(report["warnings"]) == 2
            with patch.dict(globals(), {"build_master": raise_error}):
                assert check("origin/master", path, editor) == 0
                unavailable = json.loads(path.read_text(encoding="utf-8"))
                assert unavailable["master_increment"]["status"] == "UNAVAILABLE"
            with patch.dict(globals(), {"audited_editor": raise_error}):
                assert check("origin/master", path, editor) == 1
                blocked = json.loads(path.read_text(encoding="utf-8"))
                assert blocked["status"] == "BLOCKED"
                assert "production asset integrity" in blocked["errors"][0]


def self_test() -> int:
    assert delta(10000, 10100)["status"] == "PASS"
    assert delta(10000, 10101)["status"] == "WARN"
    assert delta(10000, 20000)["advisory_only"] is True
    assert delta(10000, 9000)["status"] == "PASS"
    assert delta(10000, 10000)["status"] == "PASS"
    for invalid in [(0, 1), (-1, 100), (100, -1), (True, 2)]:
        try:
            delta(*invalid)
            raise AssertionError(f"expected invalid build sizes {invalid}")
        except ValueError:
            pass
    assert MASTER_GROWTH_LIMIT_PCT == 1.0
    self_test_advisory_and_integrity()
    print("FRONTEND_SIZE_GATE_SELF_TEST=PASS advisory-threshold/unavailable/strict-integrity/invalid")
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
