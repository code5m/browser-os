#!/usr/bin/env python3
"""Check WBS routing labels in the repository plan."""

import argparse
import re
import sys
import tempfile
from pathlib import Path


EXPECTED_COUNT = 50
WBS_RE = re.compile(r"^\s*- \[[ x]\] \*\*(M\d+-\d+)\b[^*]*\*\*\s+`(\[[^`]*\])`")
WBS_PREFIX_RE = re.compile(r"^\s*- \[[ x]\] \*\*M\d+-\d+\b")
LABEL_RE = re.compile(
    r"^\[(S[0-3])\|LEVERAGE:([1-3])\|(SIMPLE|MEDIUM|COMPLEX)"
    r"\|AI:(FAST|BALANCED|DEEP)\|R:(low|medium|high|xhigh)"
    r"(?:\|ESCALATE:(BALANCED|DEEP))?\]$"
)


def check_plan(path: Path, expected_count: int = EXPECTED_COUNT) -> list[str]:
    errors = []
    ids = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not WBS_PREFIX_RE.match(line):
            continue
        match = WBS_RE.match(line)
        if not match:
            errors.append(f"line {line_number}: invalid WBS shape or missing label")
            continue
        wbs_id, label = match.groups()
        if wbs_id in ids:
            errors.append(f"line {line_number}: duplicate WBS ID {wbs_id} (line {ids[wbs_id]})")
        else:
            ids[wbs_id] = line_number
        if not LABEL_RE.fullmatch(label):
            errors.append(f"line {line_number}: invalid routing label for {wbs_id}")

    if len(ids) != expected_count:
        errors.append(f"expected {expected_count} WBS rows, found {len(ids)}")
    return errors


def run_self_test() -> int:
    valid = "  - [x] **M0-0 Example** `[S0|LEVERAGE:1|SIMPLE|AI:FAST|R:low|ESCALATE:DEEP]`\n"
    invalid = "- [ ] **M0-1 Arrow** `[S0|LEVERAGE:1|SIMPLE->COMPLEX|AI:FAST->DEEP|R:low]`\n"
    duplicate = valid + valid
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        valid_path = root / "valid.md"
        valid_path.write_text(valid, encoding="utf-8")
        if check_plan(valid_path, expected_count=1):
            return 1
        invalid_path = root / "invalid.md"
        invalid_path.write_text(invalid, encoding="utf-8")
        if not check_plan(invalid_path, expected_count=1):
            return 1
        duplicate_path = root / "duplicate.md"
        duplicate_path.write_text(duplicate, encoding="utf-8")
        if not any("duplicate WBS ID" in error for error in check_plan(duplicate_path, expected_count=2)):
            return 1
    print("self-test: ok")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Check plan WBS routing labels.")
    parser.add_argument("--self-test", action="store_true", help="run built-in tests")
    args = parser.parse_args(argv)
    if args.self_test:
        return run_self_test()

    errors = check_plan(Path(__file__).resolve().parent.parent / "详细设计与实施计划.md")
    if errors:
        print(f"check-plan-routing: failed ({len(errors)} issue(s))")
        for error in errors:
            print(error)
        return 1
    print(f"check-plan-routing: ok ({EXPECTED_COUNT} WBS rows)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
