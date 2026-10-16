#!/usr/bin/env python3
"""Idempotent integration, limited to the authorized Developer Studio branch.

This is a temporary development tool, not a runtime code generator. It never
changes thresholds, baseline evidence, remote permissions or browser geometry.
The builder publishes its complete source diff only after real tests pass.
"""
from pathlib import Path
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
BRANCH = 'feature/developer-studio-unified-20261016'

def read(path):
    return (ROOT / path).read_text(encoding='utf-8')

def write(path, text):
    p = ROOT / path
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text, encoding='utf-8')

def once(text, old, new):
    if new in text:
        return text
    if text.count(old) != 1:
        raise RuntimeError('INTEGRATION_ANCHOR_NOT_UNIQUE: ' + old[:100])
    return text.replace(old, new, 1)

def scope():
    output = subprocess.check_output(['git', 'status', '--porcelain', '-z'], cwd=ROOT).decode()
    allowed = ('src/', 'src-tauri/', 'scripts/', 'docs/', 'artifacts/native-contracts/')
    for item in output.split('\0'):
        if not item:
            continue
        path = item[3:]
        if not path.startswith(allowed) or any(x in path for x in ('/linux.rs', 'logs/m0-', 'measure-build-metrics.py', 'pre-merge.sh')):
            raise RuntimeError('INTEGRATION_PATH_NOT_AUTHORIZED: ' + path)
    print('DEVELOPER_STUDIO_WRITE_SCOPE=PASS')

if '--verify-scope' in sys.argv:
    scope()
    raise SystemExit(0)

main = read('src-tauri/src/main.rs')
registry = read('docs/architecture/native-boundary/native-commands.yaml')
permissions = read('src-tauri/permissions/default-commands.toml')
bridge = read('src/bridge.ts')
governance = json.loads(read('docs/engineering/governance.json'))

specifications = [
    ('git', 'studio', 'GitStudioRuntime', [
        ('git_studio_read', 'gitStudioRead', 'source: Record<string, unknown>, input: Record<string, unknown>', '{ source, input }'),
        ('git_studio_preview', 'gitStudioPreview', 'source: Record<string, unknown>, input: Record<string, unknown>', '{ source, input }'),
        ('git_studio_confirm', 'gitStudioConfirm', 'jobId: string, confirmed: boolean', '{ jobId, confirmed }'),
        ('git_studio_cancel', 'gitStudioCancel', 'jobId: string', '{ jobId }'),
    ]),
    ('database', 'studio', 'DatabaseStudioRuntime', [
        ('db_studio_open', 'dbStudioOpen', 'connId: string', '{ connId }'),
        ('db_studio_call', 'dbStudioCall', 'sessionId: string, request: Record<string, unknown>', '{ sessionId, request }'),
        ('db_studio_close', 'dbStudioClose', 'sessionId: string', '{ sessionId }'),
        ('db_studio_cancel', 'dbStudioCancel', 'sessionId: string', '{ sessionId }'),
    ]),
    ('workspace', 'file_history', 'FileHistoryRuntime', [
        ('file_history', 'fileHistory', 'path: string, action: string, version: string | null, confirmed: boolean', '{ path, action, version, confirmed }'),
    ]),
]
for owner, module, state, commands in specifications:
    file = f'src-tauri/src/capabilities/{owner}/{module}.rs'
    if not (ROOT / file).is_file():
        continue
    mod_path = f'src-tauri/src/capabilities/{owner}/mod.rs'
    mods = read(mod_path)
    if f'pub mod {module};' not in mods:
        write(mod_path, mods.rstrip() + f'\npub mod {module};\n')
    manage = f'.manage(capabilities::{owner}::{module}::{state}::default())'
    handler = '.invoke_handler(tauri::generate_handler!['
    if manage not in main:
        at = main.rfind(handler)
        if at < 0:
            raise RuntimeError('MAIN_HANDLER_ANCHOR_MISSING')
        main = main[:at] + manage + '\n        ' + main[at:]
    for command, method, args, payload in commands:
        rust_call = f'capabilities::{owner}::{module}::{command},'
        if rust_call not in main:
            at = main.rfind(handler)
            start = at + len(handler)
            main = main[:start] + '\n            ' + rust_call + main[start:]
        if not re.search(r'^  ' + re.escape(command) + r':\s*$', registry, re.M):
            block = f'''  {command}:
    file: "{file}"
    owner: "{owner}"
    category: "CAPABILITY_NATIVE_ADAPTER"
    callers: "src/capabilities/{owner}/**"
    side_effect: "Authorized Developer Studio operation with native validation, preview and cleanup"
    resource: "{'CHILD_PROCESS' if owner == 'git' else 'NETWORK' if owner == 'database' else 'FILESYSTEM'}"
    permission: "{'fs.read,fs.write,process.spawn,network.connect' if owner == 'git' else 'network.connect,keyring.access' if owner == 'database' else 'fs.read,fs.write'}"
    current_boundary: "Capability-owned Developer Studio native module"
    target_boundary: "Capability-owned Developer Studio native module"
'''
            registry = once(registry, 'commands:\n', 'commands:\n' + block)
        if f'"{command}"' not in permissions:
            permissions = once(permissions, 'commands.allow = [\n', f'commands.allow = [\n    "{command}",\n')
        if f'{method}:' not in bridge:
            wrapper = f'  {method}: ({args}) => invoke<any>("{command}", {payload}),\n'
            bridge = once(bridge, 'export const bridge = {\n', 'export const bridge = {\n' + wrapper)

# Read-only Git queries must not refresh the index behind a preview fingerprint.
file = 'src-tauri/src/capabilities/git/studio.rs'
text = read(file)
if '.env("GIT_OPTIONAL_LOCKS", "0")' not in text:
    text = text.replace('.env("GIT_LITERAL_PATHSPECS", "1")', '.env("GIT_OPTIONAL_LOCKS", "0").env("GIT_LITERAL_PATHSPECS", "1")', 1)
write(file, text)

governance['nativeSemantics']['expectedRegisteredCommands'] = len(re.findall(r'^  ([a-z_]\w*):\s*$', registry, re.M))
write('src-tauri/src/main.rs', main)
write('docs/architecture/native-boundary/native-commands.yaml', registry)
write('src-tauri/permissions/default-commands.toml', permissions)
write('src/bridge.ts', bridge)
write('docs/engineering/governance.json', json.dumps(governance, ensure_ascii=False, indent=2) + '\n')
scope()
print('DEVELOPER_STUDIO_INTEGRATION=APPLIED')
