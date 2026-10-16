"""Unify existing entrypoints; preserve existing guards and add equivalent Studio guards."""
import re

native = 'src-tauri/src/capabilities/git/studio.rs'
s = read(native)
if 'GIT_STUDIO_READ_BASIS_CHANGED' not in s:
    s = once(s, '    pub pr: Option<u32>,', '    pub pr: Option<u32>,\n    pub expected_fingerprint: Option<String>,')
    s = once(s, '        "file" => {', '        "file" => {\n            let basis = snapshot(root)?;')
    original = 'json!({"path":p,"head":head,"index":staged,"worktree":working})'
    updated = 'json!({"path":p,"head":head,"index":staged,"worktree":working,"fingerprint":if snapshot(root)? == basis { basis } else { return Err("GIT_STUDIO_READ_BASIS_CHANGED".into()); }})'
    s = once(s, original, updated)
    s = once(s, '    let action = input.action.as_str();', '''    let action = input.action.as_str();
    if matches!(action, "index_edit" | "resolve") {
        let expected = input.expected_fingerprint.as_deref().ok_or("GIT_STUDIO_READ_BASIS_REQUIRED")?;
        if snapshot(root)? != expected { return Err("GIT_STUDIO_READ_BASIS_CHANGED".into()); }
    }''')
if '"conflict" =>' not in s:
    block = '''        "conflict" => {
            let path = selected.ok_or("GIT_STUDIO_SELECT_FILE")?;
            let basis = snapshot(root)?;
            let repo = git2::Repository::open(root).map_err(|_| "GIT_STUDIO_REPOSITORY_MISSING")?;
            let index = repo.index().map_err(|_| "GIT_STUDIO_INDEX_READ")?;
            let mut versions = Vec::new();
            let mut found = false;
            for stage in [1, 2, 3] {
                if let Some(entry) = index.get_path(Path::new(path), stage) {
                    found = true;
                    let blob = repo.find_blob(entry.id).map_err(|_| "GIT_STUDIO_BLOB_READ")?;
                    if blob.size() > TEXT_CAP || blob.is_binary() { return Err("GIT_STUDIO_BINARY_FILE".into()); }
                    versions.push(String::from_utf8(blob.content().to_vec()).map_err(|_| "GIT_STUDIO_BINARY_FILE")?);
                } else { versions.push(String::new()); }
            }
            if !found { return Err("GIT_STUDIO_NO_CONFLICT".into()); }
            if snapshot(root)? != basis { return Err("GIT_STUDIO_READ_BASIS_CHANGED".into()); }
            Ok(json!({"path":path,"ancestor":versions[0],"ours":versions[1],"theirs":versions[2],"fingerprint":basis}))
        },
'''
    s = once(s, '        "blame" =>', block + '        "blame" =>')
write(native, s)

legacy = 'src-tauri/src/capabilities/git/local_workbench.rs'
s = read(legacy)
if 'unwrap_or("SUCCESS")' in s:
    s, count = re.subn(r'checks\.iter\(\)\.any\(\|c\|\s*\{[\s\S]*?\}\)', 'checks.iter().any(|c| !super::studio::studio_check_passed(c))', s, count=1)
    if count != 1:
        raise RuntimeError('LEGACY_CI_CHECK_ANCHOR_MISSING')
write(legacy, s)

write('src/capabilities/git/ui/GitPanel.vue', '''<script setup lang="ts">
import GitStudio from "./GitStudio.vue";
</script>
<template><GitStudio /></template>
''')
write('src/capabilities/git/ui/GitLocalInspector.vue', '''<script setup lang="ts">
import GitStudio from "./GitStudio.vue";
const props = defineProps<{path:string;mode:string;closeLabel?:string;searchId?:string;initialAction?:string}>();
const emit = defineEmits<{close:[]}>();
</script>
<template><GitStudio v-bind="props" @close="emit('close')" /></template>
''')
write('src/capabilities/git/ui/GitHistory.vue', '''<script setup lang="ts">
import { ref, onUnmounted } from "vue";
import GitStudio from "./GitStudio.vue";
import { currentLocalGitIntent, clearLocalGitIntent, subscribeLocalGitIntent } from "../../../composables/gitLocalNavigation";
const local = ref(currentLocalGitIntent());
const stop = subscribeLocalGitIntent(intent => { local.value = intent; });
onUnmounted(stop);
</script>
<template><GitStudio :path="local?.path" :mode="local?.mode || 'history'" :close-label="local ? '返回配置仓库' : undefined" @close="clearLocalGitIntent()" /></template>
''')
write('src/capabilities/git/ui/LocalGitWorkbench.vue', '''<script setup lang="ts">
// Compatibility entrypoint only; no second operation implementation.
import GitStudio from "./GitStudio.vue";
const props = defineProps<{path:string;initialAction?:string}>();
</script>
<template><GitStudio v-bind="props" /></template>
''')

policy = 'scripts/check-git-ui-policy.py'
p = read(policy)
if '"studio": "src/capabilities/git/ui/GitStudio.vue"' not in p:
    p = once(p, 'FILES = {\n', 'FILES = {\n    "studio": "src/capabilities/git/ui/GitStudio.vue",\n')
start = p.index('    # 5) GitPanel')
end = p.index('    # 6) plugin shell', start)
section = '''    # 5) The compatibility panel must use the one real Studio implementation.
    # Legacy test fixtures continue to exercise the original state-store guards.
    panel = files.get("panel", "")
    if "<GitStudio" in panel:
        studio = files.get("studio", "")
        required = [
            ("selected.value.length", "must explicitly select file paths"),
            ("message.value.trim()", "must validate a commit message"),
            ("gitStudioPreview", "must request a native preview"),
            ("gitStudioConfirm", "must confirm the preview, not write directly"),
            ("!acknowledged.value", "must require explicit confirmation"),
            ("sourceIdentity(source.value)!==previewSource", "must reject source changes"),
            ("canSuspend:()=>!writeBusy.value", "must protect a running native write"),
        ]
        for token, why in required:
            if token not in studio:
                diags.append(Diag("GITUI_EMPTY_INPUT_IGNORED", FILES["studio"], why))
        if re.search(r'\\binvoke\\s*\\(', studio):
            diags.append(Diag("GITUI_INVOKE_DIRECT", FILES["studio"], "Studio must use the typed bridge"))
    else:
        for token, why in [("selectedCount", "must gate file ops by selection"),
                           ("commitMessage.trim", "must gate commit on non-empty message"),
                           ("branchName.trim", "must gate branch create on non-empty name")]:
            if token not in panel:
                diags.append(Diag("GITUI_EMPTY_INPUT_IGNORED", FILES["panel"], why))

'''
p = p[:start] + section + p[end:]
p = p.replace('    if local.exists():\n', '    if local.exists() and "<GitStudio" not in local.read_text(encoding="utf-8"):\n')
write(policy, p)
print('DEVELOPER_STUDIO_UNIFIED_GIT_ENTRYPOINTS=APPLIED')
