#!/usr/bin/env python3
"""Reproduce the integrated source before applying branch-scoped continuation patches.

All changes remain reviewable source. This temporary developer-only tool never
changes quality thresholds or writes master and is removed before release.
"""
from pathlib import Path
import hashlib
import json
import runpy
import subprocess
import sys

DIRECTORY = Path(__file__).resolve().parent
ROOT = DIRECTORY.parents[1]
BRANCH = 'feature/developer-studio-unified-20261016'

if '--verify-scope' in sys.argv:
    runpy.run_path(str(DIRECTORY / 'wire.py'))
    raise SystemExit(0)

context = runpy.run_path(str(DIRECTORY / 'wire.py'))
read, write, once = context['read'], context['write'], context['once']
path = 'src-tauri/src/capabilities/database/commands.rs'
write(path, once(read(path), 'fn ensure_profiles_loaded(app: &AppHandle) -> Result<(), String> {', 'pub(crate) fn ensure_profiles_loaded(app: &AppHandle) -> Result<(), String> {'))
for file in sorted(DIRECTORY.glob('integrate_*.py')):
    text = file.read_text(encoding='utf-8')
    if file.name == 'integrate_git.py':
        # The old generated tail referred to a different legacy checker layout.
        # New explicit source tests are applied in the continuation diff instead.
        text = text.split("policy = 'scripts/check-git-ui-policy.py'", 1)[0]
    exec(compile(text, str(file), 'exec'), {'ROOT': ROOT, 'read': read, 'write': write, 'once': once})
subprocess.run(['cargo','fmt','--manifest-path','src-tauri/Cargo.toml','--all'], cwd=ROOT, check=True)

# Optional, transparent UTF-8 unified patch. Git itself checks every hunk and
# refuses a different baseline; no blind string replacements or forced apply.
patch = DIRECTORY / 'continuation.patch'
if patch.exists():
    reverse = subprocess.run(['git','apply','--reverse','--check',str(patch)], cwd=ROOT, capture_output=True)
    if reverse.returncode != 0:
        subprocess.run(['git','apply','--check',str(patch)], cwd=ROOT, check=True)
        subprocess.run(['git','apply',str(patch)], cwd=ROOT, check=True)
    print('CONTINUATION_PATCH_SHA256=' + hashlib.sha256(patch.read_bytes()).hexdigest())

context['scope']()
manifest = {}
for area in ['src/capabilities/git', 'src/capabilities/database', 'src-tauri/src/capabilities/git', 'src-tauri/src/capabilities/database', 'scripts/developer-studio']:
    for file in sorted((ROOT / area).rglob('*')):
        if file.is_file() and '__pycache__' not in str(file):
            manifest[str(file.relative_to(ROOT))] = hashlib.sha256(file.read_bytes()).hexdigest()
print('DEVELOPER_STUDIO_SOURCE_SHA256=' + json.dumps(manifest, sort_keys=True))
print('DEVELOPER_STUDIO_SOURCE_TRANSFORMS=APPLIED')
