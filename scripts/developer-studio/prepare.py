#!/usr/bin/env python3
"""Apply auditable, idempotent source transformations on the integration branch.
The builder does not publish any transformations until compilation and tests pass.
"""
from pathlib import Path
import runpy

DIRECTORY = Path(__file__).resolve().parent
context = runpy.run_path(str(DIRECTORY / 'wire.py'))
# wire.py handles --verify-scope and exits before transformations in that mode.
ROOT = context['ROOT']
read, write, once = context['read'], context['write'], context['once']

if (ROOT / 'src-tauri/src/capabilities/database/studio.rs').exists():
    path = 'src-tauri/src/capabilities/database/commands.rs'
    text = read(path)
    old = 'fn ensure_profiles_loaded(app: &AppHandle) -> Result<(), String> {'
    new = 'pub(crate) fn ensure_profiles_loaded(app: &AppHandle) -> Result<(), String> {'
    write(path, once(text, old, new))

for integration in sorted(DIRECTORY.glob('integrate_*.py')):
    runpy.run_path(str(integration), init_globals={'ROOT': ROOT, 'read': read, 'write': write, 'once': once})
context['scope']()
print('DEVELOPER_STUDIO_COMPLETE_SOURCE_TRANSFORMS=APPLIED')
