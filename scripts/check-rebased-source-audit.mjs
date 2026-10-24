import fs from 'node:fs';

const files = {
  vue: 'src/capabilities/git/ui/GitStudio.vue',
  tree: 'src/capabilities/git/ui/GitCommitFileTree.vue',
  preferences: 'src/capabilities/git/model/gitPreferences.ts',
  bridge: 'src/bridge.ts',
  rust: 'src-tauri/src/capabilities/git/studio.rs',
  prototype: 'docs/design/rebased-screenshot-replica.html',
  matrix: 'docs/design/rebased-visual-migration-matrix.md',
  fixtureHtml: 'tests/visual/git-studio-rebased.html',
  fixtureTs: 'tests/visual/git-studio-rebased.ts',
};

const source = Object.fromEntries(Object.entries(files).map(([name, path]) => {
  if (!fs.existsSync(path)) {
    console.error(`[FAIL] missing ${path}`);
    process.exit(1);
  }
  return [name, fs.readFileSync(path, 'utf8')];
}));

const checks = [
  ['real history pane', source.vue.includes("pane==='log'")],
  ['commit inspector', source.vue.includes('提交详情、变更文件及差异预览')],
  ['commit files load', source.vue.includes("action:'commit_files'")],
  ['commit file diff load', source.vue.includes("'commit_file_diff'") && source.vue.includes("'commit_file_worktree_diff'")],
  ['rename file history is preserved', source.rust.includes('previous_path') && source.rust.includes("status.starts_with('R')") && source.tree.includes('file.previous_path')],
  ['commit file status summary', source.vue.includes('commitFileStats') && source.vue.includes('新增 {{commitFileStats.added}}') && source.vue.includes('重命名 {{commitFileStats.renamed}}')],
  ['historical editor collapse', source.vue.includes('historyEditorCollapsed')],
  ['historical change-only toggle', source.vue.includes('historyOnlyChanges')],
  ['history width resize', source.vue.includes("beginHistoryResize('inspector'")],
  ['history bottom resize', source.vue.includes("beginHistoryResize('bottom'")],
  ['three-way conflict editor', source.vue.includes("diffMode==='conflict'") && source.vue.includes('冲突编辑区')],
  ['native conflict hunk metadata', source.rust.includes('"hunks":hunks') && source.vue.includes('Array.isArray(conflict.value.hunks)')],
  ['conflict navigation', source.vue.includes('moveConflict') && source.vue.includes('上一个冲突') && source.vue.includes('下一个冲突')],
  ['conflict line apply', source.vue.includes('applyConflictLine') && source.vue.includes('当前') && source.vue.includes('目标')],
  ['preview confirm boundary', source.vue.includes('gitStudioPreview') && source.vue.includes('gitStudioConfirm')],
  ['source bridge read', source.bridge.includes('gitStudioRead')],
  ['source bridge preview', source.bridge.includes('gitStudioPreview')],
  ['source bridge confirm', source.bridge.includes('gitStudioConfirm')],
  ['repository state projection', source.vue.includes('repositoryState') && source.vue.includes('repositoryStateLabel')],
  ['repository mode projection', source.vue.includes('repositoryMode') && source.vue.includes('MULTI_ROOT') && source.vue.includes('AHEAD_BEHIND') && source.vue.includes('data-repository-mode')],
  ['history context projection', source.vue.includes('historyContext') && source.vue.includes("'FILE_HISTORY'") && source.vue.includes("'DIFF'") && source.vue.includes('data-history-context')],
  ['incoming outgoing projection', source.vue.includes('branchTrackingLabel') && source.vue.includes('待推送') && source.vue.includes('待更新')],
  ['file history action', source.vue.includes("inspectFile('follow')") && source.vue.includes('Vcs.ShowTabbedFileHistory')],
  ['history exposes real path filter', source.vue.includes('pathFilter') && source.vue.includes('按路径筛选历史') && source.vue.includes('const paths=pathFilter.value.trim()?')],
  ['history filters can be cleared', source.vue.includes('clearAllHistoryFilters') && source.vue.includes('>清空</button>')],
  ['native preview command', source.rust.includes('pub async fn git_studio_preview')],
  ['native confirm command', source.rust.includes('pub async fn git_studio_confirm')],
  ['prototype is visual baseline', source.matrix.includes('唯一视觉基准') && source.prototype.includes('Rebased Git client')],
  ['migration file history action is exact', source.matrix.includes("inspectFile('follow')") && source.matrix.includes('fileHistoryContext')],
  ['migration rule keeps real behavior authoritative', source.matrix.includes('真实 GitStudio 的行为和 Adapter 是权威')],
  ['prototype separates Update and Push', source.prototype.includes('id="updateProject"') && source.prototype.includes('id="pushProject"') && source.prototype.includes("Vcs.UpdateProject") && source.prototype.includes("Vcs.Push")],
  ['prototype exposes Fetch path', source.prototype.includes('openFetchPreview') && source.prototype.includes("['Fetch'")],
  ['prototype Update strategy aligns with adapter strategies', source.prototype.includes('Fast-forward only（默认安全策略）') && source.prototype.includes("['Merge','Merge']") && source.prototype.includes("['Rebase','Rebase']")],
  ['prototype commit validation', source.prototype.includes('validateCommit') && source.prototype.includes(".message textarea').value.trim()") && source.prototype.includes('stageInput') && source.prototype.includes('amendInput')],
  ['prototype commit file linkage', source.prototype.includes('updateCommitFiles') && source.prototype.includes('selectedRows') && source.prototype.includes('openFile')],
  ['prototype commit context actions', source.prototype.includes('oncontextmenu') && source.prototype.includes('Cherry-pick') && source.prototype.includes('Create tag at commit')],
  ['prototype menu focus and escape', source.prototype.includes('closeDemoMenu(true)') && source.prototype.includes("event.key==='Escape'") && source.prototype.includes('getBoundingClientRect')],
  ['prototype branch state matrix', source.prototype.includes('No project') && source.prototype.includes('No VCS') && source.prototype.includes('Untrusted project') && source.prototype.includes('2 Git roots')],
  ['prototype file history semantics', source.prototype.includes('File History') && source.prototype.includes('Vcs.ShowTabbedFileHistory') && source.prototype.includes('需先选择文件')],
  ['real commit context actions', source.vue.includes('openCommitActionMenu') && source.vue.includes('commit-action-menu') && source.vue.includes('prepareCommitAction')],
  ['real commit multi-select compare', source.vue.includes('selectedCommitOids') && source.vue.includes('handleCommitSelection') && source.vue.includes('compareSelectedCommits')],
  ['real branch context actions', source.vue.includes('openBranchActionMenu') && source.vue.includes('branch-action-menu') && source.vue.includes('prepareBranchAction')],
  ['real file history context', source.vue.includes('fileHistoryContext') && source.vue.includes('returnToRepositoryHistory') && source.vue.includes('Vcs.ShowTabbedFileHistory') && source.vue.includes("pathFilter.value=kind==='follow'?activeFile.value:''") && source.vue.includes('Boolean(fileHistoryContext)')],
  ['real menu keyboard focus', source.vue.includes('querySelector(\'.commit-action-menu button\')') && source.vue.includes("event.key === 'ArrowDown'") && source.vue.includes("event.key === 'Escape'") && source.vue.includes("event.key === 'Home'") && source.vue.includes("event.key === 'End'") && source.vue.includes('menuTrigger') && source.vue.includes('trigger?.focus()')],
  ['git menus stay inside layout', source.vue.includes('menuPoint(event)') && !source.vue.includes('.commit-action-menu{position:fixed') && !source.vue.includes('.branch-action-menu{position:fixed')],
  ['branch create target guard', source.vue.includes("action === 'branch_create' && !branchTarget.value.trim()") && source.vue.includes('target: action === \'branch_create\' ? branchTarget.value.trim()') && source.vue.includes('新建目标分支（当前 HEAD）')],
  ['real Update exposes explicit strategy selection', source.vue.includes("updateStrategy") && source.vue.includes("Fast-forward only") && source.vue.includes("Merge") && source.vue.includes("Rebase")],
  ['Update strategy is enforced by native adapter', source.rust.includes('GIT_STUDIO_UPDATE_STRATEGY_INVALID') && source.rust.includes('"Merge" => "--no-rebase"') && source.rust.includes('"Rebase" => "--rebase"')],
  ['Git product settings are persisted and applied', source.preferences.includes('browseros.git-studio.preferences') && source.preferences.includes('localStorage') && source.vue.includes('logPlacement') && source.vue.includes('writeIdeaMetadata') && source.vue.includes('indexEnabled') && source.vue.includes('data-log-placement') && source.vue.includes('data-log-placement=toolwindow')],
  ['remote result keeps phase and message', source.vue.includes('remoteOperation') && source.vue.includes("phase:'checking'") && source.vue.includes("phase:'executing'") && source.vue.includes("phase:'cancelled'") && source.vue.includes('远程操作完成并已刷新分支状态') && source.vue.includes('预览已取消，工作区和当前选择未改变') && source.vue.includes('data-phase')],
  ['push preview exposes empty and pending states', source.vue.includes('outgoingCount') && source.vue.includes('没有待推送提交') && source.vue.includes('当前分支有 {{outgoingCount}} 个待推送提交')],
  ['real commit requires staged selection', source.vue.includes('selectedStaged') && source.vue.includes('selectedUnstaged') && source.vue.includes('提交仅接受已暂存文件')],
  ['real Rebased commit form exposes amend and amend-push', source.vue.includes('rebasedAmend') && source.vue.includes('Amend last commit') && source.vue.includes("prepareCommitAndPush(rebasedAmend?'amend':'commit')") && source.rust.includes('args.push("--amend".into())')],
  ['real Rebased toolbar moves changes through preview contract', source.vue.includes('移动选中改动到暂存区或工作区') && source.vue.includes("prepare(selectedUnstaged.length?'stage':'unstage')")],
  ['real Rebased toolbar selects the opened file', source.vue.includes('选中已打开文件') && source.vue.includes("selected.push(rebasedWorktreePath)")],
  ['real Rebased change groups expand and collapse', source.vue.includes('rebasedChangesExpanded=true;rebasedUnversionedExpanded=true') && source.vue.includes('rebasedChangesExpanded=false;rebasedUnversionedExpanded=false') && source.vue.includes(':open="rebasedChangesExpanded"') && source.vue.includes(':open="rebasedUnversionedExpanded"')],
  ['real Rebased appbar focuses history search', source.vue.includes('focusHistorySearch') && source.vue.includes('historySearchInput.value?.focus()') && source.vue.includes('aria-label="搜索提交历史"') && source.vue.includes('ref="historySearchInput"')],
  ['visual fixture mounts the production GitStudio component', source.fixtureTs.includes("import GitStudio from '../../src/capabilities/git/ui/GitStudio.vue'") && source.fixtureTs.includes('createApp(GitStudio') && source.fixtureHtml.includes('./git-studio-rebased.ts')],
  ['real recursive file tree owns dark-theme-safe button layout', source.tree.includes('background: transparent') && source.tree.includes('var(--git-selected') && source.tree.includes('width: 100%')],
  ['Branches entry does not cover the historical editor', source.vue.includes(':has(.history-editor-pane .history-editor) .rebased-branches-strip{display:none}')],
  ['Selecting a history file reopens the editor', source.vue.includes('historyEditorCollapsed.value=false')],
  ['Selecting a history file opens focused diff mode', source.vue.includes("historicalDiffMode.value='split'") && source.vue.includes('historyOnlyChanges.value=true')],
  ['real commit footer exposes selection state', source.vue.includes('已暂存 {{selectedStaged.length}}') && source.vue.includes('未暂存 {{selectedUnstaged.length}}')],
  ['merge parent selection and root commit boundary are explicit', source.vue.includes('选择父提交后，变更文件和 Diff 会按该父提交重新计算') && source.vue.includes('selectedParent') && source.vue.includes('这是初始提交，没有父提交') && source.rust.includes('GIT_STUDIO_PARENT_NOT_FOUND')],
  ['branch actions respect local and protected refs', source.vue.includes('canCheckoutBranch') && source.vue.includes('canDeleteBranch') && source.vue.includes('只能切换本地分支')],
  ['commit and push keeps two-step confirmation', source.vue.includes('prepareCommitAndPush') && source.vue.includes('shouldPushAfterCommit') && source.vue.includes("await prepare('push')")],
  ['branch toolbar shares action guards', source.vue.includes('canRunBranchAction') && source.vue.includes("@click=\"prepare(op)\"") && source.vue.includes('仅允许删除非当前、非保护的本地分支')],
  ['repository state matrix is visible', ['NO_PROJECT','LOADING','NO_VCS','INVALID_PATH','READY','UNAVAILABLE'].every(state => source.vue.includes(`${state}:`)) && source.vue.includes('state-badge')],
  ['repository trust state is visible', source.vue.includes("repositoryState.value === 'UNTRUSTED'") && source.vue.includes('项目未受信任') && source.vue.includes('repository-state-panel')],
  ['no VCS offers preview-gated repository initialization', source.vue.includes("prepare('init')") && source.vue.includes('创建 Git 仓库') && source.rust.includes('GIT_STUDIO_REPOSITORY_ALREADY_EXISTS') && source.rust.includes('"init"')],
  ['branch widget opens repository history context', source.vue.includes('branch-widget') && source.vue.includes('打开 Git 分支菜单') && source.vue.includes('branchWidgetOpen') && source.vue.includes('查看分支与提交历史')],
  ['cancelled commit push clears follow-up state', source.vue.includes('function cancelPreview') && source.vue.includes('pushAfterCommit.value=false')],
  ['repository state is announced accessibly', source.vue.includes('class="state-badge" role="status" aria-live="polite"')],
  ['no-project hides git workspace controls', source.vue.includes("(info||repositoryState==='LOADING')&&!(pane==='log'&&rebasedLayout)") && source.vue.includes(':disabled="writeBusy||!info"')],
  ['history reload clears stale commit context', source.vue.includes("logSelection.value='';commitFiles.value=[];commitFileFilter.value='';commitFilePath.value='';rawDetail.value='';historicalContent.value='';") && source.vue.includes('detailGeneration++')],
  ['tag creation uses preview contract', source.vue.includes("prepareCommitAction('tag_create'") && source.vue.includes('tagTarget') && source.rust.includes('"tag_create"')],
  ['revert rejects merge ambiguity and uses preview contract', source.vue.includes("prepareCommitAction('revert'") && source.rust.includes('"revert"') && source.rust.includes('GIT_STUDIO_REVERT_MERGE_PARENT_REQUIRED')],
  ['reset is explicit and preview-gated', source.vue.includes("prepareReset") && source.vue.includes('resetMode') && source.rust.includes('"reset"') && source.rust.includes('GIT_STUDIO_HARD_RESET_ACK_REQUIRED')],
];

const failed = checks.filter(([, ok]) => !ok);
if (failed.length) {
  failed.forEach(([label]) => console.error(`[FAIL] ${label}`));
  process.exit(1);
}

console.log(`[PASS] rebased source audit checks=${checks.length}`);
