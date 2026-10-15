<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { bridge } from "../../../bridge";
import { redactSecrets } from "../../../utils/redact";

type Action = "stage" | "commit" | "checkout" | "fetch" | "pull" | "stash" | "resolve" | "github_merge";
const props = defineProps<{ path: string; initialAction?: Action }>();
const emit = defineEmits<{ updated: [] }>();
type Summary = Awaited<ReturnType<typeof bridge.gitLocalInspect>>;
type Preview = Awaited<ReturnType<typeof bridge.gitLocalWriteRequest>>;
type Conflict = Awaited<ReturnType<typeof bridge.gitLocalConflict>>;
type Follow = Awaited<ReturnType<typeof bridge.gitLocalFollow>>;
type Blame = Awaited<ReturnType<typeof bridge.gitLocalBlame>>;
type Github = Awaited<ReturnType<typeof bridge.gitLocalGithub>>;

const action = ref<Action>(props.initialAction ?? "stage");
const info = ref<Summary | null>(null);
const preview = ref<Preview | null>(null);
const acknowledged = ref(false);
const busy = ref(false);
const error = ref("");
const notice = ref("");
const message = ref("");
const branch = ref("");
const prNumber = ref<number | null>(null);
const conflict = ref<Conflict | null>(null);
const resolution = ref("");
const follow = ref<Follow | null>(null);
const blame = ref<Blame | null>(null);
const github = ref<Github | null>(null);
const detailBusy = ref(false);
const isFile = computed(() => !!info.value?.scope && !props.path.endsWith("/"));
const actions: { id: Action; title: string; scope: string }[] = [
  {id:"stage",title:"暂存",scope:"只操作右键选择的文件/目录"},
  {id:"commit",title:"提交",scope:"只提交选定路径内已暂存改动，不会自动添加未暂存文件"},
  {id:"checkout",title:"Checkout",scope:"必须干净工作区，禁止强制覆盖"},
  {id:"fetch",title:"Fetch",scope:"从 origin 更新远端跟踪引用"},
  {id:"pull",title:"Pull (仅 FF)",scope:"先 Fetch；分叉时拒绝自动合并"},
  {id:"stash",title:"Stash",scope:"保存整个仓库改动，包含未跟踪文件"},
  {id:"resolve",title:"三方冲突解决",scope:"只有冲突文件可写入手动合并内容"},
  {id:"github_merge",title:"合并 GitHub PR",scope:"校验 PR / Review / CI，再执行非强制 merge"},
];
const currentAction = computed(() => actions.find(a => a.id === action.value)!);
let generation = 0;
function describe(e:unknown) { return redactSecrets(e).slice(0,300); }
async function refresh() {
  const token = ++generation;
  info.value = null; error.value = ""; preview.value = null;
  try {
    const result = await bridge.gitLocalInspect(props.path);
    if(token !== generation) return;
    info.value = result;
    branch.value = result.local_branches.find(b => b === result.branch) ?? result.local_branches[0] ?? "";
  } catch(e) { if(token === generation) error.value = describe(e); }
}
async function prepare() {
  if(busy.value || !info.value) return;
  busy.value = true; error.value = ""; notice.value = ""; preview.value = null; acknowledged.value = false;
  try {
    preview.value = await bridge.gitLocalWriteRequest(props.path,action.value,{
      message:action.value==="commit"?message.value:undefined,
      branch:action.value==="checkout"?branch.value:undefined,
      resolved:action.value==="resolve"?resolution.value:undefined,
      pr:action.value==="github_merge"?prNumber.value ?? undefined:undefined,
    });
  } catch(e) { error.value = describe(e); }
  finally { busy.value = false; }
}
async function confirm() {
  const job = preview.value;
  if(!job || busy.value || (job.dangerous && !acknowledged.value)) return;
  // One-time server-side ticket. Re-check HEAD and index after the user's approval.
  busy.value=true;error.value="";preview.value=null;
  try {
    notice.value = await bridge.gitLocalWriteConfirm(job.job_id,job.dangerous && acknowledged.value);
    if(job.action==="commit") message.value="";
    await refresh();
    emit("updated");
  } catch(e) { error.value = describe(e); }
  finally { busy.value = false; acknowledged.value=false; }
}
async function loadConflict() {
  detailBusy.value=true;error.value="";conflict.value=null;
  try {
    conflict.value=await bridge.gitLocalConflict(props.path);
    resolution.value=conflict.value.ours;
    action.value="resolve";
  }catch(e){error.value=describe(e)}
  finally{detailBusy.value=false}
}
async function loadBlame() {
  detailBusy.value=true;error.value="";blame.value=null;
  try{blame.value=await bridge.gitLocalBlame(props.path)}
  catch(e){error.value=describe(e)} finally{detailBusy.value=false}
}
async function loadFollow() {
  detailBusy.value=true;error.value="";follow.value=null;
  try{follow.value=await bridge.gitLocalFollow(props.path)}
  catch(e){error.value=describe(e)} finally{detailBusy.value=false}
}
async function loadGithub() {
  detailBusy.value=true;error.value="";github.value=null;
  try{github.value=await bridge.gitLocalGithub(props.path)}
  catch(e){error.value=describe(e)} finally{detailBusy.value=false}
}
function choosePr(num:number){prNumber.value=num;action.value="github_merge";preview.value=null}
watch(() => [props.path,props.initialAction] as const,()=>{
  action.value=props.initialAction??"stage";preview.value=null;resolution.value="";github.value=null;
  follow.value=null;blame.value=null;conflict.value=null;void refresh();
},{immediate:true});
watch(action,()=>{preview.value=null;acknowledged.value=false;error.value="";});
onUnmounted(()=>{++generation;});
</script>

<template>
  <section class="local-workbench" aria-label="本地 Git 操作工作台">
    <header class="local-header">
      <strong>原仓库操作</strong>
      <span class="muted" v-if="info" :title="info.root">{{info.branch}} · {{info.scope || "整个仓库"}}</span>
      <button type="button" :disabled="busy" @click="refresh">刷新状态</button>
    </header>
    <p class="hint">所有写操作必须先预览，再确认；仅允许 BrowserOS 已授权的本机仓库。GitHub 协作需系统安装并登录 <code>gh</code>。</p>
    <p v-if="error" role="alert" class="error">{{error}}</p>
    <p v-if="notice" role="status" class="success">{{notice}}</p>
    <div v-if="info" class="local-summary">
      <span>已暂存 {{info.files.filter(f=>f.staged).length}} 项</span>
      <span>当前范围共 {{info.files.length}} 项改动{{info.truncated?"（已截断）":""}}</span>
      <span v-if="info.has_conflicts" class="error">存在冲突</span>
    </div>
    <div class="operation">
      <label>操作
        <select v-model="action" :disabled="busy">
          <option v-for="a in actions" :key="a.id" :value="a.id">{{a.title}}</option>
        </select>
      </label>
      <label v-if="action==='commit'">提交说明
        <input v-model="message" maxlength="2048" placeholder="必须填写提交信息" />
      </label>
      <label v-if="action==='checkout'">分支
        <select v-model="branch">
          <option v-for="b in info?.local_branches || []" :key="b" :value="b">{{b}}</option>
        </select>
      </label>
      <label v-if="action==='github_merge'">PR 编号
        <input v-model.number="prNumber" type="number" min="1" placeholder="PR #" />
      </label>
      <button v-if="action==='resolve' && !conflict" type="button" @click="loadConflict">读取三方版本</button>
      <button class="primary" :disabled="busy || !info || (action==='resolve'&&!conflict)"
        type="button" @click="prepare">预览{{currentAction.title}}</button>
      <small class="hint">{{currentAction.scope}}</small>
    </div>

    <div v-if="conflict" class="conflict-editor">
      <strong>冲突文件：{{conflict.path}}</strong>
      <div class="three-way">
        <label>共同祖先 <textarea :value="conflict.ancestor" readonly /></label>
        <label>本地版本 <textarea :value="conflict.ours" readonly /></label>
        <label>远端版本 <textarea :value="conflict.theirs" readonly /></label>
      </div>
      <label>合并结果（由你编辑，点击预览后才会允许写入）
        <textarea v-model="resolution" class="result-editor" maxlength="131072" />
      </label>
    </div>
    <div v-if="preview" class="preview" role="group" aria-label="待确认 Git 写入任务">
      <strong>操作预览 · {{preview.action}}</strong>
      <p>{{preview.summary}}</p>
      <small>确认令牌有效期 {{preview.expires_in_seconds}} 秒，且不可重放；仓库状态变化后会拒绝写入。</small>
      <label v-if="preview.dangerous" class="danger-ack">
        <input v-model="acknowledged" type="checkbox" />我确认本操作可能修改工作区或 GitHub PR
      </label>
      <div><button :disabled="busy || (preview.dangerous && !acknowledged)" @click="confirm">确认执行</button>
        <button :disabled="busy" @click="preview=null">取消</button></div>
    </div>
    <div class="local-tools">
      <button :disabled="detailBusy" @click="loadBlame">Blame</button>
      <button :disabled="detailBusy" @click="loadFollow">跨重命名历史</button>
      <button :disabled="detailBusy" @click="loadGithub">GitHub PR / CI</button>
    </div>
    <div v-if="blame" class="detail">
      <strong>Blame · {{blame.path}}</strong>
      <p v-if="blame.truncated">最多分析 4000 行，结果已截断。</p>
      <div v-for="(b,i) in blame.hunks" :key="i" class="line">{{b.start}}–{{b.start+b.lines-1}} · {{b.author}} · {{b.commit.slice(0,10)}}</div>
    </div>
    <div v-if="follow" class="detail">
      <strong>Rename-follow（限制 200 条 / 4000 次扫描）</strong>
      <div v-for="e in follow.entries" :key="e.oid" class="line">{{e.oid.slice(0,9)}} · {{e.summary}} · {{e.file_at_commit}}
        <small v-if="e.renamed_from">← {{e.renamed_from}}</small></div>
      <p v-if="follow.scan_limited">已达到资源限制；更早历史未展示。</p>
    </div>
    <div v-if="github" class="detail">
      <strong>{{github.repository}} · Pull Requests</strong>
      <p v-if="!github.prs.length">没有开放的 PR</p>
      <div v-for="p in github.prs" :key="p.number" class="line gh-entry">
        <span>#{{p.number}} {{p.title}} · Review: {{p.reviewDecision || "未要求"}} · {{p.mergeStateStatus}}</span>
        <a :href="p.url" target="_blank" rel="noopener noreferrer">查看</a>
        <button type="button" @click="choosePr(p.number)">选择合并</button>
      </div>
      <strong>近期 CI</strong>
      <div v-for="run in github.runs" :key="run.databaseId" class="line gh-entry">
        <span>{{run.workflowName}} · {{run.conclusion || run.status}} · {{run.headSha.slice(0,9)}}</span>
        <a :href="run.url" target="_blank" rel="noopener noreferrer">日志</a>
      </div>
    </div>
  </section>
</template>

<style scoped>
.local-workbench{padding:10px 12px;background:#f7faff;border-top:1px solid #dce5f1;color:#314154;font-size:12px;overflow:auto;max-height:380px}
.local-header{display:flex;align-items:center;gap:10px}.local-header strong{white-space:nowrap}.local-header .muted{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;min-width:0;flex:1;color:#63758a}
.local-workbench button{border:1px solid #c2d2e4;background:white;border-radius:6px;padding:5px 8px;cursor:pointer;color:#2d5279}
.local-workbench button:disabled{opacity:.5;cursor:default}.local-workbench .primary{background:#1f60a4;color:#fff}
.hint{color:#60778d;font-size:11px;line-height:1.5}.error{color:#a2242e}.success{color:#16703b}
.local-summary,.local-tools{display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin:6px 0}
.operation{display:flex;flex-wrap:wrap;gap:8px;align-items:end;margin:8px 0}
.operation label{display:flex;flex-direction:column;gap:3px}
.operation input,.operation select{font-size:12px;padding:5px;border:1px solid #bdcee1;border-radius:6px;max-width:230px}
.operation .hint{flex-basis:100%;margin:0}
.preview,.detail,.conflict-editor{padding:9px;background:white;border:1px solid #dce7f3;border-radius:8px;margin:8px 0}
.preview p{margin:6px 0}.preview small{color:#60788b}.danger-ack{display:flex;gap:6px;align-items:center;margin:8px 0;color:#9c3c1a}
.three-way{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:6px}
.conflict-editor label{display:flex;flex-direction:column;gap:4px;font-size:11px}
.conflict-editor textarea{width:100%;min-height:70px;box-sizing:border-box;resize:vertical;font:11px/1.4 ui-monospace,monospace}
.conflict-editor .result-editor{min-height:100px}
.line{display:flex;gap:8px;padding:5px 0;border-bottom:1px solid #eef2f7;overflow-wrap:anywhere}
.gh-entry span{flex:1}.gh-entry a{white-space:nowrap}
@media(max-width:750px){.three-way{grid-template-columns:1fr}}
</style>
