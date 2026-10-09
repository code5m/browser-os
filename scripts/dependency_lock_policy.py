"""Shared npm lockfile policy helpers.

Domain checkers freeze direct dependency declarations, not the byte-for-byte
serialization of package-lock.json. Transitive lockfile remediation is governed
by Supply Chain Assurance (npm audit/SBOM) while this module ensures the lock
root cannot silently add, remove, move, or rewrite declared dependencies.
"""

from __future__ import annotations

import json

PROTECTED_DEP_FIELDS = (
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
)


def extract_lock_root_deps(lock_text: str) -> dict:
    data = json.loads(lock_text)
    if data.get("lockfileVersion") != 3:
        raise ValueError("package-lock.json lockfileVersion must remain 3")
    root = (data.get("packages") or {}).get("")
    if not isinstance(root, dict):
        raise ValueError("package-lock.json packages[''] root is missing")
    return {field: dict(root.get(field) or {}) for field in PROTECTED_DEP_FIELDS}


def lock_declared_deps_match(lock_text: str, expected: dict) -> bool:
    try:
        return extract_lock_root_deps(lock_text) == {
            field: dict(expected.get(field) or {}) for field in PROTECTED_DEP_FIELDS
        }
    except (ValueError, TypeError, json.JSONDecodeError):
        return False


def mutate_lock_root_dependency(lock_text: str, name: str, version: str) -> str:
    """Self-test helper: create a real lock-root dependency drift."""
    data = json.loads(lock_text)
    root = data.setdefault("packages", {}).setdefault("", {})
    deps = root.setdefault("dependencies", {})
    deps[name] = version
    return json.dumps(data, ensure_ascii=False, indent=2) + "\n"
