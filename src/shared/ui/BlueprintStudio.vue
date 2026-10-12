<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import type { PermissionPreview, ValidationReport } from "../../types";
import { createStudioDraft, parseStudioList, saveStudioList, studioKey, studioProgress, validateStudioDraft, type StudioDraft, type StudioKind } from "../../utils/studioDrafts";
const props = defineProps<{ kind: StudioKind; validateDefinition: (text: string) => Promise<void>; validation: ValidationReport | null; preview: PermissionPreview | null; backendReady: boolean; busy: boolean; runSafeReference?: (text:string)=>Promise<string>; localChat?: (model:string,message:string,system:string)=>Promise<{content:string;model:string;provider:string}> }>();
const records = ref<StudioDraft[]>([]);
const chosen = ref<string | null>(null);
const search = ref("");
const activeTab = ref<"overview" | "steps" | "definition" | "permission" | "chat">("overview");
const current = computed(() => records.value.find(d => d.id === chosen.value) ?? null);
const matches = computed(() => records.value.filter(d => (d.name+" "+d.description).toLowerCase().includes(search.value.toLowerCase())));
const notice = ref("");
const blocked = ref(false);
const fileInput = ref<HTMLInputElement|null>(null);
const definitionText = ref("");
const verifiedText = ref("");
const chatModel=ref("qwen2.5:3b");
const chatInput=ref("");
const chatOutput=ref("");
const chatBusy=ref(false);
const chatError=ref("");
async function submitChat(){
  if(!props.localChat || !current.value || chatBusy.value || !chatInput.value.trim()) return;
  chatBusy.value=true; chatError.value=""; chatOutput.value="";
  try {
    const response=await props.localChat(chatModel.value.trim(),chatInput.value,
      "你是本地知识助手。只能回答问题，不能调用工具或执行命令。");
    chatOutput.value=response.content;
  } catch(e) { chatError.value="本地模型连接失败："+String(e)+"。请确认 Ollama 已运行且已下载所选模型。"; }
  finally {chatBusy.value=false;}
}
const kindLabel = computed(() => props.kind === "agent" ? "智能体" : "技能");
onMounted(() => {
  try {
    records.value = parseStudioList(localStorage.getItem(studioKey(props.kind)),props.kind);
    chosen.value = records.value[0]?.id ?? null;
    if (current.value) definitionText.value = current.value.definition;
  } catch (e) {
    blocked.value = true; notice.value = "已有本地数据损坏或含敏感信息，已锁定写入以保护原始数据：" + String(e);
  }
});
watch(chosen,()=>{ definitionText.value=current.value?.definition ?? ""; verifiedText.value=""; activeTab.value="overview"; });
watch(definitionText,()=>{verifiedText.value="";});
function persist(): boolean {
  if (blocked.value) return false;
  try { saveStudioList(records.value,props.kind,localStorage); notice.value="已保存到本机工作台"; return true; }
  catch(e){notice.value=String(e);return false;}
}
function update(field: "name" | "description" | "goal" | "inputs" | "outputs" | "constraints", value:string) {
  if (!current.value || blocked.value) return;
  const next={...current.value,[field]:value,updatedAt:new Date().toISOString()};
  try {validateStudioDraft(next,props.kind);}
  catch(e){ notice.value=String(e); return; }
  records.value=records.value.map(d=>d.id===next.id?next:d); persist();
}
function create(){
  if(blocked.value)return;
  const d=createStudioDraft(props.kind);
  records.value=[d,...records.value]; if(persist()){chosen.value=d.id;notice.value="蓝图创建成功";} else records.value=records.value.filter(i=>i.id!==d.id);
}
function duplicate(){
  if(!current.value||blocked.value)return;
  const d={...current.value,id:crypto.randomUUID(),name:current.value.name+"（副本）",createdAt:new Date().toISOString(),updatedAt:new Date().toISOString(),steps:current.value.steps.map(s=>({...s,id:crypto.randomUUID()}))};
  records.value=[d,...records.value];if(persist())chosen.value=d.id;else records.value=records.value.filter(i=>i.id!==d.id);
}
function remove(){
  if(!current.value||blocked.value)return;
  if(!window.confirm("确定删除这个蓝图？此操作不能撤销，建议先导出备份。"))return;
  const old=records.value;records.value=records.value.filter(i=>i.id!==chosen.value);
  if(persist())chosen.value=records.value[0]?.id??null;else records.value=old;
}
function addStep(){
  if(!current.value||current.value.steps.length>=32)return;
  current.value.steps.push({id:crypto.randomUUID(),title:"新步骤",done:false}); current.value.updatedAt=new Date().toISOString();persist();
}
function stepEdit(index:number,title:string,done:boolean){
  if(!current.value)return;
  const steps=current.value.steps.map((s,i)=>i===index?{...s,title,done}:s);
  try{ validateStudioDraft({...current.value,steps},props.kind); }catch(e){notice.value=String(e);return;}
  current.value.steps=steps;current.value.updatedAt=new Date().toISOString();persist();
}
function deleteStep(index:number){if(!current.value)return;current.value.steps.splice(index,1);persist();}
async function verify(){
  if(!current.value||!definitionText.value.trim())return;
  const candidate=definitionText.value;
  await props.validateDefinition(candidate);
  verifiedText.value=props.validation?.valid && definitionText.value===candidate ? candidate : "";
  notice.value="原生定义校验已运行。注意校验通过不等于安装或执行成功。";
}
const runningId=ref("");
const runFeedback=ref("");
async function runReference(){
  if(!props.runSafeReference || !current.value || verifiedText.value!==definitionText.value || !props.validation?.valid) return;
  if(!window.confirm("确认执行现有已启用脚本/命令？这会在本机真实运行；运行时安全边界由现有脚本执行器负责。")) return;
  runFeedback.value="正在启动已存在的脚本或命令…";
  try {
    const id=await props.runSafeReference(definitionText.value);
    runningId.value=id;
    runFeedback.value="已提交真实执行，运行 ID："+id+"。请在脚本/命令面板查看输出和状态。";
  } catch(e) { runFeedback.value="执行拒绝或失败："+String(e); }
}
function saveDefinition(){
  if(!current.value||!props.validation?.valid||verifiedText.value!==definitionText.value){notice.value="请先通过原生校验，才允许保存定义";return;}
  const next={...current.value,definition:definitionText.value,updatedAt:new Date().toISOString()};
  try{validateStudioDraft(next,props.kind)}catch(e){notice.value=String(e);return;}
  records.value=records.value.map(d=>d.id===next.id?next:d);persist();
}
function exportFile(){
  if(!current.value)return;
  const blob=new Blob([JSON.stringify({version:1,blueprint:validateStudioDraft(current.value,props.kind)},null,2)],{type:"application/json"});
  const url=URL.createObjectURL(blob);const a=document.createElement("a");a.href=url;a.download=props.kind+"-"+current.value.id+".json";a.click();setTimeout(()=>URL.revokeObjectURL(url),300);
}
async function importFile(e:Event){
  const input=e.target as HTMLInputElement;const file=input.files?.[0];input.value="";if(!file||blocked.value)return;
  if(file.size>90_000){notice.value="导入文件超过大小限制";return;}
  try{
    const json:unknown=JSON.parse(await file.text());
    const body=json&&typeof json==="object"&&"blueprint" in json?(json as {blueprint:unknown}).blueprint:json;
    const draft=validateStudioDraft(body,props.kind);
    const copy={...draft,id:crypto.randomUUID(),name:draft.name+"（导入）",definition:"",steps:draft.steps.map(s=>({...s,id:crypto.randomUUID()})),updatedAt:new Date().toISOString()};
    records.value=[copy,...records.value];if(persist()){chosen.value=copy.id;notice.value="导入成功。外部定义已清除，请重新校验。";}else records.value=records.value.filter(i=>i.id!==copy.id);
  }catch(err){notice.value="导入失败："+String(err);}
}
</script>

<template>
  <div class="studio">
    <header class="studio-header">
      <div><p class="eyebrow">BROWSEROS · {{ kind === 'agent' ? 'AGENT STUDIO' : 'SKILL STUDIO' }}</p><h2>{{ kindLabel }}工作台</h2><p class="subtitle">从目标、步骤到定义检查，先把想法变成可管理的蓝图。</p></div>
      <div class="header-actions"><button @click="fileInput?.click()" :disabled="blocked">导入蓝图</button><button class="primary" @click="create" :disabled="blocked">＋ 新建{{ kindLabel }}</button><input ref="fileInput" type="file" accept=".json,application/json" hidden @change="importFile"/></div>
    </header>
    <div class="studio-metrics"><div><strong>{{ records.length }}</strong><span>本地蓝图</span></div><div><strong>{{ records.filter(d=>studioProgress(d)===100).length }}</strong><span>步骤已完成</span></div><div><strong>{{ records.filter(d=>!!d.definition).length }}</strong><span>已保存定义</span></div><div><strong>规划中</strong><span>安装与运行后端</span></div></div>
    <div v-if="notice" class="notice" role="status">{{ notice }}</div>
    <div class="studio-layout">
      <aside class="studio-sidebar">
        <div class="side-heading"><strong>我的{{ kindLabel }}</strong><span>{{ records.length }} 项</span></div>
        <input v-model="search" class="search" type="search" placeholder="搜索名称和简介" aria-label="搜索蓝图"/>
        <div class="item-list">
          <button v-for="d in matches" :key="d.id" class="studio-item" :class="{selected:chosen===d.id}" @click="chosen=d.id">
            <span class="item-symbol">{{ kind==='agent'?'✦':'◇' }}</span><span class="item-copy"><strong>{{ d.name }}</strong><small>{{ d.description || '尚未填写说明' }}</small><span class="progress-track"><i :style="{width:studioProgress(d)+'%'}"></i></span></span><span class="percent">{{studioProgress(d)}}%</span>
          </button>
          <div v-if="!matches.length" class="empty-list">这里还没有{{kindLabel}}蓝图。点击「新建{{kindLabel}}」开始。</div>
        </div>
        <p class="storage-hint">保存在当前设备的本地工作台；建议定期导出 JSON 备份。不要填写账号口令或 API 密钥。</p>
      </aside>
      <main class="studio-main">
        <template v-if="current">
          <div class="detail-heading"><div><span class="section-kicker">蓝图工作区</span><h3>{{current.name}}</h3><p>{{current.description||'设置目标与边界，逐步完善你的'+kindLabel+'。'}}</p></div><div class="detail-actions"><button @click="duplicate">复制</button><button @click="exportFile">导出</button><button class="delete" @click="remove">删除</button></div></div>
          <nav class="studio-tabs" aria-label="工作区视图"><button v-for="tab in [{id:'overview',label:'概览'},{id:'steps',label:'任务步骤'},{id:'definition',label:'定义检查'},{id:'permission',label:'权限与运行'},...(localChat?[{id:'chat',label:'本地 AI 对话'}] as const:[])] as const" :key="tab.id" :class="{active:activeTab===tab.id}" @click="activeTab=tab.id">{{tab.label}}</button></nav>
          <div v-if="activeTab==='overview'" class="studio-content">
            <div class="section-intro"><strong>基础信息</strong><span>修改后即时保存在本机</span></div>
            <label>名称<input :value="current.name" maxlength="120" @change="update('name',($event.target as HTMLInputElement).value)"/></label>
            <label>一句话说明<textarea :value="current.description" rows="2" maxlength="4000" placeholder="它负责帮助我解决什么问题？" @change="update('description',($event.target as HTMLTextAreaElement).value)"/></label>
            <div class="two-col"><label>目标<textarea :value="current.goal" rows="4" maxlength="4000" placeholder="完成后应该产生什么结果？" @change="update('goal',($event.target as HTMLTextAreaElement).value)"/></label><label>约束与风险<textarea :value="current.constraints" rows="4" maxlength="4000" placeholder="哪些操作不可做？哪些风险要注意？" @change="update('constraints',($event.target as HTMLTextAreaElement).value)"/></label></div>
            <div class="two-col"><label>需要提供的输入<textarea :value="current.inputs" rows="4" maxlength="4000" placeholder="文件、知识、参数……" @change="update('inputs',($event.target as HTMLTextAreaElement).value)"/></label><label>预期输出<textarea :value="current.outputs" rows="4" maxlength="4000" placeholder="报告、文件或可验证的结果……" @change="update('outputs',($event.target as HTMLTextAreaElement).value)"/></label></div>
          </div>
          <div v-else-if="activeTab==='steps'" class="studio-content">
            <div class="section-intro"><strong>任务拆解</strong><button @click="addStep" :disabled="current.steps.length>=32">＋ 添加步骤</button></div><p class="help">勾选已经完成的步骤。规划进度只反映你手动记录的结果，不代表智能体真实执行。</p>
            <div v-for="(step,i) in current.steps" :key="step.id" class="step"><input type="checkbox" :checked="step.done" @change="stepEdit(i,step.title,($event.target as HTMLInputElement).checked)"/><input :value="step.title" maxlength="180" @change="stepEdit(i,($event.target as HTMLInputElement).value,step.done)"/><button aria-label="移除步骤" @click="deleteStep(i)">×</button></div>
            <p v-if="!current.steps.length" class="help">暂时没有步骤，可以自己添加。</p><div class="total-progress"><span>规划完成度</span><strong>{{studioProgress(current)}}%</strong></div><div class="progress-track large"><i :style="{width:studioProgress(current)+'%'}"></i></div>
          </div>
          <div v-else-if="activeTab==='definition'" class="studio-content">
            <div class="section-intro"><strong>原生定义校验</strong><span>解析与权限预览可用</span></div><p class="help">粘贴符合 BrowserOS 契约的 JSON 定义。这里只校验，不会安装或执行。</p><textarea v-model="definitionText" class="json-editor" rows="14" spellcheck="false" placeholder="粘贴 JSON 定义（不包含任何口令或密钥）"></textarea>
            <div class="action-row"><button class="primary" :disabled="!definitionText.trim()||busy" @click="verify">验证定义</button><button :disabled="!validation?.valid||verifiedText!==definitionText||busy" @click="saveDefinition">保存已验证定义</button><button v-if="runSafeReference" :disabled="!validation?.valid||verifiedText!==definitionText||busy" @click="runReference">▶ 运行安全引用</button></div>
            <p v-if="runFeedback" class="help" role="status">{{runFeedback}}</p><div v-if="validation" class="result" :class="{invalid:!validation.valid}">{{validation.valid?'✓ 定义通过原生验证':'定义尚未通过验证'}}<ul v-if="!validation.valid"><li v-for="(err,i) in validation.errors" :key="i">{{err}}</li></ul></div>
          </div>
          <div v-else-if="activeTab==='chat'" class="studio-content">
            <div class="section-intro"><strong>本地 Ollama 真实对话</strong><span>仅连接 127.0.0.1:11434 · 不调用工具</span></div>
            <p class="help">需预先在本机运行 Ollama 并下载模型。网络不可用时不生成虚假回复；当前为单轮、非流式试用。</p>
            <label>模型名称<input v-model="chatModel" maxlength="100" placeholder="qwen2.5:3b"/></label>
            <label>发送消息<textarea v-model="chatInput" rows="4" maxlength="12000" placeholder="输入问题"/></label>
            <div class="action-row"><button class="primary" :disabled="chatBusy||!chatInput.trim()" @click="submitChat">{{chatBusy?'正在等待本机模型…':'发送真实请求'}}</button></div>
            <p v-if="chatError" role="alert" class="result invalid">{{chatError}}</p>
            <pre v-if="chatOutput" class="chat-output">{{chatOutput}}</pre>
          </div>
          <div v-else class="studio-content"><div class="section-intro"><strong>权限与运行状态</strong><span>真实状态</span></div>
            <div class="permission-grid"><div><small>定义权限等级</small><strong>{{preview?.gate||'未验证'}}</strong></div><div><small>请求的能力</small><strong>{{preview?.capabilities?.join('、')||'未声明'}}</strong></div><div><small>安装状态</small><strong>尚未开放</strong></div><div><small>运行记录</small><strong>无真实记录</strong></div></div>
            <p class="help">运行引擎尚未完成接入，因此这里不展示虚假结果，不提供无效的运行按钮。</p>
          </div>
        </template>
        <div v-else class="main-empty"><div class="empty-icon">✦</div><h3>让你的第一个{{kindLabel}}从蓝图开始</h3><p>不用先编写代码。先说明目标、输入、输出与步骤，随后再连接真实执行能力。</p><button class="primary" @click="create" :disabled="blocked">新建{{kindLabel}}蓝图</button></div>
      </main>
    </div>
  </div>
</template>
<style scoped>
.studio{height:100%;min-height:0;display:flex;flex-direction:column;background:#f7f9fc;color:#1c293c;font-size:13px;font-family:Inter,"Noto Sans SC",system-ui,sans-serif}
.studio-header{display:flex;align-items:center;justify-content:space-between;gap:16px;padding:22px 28px 18px;background:#fff;border-bottom:1px solid #e7ecf2}
.eyebrow{color:#59799a;font-size:10px;font-weight:800;letter-spacing:.14em;margin:0 0 6px}
h2{font-size:23px;letter-spacing:-.04em;margin:0 0 6px}h3{font-size:20px;letter-spacing:-.025em;margin:4px 0}
.subtitle,.detail-heading p{margin:0;color:#728198;line-height:1.5}
button{font:inherit;border:1px solid #d8e1ed;background:#fff;color:#31465f;border-radius:8px;padding:8px 12px;cursor:pointer;white-space:nowrap}
button:hover:not(:disabled){background:#f1f6fd}button:disabled{opacity:.45;cursor:not-allowed}
.primary{background:#2355be;color:#fff;border-color:#2355be;font-weight:600}.primary:hover:not(:disabled){background:#1c489d;color:#fff}
.header-actions,.detail-actions,.action-row{display:flex;gap:8px;align-items:center}
.studio-metrics{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));background:#fff;border-bottom:1px solid #e7ecf2;padding:15px 28px;gap:16px}
.studio-metrics>div{display:flex;flex-direction:column;gap:2px;min-width:0}
.studio-metrics strong{font-size:18px}.studio-metrics span{font-size:11px;color:#8593a8}
.notice{margin:10px 24px 0;padding:9px 12px;border:1px solid #d8e6fb;border-radius:7px;background:#f4f8fe;color:#3662a4;line-height:1.6}
.studio-layout{display:grid;grid-template-columns:minmax(232px,270px) minmax(0,1fr);flex:1;min-height:0;overflow:hidden}
.studio-sidebar{background:#fff;border-right:1px solid #e5eaf1;display:flex;flex-direction:column;min-height:0;padding:20px 12px 15px;gap:12px}
.side-heading{display:flex;justify-content:space-between;padding:0 6px;align-items:center}.side-heading>span{color:#8c9aad;font-size:11px}
.search{border:1px solid #dce5ef;border-radius:8px;padding:10px;min-width:0;background:#f9fbfe;color:#26384f}
.item-list{flex:1;overflow:auto;display:flex;flex-direction:column;gap:6px}
.studio-item{display:flex;gap:10px;text-align:left;border:1px solid transparent;padding:12px 10px;white-space:normal;align-items:flex-start;background:transparent}
.studio-item.selected{border-color:#c6daf6;background:#eef5ff}.item-symbol{background:#e4eefa;color:#3163b3;border-radius:9px;width:30px;height:30px;display:grid;place-items:center;flex-shrink:0}
.item-copy{min-width:0;flex:1;display:flex;flex-direction:column;gap:5px}.item-copy strong,.item-copy small{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;max-width:160px}
.item-copy small,.percent{font-size:11px;color:#8292a6}.percent{margin-top:6px}
.progress-track{height:4px;background:#e4ebf4;border-radius:6px;display:block;overflow:hidden}.progress-track i{display:block;background:#467bd6;height:100%;border-radius:6px}
.storage-hint{font-size:11px;line-height:1.7;color:#8794a6;padding:10px 7px 0;border-top:1px solid #eff1f6;margin:0}
.empty-list{color:#8795a8;padding:28px 15px;line-height:1.7}
.studio-main{min-width:0;overflow:auto;background:#f8fafd;padding:24px 30px}
.detail-heading{display:flex;justify-content:space-between;align-items:start;gap:12px;padding-bottom:22px}
.section-kicker{font-size:11px;letter-spacing:.04em;font-weight:700;color:#5680bb}
.detail-actions .delete{color:#bd4141;border-color:#f0d4d4}
.studio-tabs{display:flex;gap:7px;border-bottom:1px solid #dce5f0;margin-bottom:20px;overflow-x:auto}
.studio-tabs button{border:0;border-bottom:2px solid transparent;border-radius:0;background:transparent;padding:11px 14px;color:#738399;font-weight:500}
.studio-tabs button.active{border-color:#285dc4;color:#285dc4;font-weight:700}
.studio-content{border:1px solid #e7edf4;border-radius:12px;padding:24px;background:#fff;box-shadow:0 5px 18px rgba(20,53,93,.035);max-width:1000px}
.section-intro{display:flex;justify-content:space-between;align-items:center;gap:8px;margin-bottom:18px}.section-intro strong{font-size:15px}.section-intro span,.help{color:#8a97a9;font-size:12px;line-height:1.7}
.studio-content label{display:flex;flex-direction:column;gap:8px;color:#455672;font-size:12px;font-weight:600;margin-bottom:18px}
.studio-content input:not([type=checkbox]),.studio-content textarea{font:inherit;color:#223752;box-sizing:border-box;width:100%;min-width:0;background:#fcfdff;border:1px solid #d9e3ee;border-radius:8px;padding:11px 12px;resize:vertical;font-weight:400;line-height:1.5}
.studio-content input:focus,.studio-content textarea:focus,.search:focus{outline:2px solid #a3c1f0;outline-offset:0}
.two-col{display:grid;grid-template-columns:1fr 1fr;gap:18px}
.step{display:flex;align-items:center;gap:10px;padding:10px 0;border-bottom:1px solid #eff2f7}
.step input[type=checkbox]{width:17px;height:17px;accent-color:#3266c7;flex-shrink:0}.step input:not([type=checkbox]){flex:1}.step button{font-size:20px;padding:2px 10px;border:0;color:#999;background:transparent}
.total-progress{display:flex;justify-content:space-between;margin:22px 0 9px}.progress-track.large{height:8px}
.json-editor{font-family:ui-monospace,SFMono-Regular,Consolas,monospace!important;min-height:260px}
.result{margin-top:16px;padding:13px;border:1px solid #d6ecdf;background:#f1faf4;color:#197349;border-radius:8px}.result.invalid{border-color:#f1d0d0;background:#fff6f6;color:#ae3939}
.permission-grid{display:grid;grid-template-columns:1fr 1fr;gap:14px;margin-bottom:20px}.permission-grid>div{border:1px solid #e5ebf3;background:#fafcfe;border-radius:9px;padding:16px;display:flex;flex-direction:column;gap:8px}
.permission-grid small{color:#8a98a9}.permission-grid strong{overflow-wrap:anywhere}
.main-empty{text-align:center;margin:9vh auto;color:#76869d;max-width:360px;line-height:1.8}
.empty-icon{font-size:50px;color:#3974db}.main-empty h3{color:#294360}
@media(max-width:850px){.studio-layout{grid-template-columns:minmax(175px,210px) minmax(0,1fr)}.studio-header{padding:16px}.studio-metrics{padding:12px 16px}.studio-main{padding:18px}.two-col{grid-template-columns:1fr}.studio-content{padding:16px}}
@media(max-width:560px){.studio-layout{grid-template-columns:1fr;overflow:auto}.studio-sidebar{max-height:210px;border-right:0;border-bottom:1px solid #e5eaf1}.studio-main{overflow:visible}.studio-metrics{grid-template-columns:repeat(2,1fr)}.studio-header{align-items:flex-start;flex-direction:column}.permission-grid{grid-template-columns:1fr}}
.chat-output{white-space:pre-wrap;overflow-wrap:anywhere;font:13px/1.7 inherit;padding:18px;border-radius:8px;background:#f0f6ff;color:#234060}
</style>
