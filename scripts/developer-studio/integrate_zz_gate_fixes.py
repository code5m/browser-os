"""Adapt the existing Git safety fixture to the one unified, source-pinned UI.
Legacy fixtures and all other frontend restrictions remain unchanged.
"""
import re

path = 'scripts/check-git-ui-policy.py'
s = read(path)
if 'def studio_ui_violations(' not in s:
    s = s.replace('    "git_local_github",', '    "git_local_github",\n    "git_studio_read", "git_studio_preview", "git_studio_confirm", "git_studio_cancel",')
    helper = r'''def studio_ui_violations(source: str) -> list[str]:
    violations = []
    required = {
        "repos.repos": "GIT_UI_REPO_SOURCE_NOT_REUSED",
        "parseUnifiedPatch": "GIT_UI_DIFF_NOT_MOUNTED",
        "sourceIdentity(source.value)!==previewSource": "GIT_UI_SOURCE_NOT_PINNED",
        "canSuspend:()=>!writeBusy.value": "GIT_UI_WRITE_LIFECYCLE_UNGUARDED",
        "redactSecrets(value)": "GIT_UI_ERROR_NOT_REDACTED",
        'type="checkbox"': "GIT_UI_DANGEROUS_ACK_MISSING",
        '!acknowledged': "GIT_UI_CONFIRM_BUTTON_NOT_GATED",
    }
    for token, code in required.items():
        if token not in source:
            violations.append(code)
    prepare = ts_function_body(source, "prepare")
    confirm = ts_function_body(source, "confirm")
    for token in ("selected.value.length", "message.value.trim()"):
        if token not in prepare or prepare.find(token) > prepare.find("bridge.gitStudioPreview"):
            violations.append("GIT_UI_PRECHECK_AFTER_REQUEST")
    if "bridge.gitStudioPreview" not in prepare:
        violations.append("GIT_UI_REQUEST_MISSING")
    for token in ("!job", "!acknowledged.value", "writeBusy.value", "bridge.gitStudioConfirm(job.job_id,true)"):
        if token not in confirm:
            violations.append("GIT_UI_DANGEROUS_WITHOUT_DOUBLE_CONFIRM")
    for action in ("stage", "unstage", "discard", "commit", "amend", "branch_create", "checkout", "push"):
        if not re.search(r"[\"']" + action + r"[\"']", source):
            violations.append("GIT_UI_WRITE_ENTRY_MISSING")
    code = strip_comments(source)
    if re.search(r"\binvoke\s*(?:<[^>]*>)?\s*\(", code):
        violations.append("GIT_UI_GATE_BYPASSED")
    if re.search(r"localStorage|sessionStorage|indexedDB|\btoken\b", code):
        violations.append("GIT_UI_CREDENTIAL_PERSISTED")
    if re.search(r"watch\([^;]*?(?:prepare|confirm)\(", code):
        violations.append("GIT_UI_AUTO_WRITE_TRIGGER")
    return violations


'''
    at = s.index('def detect_violations(')
    s = s[:at] + helper + s[at:]
    a, b = s.index('    # ---- 8)'), s.index('    # ---- 9)')
    original = '\n'.join('    ' + line if line else '' for line in s[a:b].splitlines())
    s = s[:a] + '''    # ---- 8) Unified UI with original legacy fixtures retained ----
    studio = str(files.get("studio", ""))
    unified = "<GitStudio" in panel
    studio_violations = studio_ui_violations(studio) if unified else []
    if unified:
        violations.extend(studio_violations)
    else:
''' + original + '\n\n' + s[b:]
    s = once(s, 'if not (local_adapter and op in {"pull", "fetch", "stash"} and local_guard):', '''studio_adapter = path == "src/capabilities/git/ui/GitStudio.vue"
            studio_guard = unified and not studio_violations and text == studio
            registered_operation = op in {"pull", "fetch", "merge", "rebase", "stash", "cherry_pick"}
            if not ((local_adapter and op in {"pull", "fetch", "stash"} and local_guard)
                    or (studio_adapter and studio_guard and registered_operation)):''')
    s = once(s, '    files["src_files"] = src_files', '''    studio_path = src / "capabilities" / "git" / "ui" / "GitStudio.vue"
    if studio_path.is_file():
        files["studio"] = studio_path.read_text(encoding="utf-8")
    files["src_files"] = src_files''')
    write(path,s)

catalog = 'docs/architecture/ui-system/ui-components.yaml'
s = read(catalog)
entries = []
for id, name, owner, semantics, consumers in [
    ('git.studio','GitStudio','git','Unified source-bound Git workbench and validated native previews','GitPanel, GitHistory, GitLocalInspector'),
    ('database.workspace','DatabaseWorkspace','database','Source-bound SQL documents, results and transactions','DatabasePanel'),
    ('database.sql-editor','SqlEditor','database','Escaped SQL highlighting and bounded lexical editing','DatabaseWorkspace'),
    ('database.table-editor','TableDataEditor','database','Native typed data pages and parameterized mutation previews','DatabaseWorkspace'),
    ('database.schema-studio','SchemaStudio','database','Bounded metadata ER and explicit schema comparison','DatabaseWorkspace'),
    ('workspace.file-history','FileHistoryPanel','workspace','Opt-in encrypted file snapshots and pinned restore previews','FilePanel'),
]:
    file = f'src/capabilities/{owner}/ui/{name}.vue'
    if (ROOT / file).exists() and f'file: {file},' not in s:
        entries.append(f'  - {{ id: {id}, name: {name}, layer: CAPABILITY_UI, owner: {owner}, file: {file}, public: false, business_semantics: "{semantics}", capability_specific: true, dependencies: [bridge], consumers: [{consumers}], status: OBSERVED }}')
if entries:
    s = s.replace('components:\n','components:\n\n'+'\n'.join(entries)+'\n',1)
    total = len(re.findall(r'  - \{ id:',s))
    s = re.sub(r'^total: \d+',f'total: {total}',s,flags=re.M)
    write(catalog,s)

# Test the guards, not just a green indicator. Removing each essential condition
# from the real UI must be rejected by the same production safety fixture.
namespace = {'__name__':'studio_safety_fixture','__file__':str(ROOT / path)}
exec(compile(read(path),str(ROOT / path),'exec'),namespace)
ui = read('src/capabilities/git/ui/GitStudio.vue')
check = namespace['studio_ui_violations']
if check(ui):
    raise RuntimeError('UNIFIED_GIT_UI_GUARDS_FAILED: '+str(check(ui)))
for token in ['!acknowledged.value','selected.value.length','sourceIdentity(source.value)!==previewSource','bridge.gitStudioPreview','bridge.gitStudioConfirm','canSuspend:()=>!writeBusy.value']:
    if not check(ui.replace(token,'REMOVED_SAFETY_GUARD')):
        raise RuntimeError('UNIFIED_GIT_MUTATION_NOT_REJECTED: '+token)
print('DEVELOPER_STUDIO_SAFETY_MUTATIONS=PASS')
