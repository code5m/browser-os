#!/usr/bin/env python3
# A10 / M5-W18-R3 + R3B research asset (self-contained, zero product impact).
#
# Mechanical gate for R3 review rubric G8 ("prototypes are self-contained and synthetic"):
#   - no network resource references
#   - no native runtime / IPC invocation (Tauri invoke, bridge calls)
#   - no product source imports
#   - no npm/bare imports (a synthetic prototype must open without a build)
#   - no browser persistence APIs (no real or synthetic data may be stored)
#   - no donor branding strings (Rebased / IntelliJ / JetBrains) used as UI text
#   - no other donor product names/fonts used as UI text (R3B prototype hygiene)
#
# R3B hardening (A10, 2026-09-08):
#   1. A crashing checker must count as FAIL, never as a silent pass. Every file is
#      checked inside a guard; an internal error is reported as FAIL for that file.
#   2. `--self-test` proves each rule still detects its violation class, so the
#      2026-09-08 crash class (regex group assumption) cannot silently regress.
#   3. Donor hygiene covers donor product names and donor font names, not only
#      the three IntelliJ-platform brands.
#
# Usage:
#   python3 A10-R3-prototype-selfcontainment-check.py [FILE ...]
#   python3 A10-R3-prototype-selfcontainment-check.py --self-test
#   (default: every *.html under logs/research/M5-W18/)
#
# Exit code: 0 = all PASS, 1 = at least one FAIL (including internal error).

import glob
import os
import re
import sys
import tempfile

DEFAULT_GLOB = "logs/research/M5-W18/*.html"

# w3 / schema namespace URIs are not network fetches in practice.
NETWORK_ALLOW = ("https://www.w3.org/", "http://www.w3.org/")

RULES = [
    (
        "PROTO_NO_NETWORK",
        re.compile(r"""(?:src|href)\s*=\s*["'](https?://[^"']+)["']|url\(\s*["']?(https?://[^)"']+)""", re.I),
        "external network reference (src/href/url)",
    ),
    (
        "PROTO_NO_NATIVE_INVOKE",
        re.compile(r"""@tauri-apps|__TAURI__|\binvoke\s*\(|window\.bridge|\bbridge\.invoke""", re.I),
        "native runtime / IPC invocation",
    ),
    (
        "PROTO_NO_PRODUCT_IMPORT",
        re.compile(r"""from\s+["'](?:\.\.?/)+src/|from\s+["']@/|["']/?src/(?:components|stores|utils)/""", re.I),
        "product source import",
    ),
    (
        "PROTO_NO_NPM_IMPORT",
        re.compile(r"""(?:from|import)\s*\(?\s*["'](vue|react|svelte|solid-js|d3|@tauri-apps/[^"']+)["']""", re.I),
        "npm/bare import (prototype would need a build step)",
    ),
    (
        "PROTO_NO_PERSIST",
        re.compile(r"""\b(?:localStorage|sessionStorage|indexedDB|document\.cookie)\b""", re.I),
        "browser persistence API (prototypes must not store anything)",
    ),
    (
        "PROTO_NO_BRANDING",
        re.compile(r"""\b(?:Rebased|IntelliJ|JetBrains)\b"""),
        "donor branding string (allowed only as a documented source citation)",
    ),
    (
        "PROTO_NO_DONOR_NAME",
        re.compile(r"""\b(?:SourceGit|slio-git|DataGrip|DBeaver|Obsidian|zvec-grep)\b""", re.I),
        "other donor product/font name in prototype text (reports may cite, UI may not)",
    ),
]

# A rule may be listed here only with a written A0 exception; empty by design.
RULE_EXCEPTIONS = {}


def check_text(text):
    """Return [(code, desc, [(lineno, snippet), ...]), ...] for one prototype body."""
    lines = text.splitlines()
    results = []
    for code, rx, desc in RULES:
        hits = []
        for m in rx.finditer(text):
            captured = next((group for group in m.groups() if group), m.group(0))
            frag = captured.strip()
            if code == "PROTO_NO_NETWORK" and frag.startswith(NETWORK_ALLOW):
                continue
            lineno = text.count("\n", 0, m.start()) + 1
            snippet = lines[lineno - 1].strip()[:90] if lineno <= len(lines) else ""
            hits.append((lineno, snippet))
        # de-duplicate by line
        seen, uniq = set(), []
        for ln, sn in hits:
            if ln not in seen:
                seen.add(ln)
                uniq.append((ln, sn))
        results.append((code, desc, uniq))
    return results


def check_file(path):
    text = open(path, encoding="utf-8", errors="replace").read()
    return check_text(text)


def run_files(files):
    failed = 0
    print("A10-R3 prototype self-containment gate (rubric G8)\n")
    for path in files:
        if not os.path.exists(path):
            print(f"{path}: MISSING")
            failed += 1
            continue
        try:
            res = check_file(path)
        except Exception as exc:  # a crashing gate is a FAIL, never a pass
            print(f"[FAIL] {path}")
            print(f"        GATE_INTERNAL_ERROR: {type(exc).__name__}: {exc}")
            print()
            failed += 1
            continue
        bad = [(c, d, h) for (c, d, h) in res if h]
        status = "FAIL" if bad else "PASS"
        if bad:
            failed += 1
        print(f"[{status}] {path}")
        for code, desc, hits in res:
            if hits:
                print(f"        {code}: {desc} ({len(hits)} hit(s))")
                for ln, sn in hits[:4]:
                    print(f"            L{ln}: {sn}")
        print()
    print(f"RESULT: {'FAIL' if failed else 'PASS'} ({len(files) - failed}/{len(files)} files clean)")
    return 1 if failed else 0


CLEAN_SAMPLE = """<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><title>frame</title>
<style>:root{--mono:ui-monospace,Consolas,monospace}</style></head>
<body><main role="main" aria-label="active content">
<h2>1. Normal Mode - 1440x900</h2><svg xmlns="http://www.w3.org/2000/svg"></svg>
<script>const state={open:false};</script></main></body></html>
"""

DIRTY_SAMPLES = {
    "PROTO_NO_NETWORK": '<img src="https://cdn.example.com/logo.png">',
    "PROTO_NO_NATIVE_INVOKE": "<script>await invoke('git_status')</script>",
    "PROTO_NO_PRODUCT_IMPORT": "<script type=module>import {x} from '../../src/stores/useLayoutStore.ts'</script>",
    "PROTO_NO_NPM_IMPORT": "<script type=module>import {ref} from 'vue'</script>",
    "PROTO_NO_PERSIST": "<script>localStorage.setItem('k','v')</script>",
    "PROTO_NO_BRANDING": "<h2>Git Mode - JetBrains style</h2>",
    "PROTO_NO_DONOR_NAME": '<div class="tab">Obsidian graph</div>',
}


def self_test():
    """Prove every rule detects its violation class and that the gate cannot crash."""
    ok = True
    checks = 0

    def expect(cond, label):
        nonlocal ok, checks
        checks += 1
        if cond:
            print(f"  PASS  {label}")
        else:
            ok = False
            print(f"  FAIL  {label}")

    print("A10 self-containment gate --self-test\n")
    clean = check_text(CLEAN_SAMPLE)
    expect(all(not h for (_, _, h) in clean), "clean synthetic prototype yields zero hits")

    for code, body in DIRTY_SAMPLES.items():
        res = dict((c, h) for (c, _, h) in check_text(CLEAN_SAMPLE + body))
        expect(bool(res.get(code)), f"{code} detects its violation sample")
        others = [c for c, h in res.items() if h and c != code]
        expect(others == [], f"{code} sample does not cross-trigger ({others})")

    # regression guard for the 2026-09-08 crash class: a multi-group regex whose
    # first group does not participate must not raise.
    saved = RULES[0]
    try:
        RULES[0] = ("PROTO_NO_NETWORK", re.compile(r"""(?:src)=["'](https?://[^"']+)["']|url\((https?://[^)]+)"""), "x")
        check_text('<style>a{background:url(https://x.example/y.png)}</style>')
        expect(True, "multi-group regex with unmatched first group does not raise")
    except Exception as exc:
        expect(False, f"multi-group regex raised {type(exc).__name__}: {exc}")
    finally:
        RULES[0] = saved

    # internal-error path must be reported as FAIL, not swallowed
    with tempfile.TemporaryDirectory() as td:
        probe = os.path.join(td, "probe.html")
        open(probe, "w", encoding="utf-8").write(CLEAN_SAMPLE)
        real = globals()["check_file"]
        try:
            globals()["check_file"] = lambda p: (_ for _ in ()).throw(RuntimeError("boom"))
            rc = run_files([probe])
            expect(rc == 1, "internal error is reported as FAIL (exit 1)")
        finally:
            globals()["check_file"] = real
        expect(run_files([os.path.join(td, "missing.html")]) == 1, "missing file is reported as FAIL")

    expect(not RULE_EXCEPTIONS, "no undocumented rule exception is active")
    print(f"\nSELF_TEST: {'PASS' if ok else 'FAIL'} ({checks} checks, ACTIVE_RULES={len(RULES)})")
    return 0 if ok else 1


def main():
    args = sys.argv[1:]
    if "--self-test" in args:
        return self_test()
    files = args or sorted(glob.glob(DEFAULT_GLOB))
    if not files:
        print("no prototype files found")
        return 0
    return run_files(files)


if __name__ == "__main__":
    sys.exit(main())
