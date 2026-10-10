#!/usr/bin/env python3
"""Report physical build artifacts and source-only module inventory (no invented attribution)."""
import argparse
import tempfile
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CAPABILITY_ROOTS = [
    (ROOT / "src/capabilities", "frontend"),
    (ROOT / "src-tauri/src/capabilities", "native"),
    (ROOT / "packages", "workspace"),
]

EXCLUDED_SOURCE_DIRS = {"node_modules", "target", "dist", ".git", "__pycache__"}


def files_under(path, excluded_dirs=EXCLUDED_SOURCE_DIRS):
    """Exclude nested directories relative to the requested root, not root itself."""
    if not path.is_dir():
        return []
    return [p for p in path.rglob("*") if p.is_file() and not p.is_symlink()
            and not any(part in excluded_dirs for part in p.relative_to(path).parts[:-1])]

def measure() -> dict:
    modules = {}
    for base, layer in CAPABILITY_ROOTS:
        if not base.is_dir():
            continue
        for directory in sorted(p for p in base.iterdir() if p.is_dir()):
            if layer == "workspace" and not directory.name.startswith("capability-"):
                continue
            name = directory.name.removeprefix("capability-") if layer == "workspace" else directory.name
            files = files_under(directory)
            record = modules.setdefault(name, {"frontend_source_bytes": 0, "native_source_bytes": 0, "workspace_source_bytes": 0, "source_files": 0})
            record[f"{layer}_source_bytes"] += sum(p.stat().st_size for p in files)
            record["source_files"] += len(files)
    dist = ROOT / "dist"
    frontend = files_under(dist, EXCLUDED_SOURCE_DIRS - {"dist"}) if dist.is_dir() else []
    binaries = [ROOT / "src-tauri/target/release/mvp-browser-os"]
    deb_dir = ROOT / "src-tauri/target/release/bundle/deb"
    packages = sorted(deb_dir.glob("*.deb")) if deb_dir.is_dir() else []
    deb = packages[-1] if packages else None
    installation = None
    if deb is not None:
        try:
            result = subprocess.run(["dpkg-deb", "-f", str(deb), "Installed-Size"], capture_output=True, text=True, check=True)
            installation = int(result.stdout.strip()) * 1024
        except (OSError, subprocess.SubprocessError, ValueError):
            pass
    return {
        "schema": "browseros-footprint-v1",
        "measurement_note": "Module sizes are SOURCE BYTES only; NOT separate install/download/memory cost. Shared binary prevents exact module attribution.",
        "modules": dict(sorted(modules.items())),
        "frontend_dist_bytes": sum(p.stat().st_size for p in frontend) if frontend else None,
        "release_binary_bytes": binaries[0].stat().st_size if binaries[0].is_file() else None,
        "deb_file_bytes": deb.stat().st_size if deb else None,
        "deb_file": str(deb.relative_to(ROOT)) if deb else None,
        "deb_installed_size_field_bytes": installation,
        "runtime_rss_bytes": None,
        "runtime_cpu_percent": None,
        "runtime_note": "Unavailable without isolated GUI benchmarking; requires process-tree RSS/PSS and CPU over idle and active scenarios.",
    }

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        assert ROOT.is_dir()
        assert isinstance(measure()["modules"], dict)
        with tempfile.TemporaryDirectory() as tmp:
            sample = Path(tmp) / "dist"
            sample.mkdir()
            (sample / "index.html").write_bytes(b"index")
            assets = sample / "assets"
            assets.mkdir()
            (assets / "runtime.js").write_bytes(b"console.log(1)")
            excluded = sample / "node_modules"
            excluded.mkdir()
            (excluded / "large.js").write_bytes(b"excluded")
            # Root named dist is not automatically excluded; nested dependencies are.
            assert sum(f.stat().st_size for f in files_under(sample, EXCLUDED_SOURCE_DIRS - {"dist"})) == 19
            assert len(files_under(sample)) == 2
        print("FOOTPRINT_INVENTORY_SELF_TEST=PASS")
        return
    data = measure()
    rendered = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(rendered, encoding="utf-8")
    print(rendered)

if __name__ == "__main__":
    main()
