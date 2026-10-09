#!/usr/bin/env python3
"""Conservative change-aware GitHub Actions routing. Unknown changes run FULL."""
import argparse
import json
import os
from pathlib import PurePosixPath
import subprocess
import sys

ZERO_SHA = "0" * 40
RELEASE_SENSITIVE = (
    ".github/", ".githooks/", "scripts/", "src-tauri/",
    "tauri-browser-tabs/", "android-bridge/", "packages/",
    "config/", "templates/", "variants/",
)
NATIVE_OR_BRIDGE = ("bridge", "webview", "window", "runtime", "overlay", "startup", "drag", "browserhost")
NODE_MANIFESTS = {"package.json", "package-lock.json", "npm-shrinkwrap.json"}
RUST_MANIFESTS = {"Cargo.toml", "Cargo.lock"}


def documentation(path: str) -> bool:
    p = PurePosixPath(path)
    # Only narrative files count. YAML/JSON/HTML in docs can affect behavior.
    return path == "LICENSE" or p.suffix.lower() in {".md", ".markdown", ".txt"} and (
        len(p.parts) == 1 or p.parts[0] in {"docs", ".ai", "src", "src-tauri", "packages", "tauri-browser-tabs"}
    )


def safe_ui(path: str) -> bool:
    p = PurePosixPath(path)
    if p.suffix.lower() not in {".vue", ".ts", ".css", ".scss"}:
        return False
    if any(token in p.as_posix().lower() for token in NATIVE_OR_BRIDGE):
        return False
    if path.startswith("src/styles/") or path.startswith("src/components/"):
        return True
    return len(p.parts) >= 5 and p.parts[:2] == ("src", "capabilities") and p.parts[3] == "ui"


def classify(paths: list[str], *, event: str = "pull_request", force_full: bool = False) -> dict:
    files = sorted(set(paths))
    mode = "full"
    reason = "unknown-or-sensitive-changes"
    if not force_full and event not in {"workflow_dispatch", "schedule"} and files:
        if all(documentation(f) for f in files):
            mode, reason = "docs", "documentation-only"
        elif sum(not documentation(f) for f in files) <= 6 and all(
            documentation(f) or safe_ui(f) for f in files
        ):
            mode, reason = "ui", "bounded-leaf-ui-change"
    if force_full:
        reason = "explicit-full-override"
    elif event in {"schedule", "workflow_dispatch"}:
        reason = "manual-or-scheduled-full-validation"
    elif not files:
        reason = "unknown-or-empty-diff-fail-closed"

    node = event in {"schedule", "workflow_dispatch"} or force_full or any(
        f in NODE_MANIFESTS or f.startswith(".github/workflows/") or
        f == "scripts/generate-sbom.mjs" for f in files
    )
    rust = event in {"schedule", "workflow_dispatch"} or force_full or any(
        PurePosixPath(f).name in RUST_MANIFESTS or f.startswith(".github/workflows/") or
        f == "scripts/generate-sbom.mjs" for f in files
    )
    # When comparison is uncertain, NEVER silently skip a security check.
    if not files:
        node = rust = True
    return {
        "mode": mode,
        "reason": reason,
        "run_node": str(mode != "docs").lower(),
        "run_native": str(mode == "full").lower(),
        "run_hotplug": str(mode == "full").lower(),
        "run_ui_safety": str(mode != "docs").lower(),
        "supply_node": str(node).lower(),
        "supply_rust": str(rust).lower(),
        "changed_count": str(len(files)),
    }


def changed_paths(base: str, head: str) -> list[str]:
    if not base or not head or base == ZERO_SHA or head == ZERO_SHA:
        raise ValueError("missing commit boundary")
    # --no-renames includes BOTH old and new names for safe classification.
    command = ["git", "diff", "--no-renames", "--name-only", "-z", base, head, "--"]
    result = subprocess.run(command, check=True, capture_output=True)
    return [p.decode("utf-8") for p in result.stdout.split(b"\0") if p]


def environment_diff() -> list[str]:
    event = os.getenv("GITHUB_EVENT_NAME", "push")
    if event in {"workflow_dispatch", "schedule"}:
        return []
    base = (os.getenv("CI_COMPARE_BASE") or "").strip()
    head = (os.getenv("CI_COMPARE_HEAD") or "HEAD").strip()
    return changed_paths(base, head)


def self_test() -> None:
    cases = [
        (["README.md"], "docs"),
        (["docs/engineering/WORKFLOW-CATALOG.md", "使用指南.md"], "docs"),
        (["src/components/HelpPanel.vue"], "ui"),
        (["src/styles/main.css"], "ui"),
        (["src/components/HelpPanel.vue"] * 7, "ui"),  # duplicates are one file
        (["src/components/HelpPanel.vue", "src/App.vue"], "full"),
        (["src/components/BrowserOverlay.vue"], "full"),
        (["src-tauri/src/main.rs"], "full"),
        (["package-lock.json"], "full"),
        (["src/capabilities/git/manifest.ts"], "full"),
        ([".github/workflows/capability-v2-validation.yml"], "full"),
        (["src-tauri/src/bridge.rs", "README.md"], "full"),
        ([], "full"),
        (["src/components/Safe.vue", "src-tauri/src/bridge.rs"], "full"),
    ]
    for files, expected in cases:
        actual = classify(files)["mode"]
        assert actual == expected, f"{files}: expected {expected}, got {actual}"
    assert classify(["README.md"], event="workflow_dispatch")["mode"] == "full"
    assert classify(["README.md"], force_full=True)["mode"] == "full"
    assert classify(["README.md"])["supply_node"] == "false"
    assert classify(["README.md"])["supply_rust"] == "false"
    assert classify(["package-lock.json"])["supply_node"] == "true"
    assert classify(["src-tauri/Cargo.lock"])["supply_rust"] == "true"
    assert classify([".github/workflows/test.yml"])["supply_rust"] == "true"
    assert classify([])["supply_node"] == "true"
    assert classify([])["supply_rust"] == "true"
    assert safe_ui("src/capabilities/home/ui/HomePanel.vue")
    assert not safe_ui("src/capabilities/browser/ui/Webview.vue")
    assert documentation("src/capabilities/home/README.md")
    print(f"CI_IMPACT_SELF_TEST=PASS cases={len(cases)}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--files-json", help="Deterministic local fixture (JSON list of paths)")
    parser.add_argument("--gha", action="store_true", help="Append outputs to GITHUB_OUTPUT")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    event = os.getenv("GITHUB_EVENT_NAME", "push")
    force = os.getenv("CI_FORCE_FULL", "").lower() in {"1", "true", "yes"}
    error = None
    try:
        files = json.loads(args.files_json) if args.files_json is not None else environment_diff()
        if not isinstance(files, list) or not all(isinstance(p, str) for p in files):
            raise ValueError("path list must contain strings")
    except (ValueError, subprocess.CalledProcessError, OSError) as exc:
        error = str(exc)
        files = []
        force = True
    result = classify(files, event=event, force_full=force)
    if error:
        result["reason"] = "diff-unavailable-fail-closed"
        result["diff_error"] = error
    print("CI_IMPACT=" + json.dumps({**result, "paths": files[:50]}, ensure_ascii=False))
    print(f"CI_IMPACT_RESULT={result['mode'].upper()} ({result['reason']})")
    if args.gha:
        out = os.getenv("GITHUB_OUTPUT")
        if not out:
            raise RuntimeError("GITHUB_OUTPUT is required for --gha")
        with open(out, "a", encoding="utf-8") as stream:
            for key, value in result.items():
                if key != "diff_error":
                    stream.write(f"{key}={value}\n")
        summary = os.getenv("GITHUB_STEP_SUMMARY")
        if summary:
            with open(summary, "a", encoding="utf-8") as stream:
                stream.write(f"### CI impact: {result['mode'].upper()}\n\n"
                             f"Reason: {result['reason']}; files: {len(files)}.\n\n"
                             "Skipped work is NOT tested work. Full validation is still mandatory "
                             "for native/critical changes, manually and before release.\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
