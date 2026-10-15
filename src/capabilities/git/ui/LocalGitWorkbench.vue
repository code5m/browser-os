<script setup lang="ts">
import { onUnmounted, ref, watch } from "vue";
import { bridge } from "../../../bridge";
import { redactSecrets } from "../../../utils/redact";
type Action = "stage"|"commit"|"checkout"|"fetch"|"pull"|"stash"|"resolve"|"github_merge";
const props=defineProps<{path:string;initialAction?:Action}>();
const emit=defineEmits<{updated:[]}>();
const options:Action[]=["stage","commit","checkout","fetch","pull","stash","resolve","github_merge"];
const labels:Record<Action,string>={stage:"暂存",commit:"提交",checkout:"Checkout",fetch:"Fetch",
  pull:"Pull（仅快进）",stash:"Stash",resolve:"三方冲突解决",github_merge:"合并 GitHub PR"};
const panes=["ancestor","ours","theirs"] as const;
const action=ref<Action>(props.initialAction??"stage");
const info=ref<Awaited<ReturnType<typeof bridge.gitLocalInspect>>|null>(null);
const preview=ref<Awaited<ReturnType<typeof bridge.gitLocalWriteRequest>>|null>(null);
const conflict=ref<Awaited<ReturnType<typeof bridge.gitLocalConflict>>|null>(null);
const github=ref<Awaited<ReturnType<typeof bridge.gitLocalGithub>>|null>(null);
const details=ref("");
const message=ref(""),branch=ref(""),resolution=ref(""),prNumber=ref<number|null>(null);
const acknowledged=ref(false),busy=ref(false),error=ref(""),notice=ref("");
let generation=0;
const err=(e:unknown)=>redactSecrets(e).slice(0,220);
async function refresh(){
  const v=++generation;
  try{
    const next=await bridge.gitLocalInspect(props.path);
    if(v===generation){info.value=next;branch.value=next.branch;}
  }catch(e){if(v===generation)error.value=err(e)}
}
function reset(){preview.value=null;acknowledged.value=false;error.value="";}
async function prepare(){
  if(busy.value||!info.value)return;reset();busy.value=true;
  try{
    preview.value=await bridge.gitLocalWriteRequest(props.path,action.value,{
      message:action.value==="commit"?message.value:undefined,
      branch:action.value==="checkout"?branch.value:undefined,
      resolved:action.value==="resolve"?resolution.value:undefined,
      pr:action.value==="github_merge"?prNumber.value??undefined:undefined,
    });
  }catch(e){error.value=err(e)}finally{busy.value=false}
}
async function confirm(){
  const job=preview.value;
  if(!job||busy.value||(job.dangerous && !acknowledged.value))return;
  busy.value=true;preview.value=null;error.value="";
  try{
    notice.value=await bridge.gitLocalWriteConfirm(job.job_id,job.dangerous&&acknowledged.value);
    if(job.action==="commit")message.value="";
    await refresh();emit("updated");
  }catch(e){error.value=err(e)}finally{busy.value=false;acknowledged.value=false}
}
async function read(kind:"resolve"|"blame"|"follow"|"github"){
  if(busy.value)return;busy.value=true;error.value="";details.value="";
  try{
    if(kind==="resolve"){conflict.value=await bridge.gitLocalConflict(props.path);resolution.value=conflict.value.ours;action.value="resolve";}
    if(kind==="blame"){const a=await bridge.gitLocalBlame(props.path);details.value=a.hunks.map(h=>h.start+"–"+(h.start+h.lines-1)+" · "+h.author+" · "+h.commit.slice(0,10)).join("\n")+(a.truncated?"\n已截断":"");}
    if(kind==="follow"){const a=await bridge.gitLocalFollow(props.path);details.value=a.entries.map(h=>h.oid.slice(0,9)+" · "+h.summary+" · "+h.file_at_commit+(h.renamed_from?" ← "+h.renamed_from:"")).join("\n")+(a.scan_limited?"\n历史已截断":"");}
    if(kind==="github")github.value=await bridge.gitLocalGithub(props.path);
  }catch(e){error.value=err(e)}finally{busy.value=false}
}
function choosePr(n:number){prNumber.value=n;action.value="github_merge";reset();}
watch(()=>[props.path,props.initialAction] as const,()=>{
  action.value=props.initialAction??"stage";info.value=null;reset();conflict.value=null;github.value=null;
  details.value="";void refresh();
},{immediate:true});
watch(action,reset);
onUnmounted(()=>{++generation});
</script>
<template>
  <section class="local-git-tools" aria-label="本地 Git 操作">
    <header><strong>本地 Git / GitHub</strong><small>{{info?.branch}} · {{info?.scope||"整个仓库"}} · {{info?.files.length||0}} 项改动</small>
      <button @click="refresh">刷新</button></header>
    <p>原仓库操作：预览→确认，禁止强制覆盖。GitHub 功能需已安装并登录 gh。</p>
    <div class="row">
      <select v-model="action" aria-label="Git 操作"><option v-for="id in options" :key="id" :value="id">{{labels[id]}}</option></select>
      <input v-if="action==='commit'" v-model="message" maxlength="2048" placeholder="提交说明" aria-label="提交说明"/>
      <select v-if="action==='checkout'" v-model="branch" aria-label="目标分支"><option v-for="b in info?.local_branches||[]" :key="b">{{b}}</option></select>
      <input v-if="action==='github_merge'" v-model.number="prNumber" type="number" min="1" placeholder="PR 编号" aria-label="PR 编号"/>
      <button v-if="action==='resolve'" @click="read('resolve')">读取冲突版本</button>
      <button :disabled="busy||!info||(action==='resolve'&&!conflict)" @click="prepare">预览{{labels[action]}}</button>
    </div>
    <div v-if="conflict" class="conflict">
      <strong>冲突：{{conflict.path}}</strong>
      <div class="triple"><label v-for="k in panes" :key="k">{{k}}<textarea :value="conflict[k]" readonly /></label></div>
      <label>解决结果<textarea v-model="resolution" maxlength="131072"/></label>
    </div>
    <div v-if="preview" class="confirm">
      <strong>待确认：{{preview.summary}}</strong>
      <p>有效期 {{preview.expires_in_seconds}} 秒；HEAD 和索引变化时拒绝执行。</p>
      <label v-if="preview.dangerous"><input v-model="acknowledged" type="checkbox"/>我确认该操作会修改工作区或远端</label>
      <button :disabled="busy||(preview.dangerous&&!acknowledged)" @click="confirm">确认执行</button>
      <button @click="preview=null">取消</button>
    </div>
    <p v-if="error" role="alert" class="fail">{{error}}</p><p v-if="notice" role="status">{{notice}}</p>
    <div class="row">
      <button :disabled="busy" @click="read('blame')">Blame</button>
      <button :disabled="busy" @click="read('follow')">跨重命名历史</button>
      <button :disabled="busy" @click="read('github')">GitHub PR / CI</button>
    </div>
    <pre v-if="details" class="detail">{{details}}</pre>
    <div v-if="github" class="detail">
      <strong>PR · {{github.repository}}</strong>
      <div v-for="p in github.prs" :key="p.number" class="row">
        <button @click="choosePr(p.number)">合并 #{{p.number}}</button>
        <a :href="p.url" target="_blank" rel="noopener noreferrer">{{p.title}}</a>
        <small>{{p.reviewDecision}} · {{p.mergeStateStatus}}</small>
      </div>
      <strong>近期 CI</strong>
      <div v-for="x in github.runs" :key="x.databaseId" class="row">
        <a :href="x.url" target="_blank" rel="noopener noreferrer">{{x.workflowName}}</a>
        <small>{{x.conclusion||x.status}}</small>
      </div>
    </div>
  </section>
</template>
<style scoped>
.local-git-tools{padding:9px;background:#f6f9ff;overflow:auto;max-height:370px;border-top:1px solid #dae6f4;color:#344c66;font-size:12px}
header,.row{display:flex;align-items:center;flex-wrap:wrap;gap:8px;margin:5px 0}
header small{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
button,select,input{border:1px solid #bdd1e6;background:white;border-radius:6px;padding:5px;font-size:12px}
button{cursor:pointer;color:#225b95}button:disabled{opacity:.5}
p,small{font-size:11px;color:#6a7d91}.fail{color:#b22}
.confirm,.conflict,.detail{background:#fff;border:1px solid #dae6f4;padding:7px;margin:7px 0;border-radius:7px;overflow:auto}
.triple{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:5px}
.triple label,.conflict>label{display:flex;flex-direction:column;gap:4px}
textarea{resize:vertical;width:100%;min-height:65px;box-sizing:border-box}
pre{white-space:pre-wrap;overflow-wrap:anywhere}
@media(max-width:700px){.triple{grid-template-columns:1fr}}
</style>
