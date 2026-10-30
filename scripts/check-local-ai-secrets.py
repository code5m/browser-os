#!/usr/bin/env python3
"""Reject known local AI credentials and recognizable secret literals without printing values."""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)

def known_credentials():
    spec = importlib.util.spec_from_file_location('local_ai_import', ROOT / 'scripts/import-local-ai.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    _, values = module.discover(Path.home())
    secrets = {v.encode() for v in values.values() if len(v) >= 16}
    metadata = module.metadata_path(Path.home())
    if metadata.is_file():
        # Include manually edited keys too. The values remain in memory and never enter a report.
        import gi
        gi.require_version('Secret', '1')
        from gi.repository import Secret
        schema=Secret.Schema.new('org.freedesktop.Secret.Generic',Secret.SchemaFlags.NONE,{'service':Secret.SchemaAttributeType.STRING,'username':Secret.SchemaAttributeType.STRING})
        config=json.loads(metadata.read_text())
        for profile in config.get('profiles',[]):
            value=Secret.password_lookup_sync(schema,{'service':module.SERVICE,'username':'ai:'+profile['id']},None)
            if value and len(value)>=16:
                secrets.add(value.encode())
    return secrets

def findings(data, secrets):
    if any(secret in data for secret in secrets):
        return 'KNOWN_LOCAL_AI_CREDENTIAL'
    # High-confidence provider token literals; fictional short test fixtures remain valid.
    fictional = b'ghp_' + b'ABCDEFGHIJKLMNOPQRSTUVWXYZ012345'
    for token in re.findall(rb'(?<![\w-])(?:sk-(?:proj-)?[A-Za-z0-9_-]{32,}|gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})(?![\w-])', data):
        if token != fictional:  # Existing deterministic redaction fixture; exact known-secret checks still apply.
            return 'PROVIDER_TOKEN_LITERAL'
    return None

def check(staged=False, outgoing=False, revision='HEAD'):
    secrets = known_credentials()
    failures = []
    if outgoing:
        if revision!='HEAD' and not re.fullmatch(r'[a-fA-F0-9]{40,64}',revision):
            raise ValueError('invalid revision')
        commits = git('rev-list', revision, '--not', '--remotes=origin').decode().splitlines()
        blobs = set()
        for commit in commits:
            for row in git('ls-tree', '-r', '-z', commit).split(b'\0'):
                if not row:
                    continue
                meta, name = row.split(b'\t', 1)
                mode, kind, oid = meta.split()
                if kind == b'blob' and oid not in blobs:
                    blobs.add(oid)
                    reason = findings(git('cat-file', 'blob', oid.decode()), secrets)
                    if reason:
                        failures.append((commit[:8] + ':' + name.decode(errors='replace'), reason))
    else:
        names = git('diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z') if staged else git('ls-files', '-co', '--exclude-standard', '-z')
        for name in set(names.split(b'\0')):
            if not name:
                continue
            relative = name.decode(errors='replace')
            path = ROOT / relative
            if staged:
                data = git('show', ':' + relative)
            elif path.is_file() and not path.is_symlink():
                data = path.read_bytes()
            else:
                continue
            reason = findings(data, secrets)
            if reason:
                failures.append((relative, reason))
    for path, reason in failures:
        print(f'AI_SECRET_BLOCKED: {path}: {reason}')
    print(f'AI_SECRET_CHECK={"FAIL" if failures else "PASS"} (values never printed)')
    return bool(failures)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--staged', action='store_true')
    parser.add_argument('--outgoing', action='store_true')
    parser.add_argument('--revision', default='HEAD')
    args = parser.parse_args()
    try:
        raise SystemExit(check(args.staged, args.outgoing,args.revision))
    except Exception:
        print('AI_SECRET_CHECK=FAIL: unable to inspect inputs; no values printed')
        raise SystemExit(1)
