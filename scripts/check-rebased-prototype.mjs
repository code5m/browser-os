import fs from 'node:fs';

const path = 'docs/design/rebased-screenshot-replica.html';
const source = fs.readFileSync(path, 'utf8');
const contractPath = 'docs/design/rebased-git-behavior-contract.md';
if (!fs.existsSync(contractPath)) {
  console.error('[FAIL] Git behavior contract is missing');
  process.exit(1);
}
const contract = fs.readFileSync(contractPath, 'utf8');
const adapterContractPath = 'docs/design/rebased-git-adapter-contract.md';
if (!fs.existsSync(adapterContractPath)) {
  console.error('[FAIL] Git adapter contract is missing');
  process.exit(1);
}
const adapterContract = fs.readFileSync(adapterContractPath, 'utf8');
for (const marker of ['Vcs.UpdateProject', 'Git.Fetch', 'CheckinProject', 'OperationPhase', 'GitAdapterEvent']) {
  if (!adapterContract.includes(marker)) {
    console.error(`[FAIL] Git adapter contract marker missing: ${marker}`);
    process.exit(1);
  }
}
const historyBindings = source.match(/document\.querySelector\('#historyMenu'\)\.onclick/g) || [];
if (historyBindings.length !== 1) {
  console.error(`[FAIL] history menu binding count=${historyBindings.length}, expected=1`);
  process.exit(1);
}
for (const marker of ['CommitSnapshot', 'FileChange', 'NO_PROJECT', 'hook']) {
  if (!contract.includes(marker)) {
    console.error(`[FAIL] Git behavior contract marker missing: ${marker}`);
    process.exit(1);
  }
}
const required = [
  ['independent Update action', 'id="updateProject"'],
  ['independent Push action', 'id="pushProject"'],
  ['file History action', 'History'],
  ['Stage mode', 'id="stageMode"'],
  ['Amend mode', 'id="amendMode"'],
  ['Commit checks', 'id="commitChecks"'],
  ['branch state machine', 'const branchStates='],
  ['merge parent selector', 'Diff parent'],
  ['A/D/R file statuses', "fileStatuses="],
  ['settings state', 'const settingsState='],
  ['editable diff preview', '编辑当前版本'],
  ['remote retry/error actions', 'View last error'],
  ['branch accessibility state', 'aria-busy'],
  ['branch accessibility label', 'Git branch state:'],
  ['workspace refresh after commit', 'refreshWorkspaceAfterCommit'],
  ['remote operation phases', 'dataset.remotePhase'],
  ['remote result model', 'dataset.remoteResult'],
  ['commit snapshot source', 'commitSnapshots'],
  ['commit result model', 'dataset.commitResult'],
  ['read-only state snapshot', '__rebasedPrototype'],
  ['file history to diff', "查看 '+name+' 在此提交中的 Diff"],
  ['branch state preview', 'setBranchState'],
  ['commit compare view', 'openCommitCompare'],
  ['commit action confirmation', 'openCommitActionPreview'],
  ['push preview view', 'openPushPreview'],
  ['update preview view', 'openUpdatePreview'],
  ['fetch preview view', 'openFetchPreview'],
  ['branch action confirmation', 'openBranchActionPreview'],
  ['project state confirmation', 'openProjectActionPreview'],
  ['repository state model', 'dataset.repositoryState'],
  ['commit mode state', 'dataset.commitMode'],
  ['commit state snapshot fields', 'stagedFiles:'],
  ['shelf diff preview', 'openShelfDiff'],
  ['shelf count refresh', 'refreshShelfCount']
  ,['multi-root selection', 'selectedRoot']
  ,['staged changes region', 'id="stagedGroup"']
  ,['staged count synchronization', 'updateStagePreview']
  ,['file history tab action', 'Open File History in New Tab']
  ,['file history tab creation', 'id=\'fileHistoryTab\'']
  ,['history context state', 'historyContext']
  ,['source action contract', 'gitActionContracts']
  ,['three-way merge preview', "dataset.layout = mergePreview ? 'three-way'"]
  ,['branch filter data binding', "activeBranch='master'"]
  ,['action availability snapshot', 'actionAvailability:{']
  ,['file change metadata rendering', "file.oldPath?' · old: '+file.oldPath"]
  ,['multi-root remote actions', "!['NO_PROJECT','LOADING','NO_VCS','UNTRUSTED'].includes(document.body.dataset.repositoryState)"]
  ,['workflow action entry', 'id=\'workflowMenu\'']
  ,['persistent remote result surface', "result.id='remoteState'"]
  ,['settings state projection', 'dataset.outsideIdea=String(settingsState.outsideIdea)']
  ,['workflow state visibility', "workflow.hidden=['NO_PROJECT','LOADING','NO_VCS','UNTRUSTED'].includes(state)"]
  ,['refresh log data action', '当前筛选和提交选择已保留']
  ,['menu home end navigation', "['Home','End'].includes(event.key)"]
  ,['remote status projection', "remoteMatch&&typeof setRemoteResult==='function'"]
  ,['special file edit guard', "fileKind?.startsWith('submodule')"]
  ,['workflow visibility snapshot', 'workflowVisible:!']
  ,['workflow preview state', "dataset.workflowResult='preview'"]
  ,['workflow strategy control', "action+' strategy'"]
  ,['workflow target control', "action+' target branch'"]
  ,['conflict editor workflow state', "dataset.workflowResult='conflict-editor'"]
  ,['conflict file context', 'workflowConflictFile=conflictFile']
  ,['log location projection', "historyPanel.dataset.logLocation=settingsState.logLocation"]
];

const failures = required.filter(([, marker]) => !source.includes(marker));
if (failures.length) {
  console.error('[FAIL] Rebased prototype markers missing:');
  failures.forEach(([name]) => console.error(`- ${name}`));
  process.exit(1);
}

const script = source.match(/<script>([\s\S]*?)<\/script>/)?.[1];
if (!script) throw new Error('prototype script not found');
new Function(script);
console.log(`[PASS] Rebased prototype markers=${required.length} behavior-contract markers=4 script=valid`);
