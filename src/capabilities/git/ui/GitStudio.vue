<script setup lang="ts">
import { computed, nextTick, ref, watch, onUnmounted } from 'vue';
import { bridge } from '../../../bridge';
import { redactSecrets } from '../../../utils/redact';
import { useRepoStore } from '../../workspace/public';
import { useGitStore } from '../public';
import { gitLifecycleSnapshot, registerGitLifecycleBinding } from '../lifecycle';
import { layoutCommitGraph } from '../model/logGraph';
import { parseUnifiedPatch } from '../model/patchView';
import { readGitPreferences, writeGitPreferences } from '../model/gitPreferences';
import GitCommitFileTree from './GitCommitFileTree.vue';
import GitHistoryDiff from './GitHistoryDiff.vue';
import { patchHunks, applyPatchSelection, blameLines, sourceIdentity, safeGithubLink } from '../model/studioPatch.mjs';

const props = defineProps<{path?:string;mode?:string;initialAction?:string;closeLabel?:string;searchId?:string}>();
const emit = defineEmits<{close:[];configure:[]}>();
const repos = useRepoStore();
const git = useGitStore();
const localPath = ref(props.path || '');
const enteredPath = ref(props.path || '');
const source = computed<Record<string,unknown>|null>(() => localPath.value ? {path:localPath.value} : git.repoId ? {repo_id:git.repoId} : null);
const info = ref<any>(null), error = ref(''), errorCode = ref(''), notice = ref('');
const remoteOperation = ref<{action:string;phase:'idle'|'checking'|'executing'|'success'|'error'|'cancelled';message:string}>({action:'',phase:'idle',message:''});
const loading = ref(false), writeBusy = ref(false), preview = ref<any>(null), acknowledged = ref(false);
const pushAfterCommit = ref(false);
const selected = ref<string[]>([]), activeFile = ref(''), staged = ref(false);
function defaultPane() { return props.mode === 'history' ? 'log' : 'changes'; }
const pane = ref(defaultPane());
const panes = [['changes','改动'],['log','分支与历史'],['stash','暂存架'],['worktrees','工作树'],['github','PR / CI']];
const patch = ref(''), file = ref<any>(null), conflict = ref<any>(null), resolution = ref('');
const conflictLine = ref(0);
const diffMode = ref('split'), selectedLines = ref<number[]>([]);
const commits = ref<any[]>([]), offset = ref(0), hasMore = ref(false), search = ref(''), author = ref(''), pathFilter = ref(''), revision = ref('');
const dateFilter = ref('');
const logSelection = ref(''), blame = ref<any[]>([]), rawDetail = ref('');
const commitActionMenu = ref<{commit:any;x:number;y:number}|null>(null);
const menuTrigger = ref<HTMLElement|null>(null);
const selectedCommitOids = ref<string[]>([]);
const branchActionMenu = ref<{name:string;x:number;y:number}|null>(null);
const branchWidgetOpen = ref(false);
const historyBranchFilterOpen = ref(false);
const commitFiles = ref<{path:string;status:string;previous_path?:string}[]>([]);
const selectedParent = ref('');
const commitFileFilter = ref('');
const commitFilePath = ref('');
const historicalContent = ref('');
const historicalCompareLocal = ref(false);
const fileHistoryContext = ref('');
const historicalContentLoading = ref(false);
const historicalDiffMode = ref<'file'|'split'|'patch'>('file');
const historyEditorCollapsed = ref(false);
const historyOnlyChanges = ref(true);
const historyInspectorWidth = ref(330);
const historyBottomHeight = ref(260);
const rebasedLeftWidth = ref(300);
const rebasedRightWidth = ref(350);
const rebasedLayout = ref(true);
const rebasedShelf = ref(false);
const rebasedAmend = ref(false);
const rebasedChangesExpanded = ref(true);
const rebasedUnversionedExpanded = ref(false);
const historySearchInput = ref<HTMLInputElement|null>(null);
const historicalDiff = computed(() => parseUnifiedPatch(rawDetail.value));
const conflictRows = computed(() => {
  if (!conflict.value) return [];
  if (Array.isArray(conflict.value.hunks)) return conflict.value.hunks.slice(0, 200);
  const ours = String(conflict.value.ours || '').split('\n');
  const theirs = String(conflict.value.theirs || '').split('\n');
  const size = Math.max(ours.length, theirs.length);
  return Array.from({length: size}, (_, index) => ({index, ours: ours[index] ?? '', theirs: theirs[index] ?? ''}))
    .filter(row => row.ours !== row.theirs).slice(0, 200);
});
const historyLayoutStyle = computed(() => ({
  '--git-history-inspector-width': `${historyInspectorWidth.value}px`,
  '--git-history-bottom-height': `${historyBottomHeight.value}px`,
  '--rebased-left-width': `${rebasedLeftWidth.value}px`,
  '--rebased-right-width': `${rebasedRightWidth.value}px`,
}));
const selectedCommit = computed(() => commits.value.find(commit => commit.oid === logSelection.value));
const historyAuthors = computed(() => [...new Set(commits.value.map(commit => String(commit.author || '')).filter(Boolean))].sort());
const historyDates = computed(() => [...new Set(commits.value.map(commit => new Date(commit.time * 1000).toLocaleDateString()).filter(Boolean))]);
const visibleCommits = computed(() => dateFilter.value ? commits.value.filter(commit => new Date(commit.time * 1000).toLocaleDateString() === dateFilter.value) : commits.value);
function graphIndex(oid:string) { return Math.max(0,commits.value.findIndex(commit => commit.oid === oid)); }
const visibleCommitFiles = computed(() => {
  const needle = commitFileFilter.value.trim().toLowerCase();
  if (!needle) return commitFiles.value;
  return commitFiles.value.filter(item => `${item.status} ${item.path}`.toLowerCase().includes(needle));
});
const commitTree = computed(() => {
  type Node = {name:string; path:string; children:Node[]; files:typeof commitFiles.value};
  const root:Node = {name:'',path:'',children:[],files:[]};
  for (const file of visibleCommitFiles.value) {
    const parts = file.path.split('/');
    let current = root;
    for (const part of parts.slice(0,-1)) {
      let child = current.children.find(node => node.name === part);
      if (!child) { child = {name:part,path:[current.path,part].filter(Boolean).join('/'),children:[],files:[]}; current.children.push(child); }
      current = child;
    }
    current.files.push(file);
  }
  return root;
});
const commitFileStats = computed(() => commitFiles.value.reduce((stats, item) => {
  const key = item.status[0] === 'R' ? 'renamed' : item.status[0] === 'A' ? 'added' : item.status[0] === 'D' ? 'deleted' : 'modified';
  stats[key] += 1;
  return stats;
}, {added:0, modified:0, deleted:0, renamed:0}));
const branchTarget = ref(''), tagTarget = ref(''), compareLeft = ref('HEAD'), compareRight = ref(''), message = ref(''), stashTarget = ref(''), worktreeTarget = ref(''), resetMode = ref<'soft'|'mixed'|'hard'>('mixed');
const updateStrategy = ref<'Merge'|'Rebase'|'Fast-forward only'>('Fast-forward only');
const settingsOpen = ref(false);
const logPlacement = ref<'editor'|'toolwindow'>('editor');
const writeIdeaMetadata = ref(true);
const indexEnabled = ref(true);
function loadGitPreferences() {
  try {
    const saved = readGitPreferences();
    if (saved.logPlacement === 'editor' || saved.logPlacement === 'toolwindow') logPlacement.value = saved.logPlacement;
    if (typeof saved.writeIdeaMetadata === 'boolean') writeIdeaMetadata.value = saved.writeIdeaMetadata;
    if (typeof saved.indexEnabled === 'boolean') indexEnabled.value = saved.indexEnabled;
  } catch { /* Ignore malformed local preferences and keep safe defaults. */ }
}
function saveGitPreferences() {
  writeGitPreferences({logPlacement:logPlacement.value,writeIdeaMetadata:writeIdeaMetadata.value,indexEnabled:indexEnabled.value});
  notice.value = 'Git 设置已保存；不会修改当前仓库文件。';
}
const compareFiles = ref<{path:string;status:string}[]>([]);
const compareFilePath = ref('');
const remote = ref<any>(null), pr = ref<number|null>(null), prText = ref(''), prDetail = ref<any>(null);
const stashes = ref<string[][]>([]), worktrees = ref('');
let generation = 0, detailGeneration = 0, disposed = false, requestCount = 0;
let previewSource = '';
const blocked = computed(() => writeBusy.value || loading.value);
const repositoryState = computed(() => {
  if (!source.value) return 'NO_PROJECT';
  if (loading.value && !info.value) return 'LOADING';
  if (errorCode.value === 'GIT_STUDIO_REPOSITORY_MISSING') return 'NO_VCS';
  if (errorCode.value === 'GIT_STUDIO_PATH_INVALID' || errorCode.value === 'GIT_STUDIO_ROOT_MISSING') return 'INVALID_PATH';
  if (errorCode.value === 'GIT_STUDIO_UNTRUSTED_PROJECT' || info.value?.trusted === false || info.value?.trust === false) return 'UNTRUSTED';
  return info.value ? 'READY' : 'UNAVAILABLE';
});
const repositoryStateLabel:Record<string,string> = {NO_PROJECT:'无项目',LOADING:'正在加载 Git',NO_VCS:'未发现 Git 仓库',INVALID_PATH:'路径不可用',UNTRUSTED:'项目未受信任',READY:'Git 已加载',UNAVAILABLE:'Git 不可用'};
const repositoryMode = computed(() => {
  if (repositoryState.value === 'NO_PROJECT') return 'NO_PROJECT';
  if (repositoryState.value === 'LOADING') return 'LOADING';
  if (repositoryState.value === 'NO_VCS') return 'NO_VCS';
  if (repositoryState.value === 'INVALID_PATH') return 'INVALID_PATH';
  if (repositoryState.value === 'UNTRUSTED') return 'UNTRUSTED';
  if (repositoryState.value === 'UNAVAILABLE') return 'UNAVAILABLE';
  const roots = Array.isArray(info.value?.roots) ? info.value.roots : [];
  if (roots.length > 1) return 'MULTI_ROOT';
  const tracking = branchTracking(info.value?.branch || '');
  if (/ahead\s+\d+|behind\s+\d+/i.test(tracking)) return 'AHEAD_BEHIND';
  return 'SINGLE_ROOT';
});
const repositoryModeLabel:Record<string,string> = {NO_PROJECT:'无项目',LOADING:'正在加载 Git',NO_VCS:'未发现 Git 仓库',INVALID_PATH:'路径不可用',UNTRUSTED:'项目未受信任',UNAVAILABLE:'Git 不可用',MULTI_ROOT:'多仓库根目录',AHEAD_BEHIND:'分支有待同步提交',SINGLE_ROOT:'单仓库'};
const historyContext = computed(() => fileHistoryContext.value ? 'FILE_HISTORY' : pane.value === 'log' ? 'LOG' : activeFile.value && ['split','patch','full','partial','blame','conflict'].includes(diffMode.value) ? 'DIFF' : 'CHANGES');
const outgoingCount = computed(() => Number(branchTracking(info.value?.branch || '').match(/ahead\s+(\d+)/i)?.[1] || 0));
const incomingCount = computed(() => Number(branchTracking(info.value?.branch || '').match(/behind\s+(\d+)/i)?.[1] || 0));
const groups = computed(() => [
  {id:'conflict',name:'冲突',files:(info.value?.files||[]).filter((f:any)=>f.conflicted)},
  {id:'index',name:'已暂存（Index）',files:(info.value?.files||[]).filter((f:any)=>f.staged&&!f.conflicted)},
  {id:'worktree',name:'未暂存（Working Tree）',files:(info.value?.files||[]).filter((f:any)=>f.unstaged&&!f.conflicted)},
].filter(group=>group.files.length));
const selectedEntries = computed(() => (info.value?.files || []).filter((entry:any) => selected.value.includes(entry.path)));
const selectedUnstaged = computed(() => selectedEntries.value.filter((entry:any) => entry.unstaged && !entry.staged));
const selectedStaged = computed(() => selectedEntries.value.filter((entry:any) => entry.staged && !entry.conflicted));
const rebasedChanges = computed(() => (info.value?.files || []).filter((entry:any) => entry.index !== '?'));
const rebasedUnversioned = computed(() => (info.value?.files || []).filter((entry:any) => entry.index === '?'));
const rebasedWorktreePath = ref('');
async function focusHistorySearch() {
  await nextTick();
  historySearchInput.value?.focus();
  historySearchInput.value?.select();
}
// Branches are the real local refs and remote-tracking refs from native git
// for-each-ref. Never infer remote branch names by querying GitHub instead.
const branchQuery = ref('');
const localRefs = computed<string[]>(() => (info.value?.branches || [])
  .filter((row:string[]) => row[3]?.startsWith('refs/heads/'))
  .map((row:string[]) => row[0]).filter(Boolean));
const remoteRefs = computed<string[]>(() => (info.value?.branches || [])
  .filter((row:string[]) => row[3]?.startsWith('refs/remotes/') && !row[3].endsWith('/HEAD'))
  .map((row:string[]) => row[0]).filter(Boolean));
function branchTracking(name:string) {
  const row = (info.value?.branches || []).find((item:string[]) => item[0] === name);
  return row?.[2] || '';
}
function branchTrackingLabel(name:string) {
  const track = branchTracking(name);
  if (!track) return '';
  const ahead = Number(track.match(/ahead\s+(\d+)/i)?.[1] || 0);
  const behind = Number(track.match(/behind\s+(\d+)/i)?.[1] || 0);
  if (ahead && behind) return `↑ ${ahead} · ↓ ${behind}`;
  if (ahead) return `↑ ${ahead} 待推送`;
  if (behind) return `↓ ${behind} 待更新`;
  return '已同步';
}
const visibleLocalRefs = computed(() => localRefs.value.filter(name => name.toLowerCase().includes(branchQuery.value.toLowerCase())));
const visibleRemoteRefs = computed(() => remoteRefs.value.filter(name => name.toLowerCase().includes(branchQuery.value.toLowerCase())));
function isLocalBranch(name:string) { return localRefs.value.includes(name); }
function canCheckoutBranch(name:string) { return isLocalBranch(name) && name !== info.value?.branch; }
function canDeleteBranch(name:string) { return isLocalBranch(name) && name !== info.value?.branch && name !== 'master' && name !== 'main'; }
function canRunBranchAction(action:string) {
  const target = branchTarget.value.trim();
  if (!target) return false;
  if (action === 'checkout') return canCheckoutBranch(target);
  if (action === 'branch_delete') return canDeleteBranch(target);
  return true;
}
const refGroups = computed(() => [
  {id:'local',title:'本地',list:visibleLocalRefs.value,total:localRefs.value.length},
  {id:'remote',title:'远程',list:visibleRemoteRefs.value,total:remoteRefs.value.length},
]);
const graph = computed(() => layoutCommitGraph(commits.value));
const parsed = computed(() => parseUnifiedPatch(patch.value));
const hunks = computed(() => { try { return patchHunks(patch.value); } catch { return []; } });
const scope = computed(() => props.path && info.value?.root && props.path.startsWith(info.value.root + '/') ? props.path.slice(info.value.root.length+1) : '');
const labels:Record<string,string> = {init:'创建 Git 仓库',stage:'暂存',unstage:'取消暂存',discard:'丢弃未暂存改动',rollback:'Rollback',commit:'提交',amend:'修改未发布提交',index_edit:'应用选中的变更',branch_create:'新建分支',branch_delete:'删除已合并分支',checkout:'切换分支',tag_create:'创建标签',revert:'撤销此提交',reset:'Reset 到此提交',fetch:'获取远端更新',update:'Update',push:'推送当前分支',merge:'合并分支',rebase:'变基未发布提交',cherry_pick:'挑选提交',stash_save:'保存全部改动到暂存架',stash_apply:'应用暂存',stash_pop:'应用并移除暂存',stash_drop:'删除暂存',resolve:'保存冲突解决结果',worktree_add:'新建工作树',worktree_remove:'移除干净工作树',pr_create:'创建 PR',pr_comment:'评论 PR',pr_approve:'批准 PR',pr_request_changes:'请求修改',pr_merge:'合并 PR',merge_continue:'继续合并',merge_abort:'中止合并',rebase_continue:'继续变基',rebase_abort:'中止变基',cherry_pick_continue:'继续挑选',cherry_pick_abort:'中止挑选'};
const friendly = (value:unknown) => {
  const text=redactSecrets(value).slice(0,300);
  errorCode.value=text;
  const hints:Record<string,string>={GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE:'仓库正在合并、变基或挑选提交；请使用对应的继续操作，不能部分提交。',GIT_STUDIO_CHANGED_SINCE_PREVIEW:'预览后仓库已变化，请刷新并重新预览。',GIT_STUDIO_READ_BASIS_CHANGED:'文件自读取后已变化，请重新读取，不能覆盖较新的改动。',GIT_STUDIO_CLEAN_WORKTREE_REQUIRED:'工作区有改动或未完成操作，请先提交、保存暂存架或解决冲突。',GIT_STUDIO_SELECT_STAGED_FILES:'请勾选已暂存的文件；提交不会自动包含未暂存改动。',GIT_STUDIO_SELECT_PATHS:'请明确勾选需要操作的文件。',GIT_STUDIO_PREMERGE_NOT_EXECUTED:'完整验收未实际执行，不能合并。',GIT_STUDIO_CI_NOT_PASSED:'CI 缺失、运行中或未全部通过，不能合并。',GIT_STUDIO_GITHUB_AUTH_OR_API_FAILED:'请先在系统中完成 gh 登录，并确认有仓库权限。',GIT_STUDIO_BINARY_FILE:'二进制文件不支持文本编辑。',GIT_STUDIO_REPOSITORY_BUSY:'同一仓库已有操作在运行。'};
  return hints[text] || ({GIT_STUDIO_REPOSITORY_MISSING:'当前路径不是 Git 仓库。',GIT_STUDIO_PATH_INVALID:'仓库路径无效。',GIT_STUDIO_ROOT_MISSING:'仓库目录不存在。'} as Record<string,string>)[text] || text;
};
function currentSource() {
  if (!source.value || !gitLifecycleSnapshot().active) throw new Error('请先选择已授权的仓库');
  return {...source.value};
}
async function read(input:Record<string,unknown>) {
  const value=currentSource(), key=sourceIdentity(value), gen=generation;
  requestCount++; loading.value=true;
  try {
    const result=await bridge.gitStudioRead(value,input);
    if(disposed||gen!==generation||!source.value||key!==sourceIdentity(source.value)) return null;
    return result;
  } finally { requestCount--; loading.value=requestCount>0; }
}
async function refresh() {
  if(!source.value||writeBusy.value)return;
  error.value=''; errorCode.value='';
  try {
    const next=await read({action:'status'});if(!next)return;info.value=next;
    selected.value=selected.value.filter(path=>next.files.some((f:any)=>f.path===path));
    if(scope.value && !selected.value.length && props.initialAction && props.initialAction!=='history') selected.value=next.files.filter((f:any)=>f.path===scope.value||f.path.startsWith(scope.value+'/')).map((f:any)=>f.path);
    if(pane.value==='log') await loadLog();
  } catch(e){error.value=friendly(e);}
}
function toggle(path:string){selected.value=selected.value.includes(path)?selected.value.filter(p=>p!==path):[...selected.value,path];}
async function openFile(path:string, isStaged=staged.value) {
  if(blocked.value)return;
  activeFile.value=path;staged.value=isStaged;patch.value='';file.value=null;conflict.value=null;blame.value=[];selectedLines.value=[];
  const gen=++detailGeneration;
  try {
    const [difference,content]=await Promise.all([read({action:'diff',paths:[path],staged:isStaged}),read({action:'file',paths:[path]})]);
    if(gen!==detailGeneration)return;
    if(difference)patch.value=difference.patch;if(content)file.value=content;
  }catch(e){error.value=friendly(e);}
}
async function openRebasedWorktreeFile(entry:any) {
  if (!entry?.path) return;
  rebasedWorktreePath.value=entry.path;
  await openFile(entry.path,Boolean(entry.staged));
}
async function selectPane(name:string){if(writeBusy.value)return;pane.value=name;fileHistoryContext.value='';pathFilter.value='';error.value='';rawDetail.value='';try{
  if(name==='log')await loadLog();
  if(name==='stash'){const data=await read({action:'stash'});if(data)stashes.value=data.text.trim().split('\n').filter(Boolean).map((s:string)=>s.split('\t'));}
  if(name==='worktrees'){const data=await read({action:'worktrees'});if(data)worktrees.value=data.text;}
  if(name==='github'){const data=await read({action:'github'});if(data)remote.value=data;}
}catch(e){error.value=friendly(e);}}
async function openRebasedShelf() {
  rebasedShelf.value=true;
  try { const data=await read({action:'stash'});if(data)stashes.value=data.text.trim().split('\n').filter(Boolean).map((row:string)=>row.split('\t')); }
  catch(e) { error.value=friendly(e); }
}
async function loadLog(next=offset.value){if(!source.value)return;offset.value=next;selectedCommitOids.value=[];logSelection.value='';commitFiles.value=[];commitFileFilter.value='';commitFilePath.value='';rawDetail.value='';historicalContent.value='';historicalContentLoading.value=false;selectedParent.value='';compareFiles.value=[];compareFilePath.value='';detailGeneration++;try{const paths=pathFilter.value.trim()?[pathFilter.value.trim()]:scope.value?[scope.value]:[];const data=await read({action:'log',offset:next,search:search.value||null,author:author.value||null,revision:revision.value||null,paths});if(data){commits.value=data.commits;hasMore.value=data.has_more;}}catch(e){error.value=friendly(e);}}
function clearHistoryFilters(){search.value='';author.value='';if(!fileHistoryContext.value)pathFilter.value='';void loadLog(0);}
function clearAllHistoryFilters(){search.value='';author.value='';dateFilter.value='';revision.value='';if(!fileHistoryContext.value)pathFilter.value='';void loadLog(0);}
async function selectBranchFromTree(name:string) {
  if(blocked.value)return;
  historyBranchFilterOpen.value=false;
  revision.value=name; branchTarget.value=name; pane.value='log';
  offset.value=0; logSelection.value=''; selectedCommitOids.value=[]; rawDetail.value=''; commitFiles.value=[]; commitFilePath.value=''; commits.value=[];
  await loadLog(0);
}
function toggleBranchWidget() {
  if (blocked.value || !info.value) return;
  branchWidgetOpen.value = !branchWidgetOpen.value;
  if (branchWidgetOpen.value) pane.value = 'log';
}
async function selectCommit(oid:string, compareLocal=false, preserveSelection=false) {
  if (blocked.value) return;
  rebasedWorktreePath.value='';
  if (!preserveSelection) selectedCommitOids.value = [oid];
  const gen=++detailGeneration;
  logSelection.value=oid; commitFiles.value=[]; commitFileFilter.value=''; commitFilePath.value=''; rawDetail.value=''; historicalContent.value=''; selectedParent.value = preserveSelection && selectedParent.value && selectedCommit.value?.parents?.includes(selectedParent.value) ? selectedParent.value : (selectedCommit.value?.parents?.[0] || '');
  try {
    const result=await read({action:'commit_files',revision:oid,parent:selectedParent.value||undefined});
    if (gen!==detailGeneration || logSelection.value!==oid) return;
    if(result) {
      commitFiles.value=result.files||[];
      selectedParent.value = result.selected_parent || selectedParent.value;
      if (commitFiles.value[0]) await selectCommitFile(commitFiles.value[0].path, compareLocal);
    }
  } catch(e) { if(gen===detailGeneration) error.value=friendly(e); }
}
function handleCommitSelection(event: MouseEvent, commit:any, index:number) {
  if (event.shiftKey && selectedCommitOids.value.length) {
    const anchor = commits.value.findIndex(item => item.oid === selectedCommitOids.value[selectedCommitOids.value.length - 1]);
    const start = Math.min(anchor < 0 ? index : anchor, index);
    const end = Math.max(anchor < 0 ? index : anchor, index);
    selectedCommitOids.value = commits.value.slice(start, end + 1).map(item => item.oid);
  } else if (event.metaKey || event.ctrlKey) {
    selectedCommitOids.value = selectedCommitOids.value.includes(commit.oid)
      ? selectedCommitOids.value.filter(oid => oid !== commit.oid)
      : [...selectedCommitOids.value, commit.oid];
  } else {
    selectedCommitOids.value = [commit.oid];
  }
  void selectCommit(commit.oid, false, true);
}
async function compareSelectedCommits() {
  if (selectedCommitOids.value.length !== 2 || blocked.value) return;
  compareLeft.value = selectedCommitOids.value[0];
  compareRight.value = selectedCommitOids.value[1];
  await loadComparison();
}
async function selectCommitFile(path:string, compareLocal=false) {
  rebasedWorktreePath.value='';
  if(!logSelection.value || blocked.value) return;
  const revision=logSelection.value, gen=++detailGeneration;
  // Selecting a file from the history tree must always reveal a fresh editor.
  // This mirrors the Rebased/IDEA interaction: a previously collapsed editor
  // must not stay hidden when the user chooses the next file, and the first
  // view should be the focused changed-region diff rather than a stale full
  // file view from the previous selection.
  commitFilePath.value=path;
  historicalCompareLocal.value=compareLocal;
  historyEditorCollapsed.value=false;
  historyOnlyChanges.value=true;
  historicalDiffMode.value='split';
  rawDetail.value=''; historicalContent.value=''; historicalContentLoading.value=true;
  try {
    const [result, content] = await Promise.all([
      read({action:compareLocal?'commit_file_worktree_diff':'commit_file_diff',revision,parent:selectedParent.value||undefined,paths:[path]}),
      compareLocal ? read({action:'file',paths:[path]}) : read({action:'commit_file_content',revision,paths:[path]}),
    ]);
    if(gen!==detailGeneration || logSelection.value!==revision || commitFilePath.value!==path) return;
    if(result) rawDetail.value=result.patch||'';
    if(content) historicalContent.value=(compareLocal ? content.worktree : content.content)||'';
  } catch(e) { if(gen===detailGeneration) error.value=friendly(e); }
  finally { if(gen===detailGeneration) historicalContentLoading.value=false; }
}
async function selectCommitParent(parent:string) {
  if (!logSelection.value || blocked.value || !parent) return;
  selectedParent.value = parent;
  const selectedPath = commitFilePath.value;
  await selectCommit(logSelection.value, false, true);
  if (selectedPath && commitFiles.value.some(item => item.path === selectedPath)) await selectCommitFile(selectedPath);
}
function beginHistoryResize(kind:'inspector'|'bottom', event:PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  const start = kind === 'inspector' ? historyInspectorWidth.value : historyBottomHeight.value;
  const position = kind === 'inspector' ? event.clientX : event.clientY;
  const onMove = (move:PointerEvent) => {
    const delta = kind === 'inspector' ? move.clientX - position : position - move.clientY;
    if (kind === 'inspector') historyInspectorWidth.value = Math.max(260, Math.min(520, start + delta));
    else historyBottomHeight.value = Math.max(180, Math.min(620, start + delta));
  };
  const onUp = () => {
    window.removeEventListener('pointermove', onMove);
    window.removeEventListener('pointerup', onUp);
  };
  window.addEventListener('pointermove', onMove);
  window.addEventListener('pointerup', onUp, {once:true});
}
function beginRebasedResize(kind:'left'|'right', event:PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  const container=(event.currentTarget as HTMLElement).closest('.history-layout') as HTMLElement|null;
  const width=container?.getBoundingClientRect().width || 1200;
  const startX=event.clientX;
  const start=kind==='left' ? rebasedLeftWidth.value : rebasedRightWidth.value;
  const onMove=(move:PointerEvent) => {
    const delta=kind==='left' ? move.clientX-startX : startX-move.clientX;
    const other=kind==='left' ? rebasedRightWidth.value : rebasedLeftWidth.value;
    const next=Math.max(240,Math.min(width-other-390,start+delta));
    if (kind==='left') rebasedLeftWidth.value=next;
    else rebasedRightWidth.value=next;
  };
  const onUp=() => { window.removeEventListener('pointermove',onMove);window.removeEventListener('pointerup',onUp); };
  window.addEventListener('pointermove',onMove);
  window.addEventListener('pointerup',onUp,{once:true});
}
function moveCommitFile(delta:number) {
  const files=visibleCommitFiles.value;
  if (!files.length) return;
  const current=Math.max(0, files.findIndex(item=>item.path===commitFilePath.value));
  const next=Math.min(files.length-1, Math.max(0, current+delta));
  if (files[next] && files[next].path!==commitFilePath.value) void selectCommitFile(files[next].path);
}
function menuPoint(event: MouseEvent) {
  const host = (event.currentTarget as HTMLElement | null)?.closest('.git-studio') as HTMLElement | null;
  const rect = host?.getBoundingClientRect();
  const width = host?.clientWidth || window.innerWidth;
  const height = host?.clientHeight || window.innerHeight;
  return {x: Math.max(8, Math.min(width - 258, event.clientX - (rect?.left || 0))), y: Math.max(8, Math.min(height - 198, event.clientY - (rect?.top || 0)))};
}
function openCommitActionMenu(event: MouseEvent, commit: any) {
  event.preventDefault();
  menuTrigger.value = event.currentTarget as HTMLElement;
  commitActionMenu.value = {commit, ...menuPoint(event)};
  void selectCommit(commit.oid);
  window.setTimeout(() => (document.querySelector('.commit-action-menu button') as HTMLButtonElement | null)?.focus(), 0);
}
function closeCommitActionMenu() { commitActionMenu.value = null; const trigger = menuTrigger.value; menuTrigger.value = null; window.setTimeout(() => trigger?.focus(), 0); }
function openBranchActionMenu(event: MouseEvent, name: string) {
  event.preventDefault();
  menuTrigger.value = event.currentTarget as HTMLElement;
  branchActionMenu.value = {name, ...menuPoint(event)};
  window.setTimeout(() => (document.querySelector('.branch-action-menu button') as HTMLButtonElement | null)?.focus(), 0);
}
function closeBranchActionMenu() { branchActionMenu.value = null; const trigger = menuTrigger.value; menuTrigger.value = null; window.setTimeout(() => trigger?.focus(), 0); }
function compareBranchToHead(name:string) {
  closeBranchActionMenu();
  compareLeft.value = 'HEAD'; compareRight.value = name;
  void loadComparison();
}
function prepareBranchAction(action:'checkout'|'branch_create'|'branch_delete'|'merge'|'rebase', name:string) {
  closeBranchActionMenu();
  if (action === 'branch_create' && !branchTarget.value.trim()) { error.value = '请先在分支面板填写新分支名'; return; }
  void prepare(action, {target: action === 'branch_create' ? branchTarget.value.trim() : name, revision: action === 'branch_create' ? name : undefined});
}
function compareCommitToLocal(commit:any) {
  closeCommitActionMenu();
  void selectCommit(commit.oid, true);
}
function prepareCommitAction(action:'cherry_pick'|'branch_create'|'tag_create'|'revert', commit:any) {
  closeCommitActionMenu();
  if (action === 'tag_create' && !tagTarget.value.trim()) { error.value = '请先填写标签名'; return; }
  void prepare(action, {revision: commit.oid, target: action === 'tag_create' ? tagTarget.value.trim() : action === 'branch_create' ? branchTarget.value || undefined : commit.oid});
}
function prepareReset(commit:any) {
  closeCommitActionMenu();
  const acknowledgement = resetMode.value === 'hard' ? 'I_UNDERSTAND_HARD_RESET' : undefined;
  void prepare('reset', {revision: commit.oid, target: commit.oid, other: resetMode.value, message: acknowledgement});
}
async function loadComparison(path=''){if(!compareLeft.value.trim()||!compareRight.value.trim())return;try{const data=await read({action:'compare',revision:compareLeft.value.trim(),other:compareRight.value.trim(),paths:path?[path]:[]});if(data){commitFilePath.value='';logSelection.value='';compareFilePath.value=path;compareFiles.value=path?compareFiles.value:(data.files||[]);rawDetail.value=data.patch;}}catch(e){error.value=friendly(e);}}
async function compare(){compareLeft.value='HEAD';compareRight.value=branchTarget.value;await loadComparison();}
async function compareCommits(){await loadComparison();}
function applyConflictLine(side:'ours'|'theirs', index:number) {
  const lines = String(resolution.value || '').split('\n');
  const value = String(conflict.value?.[side] || '').split('\n')[index] ?? '';
  while (lines.length <= index) lines.push('');
  lines[index] = value;
  resolution.value = lines.join('\n');
  conflictLine.value = Math.max(0, conflictRows.value.findIndex(row => row.index === index));
}
function moveConflict(delta:number) {
  if (!conflictRows.value.length) return;
  conflictLine.value = (conflictLine.value + delta + conflictRows.value.length) % conflictRows.value.length;
}
async function inspectFile(kind:string){if(!activeFile.value)return;try{const data=await read({action:kind,paths:[activeFile.value]});if(!data)return;if(kind==='blame'){blame.value=blameLines(data.text);diffMode.value='blame';}else if(kind==='conflict'){conflict.value=data;resolution.value=data.ours;conflictLine.value=0;diffMode.value='conflict';}else{commits.value=data.commits;hasMore.value=data.has_more;offset.value=0;logSelection.value='';selectedCommitOids.value=[];commitFiles.value=[];commitFilePath.value='';fileHistoryContext.value=kind==='follow'?activeFile.value:'';pathFilter.value=kind==='follow'?activeFile.value:'';pane.value='log';}}catch(e){error.value=friendly(e);}}
function returnToRepositoryHistory(){fileHistoryContext.value='';pathFilter.value='';void loadLog(0);}
function line(id:number){selectedLines.value=selectedLines.value.includes(id)?selectedLines.value.filter(x=>x!==id):[...selectedLines.value,id];}
function hunkSelect(hunk:any){const ids=hunk.lines.filter((l:any)=>l.sign!==' ').map((l:any)=>l.id);selectedLines.value=ids.every((id:number)=>selectedLines.value.includes(id))?selectedLines.value.filter(id=>!ids.includes(id)):[...new Set([...selectedLines.value,...ids])];}
async function partial(){
  if(!file.value||!activeFile.value||!selectedLines.value.length)return;
  try {
    const all=hunks.value.flatMap((h:any)=>h.lines.filter((l:any)=>l.sign!==' ').map((l:any)=>l.id));
    const retained=staged.value?all.filter((id:number)=>!selectedLines.value.includes(id)):selectedLines.value;
    const content=applyPatchSelection(staged.value?file.value.head:file.value.index,patch.value,retained);
    await prepare('index_edit',{paths:[activeFile.value],content,expected_fingerprint:file.value.fingerprint});
  }catch(e){error.value=friendly(e);}
}
async function cancelPreview(){const job=preview.value;preview.value=null;acknowledged.value=false;pushAfterCommit.value=false;if(job){if(['fetch','update','push'].includes(job.action)) remoteOperation.value={action:job.action,phase:'cancelled',message:'预览已取消，工作区和当前选择未改变。'};try{await bridge.gitStudioCancel(job.job_id);}catch{ /* Expired tickets are already unusable. */ }}}
async function prepare(action:string,extras:Record<string,unknown>={}) {
  if(blocked.value||preview.value)return;
  if(!labels[action])return;
  if(['stage','unstage','discard','commit','amend'].includes(action)&&!selected.value.length){error.value='请先勾选文件';return;}
  if(['commit','amend'].includes(action)&&!message.value.trim()){error.value='提交说明不能为空';return;}
  if(['commit','amend'].includes(action) && selectedUnstaged.value.length){error.value='提交仅接受已暂存文件；请先暂存未暂存改动，或取消勾选它们';return;}
  if(['commit','amend'].includes(action) && !selectedStaged.value.length){error.value='请至少选择一个已暂存文件';return;}
  const value=currentSource(),key=sourceIdentity(value);loading.value=true;error.value='';
  if (['fetch','update','push'].includes(action)) remoteOperation.value={action,phase:'checking',message:`正在准备${labels[action]}…`};
  try{
    const next=await bridge.gitStudioPreview(value,{action,paths:selected.value,message:message.value||null,target:branchTarget.value||null,strategy:action==='update'?updateStrategy.value:undefined,...extras});
    if(disposed||!source.value||key!==sourceIdentity(source.value)){await bridge.gitStudioCancel(next.job_id);return;}
    previewSource=key;preview.value=next;acknowledged.value=false;
  }catch(e){if(['commit','amend'].includes(action))pushAfterCommit.value=false;if(['fetch','update','push'].includes(action)) remoteOperation.value={action,phase:'error',message:friendly(e)};error.value=friendly(e);}finally{loading.value=requestCount>0;}
}
async function confirm(){
  const job=preview.value;
  if(!job||writeBusy.value||!acknowledged.value||!source.value||sourceIdentity(source.value)!==previewSource)return;
  const shouldPushAfterCommit = pushAfterCommit.value && ['commit','amend'].includes(job.action);
  writeBusy.value=true;error.value='';preview.value=null;
  if (['fetch','update','push'].includes(job.action)) remoteOperation.value={action:job.action,phase:'executing',message:`正在执行${labels[job.action]}…`};
  try{
    const result=await bridge.gitStudioConfirm(job.job_id,true);
    notice.value=result.success?'操作完成；已重新读取真实仓库状态。':'操作未完成，请查看仓库状态；不要重复执行写入。';
    if (['fetch','update','push'].includes(job.action)) remoteOperation.value={action:job.action,phase:result.success?'success':'error',message:result.success?'远程操作完成并已刷新分支状态。':friendly(result.message)};
    if(!result.success)error.value=friendly(result.message);
  }catch(e){error.value=friendly(e);}finally{writeBusy.value=false;acknowledged.value=false;pushAfterCommit.value=false;if(disposed)unregister();else{await refresh();if(activeFile.value)await openFile(activeFile.value);if(shouldPushAfterCommit&&!error.value&&!disposed)await prepare('push');}}
}
function prepareCommitAndPush(action:'commit'|'amend'='commit'){
  pushAfterCommit.value=true;
  void prepare(action);
}
async function inspectStash(target:string){stashTarget.value=target;try{const result=await read({action:'stash_diff',target});if(result)rawDetail.value=result.patch;}catch(e){error.value=friendly(e);}}
async function inspectPr(number:number){pr.value=number;try{const result=await read({action:'pr_detail',pr:number});if(result)prDetail.value=result;}catch(e){error.value=friendly(e);}}
function prAction(action:string){void prepare(action,{pr:pr.value,message:prText.value,target:branchTarget.value});}
function selectRepo(id:string){if(writeBusy.value)return;localPath.value='';void git.selectRepo(id||null);}
function openLocal(){if(writeBusy.value||!enteredPath.value.trim())return;localPath.value=enteredPath.value.trim();}
function handleGitKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && (commitActionMenu.value || branchActionMenu.value || branchWidgetOpen.value)) { closeCommitActionMenu(); closeBranchActionMenu(); branchWidgetOpen.value = false; return; }
  const menu = document.querySelector('.commit-action-menu,.branch-action-menu') as HTMLElement | null;
  if (menu && ['ArrowDown','ArrowUp','Home','End'].includes(event.key)) {
    const buttons = [...menu.querySelectorAll('button:not(:disabled)')] as HTMLButtonElement[];
    if (buttons.length) {
      const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (current + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
      buttons[next].focus(); event.preventDefault(); return;
    }
  }
  if (writeBusy.value || pane.value !== 'log' || !commits.value.length) return;
  if (event.ctrlKey && event.key.toLowerCase() === 'd' && commitFilePath.value) {
    event.preventDefault();
    historicalDiffMode.value = historicalDiffMode.value === 'split' ? 'patch' : 'split';
    return;
  }
  if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    const current = commits.value.findIndex(commit => commit.oid === logSelection.value);
    const next = event.key === 'ArrowDown' ? Math.min(commits.value.length - 1, current + 1) : Math.max(0, current <= 0 ? 0 : current - 1);
    event.preventDefault();
    void selectCommit(commits.value[next].oid);
  }
}
function handleCommitMenuPointer(event: PointerEvent) {
  const target = event.target as HTMLElement | null;
  if (!target?.closest('.commit-action-menu')) closeCommitActionMenu();
  if (!target?.closest('.branch-action-menu')) closeBranchActionMenu();
  if (!target?.closest('.branch-widget-wrap') && !target?.closest('.rebased-top-branch-wrap')) branchWidgetOpen.value = false;
}
// Repository-bound targets and displayed results must never follow a new source.
// Invalidate synchronously so an action in the same event cannot reuse old state.
function resetSourceContext() {
  generation++; detailGeneration++;
  void cancelPreview(); previewSource='';
  pane.value=defaultPane();
  info.value=null; selected.value=[]; activeFile.value=''; patch.value=''; selectedParent.value=''; remoteOperation.value={action:'',phase:'idle',message:''};
  file.value=null; rawDetail.value=''; commitFiles.value=[]; commitFilePath.value=''; conflict.value=null; conflictLine.value=0; resolution.value='';
  remote.value=null; prDetail.value=null; pr.value=null; prText.value='';
  branchWidgetOpen.value=false;
  branchTarget.value=''; tagTarget.value=''; compareLeft.value='HEAD'; compareRight.value=''; stashTarget.value=''; worktreeTarget.value=''; resetMode.value='mixed';
  revision.value=''; branchQuery.value=''; pathFilter.value=''; logSelection.value=''; message.value='';
  commits.value=[]; commitFileFilter.value=''; stashes.value=[]; worktrees.value=''; blame.value=[];
  selectedCommitOids.value=[];
  selectedLines.value=[]; offset.value=0; fileHistoryContext.value=''; error.value=''; errorCode.value=''; notice.value='';
}
const unregister=registerGitLifecycleBinding({canSuspend:()=>!writeBusy.value,cleanup:resetSourceContext});
watch(()=>props.path,path=>{if(!writeBusy.value){localPath.value=path||'';enteredPath.value=path||'';}});
loadGitPreferences();
watch(()=>props.mode,()=>{if(!writeBusy.value) { pane.value=defaultPane(); if(pane.value==='log') void loadLog(0); } });
watch(()=>source.value?sourceIdentity(source.value):'',()=>{resetSourceContext();void refresh();},{immediate:true,flush:'sync'});
  if (typeof window !== 'undefined' && typeof window.addEventListener === 'function') window.addEventListener('keydown', handleGitKeydown);
  if (typeof document !== 'undefined') document.addEventListener('pointerdown', handleCommitMenuPointer);
  onUnmounted(()=>{if (typeof window !== 'undefined' && typeof window.removeEventListener === 'function') window.removeEventListener('keydown', handleGitKeydown);if (typeof document !== 'undefined') document.removeEventListener('pointerdown', handleCommitMenuPointer);disposed=true;generation++;detailGeneration++;void cancelPreview();if(!writeBusy.value)unregister();});
</script>
<template>
<section class="git-studio" :class="{'rebased-product-theme':pane==='log'&&rebasedLayout}" aria-label="统一 Git 工作台" :data-history-context="historyContext" :data-log-placement="logPlacement">
  <header v-if="pane==='log'&&rebasedLayout" class="rebased-appbar" aria-label="Rebased Git 顶栏">
    <button class="rebased-product-menu" aria-label="仓库配置" title="仓库配置" @click="emit('configure')">☰</button>
    <label class="rebased-repo-picker"><span aria-hidden="true">R</span><select :value="localPath?'':git.repoId||''" :disabled="writeBusy" aria-label="切换仓库" @change="selectRepo(($event.target as HTMLSelectElement).value)"><option value="">{{info?.root?.split('/').filter(Boolean).at(-1)||'选择仓库'}}</option><option v-for="item in repos.repos" :key="item.id" :value="item.id">{{item.name}}</option></select></label>
    <span class="rebased-top-branch-wrap"><button class="rebased-top-branch" :disabled="blocked||!info" :aria-expanded="branchWidgetOpen" @click="toggleBranchWidget">⑂ {{info?.branch||'HEAD'}}⌄</button><section v-if="branchWidgetOpen&&info" class="branch-widget-menu rebased-top-branch-menu" aria-label="当前仓库分支菜单"><strong>当前分支</strong><button class="branch-item active" @click="selectBranchFromTree(info.branch);branchWidgetOpen=false">{{info.branch}} · HEAD</button><button @click="historyBranchFilterOpen=true;branchWidgetOpen=false">查看所有分支</button><button @click="prepare('fetch');branchWidgetOpen=false" :disabled="blocked||preview">获取远端更新</button><button @click="prepare('update');branchWidgetOpen=false" :disabled="blocked||preview">更新当前分支</button><button @click="prepare('push');branchWidgetOpen=false" :disabled="blocked||preview">推送当前分支</button></section></span>
    <button :disabled="blocked||!info||preview" @click="prepare('update')">↙ Update</button>
    <button :disabled="blocked||!info||preview" @click="prepare('push')">↗ Push</button>
    <span class="rebased-product-title">Rebased Git client <small>BrowserOS</small></span>
    <button aria-label="搜索提交历史" title="搜索提交历史" :disabled="blocked||!info" @click="focusHistorySearch">⌕</button>
    <button aria-label="刷新" title="刷新" :disabled="blocked||!source" @click="refresh">⟳</button>
    <button aria-label="Git 设置" title="Git 设置" :aria-pressed="settingsOpen" @click="settingsOpen=!settingsOpen">⚙</button>
    <select class="rebased-more-workspaces" aria-label="更多 Git 工作区" :value="pane" :disabled="writeBusy||!info" @change="selectPane(($event.target as HTMLSelectElement).value)"><option value="log">Log</option><option v-for="[id,label] in panes.filter(([id])=>id!=='log')" :key="id" :value="id">{{label}}</option></select>
    <button v-if="closeLabel" :disabled="writeBusy" @click="emit('close')">×</button>
  </header>
  <header v-else class="toolbar"><strong>Git Studio</strong><select :value="localPath?'':git.repoId||''" :disabled="writeBusy" aria-label="配置仓库" @change="selectRepo(($event.target as HTMLSelectElement).value)"><option value="">选择配置仓库</option><option v-for="item in repos.repos" :key="item.id" :value="item.id">{{item.name}}</option></select><input v-model="enteredPath" :disabled="writeBusy" aria-label="原始本地仓库路径" placeholder="已授权的本地目录或文件" @keydown.enter.prevent="openLocal"/><button :disabled="blocked||!enteredPath.trim()" @click="openLocal">打开本地仓库</button><button :disabled="blocked||!source" @click="refresh">刷新</button><button :aria-pressed="settingsOpen" @click="settingsOpen=!settingsOpen">Git 设置</button><button v-if="closeLabel" :disabled="writeBusy" @click="emit('close')">{{closeLabel}}</button></header>
  <section v-if="settingsOpen" class="git-settings" aria-label="Git 设置">
    <strong>Git 产品设置</strong>
    <label v-if="pane==='log'&&rebasedLayout" class="git-settings-path">本地仓库<input v-model="enteredPath" :disabled="writeBusy" aria-label="原始本地仓库路径" placeholder="已授权的本地目录" @keydown.enter.prevent="openLocal"/><button :disabled="blocked||!enteredPath.trim()" @click="openLocal">打开</button></label>
    <label>Log 位置<select v-model="logPlacement"><option value="editor">主编辑区域</option><option value="toolwindow">底部工具窗口</option></select></label>
    <label><input v-model="writeIdeaMetadata" type="checkbox"/>允许写入项目 `.idea` 元数据</label><label><input v-model="indexEnabled" type="checkbox"/>启用 Git 索引</label>
    <button @click="saveGitPreferences">保存设置</button>
    <span class="hint">偏好仅保存在本机；不会改变或删除现有 Git 数据。</span>
  </section>
  <p v-if="(info||repositoryState==='LOADING')&&!(pane==='log'&&rebasedLayout)" class="context"><span v-if="info?.branch" class="branch-widget-wrap"><button class="branch-widget" :disabled="blocked" aria-label="打开 Git 分支菜单" :aria-expanded="branchWidgetOpen" @click="toggleBranchWidget"><b>⑂ {{info.branch}}⌄</b><span v-if="branchTrackingLabel(info.branch)" class="tracking-badge">{{branchTrackingLabel(info.branch)}}</span></button><section v-if="branchWidgetOpen" class="branch-widget-menu" aria-label="当前仓库分支菜单"><strong>当前分支</strong><button class="branch-item active" @click="selectBranchFromTree(info.branch);branchWidgetOpen=false">{{info.branch}} · HEAD</button><button @click="selectPane('log');branchWidgetOpen=false">查看分支与提交历史</button><button @click="prepare('fetch');branchWidgetOpen=false" :disabled="blocked||preview">获取远端更新</button><button @click="prepare('push');branchWidgetOpen=false" :disabled="blocked||preview">推送当前分支</button><button @click="prepare('update');branchWidgetOpen=false" :disabled="blocked||preview">更新当前分支</button></section></span><b v-else>正在加载仓库…</b> · {{info?.root||'正在读取仓库状态'}} <span class="state-badge" role="status" aria-live="polite" :data-state="repositoryState">{{repositoryStateLabel[repositoryState]}}</span><span class="state-badge repository-mode" role="status" aria-live="polite" :data-state="repositoryMode" :data-repository-mode="repositoryMode">{{repositoryModeLabel[repositoryMode]}}</span><span v-if="scope"> · 初始范围：{{scope}}</span><span v-if="info?.state&&info.state!=='Clean'" class="warning"> · {{info.state}} 待处理</span></p>
  <section v-if="repositoryState!=='READY'&&repositoryState!=='LOADING'&&repositoryState!=='NO_PROJECT'" class="repository-state-panel" role="status" aria-live="polite"><strong>{{repositoryStateLabel[repositoryState]}}</strong><p v-if="repositoryState==='NO_VCS'">当前项目没有 Git 仓库。创建前会先生成预览，确认后才会在该目录创建 `.git`。</p><button v-if="repositoryState==='NO_VCS'" :disabled="blocked||preview" @click="prepare('init')">创建 Git 仓库</button><p v-else-if="repositoryState==='UNTRUSTED'">当前项目未受信任，Git 读取和写入操作已暂停。请先在项目安全设置中完成信任后重新打开。</p><p v-else-if="repositoryState==='INVALID_PATH'">仓库路径不存在或不是可访问目录，请重新选择本地仓库。</p><p v-else>Git 状态暂时不可用，请刷新或重新选择仓库。</p></section>
  <section v-if="repositoryState==='NO_PROJECT'&&pane==='log'&&rebasedLayout" class="rebased-empty-project" aria-label="选择 Git 仓库">
    <span class="rebased-empty-project-icon" aria-hidden="true">R</span>
    <strong>选择一个 Git 仓库</strong>
    <p>打开本地仓库后，这里会显示 Commit / Shelf、Log、文件树和差异编辑区。</p>
    <div><button @click="settingsOpen=true">打开本地仓库</button><button @click="emit('configure')">配置远程仓库</button></div>
  </section>
  <nav v-if="(info||repositoryState==='LOADING')&&!(pane==='log'&&rebasedLayout)" class="toolbar git-workspace-nav" aria-label="Git 工作台视图">
    <template v-if="pane==='log'&&rebasedLayout"><button :disabled="blocked||!info" @click="refresh">⟳ Refresh</button><button :disabled="blocked||!info||preview" @click="prepare('update')">↙ Update</button><button :disabled="blocked||!info||preview" @click="prepare('push')">↗ Push</button><span class="grow"/><label class="update-strategy">Update<select v-model="updateStrategy" aria-label="Update 策略" :disabled="!info"><option value="Fast-forward only">Fast-forward only</option><option value="Merge">Merge</option><option value="Rebase">Rebase</option></select></label><select aria-label="更多 Git 工作区" :value="pane" :disabled="writeBusy||!info" @change="selectPane(($event.target as HTMLSelectElement).value)"><option value="log">Log</option><option v-for="[id,label] in panes.filter(([id])=>id!=='log')" :key="id" :value="id">{{label}}</option></select></template>
    <template v-else><button v-for="[id,label] in panes" :key="id" :aria-pressed="pane===id" :disabled="writeBusy||!info" @click="selectPane(id)">{{label}}</button><span class="grow"/><label class="update-strategy">Update 策略<select v-model="updateStrategy" aria-label="Update 策略" :disabled="!info"><option value="Fast-forward only">Fast-forward only</option><option value="Merge">Merge</option><option value="Rebase">Rebase</option></select></label><button v-for="op in ['fetch','update','push']" :key="op" :disabled="blocked||!info||preview" @click="prepare(op)">{{labels[op]}}</button></template>
  </nav>
  <p v-if="error" role="alert" class="error">{{error}}</p><p v-if="notice" role="status">{{notice}}</p><p v-if="remoteOperation.phase!=='idle'" class="remote-result" :data-phase="remoteOperation.phase" role="status" aria-live="polite">{{labels[remoteOperation.action]||remoteOperation.action}} · {{remoteOperation.message}}</p><p v-if="loading" role="status">正在读取或生成预览…</p><p v-if="writeBusy" role="status">正在执行已确认的操作；此时不能切换仓库或停用 Git。</p>
  <section v-if="preview" class="preview" aria-label="Git 写入预览"><strong>确认{{labels[preview.action]||preview.action}}</strong><p>{{preview.root}} · {{preview.target||''}} · {{preview.scope==='whole_worktree_including_untracked'?'整个仓库，包含未跟踪文件':'仅明确指定的操作范围'}}</p><p v-if="preview.action==='update'" class="hint">Update 策略：{{updateStrategy}}；确认后才会执行对应整合方式。</p><p v-if="preview.action==='push'&&outgoingCount===0" class="hint">没有待推送提交；确认后仍会执行一次远端状态校验。</p><p v-else-if="preview.action==='push'">当前分支有 {{outgoingCount}} 个待推送提交。</p><p v-if="preview.action==='update'&&incomingCount===0" class="hint">没有待更新提交；确认后仍会执行 {{updateStrategy}} 校验。</p><p>有效期 {{preview.expires_in_seconds}} 秒；HEAD、暂存区或工作区变化后拒绝执行。</p><pre v-if="preview.paths?.length">{{preview.paths.join('\n')}}</pre><p v-if="preview.message">{{preview.message}}</p><label><input v-model="acknowledged" type="checkbox"/>我已检查目标、范围和改动，确认执行。</label><button :disabled="writeBusy||!acknowledged" @click="confirm">确认执行</button><button :disabled="writeBusy" @click="cancelPreview">取消预览</button></section>
  <div v-if="info" class="workspace" :class="{ 'history-layout': pane === 'log' }" :style="pane==='log' ? historyLayoutStyle : undefined">
    <template v-if="pane==='changes'">
      <aside class="files"><div class="toolbar"><strong>文件</strong><button :disabled="blocked" @click="selected=[]">清空选择</button></div><p class="hint">同一文件可同时有已暂存和未暂存改动。</p><section v-for="group in groups" :key="group.id"><h4>{{group.name}}（{{group.files.length}}）</h4><label v-for="entry in group.files" :key="entry.path" class="file-row" :class="{active:activeFile===entry.path}"><input type="checkbox" :checked="selected.includes(entry.path)" :disabled="writeBusy" @change="toggle(entry.path)"/><button :title="entry.path" :disabled="blocked" @click="openFile(entry.path,group.id==='index')">{{entry.path}}</button><small>{{entry.index}}{{entry.worktree}}</small></label></section><p v-if="!groups.length">工作区干净。</p></aside>
      <main class="detail"><div class="toolbar"><strong>{{activeFile||'选择文件查看差异'}}</strong><button :disabled="blocked||!activeFile" @click="openFile(activeFile,!staged)">{{staged?'HEAD → Index':'Index → Working Tree'}}</button><select v-model="diffMode" aria-label="差异视图"><option value="split">并排变更</option><option value="patch">补丁</option><option value="full">全文三版本</option><option value="partial">选择代码块 / 行</option><option v-if="blame.length" value="blame">行级归属</option><option v-if="conflict" value="conflict">三方冲突</option></select><button :disabled="blocked||!activeFile" @click="inspectFile('blame')">Blame</button><button :disabled="blocked||!activeFile" @click="inspectFile('follow')" title="Vcs.ShowTabbedFileHistory：打开当前文件历史">文件历史</button><button :disabled="blocked||!activeFile" @click="inspectFile('conflict')">解决冲突</button></div>
        <div v-if="diffMode==='split'" class="patch"><div v-for="(row,index) in parsed.rows" :key="index" class="split-row"><pre :class="{removed:row.removed}">{{row.beforeLine??''}} {{row.before}}</pre><pre :class="{added:row.added}">{{row.afterLine??''}} {{row.after}}</pre></div><p v-if="parsed.truncated">显示达到 4000 行上限，内容不完整。</p></div>
        <pre v-else-if="diffMode==='patch'" class="patch">{{patch||'没有可显示的文本补丁。'}}</pre>
        <div v-else-if="diffMode==='full'&&file" class="three"><label v-for="version in ['head','index','worktree']" :key="version">{{version}}<textarea :value="file[version]" readonly spellcheck="false"/></label></div>
        <div v-else-if="diffMode==='partial'" class="patch"><p>只改变暂存区，不覆盖工作区；每个选中的增删行将共同组成新的暂存内容。</p><section v-for="hunk in hunks" :key="hunk.id"><button :disabled="blocked" @click="hunkSelect(hunk)">选择 / 取消此代码块</button><label v-for="change in hunk.lines" :key="change.id" class="patch-line" :class="{added:change.sign==='+',removed:change.sign==='-'}"><input v-if="change.sign!==' '" type="checkbox" :checked="selectedLines.includes(change.id)" :disabled="blocked" @change="line(change.id)"/><code>{{change.sign}}{{change.text}}</code></label></section><button :disabled="blocked||preview||!selectedLines.length||!file" @click="partial">{{staged?'取消暂存选中行':'暂存选中行'}}并预览</button></div>
        <div v-else-if="diffMode==='blame'" class="patch"><div v-for="item in blame" :key="item.line" class="blame-line"><button :title="item.summary" @click="selectCommit(item.oid);pane='log'">{{item.oid.slice(0,8)}} · {{item.author}}</button><small>{{item.line}}</small><code>{{item.text}}</code></div></div>
        <div v-else-if="diffMode==='conflict'&&conflict" class="conflict-editor"><div class="conflict-toolbar"><button :disabled="blocked||!conflictRows.length" @click="moveConflict(-1)">↑ 上一个冲突</button><span>冲突 {{conflictRows.length ? conflictLine+1 : 0}} / {{conflictRows.length}}</span><button :disabled="blocked||!conflictRows.length" @click="moveConflict(1)">下一个冲突 ↓</button></div><section><header>当前版本（ours）<button @click="resolution=conflict.ours">采用当前全部</button></header><textarea :value="conflict.ours" readonly spellcheck="false"/></section><section><header>冲突编辑区<button @click="resolution=conflict.theirs">采用目标全部</button></header><textarea v-model="resolution" class="resolution" spellcheck="false" maxlength="131072"/></section><section><header>目标版本（theirs）</header><textarea :value="conflict.theirs" readonly spellcheck="false"/></section><div class="conflict-hunk-list" v-if="conflictRows.length"><div v-for="row in conflictRows" :key="row.index" class="conflict-hunk" :class="{active:conflictRows[conflictLine]?.index===row.index}"><code>第 {{row.index+1}} 行</code><button @click="applyConflictLine('ours',row.index)" title="将当前版本这一行应用到中央编辑区">← 当前</button><span class="conflict-preview ours">{{row.ours||'（空行）'}}</span><span class="conflict-preview theirs">{{row.theirs||'（空行）'}}</span><button @click="applyConflictLine('theirs',row.index)" title="将目标版本这一行应用到中央编辑区">目标 →</button></div></div><p class="hint">共同祖先版本用于冲突定位；中央编辑区仍可直接修改。逐行按钮只改变当前预览内容，最终必须通过“预览解决结果”确认。</p><button :disabled="blocked||preview" @click="prepare('resolve',{paths:[activeFile],content:resolution,expected_fingerprint:conflict.fingerprint})">预览解决结果</button></div>
      </main>
    </template>
  <template v-else-if="pane==='log'">
    <div v-if="commitActionMenu" class="commit-action-menu" role="menu" aria-label="提交操作" :style="{left:`${commitActionMenu.x}px`,top:`${commitActionMenu.y}px`}">
      <button role="menuitem" @click="compareCommitToLocal(commitActionMenu.commit)">与工作区比较</button>
      <button role="menuitem" :disabled="blocked||preview" @click="prepareCommitAction('cherry_pick',commitActionMenu.commit)">挑选此提交</button>
      <button role="menuitem" :disabled="blocked||preview" @click="prepareCommitAction('revert',commitActionMenu.commit)">撤销此提交</button>
      <button role="menuitem" :disabled="blocked||preview" @click="prepareReset(commitActionMenu.commit)">Reset 到此提交（{{resetMode}}）</button>
      <button role="menuitem" :disabled="blocked||preview" @click="prepareCommitAction('branch_create',commitActionMenu.commit)">从此提交新建分支</button>
      <button role="menuitem" :disabled="blocked||preview||!tagTarget.trim()" title="先填写标签名，再从提交菜单创建标签" @click="prepareCommitAction('tag_create',commitActionMenu.commit)">从此提交创建标签</button>
      <button role="menuitem" @click="closeCommitActionMenu">取消</button>
    </div>
    <div v-if="branchActionMenu" class="branch-action-menu" role="menu" aria-label="分支操作" :style="{left:`${branchActionMenu.x}px`,top:`${branchActionMenu.y}px`}">
      <button role="menuitem" :disabled="blocked||preview||!canCheckoutBranch(branchActionMenu.name)" :title="!isLocalBranch(branchActionMenu.name)?'只能切换本地分支':branchActionMenu.name===info?.branch?'当前已在此分支':''" @click="prepareBranchAction('checkout',branchActionMenu.name)">切换到 {{branchActionMenu.name}}</button>
      <button role="menuitem" @click="compareBranchToHead(branchActionMenu.name)">与当前 HEAD 比较</button>
      <button role="menuitem" :disabled="blocked||preview" @click="prepareBranchAction('branch_create',branchActionMenu.name)">新建目标分支（当前 HEAD）</button>
      <button role="menuitem" :disabled="blocked||preview||!canDeleteBranch(branchActionMenu.name)" title="仅允许删除非当前、非保护的本地分支" @click="prepareBranchAction('branch_delete',branchActionMenu.name)">删除已合并分支</button>
      <button role="menuitem" @click="closeBranchActionMenu">取消</button>
    </div>
      <aside class="commit-inspector" aria-label="提交详情、变更文件及差异预览">
        <div class="rebased-inspector-tools"><button title="显示文件差异" aria-label="显示文件差异" :disabled="!commitFilePath" @click="selectCommitFile(commitFilePath)">⇄</button><button title="撤销此提交（需预览确认）" aria-label="撤销此提交" :disabled="blocked||preview||!selectedCommit" @click="selectedCommit&&prepareCommitAction('revert',selectedCommit)">↶</button><button title="提交版本预览" aria-label="提交版本预览" :disabled="!commitFilePath" @click="historicalDiffMode='file'">◷</button><button title="并排差异" aria-label="并排差异" :disabled="!commitFilePath" @click="historicalDiffMode='split'">◉</button></div>
        <template v-if="logSelection">
          <div class="inspector-file-heading"><strong>变更文件（{{commitFiles.length}}）</strong><small class="change-summary">新增 {{commitFileStats.added}} · 修改 {{commitFileStats.modified}} · 删除 {{commitFileStats.deleted}} · 重命名 {{commitFileStats.renamed}}</small></div>
          <input v-model="commitFileFilter" class="history-file-filter" type="search" placeholder="筛选路径或状态…" aria-label="筛选提交变更文件" />
          <div class="history-files">
            <details open><summary>▱ {{visibleCommitFiles.length}} files</summary><GitCommitFileTree :node="commitTree" :selected-path="commitFilePath" @select="selectCommitFile"/></details>
            <p v-if="!visibleCommitFiles.length" class="hint">没有匹配的变更文件。</p>
          </div>
          <div class="inspector-commit-meta">
            <strong>{{selectedCommit?.summary}}</strong>
            <p><code>{{logSelection.slice(0,12)}}</code> · {{selectedCommit?.author}}</p>
            <p v-if="selectedCommit?.time">{{new Date(selectedCommit.time*1000).toLocaleString()}}</p>
            <p v-if="selectedCommit?.parents?.length" class="hint">父提交：<code>{{selectedCommit.parents.join(' · ')}}</code><span v-if="selectedCommit.parents.length>1"> · 选择父提交后，变更文件和 Diff 会按该父提交重新计算。</span></p>
            <label v-if="selectedCommit?.parents?.length>1" class="parent-selector">Diff 基准父提交
              <select :value="selectedParent" aria-label="选择 Merge Commit 父提交" :disabled="blocked" @change="selectCommitParent(($event.target as HTMLSelectElement).value)">
                <option v-for="parent in selectedCommit.parents" :key="parent" :value="parent">{{parent.slice(0,12)}}</option>
              </select>
            </label>
            <p v-else-if="selectedCommit && !selectedCommit.parents?.length" class="hint">这是初始提交，没有父提交；文件内容按提交快照展示。</p>
          </div>
          <div v-if="commitFilePath" class="toolbar">
            <button @click="moveCommitFile(-1)" :disabled="blocked||!visibleCommitFiles.length||visibleCommitFiles[0].path===commitFilePath" aria-label="上一个变更文件">↑ 上一个</button>
            <button @click="moveCommitFile(1)" :disabled="blocked||!visibleCommitFiles.length||visibleCommitFiles[visibleCommitFiles.length-1].path===commitFilePath" aria-label="下一个变更文件">下一个 ↓</button>
            <button @click="selectCommitFile(commitFilePath,true)" :disabled="blocked">与本地比较</button>
            <button @click="selectCommitFile(commitFilePath)" :disabled="blocked">提交版本</button>
          </div>
        </template>
        <template v-else><p class="hint">从下方历史选择提交；提交详情和变更文件会固定显示在这里。</p></template>
      </aside>
      <div v-if="rebasedLayout" class="rebased-column-splitter rebased-left-splitter" role="separator" aria-label="调整 Commit 与 Log 宽度" aria-orientation="vertical" @pointerdown="beginRebasedResize('left',$event)"/>
      <div v-if="rebasedLayout" class="rebased-column-splitter rebased-right-splitter" role="separator" aria-label="调整 Log 与文件树宽度" aria-orientation="vertical" @pointerdown="beginRebasedResize('right',$event)"/>
      <div class="history-vertical-splitter" role="separator" aria-label="调整提交详情宽度" aria-orientation="vertical" @pointerdown="beginHistoryResize('inspector',$event)"/>
      <main class="detail history-editor-pane" aria-label="文件编辑和差异预览">
        <header class="history-editor-toolbar"><button v-if="commitFilePath||rebasedWorktreePath||rebasedShelf&&stashTarget" @click="commitFilePath='';rebasedWorktreePath='';stashTarget=''">← 返回 Log</button><strong>{{rebasedWorktreePath||commitFilePath||rebasedShelf&&stashTarget||'选择提交和文件'}}</strong><span class="grow"/><template v-if="commitFilePath"><button @click="historyEditorCollapsed=!historyEditorCollapsed">{{historyEditorCollapsed?'展开编辑器':'折叠编辑器'}}</button><button :disabled="historicalDiffMode==='file'" :title="historicalDiffMode==='file'?'切换到并排 Diff 后可筛选变更区域':''" @click="historyOnlyChanges=!historyOnlyChanges">{{historyOnlyChanges?'显示全部内容':'只显示变更区域'}}</button><select v-model="historicalDiffMode" aria-label="历史文件预览方式"><option value="file">文件编辑器</option><option value="split">并排 Diff</option><option value="patch">统一 Patch</option></select></template></header>
        <section v-if="rebasedWorktreePath" class="history-editor" aria-label="工作区文件差异"><GitHistoryDiff :patch="patch" :after-content="(staged ? file?.index : file?.worktree) || ''" :only-changes="historyOnlyChanges" :before-label="staged ? 'HEAD' : '暂存区'" :after-label="staged ? '暂存区' : '工作区'"/></section>
        <section v-else-if="rebasedShelf&&stashTarget" class="history-editor" aria-label="暂存架文件差异"><pre class="patch history-patch">{{rawDetail||'此暂存记录没有可显示的文本差异。'}}</pre></section>
        <p v-else-if="!logSelection" class="history-empty">从中间历史选择提交，或从左侧选择工作区文件。</p>
        <p v-else-if="!commitFilePath" class="history-empty">正在读取提交的变更文件，或该提交没有文本文件。</p>
        <section v-else class="history-editor" aria-label="历史文件编辑区">
          <p v-if="historicalContentLoading" class="hint">正在读取文件和差异…</p>
          <template v-else-if="!historyEditorCollapsed">
            <textarea v-if="historicalDiffMode==='file'" :value="historicalContent" readonly spellcheck="false" aria-label="提交版本文件内容" placeholder="这个版本中该文件不存在、为二进制文件，或超过预览大小限制；仍可切换到 Diff 查看变更。"/>
            <GitHistoryDiff v-else-if="historicalDiffMode==='split'" :patch="rawDetail" :after-content="historicalContent" :only-changes="historyOnlyChanges" :before-label="historicalCompareLocal ? logSelection.slice(0,12) : selectedParent ? '父提交 '+selectedParent.slice(0,12) : '空版本（初始提交）'" :after-label="historicalCompareLocal ? '工作区' : '当前提交 '+logSelection.slice(0,12)"/>
            <pre v-else class="patch history-patch">{{rawDetail||'此文件没有可显示的文本差异。'}}</pre>
          </template>
        </section>
      </main>
      <div class="history-horizontal-splitter" role="separator" aria-label="调整分支和历史高度" aria-orientation="horizontal" @pointerdown="beginHistoryResize('bottom',$event)"/>
      <section class="history-bottom" :class="{'rebased-history':rebasedLayout}" aria-label="分支和提交历史">
        <aside v-if="rebasedLayout" class="rebased-commit-panel" aria-label="提交与暂存架">
          <div class="rebased-panel-tabs"><button :aria-pressed="!rebasedShelf" @click="rebasedShelf=false">Commit</button><button :aria-pressed="rebasedShelf" @click="openRebasedShelf">Shelf</button></div>
          <div class="rebased-commit-tools"><button title="刷新工作区" aria-label="刷新工作区" @click="refresh">⟳</button><button title="丢弃选中改动（需预览确认）" aria-label="丢弃选中改动" :disabled="blocked||preview||!selected.length" @click="prepare('discard')">↶</button><button title="移动选中改动到暂存区或工作区" aria-label="移动选中改动" :disabled="blocked||preview||!selected.length" @click="prepare(selectedUnstaged.length?'stage':'unstage')">⇄</button><button title="保存到暂存架（需预览确认）" aria-label="保存到暂存架" :disabled="blocked||preview" @click="prepare('stash_save')">⇥</button><button title="查看选中文件差异" aria-label="查看选中文件差异" :disabled="blocked||!selected.length" @click="openRebasedWorktreeFile((info.files||[]).find((entry:any)=>entry.path===selected[0]))">◉</button><button title="选中已打开文件" aria-label="选中已打开文件" :disabled="!rebasedWorktreePath" @click="rebasedWorktreePath&&!selected.includes(rebasedWorktreePath)&&selected.push(rebasedWorktreePath)">⌖</button><button title="展开全部" aria-label="展开全部" @click="rebasedChangesExpanded=true;rebasedUnversionedExpanded=true">⊞</button><button title="折叠全部" aria-label="折叠全部" @click="rebasedChangesExpanded=false;rebasedUnversionedExpanded=false">⊟</button></div>
          <div v-if="!rebasedShelf" class="rebased-changes">
            <details :open="rebasedChangesExpanded" @toggle="rebasedChangesExpanded=($event.target as HTMLDetailsElement).open"><summary>Changes <small>{{rebasedChanges.length}} files</small></summary><div v-for="entry in rebasedChanges" :key="entry.path" class="rebased-file-row"><input type="checkbox" :checked="selected.includes(entry.path)" :disabled="blocked" :aria-label="`选择 ${entry.path}`" @change="toggle(entry.path)"/><button :class="{active:rebasedWorktreePath===entry.path}" :title="entry.path" @click="openRebasedWorktreeFile(entry)">{{entry.path}}</button></div></details>
            <details :open="rebasedUnversionedExpanded" @toggle="rebasedUnversionedExpanded=($event.target as HTMLDetailsElement).open"><summary>Unversioned Files <small>{{rebasedUnversioned.length}} files</small></summary><div v-for="entry in rebasedUnversioned" :key="entry.path" class="rebased-file-row"><input type="checkbox" :checked="selected.includes(entry.path)" :disabled="blocked" :aria-label="`选择 ${entry.path}`" @change="toggle(entry.path)"/><button :class="{active:rebasedWorktreePath===entry.path}" :title="entry.path" @click="openRebasedWorktreeFile(entry)">{{entry.path}}</button></div></details>
          </div>
          <div v-else class="rebased-changes"><strong>Shelf · {{stashes.length}}</strong><button v-for="item in stashes" :key="item[0]" @click="inspectStash(item[0])">{{item[0]}} · {{item[2]}}</button><p v-if="!stashes.length">没有保存的暂存记录。</p><div class="toolbar"><button :disabled="blocked||preview||!stashTarget" @click="prepare('stash_apply',{target:stashTarget})">恢复选中暂存</button><button :disabled="blocked||preview" @click="prepare('stash_save')">保存当前改动</button></div></div>
          <div v-if="!rebasedShelf" class="rebased-commit-form"><div><button :disabled="blocked||preview||!selected.length" @click="prepare('stage')">Stage selected files</button><button :disabled="blocked||preview||!selected.length" @click="prepare('unstage')">Unstage</button></div><label class="rebased-amend"><input v-model="rebasedAmend" type="checkbox" :disabled="blocked||preview"/> Amend last commit</label><textarea v-model="message" maxlength="2048" placeholder="Commit message" aria-label="提交说明"/><div><button :disabled="blocked||preview||!selectedStaged.length||selectedUnstaged.length||!message.trim()" @click="prepare(rebasedAmend?'amend':'commit')">{{rebasedAmend?'Amend':'Commit'}}</button><button :disabled="blocked||preview||!selectedStaged.length||selectedUnstaged.length||!message.trim()" @click="prepareCommitAndPush(rebasedAmend?'amend':'commit')">{{rebasedAmend?'Amend and Push…':'Commit and Push…'}}</button></div></div>
        </aside>
        <button v-if="rebasedLayout" class="rebased-branches-strip" :aria-expanded="historyBranchFilterOpen" @click="historyBranchFilterOpen=!historyBranchFilterOpen">› Branches</button>
        <aside class="history-branches" aria-label="Git 本地与远程分支">
          <strong>分支 · {{localRefs.length}} 本地 / {{remoteRefs.length}} 远程</strong>
          <input v-model="branchQuery" type="search" class="branch-search" aria-label="筛选分支" placeholder="搜索分支…"/>
          <button class="branch-item" :aria-pressed="!revision" :disabled="blocked" @click="selectBranchFromTree('')">全部分支</button>
          <section v-for="group in refGroups" :key="group.id"><h4>{{group.title}}（{{group.total}}）</h4><button v-for="name in group.list" :key="name" class="branch-item" :aria-pressed="revision===name" :aria-haspopup="'menu'" :aria-expanded="branchActionMenu?.name===name" :disabled="blocked" @click="selectBranchFromTree(name)" @contextmenu="openBranchActionMenu($event,name)">{{name}}{{name===info.branch?' · HEAD':''}}<span v-if="branchTrackingLabel(name)" class="tracking-badge">{{branchTrackingLabel(name)}}</span></button></section>
          <p class="hint">远程引用来自本机；Fetch 后刷新。</p><input v-model="branchTarget" placeholder="目标分支" aria-label="目标分支"/><input v-model="tagTarget" placeholder="标签名（提交右键创建）" aria-label="标签名"/><label class="reset-mode">Reset 模式<select v-model="resetMode" aria-label="Reset 模式"><option value="soft">soft（保留暂存）</option><option value="mixed">mixed（重置暂存）</option><option value="hard">hard（丢弃工作区）</option></select></label><button :disabled="blocked||!branchTarget" @click="compare">与 HEAD 比较</button><div class="toolbar"><button v-for="op in ['branch_create','checkout','branch_delete','merge','rebase']" :key="op" :disabled="blocked||preview||!canRunBranchAction(op)" :title="op==='checkout'&&!canCheckoutBranch(branchTarget.trim())?'只能切换到其他本地分支':op==='branch_delete'&&!canDeleteBranch(branchTarget.trim())?'仅允许删除非当前、非保护的本地分支':''" @click="prepare(op)">{{labels[op]}}</button></div>
        </aside>
          <main class="history-log" aria-label="分支提交历史">
          <div class="rebased-log-tabs" role="tablist" aria-label="历史编辑区"><button role="tab" aria-selected="true">⑂ Log: {{revision||'HEAD'}}</button><span class="grow"/><button class="rebased-log-menu" aria-label="历史选项" @click="settingsOpen=!settingsOpen">⋮</button></div>
          <form class="filters rebased-history-filters" @submit.prevent="loadLog(0)">
            <input ref="historySearchInput" v-model="search" placeholder="Text or hash" aria-label="提交说明或哈希"/>
            <div class="history-branch-filter"><button type="button" :aria-expanded="historyBranchFilterOpen" @click="historyBranchFilterOpen=!historyBranchFilterOpen">Branch: {{revision||'HEAD'}} {{revision?'×':'⌄'}}</button><div v-if="historyBranchFilterOpen" class="history-filter-menu"><button type="button" @click="selectBranchFromTree('')">HEAD / 全部分支</button><button v-for="name in [...localRefs,...remoteRefs]" :key="name" type="button" :aria-pressed="revision===name" :title="isLocalBranch(name)?'点击筛选历史；右键打开分支操作':'远程引用只读；点击筛选历史'" @click="selectBranchFromTree(name)" @contextmenu="openBranchActionMenu($event,name)">{{name}}{{name===info.branch?' · HEAD':''}}</button><div class="history-branch-actions"><input v-model="branchTarget" placeholder="目标分支" aria-label="分支操作目标"/><button type="button" :disabled="blocked||preview||!branchTarget.trim()" @click="prepare('branch_create')">新建</button><button type="button" :disabled="blocked||preview||!canCheckoutBranch(branchTarget.trim())" @click="prepare('checkout')">切换</button><button type="button" :disabled="blocked||preview||!branchTarget.trim()" @click="prepare('merge')">Merge</button><button type="button" :disabled="blocked||preview||!branchTarget.trim()" @click="prepare('rebase')">Rebase</button><button type="button" :disabled="blocked||preview||!canDeleteBranch(branchTarget.trim())" @click="prepare('branch_delete')">删除已合并分支</button></div></div></div>
            <select v-model="author" aria-label="筛选作者" @change="loadLog(0)"><option value="">User</option><option v-for="name in historyAuthors" :key="name" :value="name">{{name}}</option></select>
            <select v-model="dateFilter" aria-label="筛选日期"><option value="">Date</option><option v-for="date in historyDates" :key="date" :value="date">{{date}}</option></select>
            <input v-model="pathFilter" :disabled="blocked||Boolean(fileHistoryContext)" :title="fileHistoryContext?'文件历史路径已锁定':''" placeholder="Paths…" aria-label="按路径筛选历史"/>
            <button :disabled="blocked">检索</button><button type="button" :disabled="blocked||(!search&&!author&&!pathFilter&&!dateFilter&&!revision)" @click="clearAllHistoryFilters">清空</button>
          </form>
          <p v-if="fileHistoryContext" class="history-context"><strong>文件历史</strong> · {{fileHistoryContext}} <button @click="returnToRepositoryHistory">返回仓库 Log</button></p><p v-else>历史：{{revision||'全部分支'}}<span v-if="selectedCommitOids.length" class="selection-count"> · 已选 {{selectedCommitOids.length}} 条</span></p><section v-if="compareFiles.length" class="compare-file-list" aria-label="比较变更文件"><strong>比较变更文件（{{compareFiles.length}}）</strong><button v-for="item in compareFiles" :key="item.status+item.path" class="compare-file" :class="{active:compareFilePath===item.path}" @click="loadComparison(item.path)"><b>{{item.status}}</b> {{item.path}}</button></section>
          <button v-for="commit in visibleCommits" :key="commit.oid" class="commit rebased-commit-row" :class="{active:logSelection===commit.oid,selected: selectedCommitOids.includes(commit.oid)}" :aria-selected="selectedCommitOids.includes(commit.oid)" aria-haspopup="menu" :aria-expanded="commitActionMenu?.commit?.oid===commit.oid" @click="handleCommitSelection($event,commit,graphIndex(commit.oid))" @contextmenu="openCommitActionMenu($event,commit)"><svg viewBox="0 0 150 32" aria-hidden="true"><line v-for="lane in graph[graphIndex(commit.oid)]?.through||[]" :key="lane" :x1="10+lane*14" y1="0" :x2="10+lane*14" y2="32" stroke="currentColor"/><line v-for="(lane,i) in graph[graphIndex(commit.oid)]?.parentLanes||[]" :key="'p'+i" :x1="10+(graph[graphIndex(commit.oid)]?.lane||0)*14" y1="16" :x2="10+lane*14" y2="32" stroke="currentColor"/><circle :cx="10+(graph[graphIndex(commit.oid)]?.lane||0)*14" cy="16" r="4" fill="currentColor"/></svg><span class="commit-subject">{{commit.summary}}<template v-if="commit.refs?.length"> <em v-for="ref in commit.refs" :key="ref" class="ref-badge">{{ref}}</em></template></span><span class="commit-author">{{commit.author}}</span><time class="commit-time">{{new Date(commit.time*1000).toLocaleString()}}</time></button>
          <p v-if="!commits.length&&!loading">没有符合条件的提交。</p><div class="toolbar"><button :disabled="blocked||offset===0" @click="loadLog(Math.max(0,offset-50))">上一页</button><span>{{Math.floor(offset/50)+1}}</span><button :disabled="blocked||!hasMore" @click="loadLog(offset+50)">下一页</button><button :disabled="blocked||selectedCommitOids.length!==2" @click="compareSelectedCommits">比较选中提交</button><button :disabled="blocked||preview||!logSelection" @click="prepare('cherry_pick',{target:logSelection})">挑选选中提交</button></div>
        </main>
      </section>
    </template>
    <template v-else-if="pane==='stash'"><aside class="files"><button v-for="item in stashes" :key="item[0]" class="commit" @click="inspectStash(item[0])">{{item[0]}} · {{item[2]}}</button><p v-if="!stashes.length">没有保存的暂存记录。</p></aside><main class="detail"><div class="toolbar"><input v-model="stashTarget" placeholder="stash@{0}" aria-label="暂存记录"/><button v-for="op in ['stash_save','stash_apply','stash_pop','stash_drop']" :key="op" :disabled="blocked||preview||op!=='stash_save'&&!stashTarget" @click="prepare(op,{target:stashTarget})">{{labels[op]}}</button></div><p>保存暂存架会收纳整个仓库的已跟踪与未跟踪改动，不局限于右键文件。</p><pre class="patch">{{rawDetail}}</pre></main></template>
    <main v-else-if="pane==='worktrees'" class="detail"><p>创建受控子目录工作树（分离 HEAD）；移除仅允许已登记且干净的子工作树，不强制删除。</p><div class="toolbar"><input v-model="worktreeTarget" aria-label="工作树子目录" placeholder="新子目录名"/><input v-model="revision" aria-label="工作树基线" placeholder="基线分支 / 提交，默认 HEAD"/><button :disabled="blocked||preview||!worktreeTarget" @click="prepare('worktree_add',{target:worktreeTarget,revision:revision||'HEAD'})">创建</button><button :disabled="blocked||preview||!worktreeTarget" @click="prepare('worktree_remove',{target:worktreeTarget})">安全移除</button></div><pre>{{worktrees}}</pre></main>
    <template v-else-if="pane==='github'"><aside class="files"><p>使用系统中已登录的 gh；BrowserOS 不接收登录凭据。</p><button v-for="item in remote?.prs||[]" :key="item.number" class="commit" @click="inspectPr(item.number)"><span>#{{item.number}} {{item.title}}<small>{{item.isDraft?'草稿':item.mergeStateStatus}} · {{item.reviewDecision}}</small></span></button></aside><main class="detail"><div class="toolbar"><input v-model.number="pr" type="number" min="1" placeholder="PR 编号" aria-label="PR 编号"/><input v-model="branchTarget" placeholder="创建 PR 的目标分支" aria-label="PR 目标分支"/></div><textarea v-model="prText" maxlength="8000" placeholder="PR 标题 / 说明 / 评论 / 审查意见" aria-label="PR 正文"/><div class="toolbar"><button v-for="op in ['pr_create','pr_comment','pr_approve','pr_request_changes','pr_merge']" :key="op" :disabled="blocked||preview||op!=='pr_create'&&!pr" @click="prAction(op)">{{labels[op]}}</button></div><details v-if="prDetail" open><summary>PR 详情、文件和审查</summary><pre>{{JSON.stringify(prDetail,null,2)}}</pre></details><h4>近期 CI</h4><div v-for="run in remote?.runs||[]" :key="run.databaseId" class="toolbar"><a v-if="safeGithubLink(run.url)" :href="safeGithubLink(run.url)" target="_blank" rel="noopener noreferrer">{{run.workflowName}}</a><span>{{run.conclusion||run.status}}</span><code>{{run.headSha?.slice(0,9)}}</code></div></main></template>
  </div>
  <footer v-if="info" class="footer"><template v-if="pane==='changes'"><div class="toolbar"><span>明确选择 {{selected.length}} 个文件 · 已暂存 {{selectedStaged.length}} · 未暂存 {{selectedUnstaged.length}}</span><button v-for="op in ['stage','unstage','discard']" :key="op" :disabled="blocked||preview||!selected.length" @click="prepare(op)">{{labels[op]}}</button><input v-model="message" maxlength="2048" placeholder="提交说明" aria-label="提交说明"/><button :disabled="blocked||preview||!selectedStaged.length||selectedUnstaged.length||!message.trim()" :title="selectedUnstaged.length?'提交前请只选择已暂存文件':''" @click="prepare('commit')">提交选中暂存内容</button><button :disabled="blocked||preview||!selectedStaged.length||selectedUnstaged.length||!message.trim()" :title="selectedUnstaged.length?'提交并推送前请只选择已暂存文件':''" @click="prepareCommitAndPush">提交并推送</button><button :disabled="blocked||preview||!selectedStaged.length||selectedUnstaged.length||!message.trim()" :title="selectedUnstaged.length?'Amend 前请只选择已暂存文件':''" @click="prepare('amend')">Amend（未发布）</button></div></template><div v-if="info.state!=='Clean'" class="toolbar"><span>正在进行：{{info.state}}</span><button v-for="op in ['merge_continue','merge_abort','rebase_continue','rebase_abort','cherry_pick_continue','cherry_pick_abort']" :key="op" :disabled="blocked||preview" @click="prepare(op)">{{labels[op]}}</button></div></footer>
</section>
</template>
<style scoped>
.git-studio{display:flex;flex-direction:column;height:100%;min-height:0;overflow:hidden;background:var(--surface,#fff);color:var(--text,#273a50);font-size:12px}.toolbar{display:flex;flex-wrap:wrap;align-items:center;gap:7px;padding:7px}.toolbar input{min-width:110px;flex:1}.grow{flex:1}.context{margin:0;padding:7px 12px;overflow-wrap:anywhere;border-bottom:1px solid #dce4ed}.workspace{display:flex;flex:1;min-height:180px;overflow:hidden}.branch-search{box-sizing:border-box;width:100%}.branch-item{display:block;text-align:left;width:100%;border:0;padding:5px 8px;overflow:hidden;text-overflow:ellipsis}.branch-item[aria-pressed=true]{background:#dcebf9;font-weight:600}.files{flex:0 0 clamp(210px,28%,360px);overflow:auto;resize:horizontal;border-right:1px solid #dce4ed;padding:8px}.detail{flex:1;min-width:0;overflow:auto;padding:8px}.commit-inspector{flex:0 0 40%;min-width:240px;overflow:auto;border-left:1px solid #dce4ed;padding:8px}.history-files{max-height:180px;overflow:auto}.history-files button{width:100%;text-align:left;white-space:normal;word-break:break-all;border:0;border-bottom:1px solid #e6edf4;padding:5px}.history-files small{display:inline-block;min-width:20px}.history-patch{max-height:55vh;overflow:auto}.history-patch pre{font-size:11px;white-space:pre;overflow:auto}@media(max-width:1100px){.commit-inspector{flex-basis:45%;min-width:220px}.workspace>.files{flex-basis:180px}}.files h4{margin:12px 0 6px}.file-row{display:flex;align-items:center;gap:4px;padding:4px}.file-row button{flex:1;text-align:left;overflow-wrap:anywhere;border:0;background:transparent}.active{background:#eaf2fd}.filters{display:grid;gap:5px}.commit{display:flex;align-items:center;width:100%;text-align:left;gap:5px;padding:8px;border:0;border-bottom:1px solid #e6edf4;background:transparent}.commit small{display:block;font-size:10px;color:#66778b}.commit svg{width:58px;min-width:58px;height:32px}.patch{overflow:auto;font:12px/1.7 ui-monospace,monospace;white-space:pre}.split-row{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr)}.split-row pre{margin:0;padding:1px 5px;overflow:auto;border-right:1px solid #dce4ed}.removed{background:#fff0f0}.added{background:#ebf8ee}.patch-line{display:flex;gap:6px;min-height:22px}.patch-line code{white-space:pre}.three{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}.three textarea{height:300px;width:100%;box-sizing:border-box;white-space:pre;font-family:ui-monospace,monospace}.resolution{display:block;min-height:180px;width:100%;box-sizing:border-box;font-family:ui-monospace,monospace}.blame-line{display:flex;gap:8px;white-space:pre}.blame-line button{width:220px;text-align:left;overflow:hidden;text-overflow:ellipsis}.preview{border:1px solid #d9b15b;background:#fff8e5;padding:10px;margin:7px;max-height:260px;overflow:auto}.preview pre{max-height:90px;overflow:auto}.preview label{display:inline-flex;gap:5px;margin-right:10px}.error{color:#9a2929;background:#fff1f1;padding:9px}.hint,.warning{color:#80602c}.footer{border-top:1px solid #dce4ed}button,select,input,textarea{font:inherit}button{cursor:pointer;border:1px solid #c6d3e0;border-radius:5px;padding:5px 8px;background:#fff;color:inherit}button:disabled{opacity:.45;cursor:not-allowed}button[aria-pressed=true]{background:#e6effc}input,select,textarea{border:1px solid #c6d3e0;border-radius:4px;padding:5px}pre{white-space:pre-wrap;overflow-wrap:anywhere}p{margin:6px 10px}a{color:#235ca1}
.git-settings{display:flex;flex-wrap:wrap;align-items:center;gap:10px;padding:8px 12px;border-bottom:1px solid #dce4ed;background:#f7f9fc}.git-settings label{display:inline-flex;align-items:center;gap:4px}.git-settings .hint{flex-basis:100%;margin:0}
.git-settings .git-settings-path{flex:1 1 420px}.git-settings-path input{flex:1;min-width:180px}
.rebased-product-theme .git-settings{background:var(--git-surface-muted);border-color:var(--git-border);color:var(--git-text)}
.git-studio[data-log-placement=toolwindow] .history-layout{grid-template-rows:minmax(0,0fr) 7px var(--git-history-bottom-height)}.git-studio[data-log-placement=toolwindow] .history-editor-pane,.git-studio[data-log-placement=toolwindow] .history-layout>.commit-inspector{visibility:hidden;overflow:hidden}
.history-layout{display:grid;grid-template-columns:var(--git-history-inspector-width) 7px minmax(0,1fr);grid-template-rows:minmax(0,1fr) 7px var(--git-history-bottom-height);gap:0}.history-layout>.commit-inspector{grid-column:1;grid-row:1;min-width:0;overflow:auto;border-right:1px solid #dce4ed;border-left:0}.history-vertical-splitter{grid-column:2;grid-row:1;cursor:col-resize;background:#edf2f7;border-left:1px solid #dce4ed;border-right:1px solid #dce4ed}.history-editor-pane{grid-column:3;grid-row:1;min-height:0;padding:0;overflow:hidden;display:flex;flex-direction:column}.history-editor-toolbar{display:flex;gap:7px;align-items:center;min-height:40px;padding:6px 10px;border-bottom:1px solid #dce4ed;background:#f7f9fc}.history-editor-toolbar strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.history-editor{display:flex;flex:1;min-height:0;margin:0;background:#fff}.history-editor textarea{flex:1;min-height:0;width:100%;box-sizing:border-box;resize:none;border:0;border-radius:0;background:#fff;font:12px/1.6 ui-monospace,monospace;padding:12px;white-space:pre;tab-size:2}.history-empty{display:flex;align-items:center;justify-content:center;flex:1;color:#718196}.history-horizontal-splitter{grid-column:1 / 4;grid-row:2;cursor:row-resize;background:#edf2f7;border-top:1px solid #dce4ed;border-bottom:1px solid #dce4ed}.history-bottom{grid-column:1 / 4;grid-row:3;display:grid;grid-template-columns:minmax(245px,30%) minmax(0,1fr);min-height:0;overflow:hidden}.history-branches{padding:8px;overflow:auto;border-right:1px solid #dce4ed;background:#fbfcfe}.history-log{min-width:0;overflow:auto;padding:8px}.history-files{max-height:none;overflow:auto}.history-patch{flex:1;max-height:none;overflow:auto}.history-patch pre{font-size:11px;white-space:pre;overflow:auto}.conflict-editor{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}.conflict-editor section{min-width:0;border:1px solid #dce4ed;border-radius:6px}.conflict-editor header{display:flex;justify-content:space-between;align-items:center;padding:6px;font-weight:600;background:#f5f8fb}.conflict-editor textarea{display:block;width:100%;height:420px;box-sizing:border-box;border:0;border-top:1px solid #dce4ed;border-radius:0;font:12px/1.6 ui-monospace,monospace;resize:vertical;white-space:pre}.conflict-editor>.hint,.conflict-editor>button{grid-column:1 / 4}.history-file-filter{width:100%;box-sizing:border-box;margin:7px 0}.ref-badge{display:inline-block;margin-left:3px;padding:1px 4px;border:1px solid #9fc5a5;border-radius:3px;background:#edf8ee;color:#2f7040;font-style:normal;font-size:9px}.compare-file-list{display:flex;gap:6px;align-items:center;flex-wrap:wrap;padding:7px;border:1px solid #dce4ed;border-radius:6px;background:#f8fbff}.compare-file{padding:2px 5px;border-radius:3px;background:#eef3f8;font-size:10px;word-break:break-all}.compare-file b{color:#2f7040}@media(max-width:900px){.history-layout{grid-template-columns:minmax(230px,34%) 7px minmax(0,1fr)}.history-bottom{grid-template-columns:210px minmax(0,1fr)}}
.conflict-toolbar{grid-column:1 / 4;display:flex;align-items:center;justify-content:center;gap:10px;padding:5px;background:#fff8e5;border:1px solid #ead39b;border-radius:5px}.conflict-hunk-list{grid-column:1 / 4;max-height:180px;overflow:auto;border:1px solid #dce4ed;border-radius:5px;background:#fbfcfe}.conflict-hunk{display:grid;grid-template-columns:58px auto minmax(0,1fr) minmax(0,1fr) auto;gap:5px;align-items:center;padding:4px 6px;border-bottom:1px solid #e6edf4;font:11px ui-monospace,monospace}.conflict-hunk.active{background:#eaf2fd}.conflict-preview{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;padding:2px 4px}.conflict-preview.ours{background:#fff0f0}.conflict-preview.theirs{background:#ebf8ee}@media(max-width:900px){.conflict-editor{grid-template-columns:1fr}.conflict-editor>.hint,.conflict-editor>button,.conflict-toolbar,.conflict-hunk-list{grid-column:1}.conflict-hunk{grid-template-columns:48px auto minmax(0,1fr) minmax(0,1fr) auto}}
.state-badge{display:inline-block;margin-left:6px;padding:2px 6px;border:1px solid #c6d3e0;border-radius:10px;background:#f5f8fb;color:#536579;font-size:10px}.tracking-badge{display:inline-block;margin-left:5px;padding:1px 4px;border:1px solid #b8c9db;border-radius:3px;background:#f2f6fa;color:#536579;font-size:10px}.state-badge[data-state=READY]{border-color:#9fc5a5;background:#edf8ee;color:#2f7040}.state-badge[data-state=NO_VCS],.state-badge[data-state=INVALID_PATH]{border-color:#e1b1b1;background:#fff1f1;color:#9a2929}.state-badge[data-state=LOADING]{background:#fff8e5;color:#80602c}
.commit-action-menu{position:absolute;z-index:20;display:grid;min-width:210px;padding:5px;border:1px solid #c6d3e0;border-radius:6px;background:#fff;box-shadow:0 8px 22px rgba(30,55,80,.18)}.commit-action-menu button{border:0;border-radius:3px;text-align:left;background:transparent}.commit-action-menu button:hover,.commit-action-menu button:focus-visible{background:#eaf2fd}.commit.selected{outline:2px solid #9fc5e8;outline-offset:-2px}.selection-count{color:#235ca1;font-weight:600}
.branch-action-menu{position:absolute;z-index:20;display:grid;min-width:230px;padding:5px;border:1px solid #c6d3e0;border-radius:6px;background:#fff;box-shadow:0 8px 22px rgba(30,55,80,.18)}.branch-action-menu button{border:0;border-radius:3px;text-align:left;background:transparent}.branch-action-menu button:hover,.branch-action-menu button:focus-visible{background:#eaf2fd}
.history-context{display:flex;align-items:center;gap:8px;padding:5px 8px;border:1px solid #c6d3e0;border-radius:5px;background:#f7f9fc}.history-context button{margin-left:auto}
.change-summary{display:block;margin:4px 0;color:#66778b;font-size:10px}
.git-studio{position:relative}
.branch-widget-wrap{position:relative;display:inline-block}.branch-widget{display:inline-flex;align-items:center;gap:5px;border:0;background:transparent;padding:1px 4px;margin:-1px 2px -1px -4px}.branch-widget:hover,.branch-widget:focus-visible{background:#eaf2fd}.branch-widget-menu{position:absolute;z-index:20;top:28px;left:0;display:grid;min-width:230px;padding:6px;border:1px solid #c6d3e0;border-radius:6px;background:#fff;box-shadow:0 8px 22px rgba(30,55,80,.18)}.branch-widget-menu strong{padding:5px 8px;color:#536579}.branch-widget-menu button{border:0;border-radius:3px;text-align:left;background:transparent}.branch-widget-menu button:hover,.branch-widget-menu button:focus-visible{background:#eaf2fd}.repository-state-panel{margin:8px 12px;padding:12px;border:1px solid #dce4ed;border-radius:6px;background:#f7f9fc}.repository-state-panel strong{display:block}.repository-state-panel p{margin:6px 0 0}.state-badge[data-state=UNTRUSTED]{border-color:#e1b1b1;background:#fff8e5;color:#80602c}
.parent-selector{display:flex;align-items:center;gap:6px;margin:6px 0;padding:6px;border:1px solid #dce4ed;border-radius:5px;background:#f7f9fc}.parent-selector select{min-width:150px;max-width:100%;}
.history-layout:has(.rebased-history){position:relative;grid-template-columns:var(--rebased-left-width) minmax(390px,1fr) var(--rebased-right-width);grid-template-rows:minmax(0,1fr)}
.history-layout:has(.rebased-history)>.commit-inspector{grid-column:3;grid-row:1;border-left:1px solid #dce4ed;border-right:0}
.history-layout:has(.rebased-history)>.commit-inspector{display:flex;flex-direction:column;min-height:0}.history-layout:has(.rebased-history)>.commit-inspector .history-files{flex:1;min-height:0}.inspector-commit-meta{margin-top:auto;padding:12px 4px;border-top:1px solid #dce4ed}.inspector-commit-meta p{margin:7px 0}
.rebased-inspector-tools{display:flex;gap:2px;margin:-8px -8px 8px;padding:5px 8px;border-bottom:1px solid #dce4ed}.rebased-inspector-tools button{border:0;background:transparent;font-size:16px}
.history-layout:has(.rebased-history)>.history-editor-pane{grid-column:2;grid-row:1;display:none}
.history-layout:has(.rebased-history):has(.history-editor-pane .history-editor)>.history-editor-pane{display:flex;background:#fff;z-index:1}
.history-layout:has(.rebased-history):has(.history-editor-pane .history-editor)>.history-bottom .history-log{display:none}
.history-layout:has(.rebased-history):has(.history-editor-pane .history-editor) .rebased-branches-strip{display:none}
.history-layout:has(.rebased-history)>.history-bottom{grid-column:1 / 3;grid-row:1;display:grid;grid-template-columns:var(--rebased-left-width) minmax(390px,1fr);border-right:1px solid #dce4ed}
.rebased-column-splitter{position:absolute;z-index:3;top:0;bottom:0;width:7px;cursor:col-resize}.rebased-left-splitter{left:calc(var(--rebased-left-width) - 3px)}.rebased-right-splitter{right:calc(var(--rebased-right-width) - 3px)}.rebased-column-splitter:hover{background:rgba(83,143,214,.35)}
.history-bottom:has(.rebased-commit-panel){position:relative}.rebased-branches-strip{position:absolute;z-index:4;top:7px;left:calc(var(--rebased-left-width) + 8px);padding:5px 8px;border:0;border-radius:3px;background:transparent;color:#718196;font-size:10px}.rebased-branches-strip:hover{background:#eaf2fd}
.rebased-commit-panel{grid-column:1;grid-row:1;display:flex;flex-direction:column;min-height:0;border-right:1px solid #dce4ed}.rebased-panel-tabs{display:flex;gap:6px;padding:8px;border-bottom:1px solid #dce4ed}.rebased-changes{flex:1;min-height:0;overflow:auto;padding:10px}.rebased-changes label{display:block;margin:7px 0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.rebased-commit-form{display:flex;flex-direction:column;gap:8px;padding:10px;border-top:1px solid #dce4ed}.rebased-commit-form textarea{min-height:90px;resize:vertical}.rebased-commit-form>div{display:flex;gap:5px}.rebased-commit-form .rebased-amend{display:flex;align-items:center;gap:6px;margin:0;color:#929aa5;font-size:11px}
.rebased-commit-tools{display:flex;gap:2px;padding:5px 8px;border-bottom:1px solid #dce4ed}.rebased-commit-tools button{border:0;background:transparent;font-size:17px}.rebased-changes details{margin:4px 0 10px}.rebased-changes summary{cursor:pointer;font-weight:600}.rebased-changes summary small{font-weight:400;color:#718196}.rebased-file-row{display:flex;align-items:center;gap:6px;min-width:0;padding:3px 6px 3px 19px}.rebased-file-row button{flex:1;min-width:0;overflow:hidden;text-align:left;text-overflow:ellipsis;white-space:nowrap;border:0;background:transparent}.rebased-file-row button.active{background:#dcebf9}.rebased-changes>button{display:block;width:100%;overflow:hidden;text-overflow:ellipsis;text-align:left;white-space:nowrap}
.history-layout:has(.rebased-history) .history-branches{display:none}
.history-layout:has(.rebased-history) .history-log{grid-column:2;grid-row:1;min-height:0}
.rebased-log-tabs{display:flex;align-items:center;gap:8px;margin:-8px -8px 8px;padding:6px 10px 6px 88px;min-height:34px;border-bottom:1px solid #dce4ed;background:#f7f9fc}.rebased-log-tabs button{border:0;background:transparent;font-weight:600}.rebased-log-tabs button[aria-selected=true]{border-bottom:2px solid #538fd6}.rebased-log-menu{font-size:18px}
.rebased-workspace-nav{min-height:38px;border-bottom:1px solid #dce4ed}.rebased-workspace-nav>button{border:0;background:transparent}.rebased-workspace-nav .update-strategy{display:flex;align-items:center;gap:6px}
.rebased-history-filters{position:relative;display:grid;grid-template-columns:minmax(150px,1fr) auto minmax(90px,130px) minmax(90px,130px) minmax(110px,160px) auto auto;align-items:center;margin-bottom:5px}.history-branch-filter{position:relative}.history-branch-filter>button{max-width:180px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.history-filter-menu{position:absolute;z-index:10;top:calc(100% + 3px);left:0;display:grid;min-width:220px;max-height:320px;overflow:auto;padding:5px;border:1px solid #c6d3e0;border-radius:5px;background:#fff;box-shadow:0 8px 22px rgba(30,55,80,.18)}.history-filter-menu button{text-align:left;border:0;background:transparent}.history-filter-menu button:hover,.history-filter-menu button[aria-pressed=true]{background:#eaf2fd}
.history-branch-actions{display:grid;grid-template-columns:1fr 1fr;gap:4px;margin-top:5px;padding-top:6px;border-top:1px solid #dce4ed}.history-branch-actions input{grid-column:1 / 3;min-width:0}.history-branch-actions button:last-child{grid-column:1 / 3}
.rebased-commit-row{display:grid;grid-template-columns:58px minmax(0,1fr) minmax(90px,18%) minmax(120px,19%);gap:8px;min-height:28px;padding:2px 7px}.rebased-commit-row .commit-subject,.rebased-commit-row .commit-author,.rebased-commit-row .commit-time{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.rebased-commit-row .commit-author,.rebased-commit-row .commit-time{color:#718196;font-size:11px}
.history-layout:has(.rebased-history)>.history-vertical-splitter,.history-layout:has(.rebased-history)>.history-horizontal-splitter{display:none}
.rebased-appbar{position:relative;display:flex;align-items:center;gap:3px;min-height:39px;padding:0 7px;border-bottom:1px solid #363b43;background:#181b20;color:#c7cbd1}.rebased-appbar button,.rebased-appbar select{height:29px;border:0;border-radius:3px;background:transparent;color:inherit}.rebased-appbar button:hover,.rebased-appbar button[aria-pressed=true],.rebased-appbar select:hover{background:#2b3038}.rebased-product-menu{font-size:17px}.rebased-repo-picker{display:flex;align-items:center;gap:3px}.rebased-repo-picker>span{display:grid;width:22px;height:22px;place-items:center;border-radius:4px;background:#303843;color:#9fc4f7;font-weight:700}.rebased-repo-picker select{max-width:180px;font-weight:600}.rebased-top-branch-wrap{position:relative;display:inline-flex}.rebased-top-branch-menu{top:32px;left:0}.rebased-product-title{display:flex;flex-direction:column;align-items:center;margin:auto;color:#9aa1aa;font-size:10px;line-height:1.05}.rebased-product-title small{font-size:9px}.rebased-more-workspaces{max-width:108px}
.rebased-empty-project{display:grid;min-height:calc(100% - 40px);place-content:center;justify-items:center;gap:10px;padding:40px;text-align:center;background:#1b1e23;color:#cbd0d7}.rebased-empty-project-icon{display:grid;width:54px;height:54px;place-items:center;border:1px solid #3f4854;border-radius:12px;background:#252a31;color:#9fc4f7;font-size:24px;font-weight:700}.rebased-empty-project p{max-width:560px;margin:0;color:#929aa5}.rebased-empty-project div{display:flex;gap:8px}.rebased-empty-project button{padding:7px 12px;border:1px solid #3f4854;border-radius:4px;background:#252a31;color:#cbd0d7}.rebased-empty-project button:hover{background:#284f7d;color:#fff}
.rebased-product-theme{--git-bg:#1b1e23;--git-surface:#23272d;--git-surface-muted:#1f2329;--git-border:#353b44;--git-border-soft:#2c3139;--git-text:#cbd0d7;--git-muted:#929aa5;--git-accent:#7faee8;--git-selected:#284f7d;--git-added:#183b2a;--git-removed:#482b32;background:var(--git-bg);color:var(--git-text)}
.rebased-product-theme .workspace,.rebased-product-theme .detail,.rebased-product-theme .files,.rebased-product-theme .commit-inspector,.rebased-product-theme .history-editor,.rebased-product-theme .history-bottom,.rebased-product-theme .rebased-commit-panel,.rebased-product-theme .history-log{background:var(--git-bg);border-color:var(--git-border);color:var(--git-text)}
.rebased-product-theme .rebased-panel-tabs,.rebased-product-theme .rebased-commit-tools,.rebased-product-theme .rebased-commit-form,.rebased-product-theme .inspector-commit-meta,.rebased-product-theme .rebased-inspector-tools,.rebased-product-theme .rebased-log-tabs{background:var(--git-surface-muted);border-color:var(--git-border)}
.rebased-product-theme button,.rebased-product-theme input,.rebased-product-theme select,.rebased-product-theme textarea{background:var(--git-surface);border-color:var(--git-border);color:var(--git-text)}
.rebased-product-theme button:hover,.rebased-product-theme button[aria-pressed=true],.rebased-product-theme .active,.rebased-product-theme .rebased-file-row button.active{background:var(--git-selected);color:#fff}.rebased-product-theme .commit{background:transparent;border-color:var(--git-border-soft);color:var(--git-text)}.rebased-product-theme .commit:hover{background:#252a31}.rebased-product-theme .commit.active{background:var(--git-selected)}
.rebased-product-theme .history-filter-menu,.rebased-product-theme .commit-action-menu,.rebased-product-theme .branch-action-menu,.rebased-product-theme .branch-widget-menu{background:var(--git-surface);border-color:var(--git-border);box-shadow:0 8px 22px #0009}.rebased-product-theme .history-filter-menu button,.rebased-product-theme .commit-action-menu button,.rebased-product-theme .branch-action-menu button,.rebased-product-theme .branch-widget-menu button{background:transparent;border-color:transparent}.rebased-product-theme .history-filter-menu button:hover{background:var(--git-selected)}
.rebased-product-theme .rebased-column-splitter:hover{background:#46688e}.rebased-product-theme .rebased-branches-strip{color:var(--git-muted)}.rebased-product-theme .history-empty,.rebased-product-theme .hint,.rebased-product-theme .rebased-commit-row .commit-author,.rebased-product-theme .rebased-commit-row .commit-time{color:var(--git-muted)}.rebased-product-theme .history-editor-toolbar{background:var(--git-surface-muted);border-color:var(--git-border)}
.rebased-product-theme .history-editor .history-patch{width:100%;height:100%;max-height:none;background:#20252d;color:#dce5f0;font:12px/1.65 ui-monospace,SFMono-Regular,Consolas,monospace;tab-size:2}
.rebased-product-theme .history-editor .split-row{min-height:23px;border-bottom:1px solid #292f38}
.rebased-product-theme .history-editor .split-row pre{min-width:0;overflow-x:auto;padding:3px 10px;background:#20252d;color:#dce5f0;border-right:1px solid #3d4652;white-space:pre;overflow-wrap:normal}
.rebased-product-theme .history-editor .split-row pre.added{background:#1b3a2d;color:#d8f2e2}
.rebased-product-theme .history-editor .split-row pre.removed{background:#4a2c33;color:#ffe1e4}
.rebased-product-theme .history-editor textarea{background:#20252d;color:#dce5f0}
.rebased-product-theme,.rebased-product-theme :deep(*){scrollbar-width:thin;scrollbar-color:#505865 var(--git-bg)}
.rebased-product-theme :deep(*::-webkit-scrollbar){width:9px;height:9px}.rebased-product-theme :deep(*::-webkit-scrollbar-track){background:var(--git-bg)}.rebased-product-theme :deep(*::-webkit-scrollbar-thumb){border:2px solid var(--git-bg);border-radius:8px;background:#505865}
:global([data-theme="dark"]) .git-studio{--git-bg:#1d222a;--git-surface:#252b34;--git-surface-muted:#20262e;--git-border:#3b4653;--git-border-soft:#303944;--git-text:#d7e0ec;--git-muted:#9aa8b8;--git-accent:#8db9f2;--git-selected:#304c70;--git-added:#183b2a;--git-removed:#482b32;background:var(--git-bg);color:var(--git-text)}
:global([data-theme="dark"]) .git-studio .toolbar,
:global([data-theme="dark"]) .git-studio .context,
:global([data-theme="dark"]) .git-studio .history-editor-toolbar{background:var(--git-surface-muted);border-color:var(--git-border);color:var(--git-text)}
:global([data-theme="dark"]) .git-studio .workspace,
:global([data-theme="dark"]) .git-studio .detail,
:global([data-theme="dark"]) .git-studio .files,
:global([data-theme="dark"]) .git-studio .commit-inspector,
:global([data-theme="dark"]) .git-studio .history-editor,
:global([data-theme="dark"]) .git-studio .history-bottom{background:var(--git-bg);border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio .rebased-panel-tabs,
:global([data-theme="dark"]) .git-studio .rebased-commit-tools,
:global([data-theme="dark"]) .git-studio .rebased-commit-form,
:global([data-theme="dark"]) .git-studio .inspector-commit-meta,
:global([data-theme="dark"]) .git-studio .rebased-inspector-tools{border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio .rebased-log-tabs{background:var(--git-surface-muted);border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio .history-filter-menu{background:var(--git-surface);border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio .history-filter-menu button:hover,
:global([data-theme="dark"]) .git-studio .history-filter-menu button[aria-pressed="true"]{background:var(--git-selected)}
:global([data-theme="dark"]) .git-studio .rebased-file-row button.active{background:var(--git-selected)}
:global([data-theme="dark"]) .git-studio .rebased-branches-strip:hover{background:var(--git-selected)}
:global([data-theme="dark"]) .git-studio .history-branches{background:var(--git-surface-muted);border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio button,
:global([data-theme="dark"]) .git-studio input,
:global([data-theme="dark"]) .git-studio select,
:global([data-theme="dark"]) .git-studio textarea{background:var(--git-surface);border-color:var(--git-border);color:var(--git-text)}
:global([data-theme="dark"]) .git-studio button:hover,
:global([data-theme="dark"]) .git-studio button[aria-pressed="true"],
:global([data-theme="dark"]) .git-studio .branch-item[aria-pressed="true"],
:global([data-theme="dark"]) .git-studio .active{background:var(--git-selected);color:#fff}
:global([data-theme="dark"]) .git-studio .commit{background:transparent;border-color:var(--git-border-soft);color:var(--git-text)}
:global([data-theme="dark"]) .git-studio .commit small,
:global([data-theme="dark"]) .git-studio .hint,
:global([data-theme="dark"]) .git-studio .notice{color:var(--git-muted)}
:global([data-theme="dark"]) .git-studio .added{background:var(--git-added)}
:global([data-theme="dark"]) .git-studio .removed{background:var(--git-removed)}
:global([data-theme="dark"]) .git-studio .history-empty{color:var(--git-muted)}
:global([data-theme="dark"]) .git-studio .history-horizontal-splitter,
:global([data-theme="dark"]) .git-studio .history-vertical-splitter{background:#2b333d;border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio .commit-action-menu,
:global([data-theme="dark"]) .git-studio .branch-action-menu,
:global([data-theme="dark"]) .git-studio .branch-widget-menu{background:var(--git-surface);border-color:var(--git-border);box-shadow:0 8px 22px #0008}
:global([data-theme="dark"]) .git-studio .commit-action-menu button,
:global([data-theme="dark"]) .git-studio .branch-action-menu button,
:global([data-theme="dark"]) .git-studio .branch-widget-menu button{background:transparent;border-color:transparent}
:global([data-theme="dark"]) .git-studio .conflict-editor section,
:global([data-theme="dark"]) .git-studio .conflict-hunk-list,
:global([data-theme="dark"]) .git-studio .compare-file-list,
:global([data-theme="dark"]) .git-studio .parent-selector{background:var(--git-surface-muted);border-color:var(--git-border)}
:global([data-theme="dark"]) .git-studio .conflict-preview.ours{background:#482b32}
:global([data-theme="dark"]) .git-studio .conflict-preview.theirs{background:#183b2a}
:global([data-theme="dark"]) .git-studio a{color:var(--git-accent)}
</style>
