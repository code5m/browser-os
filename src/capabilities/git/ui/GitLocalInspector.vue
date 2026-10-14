<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from "vue";
import { bridge } from "../../../bridge";
import { redactSecrets } from "../../../utils/redact";
import { parseUnifiedPatch } from "../model/patchView";
import type { LocalGitMode } from "../../../composables/gitLocalNavigation";

const props=defineProps<{path:string;mode:LocalGitMode}>();
const emit=defineEmits<{close:[]}>();
type Info=Awaited<ReturnType<typeof bridge.gitLocalContext>>;
type Entry=Awaited<ReturnType<typeof bridge.gitLocalLog>>["commits"][number];
const context=ref<Info|null>(null);
const commits=ref<Entry[]>([]);
const offset=ref(0),more=ref(false),scanLimited=ref(false);
const working=ref(""), selected=ref(""), patch=ref(""), truncated=ref(false);
const search=ref(""),error=ref(""),loading=ref(false),diffLoading=ref(false);
const view=ref<"history"|"diff">("history");
const parsed=computed(()=>parseUnifiedPatch(patch.value));
const visible=computed(()=>commits.value.filter(it=>{
  const term=search.value.trim().toLowerCase();
  return !term || (it.summary+" "+it.author+" "+it.oid).toLowerCase().includes(term);
}));
let generation=0,detailGeneration=0;
const PAGE=40;
const friendlyError=(err:unknown):string => redactSecrets(err instanceof Error?err.message:String(err)).slice(0,280);

async function loadPage(nextOffset:number,gen=generation) {
  if(!context.value)return;
  loading.value=true;error.value="";
  try {
    const page=await bridge.gitLocalLog(props.path,nextOffset,PAGE);
    if(gen!==generation)return;
    commits.value=page.commits;
    offset.value=nextOffset;
    more.value=page.has_more;
    scanLimited.value=page.scan_limited;
  }catch(e){if(gen===generation)error.value=friendlyError(e);}
  finally {if(gen===generation)loading.value=false;}
}
async function loadContext() {
  const gen=++generation;
  ++detailGeneration;
  context.value=null;commits.value=[];patch.value="";selected.value="";error.value="";
  view.value=props.mode;
  loading.value=true;
  try {
    const data=await bridge.gitLocalContext(props.path);
    if(gen!==generation)return;
    context.value=data;
    await loadPage(0,gen);
    if(props.mode==="diff")await loadWorkingDiff();
  }catch(e){if(gen===generation)error.value=friendlyError(e);}
  finally {if(gen===generation)loading.value=false;}
}
async function selectCommit(oid:string){
  if(!context.value)return;
  const gen=++detailGeneration;
  selected.value=oid;view.value="history";patch.value="";diffLoading.value=true;
  try{
    const data=await bridge.gitLocalCommitDiff(props.path,oid);
    if(gen!==detailGeneration)return;
    patch.value=data.patch;truncated.value=data.truncated;
  }catch(e){if(gen===detailGeneration)error.value=friendlyError(e);}
  finally{if(gen===detailGeneration)diffLoading.value=false;}
}
async function loadWorkingDiff(){
  if(!context.value)return;
  const gen=++detailGeneration;
  view.value="diff";selected.value="";patch.value="";diffLoading.value=true;error.value="";
  try{
    const data=await bridge.gitLocalDiff(props.path);
    if(gen!==detailGeneration)return;
    patch.value=data.patch;truncated.value=data.truncated;
  }catch(e){if(gen===detailGeneration)error.value=friendlyError(e);}
  finally{if(gen===detailGeneration)diffLoading.value=false;}
}
watch(()=>[props.path,props.mode] as const,loadContext,{immediate:true});
onUnmounted(()=>{++generation;++detailGeneration;});
</script>
<template>
  <section class="local-git-studio" aria-label="本地 Git 历史与差异">
    <header class="local-head">
      <strong>本地 Git（直接读取原仓库）</strong>
      <span v-if="context" class="local-branch">{{context.branch}}</span>
      <button @click="loadContext">刷新</button>
      <button @click="emit('close')">返回普通仓库</button>
    </header>
    <p class="local-scope" :title="path">{{context ? context.root : path}}
      <span v-if="context?.relative_path"> / {{context.relative_path}}</span>
    </p>
    <p class="local-help">仅查看本机 Git 内容，不克隆、不联网、不会自动 Fetch/Pull/提交；右键目录只展示该目录范围内的历史。</p>
    <p v-if="error" role="alert" class="local-error">{{error}}</p>
    <p v-if="!context && loading">正在定位真实 Git 仓库…</p>
    <div v-if="context" class="local-toolbar">
      <button :aria-pressed="view==='history'" @click="view='history'">提交历史</button>
      <button :aria-pressed="view==='diff'" @click="loadWorkingDiff">工作区改动</button>
      <span v-if="context.relative_path">范围：{{context.is_dir?'目录':'文件'}}</span>
      <span v-else>范围：整个仓库</span>
    </div>
    <div v-if="context" class="local-body">
      <aside class="local-commits">
        <label class="sr-only" for="local-git-search">过滤当前页提交</label>
        <input id="local-git-search" v-model="search" type="search" placeholder="搜索当前页提交、作者、哈希" />
        <div v-if="loading" class="local-hint">正在读取提交历史…</div>
        <div v-else-if="!visible.length" class="local-hint">当前页没有符合范围的提交</div>
        <button v-for="commit in visible" :key="commit.oid" class="local-commit"
          :class="{selected:selected===commit.oid}" :aria-pressed="selected===commit.oid" @click="selectCommit(commit.oid)">
          <b>{{commit.summary||"(无提交说明)"}}</b>
          <small>{{commit.oid.slice(0,9)}} · {{commit.author}} · {{new Date(commit.time*1000).toLocaleDateString()}}</small>
        </button>
        <footer class="local-paging">
          <button :disabled="loading||offset===0" @click="loadPage(Math.max(0,offset-PAGE))">上一页</button>
          <span>{{Math.floor(offset/PAGE)+1}}</span>
          <button :disabled="loading||!more||scanLimited" @click="loadPage(offset+PAGE)">下一页</button>
        </footer>
        <p v-if="scanLimited" class="local-hint">为保护大型仓库，单次最多扫描 4000 个提交；更早历史请使用专业 Git 客户端。</p>
        <p class="local-hint">仅显示当前页的搜索结果；重命名历史可能需要跨路径检索。</p>
      </aside>
      <div class="local-detail">
        <div class="local-detail-head">
          <strong>{{view==='diff'?'工作区（含已暂存）差异':selected?'提交 '+selected.slice(0,12):'选择提交查看差异'}}</strong>
          <span v-if="parsed.rows.length">+{{parsed.additions}} / −{{parsed.deletions}}</span>
        </div>
        <div v-if="diffLoading" class="local-hint">正在生成真实 Git 补丁…</div>
        <div v-else-if="truncated" class="local-warning">补丁已达到 256 KiB 上限，以下内容不完整。</div>
        <div v-if="!diffLoading && patch" class="local-patch" role="table" aria-label="Git 变更并排视图">
          <div class="local-row local-legend"><span>修改前</span><span>修改后</span></div>
          <div v-for="(row,index) in parsed.rows" :key="index" class="local-row">
            <div :class="{removed:row.removed}"><small>{{row.beforeLine??''}}</small><pre>{{row.before}}</pre></div>
            <div :class="{added:row.added}"><small>{{row.afterLine??''}}</small><pre>{{row.after}}</pre></div>
          </div>
          <p v-if="parsed.truncated" class="local-warning">视图最多显示 4000 行。</p>
        </div>
        <p v-else-if="!diffLoading && (selected || view==='diff')" class="local-hint">该范围没有可显示的文本差异（或文件是二进制）。</p>
      </div>
    </div>
  </section>
</template>
<style scoped>
.local-git-studio{height:100%;min-height:0;display:flex;flex-direction:column;background:#fff;color:#314154;font-size:12px}
.local-head{display:flex;gap:10px;align-items:center;padding:10px;border-bottom:1px solid #e0e7ef}
.local-head strong{font-size:13px}.local-branch{background:#e5f0ff;border-radius:5px;padding:4px 8px;color:#2365a1}
.local-head button,.local-toolbar button,.local-paging button{border:1px solid #cbd8e7;border-radius:5px;background:#fff;padding:5px 9px;cursor:pointer}
.local-head button:disabled,.local-paging button:disabled{opacity:.45}
.local-scope{padding:8px 10px;margin:0;font:11px ui-monospace,monospace;overflow-wrap:anywhere;background:#f8faff}
.local-help,.local-hint{font-size:11px;color:#697d90;line-height:1.6;margin:6px 10px}
.local-error,.local-warning{padding:8px;color:#995225;background:#fff3e4;border-radius:6px;margin:5px 8px}
.local-toolbar{display:flex;align-items:center;gap:7px;padding:8px;border-bottom:1px solid #dbe5f1}
.local-toolbar button[aria-pressed=true]{background:#e5f0ff;color:#235a94}
.local-body{display:flex;flex:1;min-height:0}.local-commits{display:flex;flex-direction:column;width:35%;min-width:205px;overflow-y:auto;border-right:1px solid #dce5ef}
.local-commits input{border:1px solid #d1dce8;border-radius:5px;padding:7px;margin:8px}
.local-commit{text-align:left;display:flex;flex-direction:column;gap:4px;background:transparent;border:0;border-bottom:1px solid #ecf0f6;padding:10px;cursor:pointer;color:inherit}
.local-commit b{font-weight:600;overflow-wrap:anywhere}.local-commit small{color:#76879b;font-size:10px}
.local-commit.selected,.local-commit:hover{background:#eaf3ff}
.local-paging{display:flex;justify-content:space-evenly;padding:8px;align-items:center}
.local-detail{flex:1;min-width:0;display:flex;flex-direction:column}
.local-detail-head{display:flex;justify-content:space-between;padding:10px;border-bottom:1px solid #e2eaf4}
.local-patch{overflow:auto;flex:1}
.local-row{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr)}
.local-row>div{display:flex;min-width:0;border-bottom:1px solid #eef1f6}
.local-row>div+div{border-left:1px solid #dae4ee}
.local-row small{font:10px/2 ui-monospace,monospace;color:#99a8b9;min-width:35px;text-align:right;background:#f4f7fa;padding:2px}
.local-row pre{margin:0;padding:3px 8px;overflow:auto;white-space:pre;font:11px/1.6 ui-monospace,monospace}
.local-row .removed{background:#fff0f0}.local-row .added{background:#eaf8ed}
.local-legend{background:#edf3fa;color:#526b84;font-weight:600}
.local-legend span{padding:7px}
.sr-only{position:absolute;width:1px;height:1px;overflow:hidden;clip-path:inset(50%)}
</style>