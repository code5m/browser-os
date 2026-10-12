<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import type { PermissionPreview, ValidationReport } from "../../types";
import { createStudioDraft, parseStudioList, saveStudioList, studioKey, studioProgress, validateStudioDraft, type StudioDraft, type StudioKind } from "../../utils/studioDrafts";
const props = defineProps<{ kind: StudioKind; validateDefinition: (text: string) => Promise<void>; validation: ValidationReport | null; preview: PermissionPreview | null; backendReady: boolean; busy: boolean }>();
const records = ref<StudioDraft[]>([]);
const chosen = ref<string | null>(null);
const search = ref("");
const activeTab = ref<"overview" | "steps" | "definition" | "permission">("overview");
const current = computed(() => records.value.find(d => d.id === chosen.value) ?? null);
const matches = computed(() => records.value.filter(d => (d.name+" "+d.description).toLowerCase().includes(search.value.toLowerCase())));
const notice = ref("");
const blocked = ref(false);
const fileInput = ref<HTMLInputElement|null>(null);
const definitionText = ref("");
const verifiedText = ref("");
const kindLabel = computed(() => props.kind === "agent" ? "智能体" : "技能");
const detailFields = [
  { key: "name", label: "名称", placeholder: "", rows: 0 },
  { key: "description", label: "一句话说明", placeholder: "它负责帮助我解决什么问题？", rows: 2 },
  { key: "goal", label: "目标", placeholder: "完成后应该产生什么结果？", rows: 4 },
  { key: "constraints", label: "约束与风险", placeholder: "哪些操作不可做？哪些风险要注意？", rows: 4 },
  { key: "inputs", label: "需要提供的输入", placeholder: "文件、知识、参数……", rows: 4 },
  { key: "outputs", label: "预期输出", placeholder: "报告、文件或可验证的结果……", rows: 4 },
] as const;
const metrics = computed(() => [
  { value: records.value.length, label: "本地蓝图" },
  { value: records.value.filter(d => studioProgress(d) === 100).length, label: "步骤已完成" },
  { value: records.value.filter(d => !!d.definition).length, label: "已保存定义" },
  { value: "规划中", label: "安装与运行后端" },
]);
const permissions = computed(() => [
  { label: "定义权限等级", value: props.preview?.gate || "未验证" },
  { label: "请求的能力", value: props.preview?.capabilities?.join("、") || "未声明" },
  { label: "安装状态", value: "尚未开放" },
  { label: "运行记录", value: "无真实记录" },
]);

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
    <div class="studio-metrics"><div v-for="metric in metrics" :key="metric.label"><strong>{{ metric.value }}</strong><span>{{ metric.label }}</span></div></div>
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
          <nav class="studio-tabs" aria-label="工作区视图"><button v-for="tab in [{id:'overview',label:'概览'},{id:'steps',label:'任务步骤'},{id:'definition',label:'定义检查'},{id:'permission',label:'权限与运行'}] as const" :key="tab.id" :class="{active:activeTab===tab.id}" @click="activeTab=tab.id">{{tab.label}}</button></nav>
          <div v-if="activeTab==='overview'" class="studio-content">
            <div class="section-intro"><strong>基础信息</strong><span>修改后即时保存在本机</span></div>
            <div class="two-col">
              <label v-for="field in detailFields" :key="field.key" :class="{wide:field.key==='name'||field.key==='description'}">
                {{ field.label }}
                <input v-if="field.key==='name'" :value="current.name" maxlength="120" @change="update('name',($event.target as HTMLInputElement).value)"/>
                <textarea v-else :value="current[field.key]" :rows="field.rows" maxlength="4000" :placeholder="field.placeholder" @change="update(field.key,($event.target as HTMLTextAreaElement).value)"/>
              </label>
            </div>
          </div>
          <div v-else-if="activeTab==='steps'" class="studio-content">
            <div class="section-intro"><strong>任务拆解</strong><button @click="addStep" :disabled="current.steps.length>=32">＋ 添加步骤</button></div><p class="help">勾选已经完成的步骤。规划进度只反映你手动记录的结果，不代表智能体真实执行。</p>
            <div v-for="(step,i) in current.steps" :key="step.id" class="step"><input type="checkbox" :checked="step.done" @change="stepEdit(i,step.title,($event.target as HTMLInputElement).checked)"/><input :value="step.title" maxlength="180" @change="stepEdit(i,($event.target as HTMLInputElement).value,step.done)"/><button aria-label="移除步骤" @click="deleteStep(i)">×</button></div>
            <p v-if="!current.steps.length" class="help">暂时没有步骤，可以自己添加。</p><div class="total-progress"><span>规划完成度</span><strong>{{studioProgress(current)}}%</strong></div><div class="progress-track large"><i :style="{width:studioProgress(current)+'%'}"></i></div>
          </div>
          <div v-else-if="activeTab==='definition'" class="studio-content">
            <div class="section-intro"><strong>原生定义校验</strong><span>解析与权限预览可用</span></div><p class="help">粘贴符合 BrowserOS 契约的 JSON 定义。这里只校验，不会安装或执行。</p><textarea v-model="definitionText" class="json-editor" rows="14" spellcheck="false" placeholder="粘贴 JSON 定义（不包含任何口令或密钥）"></textarea>
            <div class="action-row"><button class="primary" :disabled="!definitionText.trim()||busy" @click="verify">验证定义</button><button :disabled="!validation?.valid||verifiedText!==definitionText||busy" @click="saveDefinition">保存已验证定义</button></div>
            <div v-if="validation" class="result" :class="{invalid:!validation.valid}">{{validation.valid?'✓ 定义通过原生验证':'定义尚未通过验证'}}<ul v-if="!validation.valid"><li v-for="(err,i) in validation.errors" :key="i">{{err}}</li></ul></div>
          </div>
          <div v-else class="studio-content"><div class="section-intro"><strong>权限与运行状态</strong><span>真实状态</span></div>
            <div class="permission-grid"><div v-for="item in permissions" :key="item.label"><small>{{item.label}}</small><strong>{{item.value}}</strong></div></div>
            <p class="help">运行引擎尚未完成接入，因此这里不展示虚假结果，不提供无效的运行按钮。</p>
          </div>
        </template>
        <div v-else class="main-empty"><div class="empty-icon">✦</div><h3>让你的第一个{{kindLabel}}从蓝图开始</h3><p>不用先编写代码。先说明目标、输入、输出与步骤，随后再连接真实执行能力。</p><button class="primary" @click="create" :disabled="blocked">新建{{kindLabel}}蓝图</button></div>
      </main>
    </div>
  </div>
</template>
<style scoped>
.studio{display:flex;flex-direction:column;height:100%;min-height:0;background:#f7f9fc;color:#24354b;font:13px system-ui,sans-serif}
.studio-header,.studio-metrics,.studio-sidebar,.studio-content{background:white}
.studio-header,.detail-heading,.side-heading,.section-intro,.total-progress{display:flex;align-items:center;justify-content:space-between;gap:12px}
.studio-header{padding:20px 28px;border-bottom:1px solid #e3eaf3}
h2,h3{margin:4px 0;letter-spacing:-.03em}h2{font-size:23px}h3{font-size:20px}
.eyebrow,.section-kicker{font-size:11px;font-weight:700;color:#5680bb}
.subtitle,.detail-heading p,.help,.storage-hint,.empty-list{color:#7b8da4;line-height:1.6}
button{font:inherit;cursor:pointer;background:white;border:1px solid #d8e1ed;border-radius:8px;color:#31465f;padding:8px 12px}
button:disabled{opacity:.5;cursor:default}.primary{background:#2355be;color:white;border-color:#2355be;font-weight:600}
.header-actions,.detail-actions,.action-row,.studio-tabs,.step{display:flex;align-items:center;gap:8px}
.studio-metrics{display:grid;grid-template-columns:repeat(4,1fr);padding:15px 28px;gap:16px;border-bottom:1px solid #e3eaf3}
.studio-metrics>div,.item-copy,.permission-grid>div{display:flex;flex-direction:column;gap:5px;min-width:0}
.studio-metrics strong{font-size:18px}.studio-metrics span,.percent,.item-copy small{font-size:11px;color:#8191a6}
.notice,.result{padding:12px;background:#f1faf4;color:#197349;border:1px solid #d6ecdf;border-radius:8px}.notice{margin:10px 24px 0}
.studio-layout{display:grid;grid-template-columns:260px minmax(0,1fr);flex:1;min-height:0;overflow:hidden}
.studio-sidebar{display:flex;flex-direction:column;gap:12px;padding:18px 12px;border-right:1px solid #e3eaf3;min-height:0}
.search,.studio-content input:not([type=checkbox]),.studio-content textarea{min-width:0;width:100%;box-sizing:border-box;padding:10px;border:1px solid #dce5ef;border-radius:8px;background:#fcfdff;color:#26384f;font:inherit}
.item-list,.studio-main{overflow:auto;min-height:0}.item-list{display:flex;flex-direction:column;flex:1;gap:5px}
.studio-item{display:flex;gap:10px;align-items:flex-start;background:transparent;text-align:left;border-color:transparent;white-space:normal}.studio-item.selected{border-color:#c6daf6;background:#eef5ff}
.item-symbol{display:grid;place-items:center;width:30px;height:30px;flex-shrink:0;background:#e4eefa;color:#3163b3;border-radius:8px}
.item-copy{flex:1}.item-copy strong,.item-copy small{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;max-width:160px}
.progress-track{height:4px;overflow:hidden;background:#e4ebf4;border-radius:5px}.progress-track i{display:block;background:#467bd6;height:100%}
.storage-hint{font-size:11px;border-top:1px solid #eff1f6;padding-top:10px}.studio-main{padding:24px 30px}
.detail-heading{padding-bottom:20px;align-items:start}.detail-actions .delete{color:#b84c4c}
.studio-tabs{border-bottom:1px solid #dce5f0;margin-bottom:18px;overflow-x:auto}.studio-tabs button{background:transparent;border:0;border-bottom:2px solid transparent;border-radius:0;white-space:nowrap}.studio-tabs button.active{color:#2355be;border-color:#2355be;font-weight:700}
.studio-content{max-width:1000px;padding:24px;border:1px solid #e7edf4;border-radius:12px;box-shadow:0 4px 18px #14355d09}
.section-intro{margin-bottom:16px}.studio-content label{display:flex;flex-direction:column;gap:8px;margin-bottom:16px;color:#455672;font-weight:600}
.two-col,.permission-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px}.two-col .wide{grid-column:1/-1}
.studio-content textarea{resize:vertical;line-height:1.5}.step{padding:10px 0;border-bottom:1px solid #eff2f7}.step input:not([type=checkbox]){flex:1}.step button{border:0}
.total-progress{margin:20px 0 8px}.progress-track.large{height:8px}.json-editor{font:12px ui-monospace,monospace!important;min-height:260px}
.result.invalid{background:#fff6f6;border-color:#f1d0d0;color:#ae3939}.permission-grid>div{background:#fafcfe;padding:16px;border:1px solid #e5ebf3;border-radius:9px}
.main-empty{text-align:center;max-width:360px;margin:9vh auto;color:#76869d;line-height:1.8}.empty-icon{color:#3974db;font-size:50px}
@media(max-width:850px){.studio-layout{grid-template-columns:200px minmax(0,1fr)}.studio-main{padding:16px}.two-col{grid-template-columns:1fr}}
@media(max-width:560px){.studio-layout{grid-template-columns:1fr;overflow:auto}.studio-sidebar{max-height:210px}.studio-metrics{grid-template-columns:repeat(2,1fr)}.studio-header{align-items:flex-start;flex-direction:column}.permission-grid{grid-template-columns:1fr}}
</style>
