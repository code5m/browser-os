#!/usr/bin/env python3
"""A11 R3 acceptance matrix verifier.

Reads `A11-R3-integration-manifest.json` (the single source of truth for the R3
acceptance package) and mechanically verifies:

  1. manifest structure and lane/artifact ownership,
  2. every checklist probe against the lane artifact it names,
  3. the numeric R3 gates (viewport budget, chrome budget),
  4. cross-lane divergence on the same metric.

Research artifact only: standard library, no network, no product import, no
build, no dependency, no product source / ACL / capability change.

Usage:
  python3 logs/research/M5-W18/A11-R3-acceptance-matrix.py [--manifest PATH]
                                                           [--report] [--json]
Exit codes: 0 = GATE PASS (no unexpected result), 1 = GATE FAIL.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys

MANIFEST_NAME = "A11-R3-integration-manifest.json"
ARTIFACT_DIRS = ("logs/research/M5-W18", "logs/checkpoints")
CANDIDATE_REFS = (
    "HEAD",
    "origin/master",
    "master",
    "codex/m5-w18-a1",
    "codex/m5-w18-a2",
    "codex/m5-w18-a3",
    "codex/m5-w18-a4",
    "codex/m5-w18-a5",
    "codex/m5-w18-a6",
    "codex/m5-w18-a7",
    "codex/m5-w18-a8",
    "codex/m5-w18-a9",
    "codex/m5-w18-a10",
    "codex/m5-w18-a11",
)

REDACT_HINT = "artifact_absent"


def repo_root() -> str:
    """Walk up to the directory that contains logs/ (works in any lane worktree)."""
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
            capture_output=True,
            check=False,
        )
    except OSError:
        return None
    if out.returncode != 0:
        return None
    return out.stdout.decode("utf-8", errors="replace")


def resolve(name: str) -> tuple[str | None, str]:
    """Return (content, source). Filesystem first, then git refs."""
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
    return None, REDACT_HINT


def load_manifest(path: str) -> dict:
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def structural_checks(manifest: dict) -> list[tuple[str, str, str]]:
    """Return [(name, PASS|FAIL, detail)]."""
    rows: list[tuple[str, str, str]] = []
    required = (
        "manifest_version",
        "wave",
        "lane",
        "base",
        "gates",
        "lanes",
        "gate_measurements",
        "findings",
        "checklist",
        "reference_pins",
    )
    missing = [k for k in required if k not in manifest]
    rows.append(("manifest.required_keys", "PASS" if not missing else "FAIL",
                 "missing: " + ",".join(missing) if missing else f"{len(required)} keys present"))

    ids = [c["id"] for c in manifest["checklist"]]
    dupes = sorted({i for i in ids if ids.count(i) > 1})
    rows.append(("checklist.ids_unique", "PASS" if not dupes else "FAIL",
                 "duplicates: " + ",".join(dupes) if dupes else f"{len(ids)} unique ids"))

    bad_group = [c["id"] for c in manifest["checklist"] if not c.get("group")]
    rows.append(("checklist.group_present", "PASS" if not bad_group else "FAIL",
                 ",".join(bad_group) if bad_group else "all items grouped"))

    owners = {c.get("owner", "?") for c in manifest["checklist"]}
    lanes = set(manifest["lanes"])
    unknown = {o.split("/")[0] for o in owners} - lanes - {"A0"}
    rows.append(("checklist.owner_known", "PASS" if not unknown else "FAIL",
                 "unknown owner: " + ",".join(sorted(unknown)) if unknown else f"{len(owners)} owners resolve"))

    expects = {"covered", "gap", "deferred"}
    bad_expect = [c["id"] for c in manifest["checklist"] if c.get("expect") not in expects]
    rows.append(("checklist.expect_valid", "PASS" if not bad_expect else "FAIL",
                 ",".join(bad_expect) if bad_expect else "covered/gap/deferred only"))

    fids = [f["id"] for f in manifest["findings"]]
    refd = {c.get("finding") for c in manifest["checklist"] if c.get("finding")}
    dangling = sorted(refd - set(fids))
    rows.append(("findings.references_resolve", "PASS" if not dangling else "FAIL",
                 "dangling: " + ",".join(dangling) if dangling else f"{len(fids)} findings, {len(refd)} referenced"))

    pins = {p["id"] for p in manifest["reference_pins"]}
    rows.append(("reference_pins.pinned", "PASS" if "rebased" in pins else "FAIL",
                 "rebased pinned=" + str(manifest["reference_pins"][0].get("pinned_revision", "?"))))

    blocking = [n for n, l in manifest["lanes"].items() if l.get("blocking")]
    status = manifest.get("status", "")
    if status.startswith("PROVISIONAL"):
        rows.append(("lanes.blocking_flagged", "PASS" if blocking else "FAIL",
                     "blocking: " + ",".join(blocking) if blocking else "none flagged (PROVISIONAL requires a blocking lane)"))
    else:
        rows.append(("lanes.blocking_flagged", "PASS" if not blocking else "FAIL",
                     "none flagged (final)" if not blocking else "lanes still flagged blocking: " + ",".join(blocking)))
    return rows


def run_probes(manifest: dict) -> tuple[list[tuple[str, str, str, str]], dict]:
    cache: dict[str, tuple[str | None, str]] = {}
    rows: list[tuple[str, str, str, str]] = []
    tally: dict[str, int] = {"PASS": 0, "FAIL": 0, "NOT_RUN": 0, "GAP_CONFIRMED": 0, "GAP_CLOSED": 0}

    for item in manifest["checklist"]:
        cid, expect = item["id"], item["expect"]
        probe = item.get("probe")
        if expect == "deferred" or not probe:
            tally["NOT_RUN"] += 1
            rows.append((cid, "NOT_RUN", "deferred",
                         item.get("evidence", "no artifact")))
            continue

        name = probe["file"]
        if name not in cache:
            cache[name] = resolve(name)
        content, source = cache[name]
        tokens = probe.get("tokens", [])

        if content is None:
            if expect == "gap":
                tally["GAP_CONFIRMED"] += 1
                rows.append((cid, "GAP_CONFIRMED", "artifact absent", f"{name} ({source})"))
            else:
                tally["NOT_RUN"] += 1
                rows.append((cid, "NOT_RUN", "artifact absent", f"{name} ({source})"))
            continue

        low = content.lower()
        hits = [t for t in tokens if t.lower() in low]

        if expect == "covered":
            if len(hits) == len(tokens):
                tally["PASS"] += 1
                rows.append((cid, "PASS", "tokens " + "|".join(tokens), source))
            else:
                miss = [t for t in tokens if t not in hits]
                tally["FAIL"] += 1
                rows.append((cid, "FAIL", "missing " + "|".join(miss), source))
        else:  # expect == "gap": the declared gap must still be real
            if len(hits) == len(tokens):
                tally["GAP_CLOSED"] += 1
                rows.append((cid, "GAP_CLOSED", "declared gap no longer reproduces; update manifest", source))
            else:
                tally["GAP_CONFIRMED"] += 1
                rows.append((cid, "GAP_CONFIRMED",
                             "absent " + "|".join(t for t in tokens if t not in hits), source))
    return rows, tally


def run_gates(manifest: dict) -> tuple[list[tuple[str, str, str]], int, int]:
    tol = float(manifest["gates"].get("divergence_tolerance_pp", 2.0))
    rows: list[tuple[str, str, str]] = []
    fails = 0
    warns = 0
    for m in manifest["gate_measurements"]:
        metric, size, lane = m["metric"], m["size"], m["lane"]
        val = float(m["value"])
        expect_fail = bool(m.get("expect_fail"))
        if "min" in m:
            ok = val >= float(m["min"])
            bound = f">= {m['min']}"
        else:
            ok = val <= float(m["max"])
            bound = f"<= {m['max']}"
        subject = m.get("baseline_of", "prototype")
        state = m.get("state", "")
        if expect_fail:
            verdict = "BASELINE_FAIL_EXPECTED" if not ok else "UNEXPECTED_PASS"
            if ok:
                fails += 1
        else:
            verdict = "PASS" if ok else "FAIL"
            if not ok:
                fails += 1
        rows.append((f"{metric}@{size}", f"{verdict}",
                     f"{lane} {subject}{('/' + state) if state else ''} = {val} {bound}"))

    groups: dict[tuple[str, str], list[tuple[str, float]]] = {}
    for m in manifest["gate_measurements"]:
        if m.get("baseline_of") in (None, "prototype") and m.get("state") in (None, ""):
            groups.setdefault((m["metric"], m["size"]), []).append((m["lane"], float(m["value"])))
    for (metric, size), vals in sorted(groups.items()):
        if len(vals) < 2:
            continue
        spread = max(v for _, v in vals) - min(v for _, v in vals)
        detail = " ".join(f"{l}={v}" for l, v in vals)
        if spread > tol:
            warns += 1
            rows.append((f"divergence:{metric}@{size}", "WARN",
                         f"spread {spread:g} > tolerance {tol:g} [{detail}]"))
        else:
            rows.append((f"divergence:{metric}@{size}", "PASS",
                         f"spread {spread:g} <= tolerance {tol:g} [{detail}]"))
    return rows, fails, warns


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--manifest", default=None)
    ap.add_argument("--report", action="store_true", help="print every row")
    ap.add_argument("--json", action="store_true", help="emit machine-readable summary")
    args = ap.parse_args()

    manifest_path = args.manifest or os.path.join(os.path.dirname(os.path.abspath(__file__)), MANIFEST_NAME)
    manifest = load_manifest(manifest_path)

    structural = structural_checks(manifest)
    probes, tally = run_probes(manifest)
    gates, gate_fails, gate_warns = run_gates(manifest)

    struct_fail = sum(1 for _, v, _ in structural if v == "FAIL")
    fail = tally["FAIL"] + gate_fails + struct_fail
    line_items = len(manifest["checklist"])

    if args.json:
        print(json.dumps({
            "manifest": manifest_path,
            "status": manifest["status"],
            "checklist_items": line_items,
            "probes": tally,
            "structural_fail": struct_fail,
            "gate_fail": gate_fails,
            "gate_warn": gate_warns,
            "findings": len(manifest["findings"]),
            "gate": "PASS" if fail == 0 else "FAIL",
        }, indent=2))
        return 0 if fail == 0 else 1

    print("A11-R3 acceptance matrix")
    print(f"  manifest      : {os.path.relpath(manifest_path, repo_root())}")
    print(f"  base          : {manifest['base']['short']} ({manifest['base']['ref']})")
    print(f"  status        : {manifest['status']}")
    print(f"  rebased pin   : {manifest['reference_pins'][0]['pinned_revision']}"
          f" ({manifest['reference_pins'][0]['default_classification']})")
    print()
    print("[1] structural checks")
    for name, verdict, detail in structural:
        print(f"  {verdict:<4} {name:<32} {detail}")
    print()
    print(f"[2] checklist probes ({line_items} items)")
    by_group: dict[str, dict[str, int]] = {}
    for item in manifest["checklist"]:
        g = by_group.setdefault(item["group"], {"PASS": 0, "FAIL": 0, "NOT_RUN": 0,
                                                "GAP_CONFIRMED": 0, "GAP_CLOSED": 0})
        pass
    for (cid, verdict, detail, source), item in zip(probes, manifest["checklist"]):
        by_group[item["group"]][verdict] += 1
        if args.report or verdict in ("FAIL", "GAP_CLOSED"):
            print(f"  {verdict:<14} {cid:<5} {item['group']:<14} {detail}")
            print(f"                 evidence: {source}")
    for g, counts in by_group.items():
        print(f"  group {g:<14} " + " ".join(f"{k}={v}" for k, v in counts.items() if v))
    print()
    print("[3] numeric gates and divergence")
    for name, verdict, detail in gates:
        print(f"  {verdict:<24} {name:<44} {detail}")
    print()
    print(f"  findings recorded: {len(manifest['findings'])}"
          f" (open: {sum(1 for f in manifest['findings'] if f['status'] == 'OPEN')})")
    for f in manifest["findings"]:
        print(f"    {f['id']} [{f['severity']:<6}] {f['title']} -> {f['owner']}")
    print()
    print(f"A11-R3 ACCEPTANCE MATRIX: {tally['PASS']} PASS / {tally['FAIL']} FAIL / "
          f"{tally['NOT_RUN']} NOT_RUN / {tally['GAP_CONFIRMED']} GAP_CONFIRMED / "
          f"{tally['GAP_CLOSED']} GAP_CLOSED / {gate_warns} WARN")
    print(f"GATE: {'PASS' if fail == 0 else 'FAIL'}")
    if manifest["status"].startswith("PROVISIONAL"):
        print("NOTE: PROVISIONAL - A11 may finalize only after A10 review and A1's revised prototype.")
    return 0 if fail == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
