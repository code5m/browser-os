#!/usr/bin/env python3
"""Discover local AI settings without executing sources. Publication belongs to BrowserOS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
from urllib.parse import urlsplit, urlunsplit

PROTOCOL = 'browseros-ai-import-v1'
MAX_FILE_BYTES = 1024 * 1024
# CodeBuddy/WorkBuddy installations can contain many vendor skill files. Keep
# the bounded read-only scan finite, but leave enough headroom to inspect the
# configured plugin/skill manifests before the pre-commit secret check fails
# solely because of inventory size.
MAX_SCAN_BYTES = 32 * 1024 * 1024
MAX_SCAN_FILES = 2048
MAX_SCAN_ENTRIES = 32768
MAX_SCAN_DEPTH = 12
MAX_OUTPUT_BYTES = 256 * 1024
MAX_PROFILES = 256
MAX_PLUGINS = 256
MAX_PLUGIN_CREDENTIALS = 256
MAX_CREDENTIALS = MAX_PROFILES + MAX_PLUGIN_CREDENTIALS
EXCLUDED = {'node_modules', '.git', 'templates', 'examples', 'tests', '__pycache__'}

class DiscoveryLimit(RuntimeError):
    pass

def identity(*parts):
    return hashlib.sha256('\0'.join(parts).encode()).hexdigest()[:24]

def usable(value):
    return (isinstance(value, str) and 16 <= len(value) <= 4096
            and re.fullmatch(r'(?:Bearer )?[A-Za-z0-9_.:+/=@!#-]+', value) is not None
            and not re.search(r'\$|\{|\}|example|placeholder|your[_ -]|changeme|xxxx|test[-_]key', value, re.I))

def bounded_text(value, maximum):
    return (isinstance(value, str) and bool(value)
            and len(value.encode('utf-8')) <= maximum
            and not any(ord(c) < 32 or ord(c) == 127 for c in value))

def clip_text(value, maximum):
    return value.encode('utf-8')[:maximum].decode('utf-8', errors='ignore')

def endpoint(value):
    if not bounded_text(value, 2048):
        raise ValueError('unsafe endpoint')
    p = urlsplit(value)
    if p.scheme != 'https' or not p.hostname or p.username or p.password or p.query or p.fragment:
        raise ValueError('unsafe endpoint')
    path = p.path.rstrip('/')
    for suffix in ('/chat/completions', '/images/generations', '/videos'):
        if path.endswith(suffix):
            path = path[:-len(suffix)]
    return urlunsplit((p.scheme, p.netloc, path, '', ''))

def redirected(info):
    return stat.S_ISLNK(info.st_mode) or bool(getattr(info, 'st_file_attributes', 0) & 0x400)

def discover(home):
    home = Path(home).absolute()
    models, credentials, plugins, sources, skipped = {}, {}, [], [], []
    seen_plugins, cached = set(), {}
    scanned_bytes = scanned_entries = 0

    def source_name(path):
        name = str(path.relative_to(home))
        if not bounded_text(name, 1024):
            raise DiscoveryLimit('source path limit')
        return name

    def skip(source, reason):
        if len(skipped) >= MAX_SCAN_FILES:
            raise DiscoveryLimit('source error limit')
        skipped.append(source + ': ' + reason)

    def safe_info(path):
        current = home
        for part in path.relative_to(home).parts:
            if part in {'.', '..'}:
                raise OSError('unsafe source')
            current = current / part
            info = current.lstat()
            if redirected(info):
                raise OSError('redirected source')
        return path.lstat()

    def read_source(path):
        nonlocal scanned_bytes
        if path in cached:
            return cached[path]
        source = source_name(path)
        try:
            info = safe_info(path)
        except FileNotFoundError:
            return None
        except OSError:
            skip(source, 'unsupported source')
            return None
        if len(cached) >= MAX_SCAN_FILES:
            raise DiscoveryLimit('file count limit')
        cached[path] = None
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_FILE_BYTES:
            skip(source, 'unsupported source')
            return None
        try:
            fd = os.open(path, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0))
            with os.fdopen(fd, 'rb') as stream:
                opened = os.fstat(stream.fileno())
                if (not stat.S_ISREG(opened.st_mode) or redirected(opened)
                        or (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino)):
                    raise OSError('source changed')
                raw = stream.read(MAX_FILE_BYTES + 1)
                final = safe_info(path)
                if (final.st_dev, final.st_ino) != (opened.st_dev, opened.st_ino):
                    raise OSError('source changed')
            if len(raw) > MAX_FILE_BYTES:
                raise DiscoveryLimit('file byte limit')
            scanned_bytes += len(raw)
            if scanned_bytes > MAX_SCAN_BYTES:
                raise DiscoveryLimit('scan byte limit')
            text = raw.decode('utf-8')
        except (OSError, UnicodeError):
            skip(source, 'unreadable configuration')
            return None
        cached[path] = text
        sources.append(source)
        return text

    def remember_secret(reference, secret):
        if reference not in credentials and len(credentials) >= MAX_CREDENTIALS:
            raise DiscoveryLimit('credential count limit')
        if reference.startswith('ai-plugin:') and reference not in credentials:
            if sum(ref.startswith('ai-plugin:') for ref in credentials) >= MAX_PLUGIN_CREDENTIALS:
                raise DiscoveryLimit('plugin credential count limit')
        credentials.setdefault(reference, secret)

    def add_model(model, name, url, key, source, kind=None):
        if not usable(key):
            return
        if not bounded_text(model, 160) or not bounded_text(name, 4096):
            skip(source, 'unsupported model')
            return
        try:
            base = endpoint(url)
        except (ValueError, TypeError):
            skip(source, 'unsupported endpoint')
            return
        kind = kind or ('image' if 'image' in model or model.startswith('sensenova-u1') else 'video' if 'video' in model else 'unsupported' if 'music' in model else 'chat')
        mid = identity(base, model)
        if mid not in models and len(models) >= MAX_PROFILES:
            raise DiscoveryLimit('model count limit')
        models.setdefault(mid, dict(id=mid, name=clip_text(name, 120), model=model, base_url=base,
                                   kind=kind, source=source, credential_ref='ai:' + mid))
        remember_secret('ai:' + mid, key)

    def add_plugin(name, source, transport, refs):
        if not refs:
            return
        name = clip_text(str(name), 120)
        if not bounded_text(name, 120) or not bounded_text(transport, 32):
            skip(source, 'unsupported plugin metadata')
            return
        refs = list(dict.fromkeys(refs))
        if len(refs) > 64:
            raise DiscoveryLimit('plugin reference limit')
        key = (source, name)
        if key in seen_plugins:
            return
        if len(plugins) >= MAX_PLUGINS:
            raise DiscoveryLimit('plugin count limit')
        seen_plugins.add(key)
        plugins.append(dict(name=name, source=source, transport=transport,
                            credential_refs=refs, status='catalogued'))

    for root in ('.codebuddy', '.workbuddy', '.config/CodeBuddy', '.config/WorkBuddy'):
        for relative in ('models.json', 'User/models.json'):
            path = home / root / relative
            text = read_source(path)
            if text is None:
                continue
            source = source_name(path)
            try:
                data = json.loads(text)
                rows = data.get('models', []) if isinstance(data, dict) else data
                if not isinstance(rows, list):
                    raise ValueError('models must be a list')
                for row in rows:
                    if not isinstance(row, dict):
                        raise ValueError('model must be an object')
                    add_model(str(row.get('id', '')), str(row.get('name', row.get('id', ''))),
                              str(row.get('url', '')), row.get('apiKey'), source)
            except (ValueError, TypeError, RecursionError):
                skip(source, 'unreadable configuration')
    for root in ('.agents', '.codex', '.codebuddy', '.workbuddy'):
        path = home / root / 'skills/ai-image-generation/scripts/gen-image.sh'
        text = read_source(path)
        if text is None:
            continue
        source = source_name(path)
        key = url = ''
        for line in text.splitlines():
            match = re.match(r'^\s*(KEY|URL|MODEL)=["\']([^"\']+)["\']\s*$', line)
            if not match:
                continue
            field, value = match.groups()
            if field == 'KEY':
                key = value
            elif field == 'URL':
                url = value
            elif key and url:
                add_model(value, value, url, key, source, 'image')
                if 'agnes' in url:
                    for extra in ('agnes-image-2.1-flash', 'agnes-image-2.0-flash', 'agnes-video-v2.0'):
                        add_model(extra, extra, url, key, source, 'video' if 'video' in extra else 'image')

    def scan_directory(base, depth=0):
        nonlocal scanned_entries
        try:
            info = safe_info(base)
            if not stat.S_ISDIR(info.st_mode):
                return
            if depth > MAX_SCAN_DEPTH:
                raise DiscoveryLimit('directory depth limit')
            with os.scandir(base) as entries:
                children = []
                for entry in entries:
                    scanned_entries += 1
                    if scanned_entries > MAX_SCAN_ENTRIES:
                        raise DiscoveryLimit('directory entry limit')
                    children.append(entry.name)
            for name in sorted(children):
                if name in EXCLUDED:
                    continue
                path = base / name
                try:
                    info = safe_info(path)
                except FileNotFoundError:
                    continue
                except OSError:
                    skip(source_name(path), 'unsupported source')
                    continue
                if stat.S_ISDIR(info.st_mode):
                    yield from scan_directory(path, depth + 1)
                elif stat.S_ISREG(info.st_mode) and (name in {'.mcp.json', 'mcp.json', '.env', 'config.json'}
                                                    or path.suffix in {'.env', '.py', '.sh'}):
                    yield path
        except FileNotFoundError:
            return
        except OSError:
            skip(source_name(base), 'unsupported directory')

    # Only explicit settings, manifests and skill source text are inspected.
    # Sessions/accounts/cookies and executable source evaluation are excluded.
    files = set()
    for root in (home / '.codebuddy', home / '.workbuddy'):
        files.update(root / name for name in ('mcp.json', 'plugins/installed_plugins.json'))
        for folder in ('plugins', 'skills'):
            for path in scan_directory(root / folder):
                if len(files) >= MAX_SCAN_FILES:
                    raise DiscoveryLimit('file count limit')
                files.add(path)
    for path in sorted(files):
        text = read_source(path)
        if text is None:
            continue
        source = source_name(path)
        refs = []
        if path.suffix == '.json':
            try:
                value = json.loads(text)
            except (ValueError, RecursionError):
                skip(source, 'unreadable configuration')
                continue
            nodes = 0
            def walk(obj, trail='', depth=0):
                nonlocal nodes
                nodes += 1
                if nodes > 16384 or depth > 32 or len(trail) > 4096:
                    raise DiscoveryLimit('configuration shape limit')
                if isinstance(obj, dict):
                    for name, item in obj.items():
                        location = trail + '.' + name
                        if isinstance(item, str) and re.search(r'key|token|secret|authorization|password', name, re.I) and usable(item):
                            ref = 'ai-plugin:' + identity(source, location)
                            remember_secret(ref, item)
                            refs.append(ref)
                        else:
                            walk(item, location, depth + 1)
                elif isinstance(obj, list):
                    for index, item in enumerate(obj):
                        walk(item, trail + '.' + str(index), depth + 1)
            walk(value)
            servers = value.get('mcpServers', {}) if isinstance(value, dict) else {}
            if isinstance(servers, dict):
                for name, item in servers.items():
                    if isinstance(item, dict):
                        add_plugin(name, source, str(item.get('type', 'stdio')), refs)
        else:
            for match in re.finditer(r'(?im)^\s*(?:export\s+)?([\w]*(?:KEY|TOKEN|SECRET|PASSWORD)[\w]*)\s*=\s*(.+?)\s*$', text):
                name, key = match.groups()
                if len(key) > 1 and key[0] in '\"\'' and key[-1] == key[0]:
                    key = key[1:-1]
                elif re.search(r'\s|[;#()]', key):
                    continue
                if usable(key):
                    ref = 'ai-plugin:' + identity(source, name)
                    remember_secret(ref, key)
                    refs.append(ref)
        if refs and not any(plugin['source'] == source for plugin in plugins):
            add_plugin(path.parent.name, source, 'skill', refs)
    # Do not export unattached secrets when malformed metadata was skipped.
    used = {profile['credential_ref'] for profile in models.values()}
    used.update(ref for plugin in plugins for ref in plugin['credential_refs'])
    credentials = {ref: secret for ref, secret in credentials.items() if ref in used}
    data = dict(profiles=list(models.values()), plugins=plugins, sources=sources, skipped=skipped)
    public = json.dumps(data, ensure_ascii=False)
    # A value may contain an HTTP Authorization prefix. Metadata must not
    # expose either its full representation or the underlying bearer token.
    if any(secret in public or (secret.startswith('Bearer ') and secret[7:] and secret[7:] in public)
           for secret in credentials.values()):
        raise RuntimeError('secret present in metadata')
    return data, credentials

def encode_bounded(value):
    encoded = json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
    if len(encoded) > MAX_OUTPUT_BYTES:
        raise DiscoveryLimit('output byte limit')
    return encoded

def native_export(home, request_bytes):
    if len(request_bytes) > 4096:
        raise DiscoveryLimit('request byte limit')
    request = json.loads(request_bytes)
    if (not isinstance(request, dict) or set(request) != {'protocol', 'request_id'}
            or request['protocol'] != PROTOCOL
            or not isinstance(request['request_id'], str)
            or not re.fullmatch(r'[0-9a-f]{32}', request['request_id'])):
        raise ValueError('unsupported native request')
    data, secrets = discover(home)
    return encode_bounded(dict(protocol=PROTOCOL, request_id=request['request_id'],
                               credentials=secrets, **data))

def summary(home):
    data, secrets = discover(home)
    return dict(mode='read-only', next_step='在 BrowserOS 设置 → 统一 AI 配置中选择“导入本机配置”完成导入。',
                models=len(data['profiles']),
                plugin_credentials=sum(ref.startswith('ai-plugin:') for ref in secrets),
                plugins=len(data['plugins']), sources=data['sources'], skipped=data['skipped'])

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--apply', action='store_true',
                        help='retired write entry; use BrowserOS Import local configuration')
    parser.add_argument('--destination', type=Path)
    parser.add_argument('--native-export', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.apply or args.destination is not None:
        print(json.dumps({'error': 'AI_IMPORT_NATIVE_REQUIRED',
                          'message': '请使用 BrowserOS 的“导入本机配置”；现有配置和系统密钥未修改。'},
                         ensure_ascii=False))
        return 2
    try:
        home = Path.home()
        if args.native_export:
            if sys.stdin.isatty() or sys.stdout.isatty():
                raise ValueError('native pipe required')
            # This payload is consumed only by the native Owner; it never enters
            # a UI response, log, command argument or temporary file.
            payload = native_export(home, sys.stdin.buffer.read(4097))
            sys.stdout.buffer.write(payload)
        else:
            sys.stdout.buffer.write(encode_bounded(summary(home)) + b'\n')
        return 0
    except Exception:
        print('{"error":"AI_IMPORT_FAILED","message":"Local discovery failed; existing configuration and credentials are unchanged."}')
        return 1

if __name__ == '__main__':
    raise SystemExit(main())
