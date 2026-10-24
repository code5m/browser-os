#!/usr/bin/env python3
"""Local-only import. Read secrets from the explicitly supplied skills; never print/export them.

Dry-run is the default. --apply requires BrowserOS to be closed. Imported entries
are unverified, read-only, and credential values go exclusively to Secret Service.
No SQL or shell from the skill files is executed.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tempfile


def cells(line):
    return [cell.strip().strip('*').strip() for cell in line.strip().strip('|').split('|')]


def codes(text):
    return re.findall(r'`([^`]+)`', text)


def collect(backend, yili):
    records = []

    def add(label, kind, host, port, database, username, password, production):
        if not all(isinstance(v, str) and v for v in [host, database, username, password]):
            raise ValueError('Source contains an incomplete database credential')
        key = '\0'.join([kind, host, str(port), database, username])
        ident = 'skill-' + hashlib.sha256(key.encode()).hexdigest()[:24]
        if any(item['config']['id'] == ident for item in records):
            return
        now = datetime.datetime.now(datetime.timezone.utc).isoformat()
        records.append({'config': dict(id=ident, name=f'{label} · {database}（待验证）', kind=kind,
            host=host, port=port, database=database, username=username, ssl_mode='prefer',
            allow_write=False, production_hint=production, enabled=False, created_at=now, updated_at=now),
            'password': password})

    text = (backend / 'references/connections.md').read_text()
    section = ''
    for line in text.splitlines():
        if line.startswith('##'):
            section = line
        if not line.startswith('|'):
            continue
        row = cells(line)
        if section.startswith('### 1.1') and len(row) == 5 and len(codes(row[1])) == 1 and codes(row[2]) and codes(row[3]):
            label = re.sub(r'[*`]', '', row[0])
            address = codes(row[1])[0]
            host, port = address.rsplit(':', 1)
            database = 'phantom_iot' if 'iot' in label else 'phantom_admin_staging' if 'staging' in label else 'phantom_admin'
            add('Phantom ' + label, 'mysql', host, int(port), database, codes(row[2])[0], codes(row[3])[0], 'prod' in label)
        if section.startswith('## 3. PostgreSQL') and len(row) == 5 and codes(row[1]) and codes(row[3]) and codes(row[4]):
            address = codes(row[1])[0].split('/')[0]
            host, port = address.rsplit(':', 1) if ':' in address else (address, 5432)
            for database in codes(row[2]):
                add('Phantom ' + re.sub(r'[*`]', '', row[0]), 'postgres', host, int(port), database, codes(row[3])[0], codes(row[4])[0], 'prod' in row[0])

    text = (yili / 'references/connections.md').read_text()
    section = ''
    for line in text.splitlines():
        if line.startswith('## '):
            section = line
        if not line.startswith('|'):
            continue
        row = cells(line)
        if section.startswith('## 1. MySQL') and len(row) == 4 and len(codes(row[2])) == 2:
            for address in codes(row[1]):
                host, port = address.rsplit(':', 1)
                database = 'foc_iot_cooler' if '2ze8b5xb' in host else 'foc_payment'
                add('伊利 ' + row[0] + ' RDS', 'mysql', host, int(port), database, *codes(row[2]), row[0] == 'prod')
        if section.startswith('## 7. PostgreSQL') and len(row) == 3 and codes(row[1]) and len(codes(row[2])) == 2:
            address, database = codes(row[1])[0].split('/', 1)
            host, port = address.rsplit(':', 1)
            add('伊利 ' + row[0], 'postgres', host, int(port), database, *codes(row[2]), row[0] == 'prod')
    # Parse literal assignments only. Never source/eval an external skill script.
    transit = (yili / 'scripts/transit.sh').read_text()
    values = {}
    for match in re.finditer(r'(?:^|;)\s*([A-Z][A-Z0-9_]*)=(?:"([^"\n]*)"|\x27([^\x27\n]*)\x27|(\d+))', transit, re.M):
        value = next(v for v in match.groups()[1:] if v is not None)
        if '$' not in value and '`' not in value:
            values[match[1]] = value
    for prefix, label, database in [('UAT', 'uat 中转', 'foc_payment'), ('PROD', 'prod 开门柜中转', 'foc_payment'), ('KAIMEN', 'prod 智能柜中转', 'foc_iot_cooler')]:
        add('伊利 ' + label, 'mysql', values[prefix+'_HOST'], int(values[prefix+'_PORT']), database, values[prefix+'_USER'], values[prefix+'_PWD'], prefix != 'UAT')
    return records


def running_clients():
    for path in Path('/proc').glob('[0-9]*/exe'):
        try:
            if Path(os.readlink(path)).name.removesuffix(' (deleted)') == 'mvp-browser-os':
                return True
        except OSError:
            continue
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--backend-skill', type=Path, required=True)
    parser.add_argument('--yili-skill', type=Path, required=True)
    parser.add_argument('--apply', action='store_true')
    args = parser.parse_args()
    records = collect(args.backend_skill, args.yili_skill)
    print('SOURCE_SQL_CONNECTIONS=' + str(len(records)))
    print('MODE=' + ('APPLY' if args.apply else 'DRY_RUN'))
    print('DEFAULT_DATABASE=source-backed representative database per endpoint; other databases are not guessed')
    print('NOT_IMPORTED=Redis,ClickHouse,unknown/unassigned transit channels; no SQL Studio drivers/resource mapping')
    if not args.apply:
        return
    if running_clients():
        raise RuntimeError('CLIENT_RUNNING: save work and close BrowserOS before offline import')
    import gi
    gi.require_version('Secret', '1')
    from gi.repository import Secret
    schema = Secret.Schema.new('org.freedesktop.Secret.Generic', Secret.SchemaFlags.NONE,
        {'service': Secret.SchemaAttributeType.STRING, 'username': Secret.SchemaAttributeType.STRING})
    path = Path(os.environ.get('XDG_DATA_HOME', str(Path.home() / '.local/share'))) / 'com.jizhijiandan.mvp/mvp-browser-os/database-profiles.json'
    if path.is_symlink() or any(parent.is_symlink() for parent in path.parents):
        raise RuntimeError('PROFILE_PATH_UNSAFE')
    existing = json.loads(path.read_text()) if path.exists() else []
    if not isinstance(existing, list):
        raise RuntimeError('PROFILE_FORMAT_INVALID')
    identities = {(c['kind'], c.get('host'), c.get('port'), c['database'], c.get('username')) for c in existing}
    existing_ids = {c['id'] for c in existing}
    new = [r for r in records if r['config']['id'] not in existing_ids and (r['config']['kind'], r['config']['host'], r['config']['port'], r['config']['database'], r['config']['username']) not in identities]
    if len(existing) + len(new) > 64:
        raise RuntimeError('PROFILE_LIMIT: no existing connection removed')
    # Write only NEW credential IDs. Existing passwords/configurations are never replaced.
    for item in new:
        attrs = {'service': 'com.jizhijiandan.mvp', 'username': 'db:' + item['config']['id']}
        if not Secret.password_store_sync(schema, attrs, Secret.COLLECTION_DEFAULT, 'BrowserOS imported database', item['password'], None):
            raise RuntimeError('KEYRING_WRITE_FAILED: metadata unchanged')
    if running_clients():
        raise RuntimeError('CLIENT_STARTED_DURING_IMPORT: metadata unchanged; retry after closing')
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if path.exists():
        backup = path.with_name('database-profiles.backup-' + datetime.datetime.now().strftime('%Y%m%d%H%M%S') + '.json')
        shutil.copy2(path, backup)
        backup.chmod(0o600)
    descriptor, temporary = tempfile.mkstemp(prefix='database-profiles.import-', dir=path.parent)
    try:
        with os.fdopen(descriptor, 'w') as stream:
            json.dump(existing + [r['config'] for r in new], stream, ensure_ascii=False, indent=2)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    print('IMPORTED=' + str(len(new)))
    print('DUPLICATES_PRESERVED=' + str(len(records) - len(new)))
    print('STATUS=UNVERIFIED_READ_ONLY; credentials in OS keyring only; enable after connection test')


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        # Never echo parser values, credential strings, or secret-service errors.
        print('IMPORT_BLOCKED=' + (str(error) if isinstance(error, RuntimeError) else type(error).__name__))
        raise SystemExit(1)
