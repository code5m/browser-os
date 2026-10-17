<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from 'vue';
import { bridge } from '../../../bridge';
import { redactSecrets } from '../../../utils/redact';
import { useRepoStore } from '../../workspace/public';
import { useGitStore } from '../public';
import { gitLifecycleSnapshot, registerGitLifecycleBinding } from '../lifecycle';
import { layoutCommitGraph } from '../model/logGraph';
import { parseUnifiedPatch } from '../model/patchView';
import { patchHunks, applyPatchSelection, blameLines, sourceIdentity, safeGithubLink } from '../model/studioPatch.mjs';

const props = defineProps<{path?:string;mode?:string;initialAction?:string;closeLabel?:string;searchId?:string}>();
const emit = defineEmits<{close:[]}>();
const repos = useRepoStore();
const git = useGitStore();
const localPath = ref(props.path || '');
const enteredPath = ref(props.path || '');
const source = computed<Record<string,unknown>|null>(() => localPath.value ? {path:localPath.value} : git.repoId ? {repo_id:git.repoId} : null);
const info = ref<any>(null), error = ref(''), notice = ref('');
const loading = ref(false), writeBusy = ref(false), preview = ref<any>(null), acknowledged = ref(false);
const selected = ref<string[]>([]), activeFile = ref(''), staged = ref(false);
function defaultPane() { return props.mode === 'branches' ? 'branches' : props.mode === 'history' || props.initialAction === 'history' ? 'log' : 'changes'; }
const pane = ref(defaultPane());
const panes = [['changes','改动'],['log','提交历史'],['branches','分支'],['stash','暂存架'],['worktrees','工作树'],['github','PR / CI']];
const patch = ref(''), file = ref<any>(null), conflict = ref<any>(null), resolution = ref('');
const diffMode = ref('split'), selectedLines = ref<number[]>([]);
const commits = ref<any[]>([]), offset = ref(0), hasMore = ref(false), search = ref(''), author = ref(''), revision = ref('');
const logSelection = ref(''), blame = ref<any[]>([]), rawDetail = ref('');
const branchTarget = ref(''), message = ref(''), stashTarget = ref(''), worktreeTarget = ref('');
const remote = ref<any>(null), pr = ref<number|null>(null), prText = ref(''), prDetail = ref<any>(null);
const stashes = ref<string[][]>([]), worktrees = ref('');
let generation = 0, detailGeneration = 0, disposed = false, requestCount = 0;
let previewSource = '';
const blocked = computed(() => writeBusy.value || loading.value);
const groups = computed(() => [
  {id:'conflict',name:'冲突',files:(info.value?.files||[]).filter((f:any)=>f.conflicted)},
  {id:'index',name:'已暂存（Index）',files:(info.value?.files||[]).filter((f:any)=>f.staged&&!f.conflicted)},
  {id:'worktree',name:'未暂存（Working Tree）',files:(info.value?.files||[]).filter((f:any)=>f.unstaged&&!f.conflicted)},
].filter(group=>group.files.length));
// Branches are the real local refs and remote-tracking refs from native git
// for-each-ref. Never infer remote branch names by querying GitHub instead.
const branchQuery = ref('');
const localRefs = computed<string[]>(() => (info.value?.branches || [])
  .map((row:string[]) => row[0]).filter((name:string) => name && !name.includes('/')));
const remoteRefs = computed<string[]>(() => (info.value?.branches || [])
  .map((row:string[]) => row[0]).filter((name:string) => name?.includes('/') && !name.endsWith('/HEAD')));
const visibleLocalRefs = computed(() => localRefs.value.filter(name => name.toLowerCase().includes(branchQuery.value.toLowerCase())));
const visibleRemoteRefs = computed(() => remoteRefs.value.filter(name => name.toLowerCase().includes(branchQuery.value.toLowerCase())));
const graph = computed(() => layoutCommitGraph(commits.value));
const parsed = computed(() => parseUnifiedPatch(patch.value));
const hunks = computed(() => { try { return patchHunks(patch.value); } catch { return []; } });
const scope = computed(() => props.path && info.value?.root && props.path.startsWith(info.value.root + '/') ? props.path.slice(info.value.root.length+1) : '');
const labels:Record<string,string> = {stage:'暂存',unstage:'取消暂存',discard:'丢弃未暂存改动',commit:'提交',amend:'修改未发布提交',index_edit:'应用选中的变更',branch_create:'新建分支',branch_delete:'删除已合并分支',checkout:'切换分支',fetch:'获取远端更新',pull:'拉取（仅快进）',push:'推送当前分支',merge:'合并分支',rebase:'变基未发布提交',cherry_pick:'挑选提交',stash_save:'保存全部改动到暂存架',stash_apply:'应用暂存',stash_pop:'应用并移除暂存',stash_drop:'删除暂存',resolve:'保存冲突解决结果',worktree_add:'新建工作树',worktree_remove:'移除干净工作树',pr_create:'创建 PR',pr_comment:'评论 PR',pr_approve:'批准 PR',pr_request_changes:'请求修改',pr_merge:'合并 PR',merge_continue:'继续合并',merge_abort:'中止合并',rebase_continue:'继续变基',rebase_abort:'中止变基',cherry_pick_continue:'继续挑选',cherry_pick_abort:'中止挑选'};
const friendly = (value:unknown) => {
  const text=redactSecrets(value).slice(0,300);
  const hints:Record<string,string>={GIT_STUDIO_OPERATION_IN_PROGRESS_USE_CONTINUE:'仓库正在合并、变基或挑选提交；请使用对应的继续操作，不能部分提交。',GIT_STUDIO_CHANGED_SINCE_PREVIEW:'预览后仓库已变化，请刷新并重新预览。',GIT_STUDIO_READ_BASIS_CHANGED:'文件自读取后已变化，请重新读取，不能覆盖较新的改动。',GIT_STUDIO_CLEAN_WORKTREE_REQUIRED:'工作区有改动或未完成操作，请先提交、保存暂存架或解决冲突。',GIT_STUDIO_SELECT_STAGED_FILES:'请勾选已暂存的文件；提交不会自动包含未暂存改动。',GIT_STUDIO_SELECT_PATHS:'请明确勾选需要操作的文件。',GIT_STUDIO_PREMERGE_NOT_EXECUTED:'完整验收未实际执行，不能合并。',GIT_STUDIO_CI_NOT_PASSED:'CI 缺失、运行中或未全部通过，不能合并。',GIT_STUDIO_GITHUB_AUTH_OR_API_FAILED:'请先在系统中完成 gh 登录，并确认有仓库权限。',GIT_STUDIO_BINARY_FILE:'二进制文件不支持文本编辑。',GIT_STUDIO_REPOSITORY_BUSY:'同一仓库已有操作在运行。'};
  return hints[text] || text;
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
  error.value='';
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
async function selectPane(name:string){if(writeBusy.value)return;pane.value=name;error.value='';rawDetail.value='';try{
  if(name==='log')await loadLog();
  if(name==='stash'){const data=await read({action:'stash'});if(data)stashes.value=data.text.trim().split('\n').filter(Boolean).map((s:string)=>s.split('\t'));}
  if(name==='worktrees'){const data=await read({action:'worktrees'});if(data)worktrees.value=data.text;}
  if(name==='github'){const data=await read({action:'github'});if(data)remote.value=data;}
}catch(e){error.value=friendly(e);}}
async function loadLog(next=offset.value){if(!source.value)return;offset.value=next;try{const data=await read({action:'log',offset:next,search:search.value||null,author:author.value||null,revision:revision.value||null,paths:scope.value?[scope.value]:[]});if(data){commits.value=data.commits;hasMore.value=data.has_more;}}catch(e){error.value=friendly(e);}}
async function selectBranchFromTree(name:string) {
  if(blocked.value)return;
  revision.value=name; branchTarget.value=name; pane.value='log';
  offset.value=0; logSelection.value=''; rawDetail.value=''; commits.value=[];
  await loadLog(0);
}
async function selectCommit(oid:string){logSelection.value=oid;try{const data=await read({action:'commit_diff',revision:oid});if(data)rawDetail.value=data.patch;}catch(e){error.value=friendly(e);}}
async function compare(){try{const data=await read({action:'compare',revision:'HEAD',other:branchTarget.value});if(data)rawDetail.value=data.patch;}catch(e){error.value=friendly(e);}}
async function inspectFile(kind:string){if(!activeFile.value)return;try{const data=await read({action:kind,paths:[activeFile.value]});if(!data)return;if(kind==='blame'){blame.value=blameLines(data.text);diffMode.value='blame';}else if(kind==='conflict'){conflict.value=data;resolution.value=data.ours;diffMode.value='conflict';}else{commits.value=data.commits;hasMore.value=data.has_more;offset.value=0;pane.value='log';}}catch(e){error.value=friendly(e);}}
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
async function cancelPreview(){const job=preview.value;preview.value=null;acknowledged.value=false;if(job){try{await bridge.gitStudioCancel(job.job_id);}catch{ /* Expired tickets are already unusable. */ }}}
async function prepare(action:string,extras:Record<string,unknown>={}) {
  if(blocked.value||preview.value)return;
  if(!labels[action])return;
  if(['stage','unstage','discard','commit','amend'].includes(action)&&!selected.value.length){error.value='请先勾选文件';return;}
  if(['commit','amend'].includes(action)&&!message.value.trim()){error.value='提交说明不能为空';return;}
  const value=currentSource(),key=sourceIdentity(value);loading.value=true;error.value='';
  try{
    const next=await bridge.gitStudioPreview(value,{action,paths:selected.value,message:message.value||null,target:branchTarget.value||null,...extras});
    if(disposed||!source.value||key!==sourceIdentity(source.value)){await bridge.gitStudioCancel(next.job_id);return;}
    previewSource=key;preview.value=next;acknowledged.value=false;
  }catch(e){error.value=friendly(e);}finally{loading.value=requestCount>0;}
}
async function confirm(){
  const job=preview.value;
  if(!job||writeBusy.value||!acknowledged.value||!source.value||sourceIdentity(source.value)!==previewSource)return;
  writeBusy.value=true;error.value='';preview.value=null;
  try{
    const result=await bridge.gitStudioConfirm(job.job_id,true);
    notice.value=result.success?'操作完成；已重新读取真实仓库状态。':'操作未完成，请查看仓库状态；不要重复执行写入。';
    if(!result.success)error.value=friendly(result.message);
  }catch(e){error.value=friendly(e);}finally{writeBusy.value=false;acknowledged.value=false;if(disposed)unregister();else{await refresh();if(activeFile.value)await openFile(activeFile.value);}}
}
async function inspectStash(target:string){stashTarget.value=target;try{const result=await read({action:'stash_diff',target});if(result)rawDetail.value=result.patch;}catch(e){error.value=friendly(e);}}
async function inspectPr(number:number){pr.value=number;try{const result=await read({action:'pr_detail',pr:number});if(result)prDetail.value=result;}catch(e){error.value=friendly(e);}}
function prAction(action:string){void prepare(action,{pr:pr.value,message:prText.value,target:branchTarget.value});}
function selectRepo(id:string){if(writeBusy.value)return;localPath.value='';void git.selectRepo(id||null);}
function openLocal(){if(writeBusy.value||!enteredPath.value.trim())return;localPath.value=enteredPath.value.trim();}
// Repository-bound targets and displayed results must never follow a new source.
// Invalidate synchronously so an action in the same event cannot reuse old state.
function resetSourceContext() {
  generation++; detailGeneration++;
  void cancelPreview(); previewSource='';
  pane.value=defaultPane();
  info.value=null; selected.value=[]; activeFile.value=''; patch.value='';
  file.value=null; rawDetail.value=''; conflict.value=null; resolution.value='';
  remote.value=null; prDetail.value=null; pr.value=null; prText.value='';
  branchTarget.value=''; stashTarget.value=''; worktreeTarget.value='';
  revision.value=''; logSelection.value=''; message.value='';
  commits.value=[]; stashes.value=[]; worktrees.value=''; blame.value=[];
  selectedLines.value=[]; offset.value=0; error.value=''; notice.value='';
}
const unregister=registerGitLifecycleBinding({canSuspend:()=>!writeBusy.value,cleanup:resetSourceContext});
watch(()=>props.path,path=>{if(!writeBusy.value){localPath.value=path||'';enteredPath.value=path||'';}});
watch(()=>props.mode,()=>{if(!writeBusy.value) { pane.value=defaultPane(); if(pane.value==='log') void loadLog(0); } });
watch(()=>source.value?sourceIdentity(source.value):'',()=>{resetSourceContext();void refresh();},{immediate:true,flush:'sync'});
onUnmounted(()=>{disposed=true;generation++;detailGeneration++;void cancelPreview();if(!writeBusy.value)unregister();});
</script>
<template>
<section class="git-studio" aria-label="统一 Git 工作台">
  <header class="toolbar"><strong>Git Studio</strong><select :value="localPath?'':git.repoId||''" :disabled="writeBusy" aria-label="配置仓库" @change="selectRepo(($event.target as HTMLSelectElement).value)"><option value="">选择配置仓库</option><option v-for="item in repos.repos" :key="item.id" :value="item.id">{{item.name}}</option></select><input v-model="enteredPath" :disabled="writeBusy" aria-label="原始本地仓库路径" placeholder="已授权的本地目录或文件" @keydown.enter.prevent="openLocal"/><button :disabled="blocked||!enteredPath.trim()" @click="openLocal">打开本地仓库</button><button :disabled="blocked||!source" @click="refresh">刷新</button><button v-if="closeLabel" :disabled="writeBusy" @click="emit('close')">{{closeLabel}}</button></header>
  <p class="context"><b>{{info?.branch||'未选择仓库'}}</b> · {{info?.root||'直接使用原仓库，不自动克隆'}} <span v-if="scope"> · 初始范围：{{scope}}</span><span v-if="info?.state&&info.state!=='Clean'" class="warning"> · {{info.state}} 待处理</span></p>
  <nav class="toolbar" aria-label="Git 工作台视图"><button v-for="[id,label] in panes" :key="id" :aria-pressed="pane===id" :disabled="writeBusy||!info" @click="selectPane(id)">{{label}}</button><span class="grow"/><button v-for="op in ['fetch','pull','push']" :key="op" :disabled="blocked||!info||preview" @click="prepare(op)">{{labels[op]}}</button></nav>
  <p v-if="error" role="alert" class="error">{{error}}</p><p v-if="notice" role="status">{{notice}}</p><p v-if="loading" role="status">正在读取或生成预览…</p><p v-if="writeBusy" role="status">正在执行已确认的操作；此时不能切换仓库或停用 Git。</p>
  <section v-if="preview" class="preview" aria-label="Git 写入预览"><strong>确认{{labels[preview.action]||preview.action}}</strong><p>{{preview.root}} · {{preview.target||''}} · {{preview.scope==='whole_worktree_including_untracked'?'整个仓库，包含未跟踪文件':'仅明确指定的操作范围'}}</p><p>有效期 {{preview.expires_in_seconds}} 秒；HEAD、暂存区或工作区变化后拒绝执行。</p><pre v-if="preview.paths?.length">{{preview.paths.join('\n')}}</pre><p v-if="preview.message">{{preview.message}}</p><label><input v-model="acknowledged" type="checkbox"/>我已检查目标、范围和改动，确认执行。</label><button :disabled="writeBusy||!acknowledged" @click="confirm">确认执行</button><button :disabled="writeBusy" @click="cancelPreview">取消预览</button></section>
  <div v-if="info" class="workspace">
    <template v-if="pane==='changes'">
      <aside class="files"><div class="toolbar"><strong>文件</strong><button :disabled="blocked" @click="selected=[]">清空选择</button></div><p class="hint">同一文件可同时有已暂存和未暂存改动。</p><section v-for="group in groups" :key="group.id"><h4>{{group.name}}（{{group.files.length}}）</h4><label v-for="entry in group.files" :key="entry.path" class="file-row" :class="{active:activeFile===entry.path}"><input type="checkbox" :checked="selected.includes(entry.path)" :disabled="writeBusy" @change="toggle(entry.path)"/><button :title="entry.path" :disabled="blocked" @click="openFile(entry.path,group.id==='index')">{{entry.path}}</button><small>{{entry.index}}{{entry.worktree}}</small></label></section><p v-if="!groups.length">工作区干净。</p></aside>
      <main class="detail"><div class="toolbar"><strong>{{activeFile||'选择文件查看差异'}}</strong><button :disabled="blocked||!activeFile" @click="openFile(activeFile,!staged)">{{staged?'HEAD → Index':'Index → Working Tree'}}</button><select v-model="diffMode" aria-label="差异视图"><option value="split">并排变更</option><option value="patch">补丁</option><option value="full">全文三版本</option><option value="partial">选择代码块 / 行</option><option v-if="blame.length" value="blame">行级归属</option><option v-if="conflict" value="conflict">三方冲突</option></select><button :disabled="blocked||!activeFile" @click="inspectFile('blame')">Blame</button><button :disabled="blocked||!activeFile" @click="inspectFile('follow')">跟随重命名历史</button><button :disabled="blocked||!activeFile" @click="inspectFile('conflict')">解决冲突</button></div>
        <div v-if="diffMode==='split'" class="patch"><div v-for="(row,index) in parsed.rows" :key="index" class="split-row"><pre :class="{removed:row.removed}">{{row.beforeLine??''}} {{row.before}}</pre><pre :class="{added:row.added}">{{row.afterLine??''}} {{row.after}}</pre></div><p v-if="parsed.truncated">显示达到 4000 行上限，内容不完整。</p></div>
        <pre v-else-if="diffMode==='patch'" class="patch">{{patch||'没有可显示的文本补丁。'}}</pre>
        <div v-else-if="diffMode==='full'&&file" class="three"><label v-for="version in ['head','index','worktree']" :key="version">{{version}}<textarea :value="file[version]" readonly spellcheck="false"/></label></div>
        <div v-else-if="diffMode==='partial'" class="patch"><p>只改变暂存区，不覆盖工作区；每个选中的增删行将共同组成新的暂存内容。</p><section v-for="hunk in hunks" :key="hunk.id"><button :disabled="blocked" @click="hunkSelect(hunk)">选择 / 取消此代码块</button><label v-for="change in hunk.lines" :key="change.id" class="patch-line" :class="{added:change.sign==='+',removed:change.sign==='-'}"><input v-if="change.sign!==' '" type="checkbox" :checked="selectedLines.includes(change.id)" :disabled="blocked" @change="line(change.id)"/><code>{{change.sign}}{{change.text}}</code></label></section><button :disabled="blocked||preview||!selectedLines.length||!file" @click="partial">{{staged?'取消暂存选中行':'暂存选中行'}}并预览</button></div>
        <div v-else-if="diffMode==='blame'" class="patch"><div v-for="item in blame" :key="item.line" class="blame-line"><button :title="item.summary" @click="selectCommit(item.oid);pane='log'">{{item.oid.slice(0,8)}} · {{item.author}}</button><small>{{item.line}}</small><code>{{item.text}}</code></div></div>
        <div v-else-if="diffMode==='conflict'&&conflict"><div class="three"><label v-for="version in ['ancestor','ours','theirs']" :key="version">{{version}}<textarea :value="conflict[version]" readonly spellcheck="false"/></label></div><label>明确编辑最终结果<textarea v-model="resolution" class="resolution" spellcheck="false" maxlength="131072"/></label><button :disabled="blocked||preview" @click="prepare('resolve',{paths:[activeFile],content:resolution,expected_fingerprint:conflict.fingerprint})">预览解决结果</button></div>
      </main>
    </template>
    <template v-else-if="pane==='log'">
      <aside class="branch-tree" aria-label="Git 本地和远程分支">
        <header class="toolbar"><strong>分支</strong><span>{{localRefs.length}} 本地 / {{remoteRefs.length}} 远程</span></header>
        <input v-model="branchQuery" class="branch-search" type="search" aria-label="筛选分支" placeholder="搜索分支…"/>
        <button class="branch-item" :aria-pressed="!revision" :disabled="blocked" @click="selectBranchFromTree('')">全部分支</button>
        <details open><summary>本地（{{localRefs.length}}）</summary>
          <button v-for="name in visibleLocalRefs" :key="name" class="branch-item" :aria-pressed="revision===name" :disabled="blocked" @click="selectBranchFromTree(name)">{{name}}{{name===info.branch?' · HEAD':''}}</button>
        </details>
        <details open><summary>远程（{{remoteRefs.length}}）</summary>
          <button v-for="name in visibleRemoteRefs" :key="name" class="branch-item" :aria-pressed="revision===name" :disabled="blocked" @click="selectBranchFromTree(name)">{{name}}</button>
        </details>
        <p class="hint">远程显示本机已有的跟踪分支；点击「获取远端更新」后刷新。</p>
      </aside>
      <section class="git-log-commits" aria-label="选定分支的提交历史">
        <form class="filters" @submit.prevent="loadLog(0)"><input v-model="search" placeholder="提交说明" aria-label="提交说明"/><input v-model="author" placeholder="作者" aria-label="作者"/><button :disabled="blocked">检索历史</button></form>
        <p class="context">历史：{{revision||'全部分支'}}</p>
        <button v-for="(commit,index) in commits" :key="commit.oid" class="commit" :class="{active:logSelection===commit.oid}" @click="selectCommit(commit.oid)"><svg viewBox="0 0 150 32" aria-hidden="true"><line v-for="lane in graph[index]?.through||[]" :key="lane" :x1="10+lane*14" y1="0" :x2="10+lane*14" y2="32" stroke="currentColor"/><line v-for="(lane,i) in graph[index]?.parentLanes||[]" :key="'p'+i" :x1="10+(graph[index]?.lane||0)*14" y1="16" :x2="10+lane*14" y2="32" stroke="currentColor"/><circle :cx="10+(graph[index]?.lane||0)*14" cy="16" r="4" fill="currentColor"/></svg><span>{{commit.summary}}<small>{{commit.oid.slice(0,9)}} · {{commit.author}}</small></span></button>
        <p v-if="!commits.length&&!loading">没有符合条件的提交。</p>
        <div class="toolbar"><button :disabled="blocked||offset===0" @click="loadLog(Math.max(0,offset-50))">上一页</button><span>{{Math.floor(offset/50)+1}}</span><button :disabled="blocked||!hasMore" @click="loadLog(offset+50)">下一页</button></div>
      </section>
      <main class="detail log-details"><div class="toolbar"><code>{{logSelection}}</code><button :disabled="blocked||preview||!logSelection" @click="prepare('cherry_pick',{target:logSelection})">挑选此提交</button></div><pre class="patch">{{rawDetail||'选择提交查看差异。'}}</pre></main>
    </template>
    <template v-else-if="pane==='branches'"><aside class="files"><button v-for="item in info.branches" :key="item[0]" class="commit" @click="branchTarget=item[0]"><span>{{item[0]}}<small>{{item[1]}} {{item[2]}}</small></span></button></aside><main class="detail"><div class="toolbar"><input v-model="branchTarget" placeholder="分支名称或提交" aria-label="目标分支"/><button :disabled="blocked||!branchTarget" @click="compare">与 HEAD 比较</button></div><div class="toolbar"><button v-for="op in ['branch_create','checkout','branch_delete','merge','rebase']" :key="op" :disabled="blocked||preview||!branchTarget" @click="prepare(op)">{{labels[op]}}</button></div><p>删除分支仅允许安全删除；变基与 Amend 不改写已发布提交。Pull 只允许快进。</p><pre class="patch">{{rawDetail}}</pre></main></template>
    <template v-else-if="pane==='stash'"><aside class="files"><button v-for="item in stashes" :key="item[0]" class="commit" @click="inspectStash(item[0])">{{item[0]}} · {{item[2]}}</button><p v-if="!stashes.length">没有保存的暂存记录。</p></aside><main class="detail"><div class="toolbar"><input v-model="stashTarget" placeholder="stash@{0}" aria-label="暂存记录"/><button v-for="op in ['stash_save','stash_apply','stash_pop','stash_drop']" :key="op" :disabled="blocked||preview||op!=='stash_save'&&!stashTarget" @click="prepare(op,{target:stashTarget})">{{labels[op]}}</button></div><p>保存暂存架会收纳整个仓库的已跟踪与未跟踪改动，不局限于右键文件。</p><pre class="patch">{{rawDetail}}</pre></main></template>
    <main v-else-if="pane==='worktrees'" class="detail"><p>创建受控子目录工作树（分离 HEAD）；移除仅允许已登记且干净的子工作树，不强制删除。</p><div class="toolbar"><input v-model="worktreeTarget" aria-label="工作树子目录" placeholder="新子目录名"/><input v-model="revision" aria-label="工作树基线" placeholder="基线分支 / 提交，默认 HEAD"/><button :disabled="blocked||preview||!worktreeTarget" @click="prepare('worktree_add',{target:worktreeTarget,revision:revision||'HEAD'})">创建</button><button :disabled="blocked||preview||!worktreeTarget" @click="prepare('worktree_remove',{target:worktreeTarget})">安全移除</button></div><pre>{{worktrees}}</pre></main>
    <template v-else-if="pane==='github'"><aside class="files"><p>使用系统中已登录的 gh；BrowserOS 不接收登录凭据。</p><button v-for="item in remote?.prs||[]" :key="item.number" class="commit" @click="inspectPr(item.number)"><span>#{{item.number}} {{item.title}}<small>{{item.isDraft?'草稿':item.mergeStateStatus}} · {{item.reviewDecision}}</small></span></button></aside><main class="detail"><div class="toolbar"><input v-model.number="pr" type="number" min="1" placeholder="PR 编号" aria-label="PR 编号"/><input v-model="branchTarget" placeholder="创建 PR 的目标分支" aria-label="PR 目标分支"/></div><textarea v-model="prText" maxlength="8000" placeholder="PR 标题 / 说明 / 评论 / 审查意见" aria-label="PR 正文"/><div class="toolbar"><button v-for="op in ['pr_create','pr_comment','pr_approve','pr_request_changes','pr_merge']" :key="op" :disabled="blocked||preview||op!=='pr_create'&&!pr" @click="prAction(op)">{{labels[op]}}</button></div><details v-if="prDetail" open><summary>PR 详情、文件和审查</summary><pre>{{JSON.stringify(prDetail,null,2)}}</pre></details><h4>近期 CI</h4><div v-for="run in remote?.runs||[]" :key="run.databaseId" class="toolbar"><a v-if="safeGithubLink(run.url)" :href="safeGithubLink(run.url)" target="_blank" rel="noopener noreferrer">{{run.workflowName}}</a><span>{{run.conclusion||run.status}}</span><code>{{run.headSha?.slice(0,9)}}</code></div></main></template>
  </div>
  <footer v-if="info" class="footer"><template v-if="pane==='changes'"><div class="toolbar"><span>明确选择 {{selected.length}} 个文件</span><button v-for="op in ['stage','unstage','discard']" :key="op" :disabled="blocked||preview||!selected.length" @click="prepare(op)">{{labels[op]}}</button><input v-model="message" maxlength="2048" placeholder="提交说明" aria-label="提交说明"/><button :disabled="blocked||preview||!selected.length||!message.trim()" @click="prepare('commit')">提交选中暂存内容</button><button :disabled="blocked||preview||!selected.length||!message.trim()" @click="prepare('amend')">Amend（未发布）</button></div></template><div v-if="info.state!=='Clean'" class="toolbar"><span>正在进行：{{info.state}}</span><button v-for="op in ['merge_continue','merge_abort','rebase_continue','rebase_abort','cherry_pick_continue','cherry_pick_abort']" :key="op" :disabled="blocked||preview" @click="prepare(op)">{{labels[op]}}</button></div></footer>
</section>
</template>
<style scoped>
.git-studio{display:flex;flex-direction:column;height:100%;min-height:0;overflow:hidden;background:var(--surface,#fff);color:var(--text,#273a50);font-size:12px}.toolbar{display:flex;flex-wrap:wrap;align-items:center;gap:7px;padding:7px}.toolbar input{min-width:110px;flex:1}.grow{flex:1}.context{margin:0;padding:7px 12px;overflow-wrap:anywhere;border-bottom:1px solid #dce4ed}.workspace{display:flex;flex:1;min-height:180px;overflow:hidden}.branch-tree{flex:0 0 clamp(160px,20%,260px);min-width:0;overflow:auto;border-right:1px solid #dce4ed;padding:6px}.branch-tree details{padding:6px}.branch-tree summary{cursor:pointer;font-weight:600}.branch-search{box-sizing:border-box;width:100%}.branch-item{display:block;text-align:left;width:100%;border:0;padding:5px 8px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.branch-item[aria-pressed=true]{background:#dcebf9;font-weight:600}.git-log-commits{flex:0 0 37%;min-width:180px;overflow:auto;border-right:1px solid #dce4ed;padding:6px}.log-details{flex:1}.files{flex:0 0 clamp(210px,28%,360px);overflow:auto;resize:horizontal;border-right:1px solid #dce4ed;padding:8px}.detail{flex:1;min-width:0;overflow:auto;padding:8px}.files h4{margin:12px 0 6px}.file-row{display:flex;align-items:center;gap:4px;padding:4px}.file-row button{flex:1;text-align:left;overflow-wrap:anywhere;border:0;background:transparent}.active{background:#eaf2fd}.filters{display:grid;gap:5px}.commit{display:flex;align-items:center;width:100%;text-align:left;gap:5px;padding:8px;border:0;border-bottom:1px solid #e6edf4;background:transparent}.commit small{display:block;font-size:10px;color:#66778b}.commit svg{width:58px;min-width:58px;height:32px}.patch{overflow:auto;font:12px/1.7 ui-monospace,monospace;white-space:pre}.split-row{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr)}.split-row pre{margin:0;padding:1px 5px;overflow:auto;border-right:1px solid #dce4ed}.removed{background:#fff0f0}.added{background:#ebf8ee}.patch-line{display:flex;gap:6px;min-height:22px}.patch-line code{white-space:pre}.three{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}.three textarea{height:300px;width:100%;box-sizing:border-box;white-space:pre;font-family:ui-monospace,monospace}.resolution{display:block;min-height:180px;width:100%;box-sizing:border-box;font-family:ui-monospace,monospace}.blame-line{display:flex;gap:8px;white-space:pre}.blame-line button{width:220px;text-align:left;overflow:hidden;text-overflow:ellipsis}.preview{border:1px solid #d9b15b;background:#fff8e5;padding:10px;margin:7px;max-height:260px;overflow:auto}.preview pre{max-height:90px;overflow:auto}.preview label{display:inline-flex;gap:5px;margin-right:10px}.error{color:#9a2929;background:#fff1f1;padding:9px}.hint,.warning{color:#80602c}.footer{border-top:1px solid #dce4ed}button,select,input,textarea{font:inherit}button{cursor:pointer;border:1px solid #c6d3e0;border-radius:5px;padding:5px 8px;background:#fff;color:inherit}button:disabled{opacity:.45;cursor:not-allowed}button[aria-pressed=true]{background:#e6effc}input,select,textarea{border:1px solid #c6d3e0;border-radius:4px;padding:5px}pre{white-space:pre-wrap;overflow-wrap:anywhere}p{margin:6px 10px}a{color:#235ca1}
</style>
