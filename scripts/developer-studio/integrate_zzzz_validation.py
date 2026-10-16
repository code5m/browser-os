"""Repair every session constructor, including compact generated constructors.

Runs after table integration on both fresh and already-wired source. Never
changes quality thresholds, fixture expectations or the release baseline.
"""
import re

path = 'src-tauri/src/capabilities/database/studio.rs'
s = read(path)
pattern = re.compile(r'(ConnectionOwner\s*\{)([^{}]*)(\})', re.S)

def initialize_owner(match):
    body = match.group(2)
    # Skip the struct definition and already-correct constructor expressions.
    if 'transaction: TransactionState::Idle' not in body or re.search(r'\btables\s*:', body):
        return match.group(0)
    body = body.rstrip()
    if not body.endswith(','):
        body += ','
    return match.group(1) + body + '\n            tables: TableState::default(),\n        ' + match.group(3)

s = pattern.sub(initialize_owner, s)
for match in pattern.finditer(s):
    body = match.group(2)
    if 'transaction: TransactionState::Idle' in body and 'tables:' not in body:
        raise RuntimeError('DB_SESSION_TABLE_STATE_INITIALIZATION_MISSING')
if pattern.sub(initialize_owner, s) != s:
    raise RuntimeError('DB_SESSION_INITIALIZER_REPAIR_NOT_IDEMPOTENT')
write(path, s)

# Explicit error types at Result inference boundaries remain readable and avoid
# dependence on unrelated Into implementations added by driver dependencies.
path = 'src-tauri/src/capabilities/database/table_data.rs'
s = read(path)
s = s.replace('.ok_or_else(|| "DB_TABLE_UNKNOWN_COLUMN".into())',
              '.ok_or_else(|| String::from("DB_TABLE_UNKNOWN_COLUMN"))')
write(path, s)

print('DEVELOPER_STUDIO_TABLE_CONSTRUCTORS=VERIFIED')
