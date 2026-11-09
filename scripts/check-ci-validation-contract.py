#!/usr/bin/env python3
"""Fail-closed CI policy regression checks, independent of external YAML libs.

The default PR/push path must never compile native Rust or build/install
packages. Exactly one optional native job owns Rust tests OR Release packaging.
The exact-commit frontend archive is shared rather than recompiled.
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/capability-v2-validation.yml"
PREVIEW = ROOT / ".github/workflows/browseros-developer-feedback.yml"
PERFORMANCE = ROOT / ".github/workflows/browseros-sustained-performance.yml"
PREMERGE = ROOT / "scripts/pre-merge.sh"
INSTALLER = ROOT / "scripts/verify-installed-client.sh"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(f"CI_OPTIONAL_NATIVE_POLICY_FAIL: {message}")


def check() -> None:
    text = WORKFLOW.read_text()
    preview = PREVIEW.read_text()
    perf = PERFORMANCE.read_text()
    premerge = PREMERGE.read_text()
    installer = INSTALLER.read_text()

    for key in ("frontend-build", "validate", "native-regression"):
        require(text.count(f"  {key}:\n") == 1, f"missing or duplicated {key} job")
    frontend = text.split("\n  frontend-build:\n", 1)[1].split("\n  validate:\n", 1)[0]
    validate = text.split("\n  validate:\n", 1)[1].split("\n  native-regression:\n", 1)[0]
    native = text.split("\n  native-regression:\n", 1)[1]

    require('default: "off"' in text and 'CI_NATIVE_MODE_DEFAULT:' in text,
            "opt-in native mode must be OFF by default")
    for mode in ("rust-tests", "packaged-gui", "packaged-resource"):
        require(mode in text, f"missing opt-in mode {mode}")
    require("      - labeled\n" in text and "      - unlabeled\n" in text,
            "PR label changes must retrigger mode selection")
    require("python3 scripts/check-ci-native-mode.py --gha" in text,
            "mode selection must be explicitly validated")
    require("needs.impact.outputs.native_mode != 'off'" in native,
            "native job not gated by explicit opt-in")
    require("needs.frontend-build.result == 'success'" in native,
            "native job must not use unverified frontend assets")
    require(frontend.count("npm run build") == 1, "frontend must build once per revision")
    require("actions/upload-artifact@v4" in frontend,
            "missing checksum-bound exact commit frontend artifact")
    require("actions/download-artifact@v4" in native and "actions/download-artifact@v4" in validate,
            "frontend build not reused by both validation jobs")
    require(validate.count("npm run build") == 0,
            "validation must not recompile current frontend")
    for forbidden in ("cargo check --manifest-path", "cargo test --manifest-path",
                      "scripts/verify-installed-client.sh --install",
                      "npm run tauri -- build"):
        require(forbidden not in validate, f"mandatory validate job runs native build: {forbidden}")

    require(native.count("cargo test --locked --manifest-path src-tauri/Cargo.toml") == 1,
            "Rust tests must have exactly one native cargo compilation command")
    require("cargo check --manifest-path" not in native,
            "duplicate Rust cargo check plus test compilation")
    require(native.count("scripts/verify-installed-client.sh --install --prebuilt-frontend") == 2,
            "packaged modes must share the single release build code path")
    require("--resource-minutes 5" in native and "--resource-minutes 5" not in validate,
            "resource sampling cannot be mandatory")
    require("PREMERGE_SKIP_NATIVE_COMPILE: '1'" in validate and
            "PREMERGE_REUSE_FRONTEND: '1'" in validate,
            "pre-merge must reuse frontend and never duplicate Rust compilation")
    require("report-coding-editor-budget.mjs" in premerge and
            "--verify-dist --audit-budget" in premerge and
            "PREMERGE_SKIP_NATIVE_COMPILE:-0" in premerge,
            "fast pre-merge must verify production artifacts and report budget as advisory without nested cargo")
    require("browseros-ci-source-sha.txt" in premerge,
            "pre-merge must validate the SAME SHA marker uploaded in the artifact")
    require("FRONTEND_BUILD_SOURCE_SHA" not in validate and
            "browseros-ci-source-sha.txt" in frontend and "browseros-ci-source-sha.txt" in native,
            "exact SHA provenance not bound to artifact")

    require("PREMERGE_SKIP_NATIVE_COMPILE:-0" in premerge and
            'if [[ "${PREMERGE_SKIP_NATIVE_COMPILE:-0}" == "1" ]]' in premerge,
            "local pre-merge full Rust check must remain DEFAULT")
    require("browseros-ci-source-sha.txt" in installer and
            'beforeBuildCommand' in installer and '--prebuilt-frontend' in installer,
            "packager must refuse stale dist before overriding Tauri pre-build")
    require("        default: false" in preview and
            "if: github.event_name == 'workflow_dispatch' && inputs.build_preview == true" in preview,
            "developer preview still compiles on routine pushes")
    require("\n  schedule:\n" not in perf and "\n  workflow_dispatch:\n" in perf,
            "sustained benchmark still runs on a default schedule")

    print("CI_OPTIONAL_NATIVE_POLICY=PASS; native default OFF; one compile path; SHA-bound reuse")


if __name__ == "__main__":
    check()
