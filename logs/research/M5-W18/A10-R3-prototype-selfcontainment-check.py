#!/usr/bin/env python3
# A10 / M5-W18-R3 research asset (self-contained, zero product impact).
#
# Mechanical gate for R3 review rubric G8 ("prototypes are self-contained and synthetic"):
#   - no network resource references
#   - no native runtime / IPC invocation (Tauri invoke, bridge calls)
#   - no product source imports
#   - no npm/bare imports (a synthetic prototype must open without a build)
#   - no browser persistence APIs (no real or synthetic data may be stored)
#   - no donor branding strings (Rebased / IntelliJ / JetBrains) used as UI text
#
# Usage:
#   python3 A10-R3-prototype-selfcontainment-check.py [FILE ...]
#   (default: every *.html under logs/research/M5-W18/)
#
# Exit code: 0 = all PASS, 1 = at least one FAIL.

import glob
import os
import re
import sys

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
]


def check_file(path):
    text = open(path, encoding="utf-8", errors="replace").read()
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


def main():
    files = sys.argv[1:] or sorted(glob.glob(DEFAULT_GLOB))
    if not files:
        print("no prototype files found")
        return 0

    failed = 0
    print("A10-R3 prototype self-containment gate (rubric G8)\n")
    for path in files:
        if not os.path.exists(path):
            print(f"{path}: MISSING")
            failed += 1
            continue
        res = check_file(path)
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


if __name__ == "__main__":
    sys.exit(main())
