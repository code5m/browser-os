import fs from 'node:fs';

const files = {
  contract: 'docs/design/rebased-git-adapter-contract.md',
  bridge: 'src/bridge.ts',
  vue: 'src/capabilities/git/ui/GitStudio.vue',
  rust: 'src-tauri/src/capabilities/git/studio.rs',
  registration: 'src-tauri/src/main.rs',
  settings: 'src/settings/state/useSettingsStore.ts',
};
const source = Object.fromEntries(Object.entries(files).map(([key, path]) => {
  if (!fs.existsSync(path)) {
    console.error(`[FAIL] missing ${path}`);
    process.exit(1);
  }
  return [key, fs.readFileSync(path, 'utf8')];
}));

const checks = [
  ['contract declares read', source.contract.includes('git_studio_read')],
  ['contract declares preview', source.contract.includes('git_studio_preview')],
  ['contract declares confirm', source.contract.includes('git_studio_confirm')],
  ['bridge exposes read', source.bridge.includes('gitStudioRead')],
  ['bridge exposes preview', source.bridge.includes('gitStudioPreview')],
  ['bridge exposes confirm', source.bridge.includes('gitStudioConfirm')],
  ['vue calls read', source.vue.includes('bridge.gitStudioRead')],
  ['vue calls preview', source.vue.includes('bridge.gitStudioPreview')],
  ['vue calls confirm', source.vue.includes('bridge.gitStudioConfirm')],
  ['rust implements read', source.rust.includes('pub async fn git_studio_read')],
  ['rust implements preview', source.rust.includes('pub async fn git_studio_preview')],
  ['rust implements confirm', source.rust.includes('pub async fn git_studio_confirm')],
  ['rust registers read', source.registration.includes('capabilities::git::studio::git_studio_read')],
  ['rust registers preview', source.registration.includes('capabilities::git::studio::git_studio_preview')],
  ['rust registers confirm', source.registration.includes('capabilities::git::studio::git_studio_confirm')],
  ['update strategy crosses bridge', source.vue.includes("strategy:action==='update'?updateStrategy.value:undefined") && source.rust.includes('pub strategy: Option<String>')],
  ['merge update maps to no-rebase', source.rust.includes('"Merge" => "--no-rebase"')],
  ['rebase update maps to rebase', source.rust.includes('"Rebase" => "--rebase"')],
  ['fast-forward update remains default', source.rust.includes('unwrap_or("Fast-forward only")') && source.rust.includes('"--ff-only"')],
  ['update strategy rejects unknown values', source.rust.includes('GIT_STUDIO_UPDATE_STRATEGY_INVALID')],
  ['write failures preserve bounded git stderr', source.rust.includes('stderr: String') && source.rust.includes('.stderr(Stdio::piped())') && source.rust.includes('output.stderr.as_str()')],
  ['conflict resolution writes and stages content', source.rust.includes('"resolve"') && source.rust.includes('vec!["add".into()') && source.rust.includes('conflict_read_returns_explicit_hunk_metadata')],
  ['git studio supports dark theme', source.vue.includes(':global([data-theme="dark"]) .git-studio')],
  ['git studio keeps light theme defaults', source.vue.includes('background:var(--surface,#fff)') && source.vue.includes('color:var(--text,#273a50)')],
  ['rebased columns share one width source', source.vue.includes('grid-template-columns:var(--rebased-left-width) minmax(390px,1fr) var(--rebased-right-width)') && source.vue.includes('grid-template-columns:var(--rebased-left-width) minmax(390px,1fr);')],
  ['rebased side panels can be resized', source.vue.includes("beginRebasedResize('left',$event)") && source.vue.includes("beginRebasedResize('right',$event)")],
  ['rebased log exposes source-backed filters', source.vue.includes('Branch: {{revision') && source.vue.includes('v-for="name in historyAuthors"') && source.vue.includes('v-for="date in historyDates"') && source.vue.includes('placeholder="Paths…"')],
  ['rebased compact appbar matches the product hierarchy', source.vue.includes('class="rebased-appbar"') && source.vue.includes('Rebased Git client') && source.vue.includes('aria-label="切换仓库"')],
  ['rebased toolbar keeps secondary workspaces reachable', source.vue.includes('class="rebased-more-workspaces"') && source.vue.includes('aria-label="更多 Git 工作区"')],
  ['rebased history owns its dark product surface', source.vue.includes("'rebased-product-theme':pane==='log'&&rebasedLayout") && source.vue.includes('.rebased-product-theme{--git-bg:#1b1e23')],
  ['rebased branch strip remains visible beside log', source.vue.includes('class="rebased-branches-strip"') && source.vue.includes('› Branches')],
  ['rebased branch menu keeps write actions reachable', source.vue.includes('class="history-branch-actions"') && source.vue.includes("prepare('checkout')") && source.vue.includes("prepare('merge')") && source.vue.includes("prepare('rebase')") && source.vue.includes("prepare('branch_delete')")],
  ['rebased inspector exposes real file actions', source.vue.includes('class="rebased-inspector-tools"') && source.vue.includes("prepareCommitAction('revert',selectedCommit)") && source.vue.includes("historicalDiffMode='split'" )],
  ['settings applies theme without reload', source.settings.includes('document.documentElement.dataset.theme = theme.value') && source.settings.includes('function setTheme')],
];
const failed = checks.filter(([, ok]) => !ok);
if (failed.length) {
  failed.forEach(([label]) => console.error(`[FAIL] ${label}`));
  process.exit(1);
}
console.log(`[PASS] rebased adapter alignment checks=${checks.length}`);
