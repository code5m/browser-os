#!/usr/bin/env python3
"""Reject known local AI credentials and recognizable secret literals without printing values."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import stat
import sys
import uuid

ROOT = Path(__file__).resolve().parent.parent

def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT)

# Read-only mirror of the native storage location / Keyring identifiers.
# The discoverer intentionally owns neither metadata publication nor Keyring I/O.
SERVICE = 'com.jizhijiandan.mvp'
MAX_METADATA_BYTES = 1024 * 1024

def windows_data_home():
    """Use the same RoamingAppData known folder as Tauri/dirs on Windows."""
    import ctypes
    from ctypes import wintypes

    class Guid(ctypes.Structure):
        _fields_ = [('data1', wintypes.DWORD), ('data2', wintypes.WORD),
                    ('data3', wintypes.WORD), ('data4', wintypes.BYTE * 8)]

    folder = Guid.from_buffer_copy(uuid.UUID('3eb685db-65f9-4cf6-a03a-e3ef65729f3d').bytes_le)
    value = ctypes.c_wchar_p()
    shell = ctypes.WinDLL('Shell32.dll')
    memory = ctypes.WinDLL('Ole32.dll')
    shell.SHGetKnownFolderPath.argtypes = [ctypes.POINTER(Guid), wintypes.DWORD, wintypes.HANDLE, ctypes.POINTER(ctypes.c_wchar_p)]
    shell.SHGetKnownFolderPath.restype = ctypes.c_long
    memory.CoTaskMemFree.argtypes = [ctypes.c_void_p]
    memory.CoTaskMemFree.restype = None
    try:
        result = shell.SHGetKnownFolderPath(ctypes.byref(folder), 0, None, ctypes.byref(value))
        if result or not value.value:
            raise OSError('application data directory unavailable')
        return Path(value.value)
    finally:
        if value:
            memory.CoTaskMemFree(ctypes.cast(value, ctypes.c_void_p))

def metadata_path(home, platform=None, environ=None):
    native_windows = platform is None and environ is None and sys.platform == 'win32'
    platform = sys.platform if platform is None else platform
    environ = os.environ if environ is None else environ
    home = Path(home)
    if platform == 'win32':
        # Tauri app_data_dir uses RoamingAppData, not LocalAppData.
        base = windows_data_home() if native_windows else Path(environ.get('APPDATA') or home / 'AppData' / 'Roaming')
    elif platform == 'darwin':
        base = home / 'Library' / 'Application Support'
    else:
        configured = environ.get('XDG_DATA_HOME', '')
        base = Path(configured) if configured and Path(configured).is_absolute() else home / '.local' / 'share'
    return base / SERVICE / 'mvp-browser-os' / 'ai-profiles.json'

def credential_references(config):
    """Follow persisted immutable refs, with the native legacy model fallback."""
    if not isinstance(config, dict):
        raise ValueError('invalid AI metadata')
    profiles, plugins = config.get('profiles'), config.get('plugins', [])
    if not isinstance(config.get('bindings'), dict) or not isinstance(profiles, list) or not isinstance(plugins, list) or len(profiles) > 256:
        raise ValueError('invalid AI metadata')
    references = set()

    def reference(value, namespace, expected_id=None):
        if not isinstance(value, str):
            raise ValueError('invalid credential reference')
        parts = value.split(':', 2)
        if len(parts) not in (2, 3) or parts[0] != namespace or not re.fullmatch(r'[0-9a-fA-F]{24}', parts[1]):
            raise ValueError('invalid credential reference')
        if expected_id is not None and parts[1] != expected_id:
            raise ValueError('credential owner mismatch')
        if len(parts) == 3:
            # Match uuid 1.24 parse_str, not Python UUID's more permissive
            # hyphen removal. Native model refs accept URNs; plugin aliases
            # split every colon and therefore do not accept that representation.
            hyphenated = r'[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}'
            version = parts[2]
            shapes = r'(?:[0-9a-fA-F]{32}|' + hyphenated + r'|\{' + hyphenated + r'\}|urn:uuid:' + hyphenated + r')'
            if not re.fullmatch(shapes, version) or (namespace == 'ai-plugin' and ':' in version):
                raise ValueError('invalid credential version')
        references.add(value)

    for profile in profiles:
        if not isinstance(profile, dict) or not isinstance(profile.get('id'), str):
            raise ValueError('invalid AI profile')
        value = profile.get('credential_ref', '')
        if not isinstance(value, str):
            raise ValueError('invalid credential reference')
        reference(value or 'ai:' + profile['id'], 'ai', profile['id'])
    for plugin in plugins:
        if not isinstance(plugin, dict) or not isinstance(plugin.get('credential_refs'), list):
            raise ValueError('invalid AI plugin')
        for value in plugin['credential_refs']:
            reference(value, 'ai-plugin')
    return references

def keyring_credentials(reference):
    """Read existing values only; never write, delete or print them."""
    if sys.platform == 'win32':
        import ctypes
        from ctypes import wintypes

        class Credential(ctypes.Structure):
            _fields_ = [
                ('Flags', wintypes.DWORD), ('Type', wintypes.DWORD),
                ('TargetName', wintypes.LPWSTR), ('Comment', wintypes.LPWSTR),
                ('LastWritten', wintypes.FILETIME), ('CredentialBlobSize', wintypes.DWORD),
                ('CredentialBlob', ctypes.POINTER(wintypes.BYTE)), ('Persist', wintypes.DWORD),
                ('AttributeCount', wintypes.DWORD), ('Attributes', wintypes.LPVOID),
                ('TargetAlias', wintypes.LPWSTR), ('UserName', wintypes.LPWSTR),
            ]

        api = ctypes.WinDLL('Advapi32.dll', use_last_error=True)
        pointer = ctypes.POINTER(Credential)()
        api.CredReadW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.POINTER(ctypes.POINTER(Credential))]
        api.CredReadW.restype = wintypes.BOOL
        api.CredFree.argtypes = [wintypes.LPVOID]
        api.CredFree.restype = None
        # Cargo.lock keyring 3.6.3 uses user.service and UTF-16LE passwords:
        # https://github.com/open-source-cooperative/keyring-rs/blob/v3.6.3/src/windows.rs
        if not api.CredReadW(reference + '.' + SERVICE, 1, 0, ctypes.byref(pointer)):
            if ctypes.get_last_error() == 1168:  # ERROR_NOT_FOUND
                return []
            raise OSError('credential store unavailable')
        try:
            size = pointer.contents.CredentialBlobSize
            if size > 16384 or size % 2:
                raise ValueError('credential encoding invalid')
            return [ctypes.string_at(pointer.contents.CredentialBlob, size).decode('utf-16-le')]
        finally:
            api.CredFree(pointer)
    if sys.platform == 'darwin':
        result = subprocess.run(
            ['/usr/bin/security', 'find-generic-password', '-s', SERVICE, '-a', reference, '-w'],
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=10, check=False,
        )
        if result.returncode == 44:  # errSecItemNotFound
            return []
        if result.returncode:
            raise OSError('credential store unavailable')
        return [result.stdout.removesuffix(b'\n').decode('utf-8')]

    import gi
    gi.require_version('Secret', '1')
    from gi.repository import Secret
    # Rust Keyring entries do not carry libsecret's xdg:schema attribute.
    schema = Secret.Schema.new('org.freedesktop.Secret.Generic', Secret.SchemaFlags.DONT_MATCH_NAME, {
        'service': Secret.SchemaAttributeType.STRING,
        'username': Secret.SchemaAttributeType.STRING,
    })
    service = Secret.Service.get_sync(Secret.ServiceFlags.OPEN_SESSION, None)
    items = service.search_sync(schema, {'service': SERVICE, 'username': reference},
                                Secret.SearchFlags.ALL | Secret.SearchFlags.LOAD_SECRETS, None)
    values = []
    for item in items or []:
        value = item.get_secret()
        if value is None or value.get_text() is None:
            raise OSError('credential store unavailable')
        values.append(value.get_text())
    return values

def known_credentials(home=None, lookup=None):
    home = Path.home() if home is None else Path(home)
    spec = importlib.util.spec_from_file_location('local_ai_import', ROOT / 'scripts/import-local-ai.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    _, values = module.discover(home)
    secrets = set()

    def remember(value):
        if not isinstance(value, str):
            raise ValueError('credential encoding invalid')
        for text in (value, value[7:] if value.startswith('Bearer ') else value):
            if len(text) >= 16:
                secrets.add(text.encode())

    for value in values.values():
        remember(value)
    metadata = metadata_path(home)
    try:
        info = metadata.lstat()
    except FileNotFoundError:
        return secrets
    if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_METADATA_BYTES:
        raise ValueError('unsafe AI metadata')
    with metadata.open('rb') as source:
        encoded = source.read(MAX_METADATA_BYTES + 1)
    if len(encoded) > MAX_METADATA_BYTES:
        raise ValueError('AI metadata limit')
    references = credential_references(json.loads(encoded))
    reader = keyring_credentials if lookup is None else lookup
    for reference in sorted(references):
        for value in reader(reference):
            remember(value)
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
