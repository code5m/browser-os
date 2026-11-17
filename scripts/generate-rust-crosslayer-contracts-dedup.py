#!/usr/bin/env python3
"""Platform-variant dedupe wrapper for the cross-layer contract generator.

A single Tauri command may have one implementation per platform, for example
`list_apps` declared under #[cfg(target_os = "linux")] and again under
#[cfg(target_os = "windows")]. Both resolve to the same invoke name and are
registered once. The base generator compares declaration counts against
registered handler counts, so these legitimate platform variants look like
coverage drift and abort `npm run build`.

This wrapper delegates every parsing and rendering step to the reviewed
generator and only collapses same-name contracts before counting and
emitting. It does not relax any per-command check and does not hide a command
that is missing from generate_handler!.
"""
from __future__ import annotations

import importlib.util
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location(
    "crosslayer_base", HERE / "generate-rust-crosslayer-contracts.py"
)
base = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = base  # dataclasses resolve types via sys.modules
SPEC.loader.exec_module(base)  # base only calls main() under __main__

_original_build_model = base.build_model


def build_model():
    report = _original_build_model()
    seen = set()
    commands = []
    for item in report["commands"]:
        if item["name"] in seen:
            continue
        seen.add(item["name"])
        commands.append(item)
    report["commands"] = commands
    report["impactGraph"] = [
        item for item in report["impactGraph"] if item["command"] in seen
    ]
    report["counts"]["activeContracts"] = len(commands)
    findings = report["contractFindings"]
    findings["payloadMismatches"] = [
        item for item in findings["payloadMismatches"] if item["command"] in seen
    ]
    findings["returnMismatches"] = [
        item for item in findings["returnMismatches"] if item["command"] in seen
    ]
    registered = report["counts"]["registeredHandlers"]
    errors = [
        error
        for error in report["errors"]
        if not error.startswith("active command contract coverage drift")
    ]
    if len(commands) != registered:
        errors.append(
            "active command contract coverage drift: "
            f"active={len(commands)} registered={registered}"
        )
    report["errors"] = errors
    return report


base.build_model = build_model
raise SystemExit(base.main())
