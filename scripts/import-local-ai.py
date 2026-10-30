#!/usr/bin/env python3
"""Explicit local AI configuration import. Never executes source files or prints secrets."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
from urllib.parse import urlsplit, urlunsplit

SERVICE = 'com.jizhijiandan.mvp'

def metadata_path(home):
    base=Path(os.environ.get('XDG_DATA_HOME', str(home/'.local/share')))
    return base/'com.jizhijiandan.mvp/mvp-browser-os/ai-profiles.json'

def identity(*parts):
    return hashlib.sha256('\0'.join(parts).encode()).hexdigest()[:24]

def usable(value):
    return (isinstance(value, str) and 16 <= len(value) <= 4096
            and re.fullmatch(r'(?:Bearer )?[A-Za-z0-9_.:+/=@!#-]+', value) is not None
            and not any(c in value for c in '\r\n\0')
            and not re.search(r'\$|\{|\}|example|placeholder|your[_ -]|changeme|xxxx|test[-_]key', value, re.I))

def endpoint(value):
    p = urlsplit(value)
    if p.scheme != 'https' or not p.hostname or p.username or p.password or p.query or p.fragment:
        raise ValueError('unsafe endpoint')
    path = p.path.rstrip('/')
    for suffix in ('/chat/completions', '/images/generations', '/videos'):
        if path.endswith(suffix):
            path = path[:-len(suffix)]
    return urlunsplit((p.scheme, p.netloc, path, '', ''))

def discover(home):
    models, credentials, plugins, sources, skipped = {}, {}, [], [], []
    def add_model(model, name, url, key, source, kind=None):
        if not usable(key):
            return
        try:
            base = endpoint(url)
        except ValueError:
            skipped.append(source + ': unsupported endpoint')
            return
        kind = kind or ('image' if 'image' in model or model.startswith('sensenova-u1') else 'video' if 'video' in model else 'unsupported' if 'music' in model else 'chat')
        mid = identity(base, model)
        models.setdefault(mid, dict(id=mid, name=name[:120], model=model[:160], base_url=base,
                                   kind=kind, source=source, credential_ref='ai:' + mid))
        credentials.setdefault('ai:' + mid, key)
    for root in ('.codebuddy', '.workbuddy', '.config/CodeBuddy', '.config/WorkBuddy'):
        for relative in ('models.json', 'User/models.json'):
            path = home / root / relative
            if not path.is_file():
                continue
            sources.append(str(path.relative_to(home)))
            try:
                data = json.loads(path.read_text())
                for row in data.get('models', []) if isinstance(data, dict) else data:
                    add_model(str(row.get('id', '')), str(row.get('name', row.get('id', ''))),
                              str(row.get('url', '')), row.get('apiKey'), str(path.relative_to(home)))
            except (ValueError, TypeError, AttributeError):
                skipped.append(str(path.relative_to(home)) + ': unreadable configuration')
    for root in ('.agents', '.codex', '.codebuddy', '.workbuddy'):
        path = home / root / 'skills/ai-image-generation/scripts/gen-image.sh'
        if not path.is_file():
            continue
        sources.append(str(path.relative_to(home)))
        key = url = ''
        for line in path.read_text().splitlines():
            match = re.match(r'^\s*(KEY|URL|MODEL)=["\']([^"\']+)["\']\s*$', line)
            if not match:
                continue
            field, value = match.groups()
            if field == 'KEY':
                key = value
            elif field == 'URL':
                url = value
            elif key and url:
                add_model(value, value, url, key, str(path.relative_to(home)), 'image')
                if 'agnes' in url:
                    for extra in ('agnes-image-2.1-flash', 'agnes-image-2.0-flash', 'agnes-video-v2.0'):
                        add_model(extra, extra, url, key, str(path.relative_to(home)), 'video' if 'video' in extra else 'image')
    # Inspect only plugin manifests/configuration and explicit skill scripts, never sessions/accounts/cookies.
    files = set()
    for root in (home / '.codebuddy', home / '.workbuddy'):
        for name in ('mcp.json', 'plugins/installed_plugins.json'):
            if (root / name).is_file():
                files.add(root / name)
        for folder in ('plugins', 'skills'):
            base = root / folder
            if base.is_dir():
                for pattern in ('.mcp.json', 'mcp.json', '*.env', '.env', '*.py', '*.sh', 'config.json'):
                    files.update(base.rglob(pattern))
    for path in sorted(files):
        if any(part in {'node_modules', '.git', 'templates', 'examples', 'tests', '__pycache__'} for part in path.parts):
            continue
        if path.is_symlink() or path.stat().st_size > 1024 * 1024:
            continue
        source = str(path.relative_to(home))
        text = path.read_text(errors='replace')
        refs = []
        if path.suffix == '.json':
            try:
                value = json.loads(text)
            except ValueError:
                continue
            def walk(obj, trail=''):
                if isinstance(obj, dict):
                    for name, item in obj.items():
                        location = trail + '.' + name
                        if isinstance(item, str) and re.search(r'key|token|secret|authorization|password', name, re.I) and usable(item):
                            ref = 'ai-plugin:' + identity(source, location)
                            credentials.setdefault(ref, item)
                            refs.append(ref)
                        else:
                            walk(item, location)
                elif isinstance(obj, list):
                    for i, item in enumerate(obj):
                        walk(item, trail + '.' + str(i))
            walk(value)
            for name, item in value.get('mcpServers', {}).items() if isinstance(value, dict) else []:
                plugins.append(dict(name=name, source=source, transport=item.get('type', 'stdio'), credential_refs=refs, status='catalogued'))
        else:
            for match in re.finditer(r'(?im)^\s*(?:export\s+)?([\w]*(?:KEY|TOKEN|SECRET|PASSWORD)[\w]*)\s*=\s*(.+?)\s*$', text):
                name, key = match.groups()
                if len(key)>1 and key[0] in '\"\'' and key[-1]==key[0]:
                    key=key[1:-1]
                elif re.search(r'\s|[;#()]',key):
                    continue
                if usable(key):
                    ref = 'ai-plugin:' + identity(source, name)
                    credentials.setdefault(ref, key)
                    refs.append(ref)
        if refs and not any(p['source'] == source for p in plugins):
            plugins.append(dict(name=path.parent.name, source=source, transport='skill', credential_refs=refs, status='catalogued'))
    return dict(profiles=list(models.values()), plugins=plugins, sources=sources, skipped=skipped), credentials

def apply(home, destination):
    data, secrets = discover(home)
    import gi
    gi.require_version('Secret', '1')
    from gi.repository import Secret
    schema = Secret.Schema.new('org.freedesktop.Secret.Generic', Secret.SchemaFlags.NONE,
                               {'service': Secret.SchemaAttributeType.STRING, 'username': Secret.SchemaAttributeType.STRING})
    if destination.is_symlink():
        raise RuntimeError('unsafe destination')
    existing = json.loads(destination.read_text()) if destination.exists() else dict(profiles=[], bindings={}, plugins=[])
    known = {p['id'] for p in existing['profiles']}
    imported = []
    for profile in data['profiles']:
        if profile['id'] in known:
            continue
        ref = profile['credential_ref']
        attrs = {'service': SERVICE, 'username': ref}
        if not Secret.password_lookup_sync(schema, attrs, None):
            if not Secret.password_store_sync(schema, attrs, Secret.COLLECTION_DEFAULT, 'BrowserOS AI', secrets[ref], None):
                raise RuntimeError('credential store unavailable')
        imported.append(profile)
    for ref, secret in secrets.items():
        if not ref.startswith('ai-plugin:'):
            continue
        attrs = {'service': SERVICE, 'username': ref}
        if not Secret.password_lookup_sync(schema, attrs, None):
            if not Secret.password_store_sync(schema, attrs, Secret.COLLECTION_DEFAULT, 'BrowserOS plugin credential', secret, None):
                raise RuntimeError('credential store unavailable')
    existing['profiles'].extend(imported)
    if len(existing['profiles']) > 256:
        raise RuntimeError('profile limit exceeded')
    existing['plugins'] = data['plugins']
    existing['import_sources'] = data['sources']
    chats = [p for p in existing['profiles'] if p['kind'] == 'chat']
    if chats and not existing.get('bindings', {}).get('default'):
        chosen = next((p for p in chats if p['model'] == 'agnes-3.0-flash'), chats[0])
        existing.setdefault('bindings', {})['default'] = chosen['id']
    destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    fd, tmp = tempfile.mkstemp(prefix='ai-profiles.', dir=destination.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(existing, stream, ensure_ascii=False, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(tmp, destination)
    finally:
        if os.path.exists(tmp):
            os.unlink(tmp)
    return dict(imported=len(imported), total=len(existing['profiles']), plugin_credentials=sum(k.startswith('ai-plugin:') for k in secrets), plugins=len(data['plugins']), sources=data['sources'], skipped=data['skipped'])

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--destination', type=Path)
    args = parser.parse_args()
    try:
        home = Path.home()
        if args.apply:
            destination = args.destination or metadata_path(home)
            print(json.dumps(apply(home, destination), ensure_ascii=False))
        else:
            data, secrets = discover(home)
            print(json.dumps(dict(models=len(data['profiles']), plugin_credentials=sum(k.startswith('ai-plugin:') for k in secrets), plugins=len(data['plugins']), sources=data['sources'], skipped=data['skipped']), ensure_ascii=False))
    except Exception:
        print('{"error":"AI_IMPORT_FAILED: existing configuration preserved; check source format and unlocked OS keyring"}')
        raise SystemExit(1)
