#!/usr/bin/env python3
"""Select the ONE optional native CI lane (no implicit Rust compilation).

Priority: explicit workflow_dispatch choice > PR label > repository variable >
'off'. Conflicting labels and invalid values fail closed instead of silently
running an unrequested expensive or privileged native build.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

MODES = ("off", "rust-tests", "packaged-gui", "packaged-resource")
LABEL_MODES = {
    "ci:native-tests": "rust-tests",
    "ci:packaged-gui": "packaged-gui",
    "ci:packaged-resource": "packaged-resource",
}


def validated(value: str, source: str) -> str:
    if value not in MODES:
        raise ValueError(f"{source}: invalid native CI mode {value!r}; expected {MODES}")
    return value


def select_mode(event: str, manual: str, default: str, labels_json: str) -> str:
    if event == "workflow_dispatch":
        return validated(manual or "off", "manual input")
    fallback = validated(default or "off", "CI_NATIVE_MODE repository variable")
    if event != "pull_request":
        return fallback

    try:
        labels = json.loads(labels_json or "[]")
    except json.JSONDecodeError as exc:
        raise ValueError("PR labels must be a JSON array") from exc
    if not isinstance(labels, list) or any(not isinstance(x, str) for x in labels):
        raise ValueError("PR labels must be an array of strings")
    selected = [name for name in LABEL_MODES if name in labels]
    if len(selected) > 1:
        raise ValueError(f"Conflicting PR native CI labels: {selected}")
    return LABEL_MODES[selected[0]] if selected else fallback


def self_test() -> None:
    assert select_mode("pull_request", "", "", "[]") == "off"
    assert select_mode("push", "", "", "[]") == "off"
    assert select_mode("pull_request", "", "rust-tests", '["docs"]') == "rust-tests"
    assert select_mode("pull_request", "", "", '["ci:native-tests"]') == "rust-tests"
    assert select_mode("pull_request", "", "", '["ci:packaged-gui"]') == "packaged-gui"
    assert select_mode("pull_request", "", "rust-tests", '["ci:packaged-resource"]') == "packaged-resource"
    assert select_mode("workflow_dispatch", "packaged-resource", "rust-tests", '["ci:native-tests"]') == "packaged-resource"
    assert select_mode("workflow_dispatch", "", "", "[]") == "off"
    for event, manual, default, labels in (
        ("pull_request", "", "", '["ci:native-tests","ci:packaged-gui"]'),
        ("pull_request", "", "", '{"not":"list"}'),
        ("pull_request", "", "", '["ci:native-tests",123]'),
        ("push", "", "maybe", "[]"),
        ("workflow_dispatch", "oops", "", "[]"),
    ):
        try:
            select_mode(event, manual, default, labels)
        except ValueError:
            pass
        else:
            raise AssertionError(f"Should reject: {event}, {manual}, {default}, {labels}")
    print("NATIVE_CI_MODE_SELF_TEST=PASS")


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return 0
    if sys.argv[1:] not in ([], ["--gha"]):
        print("Usage: check-ci-native-mode.py [--self-test | --gha]", file=sys.stderr)
        return 2
    try:
        mode = select_mode(
            os.environ.get("GITHUB_EVENT_NAME", "push"),
            os.environ.get("CI_NATIVE_MODE_INPUT", ""),
            os.environ.get("CI_NATIVE_MODE_DEFAULT", ""),
            os.environ.get("CI_PR_LABELS", "[]"),
        )
    except ValueError as exc:
        print(f"NATIVE_CI_MODE=ERROR: {exc}", file=sys.stderr)
        return 1
    if sys.argv[1:] == ["--gha"]:
        output = os.environ.get("GITHUB_OUTPUT")
        if not output:
            print("NATIVE_CI_MODE=ERROR: GITHUB_OUTPUT missing", file=sys.stderr)
            return 1
        with Path(output).open("a", encoding="utf-8") as stream:
            stream.write(f"native_mode={mode}\n")
    print(f"NATIVE_CI_MODE={mode}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
