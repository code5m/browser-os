#!/usr/bin/env python3
"""Reconcile Rust libtest enumeration, required names and actual native results.

The Windows Job supervisor owns Windows execution. This checker gives Linux
the same required-name accounting without inferring execution from source text.
"""
from __future__ import annotations

import argparse
from copy import deepcopy
import json
import os
from pathlib import Path
import re
import sys


def require(condition: bool, reason: str) -> None:
    if not condition:
        raise ValueError(reason)


def reconcile(manifest: dict, selection_ids: list[str], listing: str,
              output: str, source_sha: str, harness_count: int) -> dict:
    require(re.fullmatch(r"[0-9a-fA-F]{40}", source_sha) is not None,
            "invalid_source_sha")
    require(manifest.get("schema_version") == 1, "invalid_manifest_version")
    require(selection_ids and len(set(selection_ids)) == len(selection_ids),
            "empty_or_duplicate_selection")
    require(harness_count >= 1, "invalid_harness_count")
    selections = []
    for selection_id in selection_ids:
        matches = [item for item in manifest.get("selections", [])
                   if item.get("id") == selection_id]
        require(len(matches) == 1, "selection_not_unique:" + selection_id)
        selection = matches[0]
        required = selection.get("required_names", [])
        require(isinstance(required, list) and bool(required)
                and all(isinstance(name, str) and re.fullmatch(r"[A-Za-z0-9_:]+", name)
                        for name in required)
                and selection.get("required_count") == len(required)
                and len(set(required)) == len(required),
                "invalid_required_names:" + selection_id)
        require(isinstance(selection.get("allow_additional_tests"), bool)
                and isinstance(selection.get("allow_ignored"), bool),
                "invalid_policy:" + selection_id)
        require(selection.get("role") in {"required", "full-suite"}
                and not (selection["role"] == "required" and selection["allow_ignored"]),
                "required_selection_allows_ignored:" + selection_id)
        selections.append(selection)

    enumerated = re.findall(r"^([^\r\n]+): test\r?$", listing, re.M)
    require(enumerated and len(set(enumerated)) == len(enumerated),
            "empty_or_duplicate_enumeration")
    resolved = set()
    groups = []
    for selection in selections:
        names = []
        for expected in selection["required_names"]:
            matches = [name for name in enumerated
                       if name == expected or name.endswith("::" + expected)]
            require(len(matches) == 1, "missing_or_ambiguous_required_name:" + expected)
            require(matches[0] not in names, "duplicate_required_resolution:" + expected)
            names.append(matches[0])
        resolved.update(names)
        groups.append({"selection": selection["id"], "package": selection["package"],
                       "required_count": len(names), "required_names": sorted(names)})
    additional = sorted(set(enumerated) - resolved)
    require(not additional or all(item["allow_additional_tests"] for item in selections),
            "unreviewed_additional_names")

    actual = re.findall(
        r"^test ([^\r\n]+?)(?: - (?:should panic|compile fail|compile))? "
        r"\.\.\. (ok|FAILED|ignored)(?:, [^\r\n]*)?\r?$", output, re.M)
    executed = dict(actual)
    require(len(executed) == len(actual) and set(executed) == set(enumerated),
            "enumeration_execution_mismatch")
    for name in resolved:
        require(executed.get(name) == "ok", "required_name_did_not_pass:" + name)
    rows = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;",
        output, re.M)
    require(len(rows) == harness_count, "result_summary_count_mismatch")
    counts = [sum(status == expected for status in executed.values())
              for expected in ("ok", "FAILED", "ignored")]
    totals = [sum(int(row[column]) for row in rows) for column in (1, 2, 3)]
    require(counts == totals, "result_count_mismatch")
    require(all(row[0] == "ok" for row in rows) and counts[0] > 0 and counts[1] == 0,
            "failed_or_empty_results")
    require(not counts[2] or all(item["allow_ignored"] for item in selections),
            "ignored_tests")
    require(re.search(r"^\s*\[?SKIP(?:PED)?\b", output, re.M | re.I) is None,
            "runtime_skip")
    return {
        "source_sha": source_sha.lower(), "status": "PASS_WITH_IGNORED" if counts[2] else "PASS",
        "selections": groups, "harness_count": harness_count,
        "required_count": len(resolved), "required_names": sorted(resolved),
        "enumerated_names": sorted(enumerated), "executed_tests": executed,
        "additional_tests": additional,
        "passed": counts[0], "failed": counts[1], "ignored": counts[2],
    }


def self_test() -> None:
    selection = {"id": "fixture", "package": "fixture", "role": "required",
                 "required_count": 1, "required_names": ["must_run"],
                 "allow_additional_tests": True, "allow_ignored": False}
    manifest = {"schema_version": 1, "selections": [selection]}
    listing = "tests::must_run: test\ntests::extra: test\n"
    output = ("test tests::must_run ... ok\ntest tests::extra ... ok\n"
              "test result: ok. 2 passed; 0 failed; 0 ignored;\n")
    sha = "a" * 40
    result = reconcile(manifest, ["fixture"], listing, output, sha, 1)
    assert result["passed"] == 2 and result["required_count"] == 1
    second = deepcopy(selection)
    second.update(id="core", required_names=["core_case"])
    combined = {"schema_version": 1, "selections": [selection, second]}
    combined_listing = listing + "core::core_case: test\n"
    combined_output = output + ("test core::core_case ... ok\n"
                                "test result: ok. 1 passed; 0 failed; 0 ignored;\n")
    assert reconcile(combined, ["fixture", "core"], combined_listing,
                     combined_output, sha, 2)["passed"] == 3
    bad_manifest = deepcopy(manifest)
    bad_manifest["selections"][0]["required_count"] = 2
    no_extra = deepcopy(manifest)
    no_extra["selections"][0]["allow_additional_tests"] = False
    cases = [
        (bad_manifest, listing, output, sha, 1),
        (no_extra, listing, output, sha, 1),
        (manifest, "", output, sha, 1),
        (manifest, listing + "tests::must_run: test\n", output, sha, 1),
        (manifest, listing.replace("must_run", "missing"), output, sha, 1),
        (manifest, listing, output.replace("must_run ... ok", "must_run ... ignored"), sha, 1),
        (manifest, listing, output.replace("extra ... ok", "extra ... FAILED"), sha, 1),
        (manifest, listing, output.replace("extra ... ok", "extra ... ignored")
         .replace("2 passed; 0 failed; 0 ignored", "1 passed; 0 failed; 1 ignored"), sha, 1),
        (manifest, listing, output.replace("2 passed", "1 passed"), sha, 1),
        (manifest, listing, output + "test tests::extra ... ok\n", sha, 1),
        (manifest, listing, output + "[SKIP] required environment missing\n", sha, 1),
        (manifest, listing, output, "not-a-sha", 1),
        (manifest, listing, output, sha, 2),
    ]
    for bad, listed, actual, source, harnesses in cases:
        try:
            reconcile(bad, ["fixture"], listed, actual, source, harnesses)
        except ValueError:
            continue
        raise AssertionError("invalid required native evidence was accepted")
    print(f"NATIVE_REQUIRED_RESULTS_SELF_TEST=PASS cases={len(cases) + 2}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--manifest", default="scripts/windows-native-required-tests.json")
    parser.add_argument("--selection", action="append", default=[])
    parser.add_argument("--listing")
    parser.add_argument("--output")
    parser.add_argument("--evidence")
    parser.add_argument("--harness-count", type=int, default=1)
    parser.add_argument("--failure-prefix", default="NATIVE_REQUIRED_RESULTS_INVALID")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    try:
        require(bool(args.selection and args.listing and args.output and args.evidence),
                "missing_arguments")
        evidence = reconcile(
            json.loads(Path(args.manifest).read_text(encoding="utf-8")),
            args.selection, Path(args.listing).read_text(encoding="utf-8", errors="replace"),
            Path(args.output).read_text(encoding="utf-8", errors="replace"),
            os.environ.get("BROWSEROS_SOURCE_SHA", ""), args.harness_count)
        Path(args.evidence).write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
        summary = (f"Source={evidence['source_sha']}; selections={','.join(args.selection)}; "
                   f"required={evidence['required_count']}; passed={evidence['passed']}; "
                   f"ignored={evidence['ignored']}; status={evidence['status']}")
        print(summary)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with Path(os.environ["GITHUB_STEP_SUMMARY"]).open("a", encoding="utf-8") as stream:
                stream.write(summary + "\n")
        return 0
    except (ValueError, OSError, TypeError, KeyError) as error:
        print(f"{args.failure_prefix}:{error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
