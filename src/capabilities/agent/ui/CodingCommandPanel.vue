<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { commandWorkbench } from '../../script/public';
import { limitUtf8Text } from '../model/codingWorkbench.mjs';
import type { CommandSnippet, RunSnapshot } from '../../../types';
const props=defineProps<{proposal?:{snippet_id:string;values:Record<string,string>};beforeExecute?:()=>Promise<void>}>();
const emit=defineEmits<{(event:'context',value:{name:string;text:string}):void;(event:'result',value:{status:string;text:string}):void;(event:'running',value:boolean):void;(event:'attempt'):void}>();
const snippets=ref<CommandSnippet[]>([]), selected=ref(''), values=ref<Record<string,string>>({}), approved=ref(false), starting=ref(false), notice=ref('');
const run=ref<RunSnapshot|null>(null);
const attempted=ref(false),startUnknown=ref(false);
const snippet=computed(()=>snippets.value.find(item=>item.id===selected.value));
const running=computed(()=>starting.value||run.value?.status==='running');
let timer:ReturnType<typeof setTimeout>|undefined, disposed=false, generation=0;
watch(running,value=>emit('running',!!value));
async function fillProposal(){if(!props.proposal||running.value)return;selected.value=props.proposal.snippet_id;await nextTick();values.value={...props.proposal.values};approved.value=false;}
watch(()=>props.proposal,()=>void fillProposal());
watch(selected,()=>{approved.value=false;values.value=Object.fromEntries((snippet.value?.params||[]).map(p=>[p.name,p.default??'']));});
watch(values,()=>{approved.value=false;},{deep:true});
async function load(){approved.value=false;try{snippets.value=(await commandWorkbench.list()).filter(s=>s.enabled);notice.value='';}catch{snippets.value=[];notice.value='无法加载命令片段，请确认脚本能力及原生服务可用';}}
async function poll(id:string,version:number){
  if(disposed||version!==generation)return;
  try{const result=await commandWorkbench.status(id);if(disposed||version!==generation)return;run.value=result;}catch{notice.value='读取运行状态失败，可继续尝试取消；不把未知状态视作成功';}
  if(!disposed&&version===generation&&run.value?.status==='running')timer=setTimeout(()=>void poll(id,version),700);
}
async function start(){
  if(!approved.value||running.value||!snippet.value||(props.proposal&&attempted.value))return;
  const id=snippet.value.id,input={...values.value};approved.value=false;starting.value=true;notice.value='';run.value=null;startUnknown.value=false;const version=++generation;
  try{if(props.beforeExecute)await props.beforeExecute();}catch{starting.value=false;notice.value='无法确认执行状态，请恢复会话核对；命令未发出。';return;}
  if(disposed){starting.value=false;return;}
  attempted.value=true;emit('attempt');
  try{const result=await commandWorkbench.run(id,input);if(disposed){if(result.status==='running')await commandWorkbench.cancel(result.run_id);return;}run.value=result;if(result.status==='running')void poll(result.run_id,version);}catch{startUnknown.value=true;notice.value='命令启动失败或状态未知；请检查运行记录，不会自动重试。';}finally{starting.value=false;for(const p of snippet.value?.params||[])if(p.secret)values.value[p.name]='';}
}
function recordUnknown(){if(startUnknown.value&&!running.value)emit('result',{status:'failed',text:'COMMAND_START_FAILED_OR_UNKNOWN'});}
async function cancel(){if(!run.value||run.value.status!=='running')return;try{await commandWorkbench.cancel(run.value.run_id);notice.value='已请求取消，等待后端终态';}catch{notice.value='取消请求失败，运行状态尚未确认';}}
function attach(){if(!run.value||run.value.status==='running')return;const text=`status=${run.value.status}; exit=${run.value.exit_code}\n${limitUtf8Text(run.value.output_tail,true,15000)}`;emit('context',{name:`命令 ${run.value.script_id}`,text});emit('result',{status:run.value.status,text});notice.value='已加入待发送上下文；仅在发送下一条指令时传给模型';}
function shareCatalog(){emit('context',{name:'可用命令片段目录',text:JSON.stringify(snippets.value.map(item=>({snippet_id:item.id,name:item.name,dangerous:item.dangerous,params:item.params.map(p=>({name:p.name,type:p.param_type,required:p.required,secret:p.secret}))})))});notice.value='已加入命令目录；不包含凭据或参数默认值。发送后模型可以提出待审批请求。';}
onMounted(async()=>{await load();await fillProposal();});
onBeforeUnmount(()=>{disposed=true;++generation;clearTimeout(timer);if(run.value?.status==='running')void commandWorkbench.cancel(run.value.run_id).catch(()=>{});});
</script>
<template><section class="controlled-command" aria-label="受控命令执行">
  <div class="command-controls"><select v-model="selected" aria-label="已保存的命令片段" :disabled="running||!!proposal"><option value="">选择已保存且启用的命令片段</option><option v-for="item in snippets" :key="item.id" :value="item.id">{{item.name}}</option></select><button :disabled="running||(!!proposal&&attempted)" @click="load">刷新命令</button></div>
  <button v-if="!proposal&&snippets.length" :disabled="running" @click="shareCatalog">将可用命令目录加入模型上下文</button>
  <template v-if="snippet"><pre aria-label="命令预览">argv: {{JSON.stringify(snippet.argv)}} · 类型 {{snippet.interpreter}} · 超时 {{snippet.timeout_secs||60}} 秒</pre><p v-if="snippet.dangerous" class="warning">此命令标记为危险操作。请确认参数和影响范围。</p><p>执行目录与路径校验沿用命令片段配置；不会自动切换到当前编码项目。</p>
  <div class="parameters"><label v-for="param in snippet.params" :key="param.name">{{param.label||param.name}}{{param.required?' *':''}}<select v-if="param.param_type==='enum'" v-model="values[param.name]" :disabled="running||(!!proposal&&attempted)"><option v-for="option in param.options" :key="option">{{option}}</option></select><input v-else v-model="values[param.name]" :type="param.secret?'password':'text'" autocomplete="off" :disabled="running||(!!proposal&&attempted)"></label></div>
  <label><input v-model="approved" type="checkbox" :disabled="running||(!!proposal&&attempted)">我已检查命令和参数，批准本次执行</label><button :disabled="running||!approved||(!!proposal&&attempted)" @click="start">执行已审核命令</button></template>
  <p v-if="!snippets.length">暂无可用命令。请先在命令片段模块配置；智能体不能自行创建或执行任意 Shell。</p>
  <button v-if="run?.status==='running'" @click="cancel">取消命令运行</button><p v-if="run">状态：{{run.status}} · 退出码：{{run.exit_code??'—'}}{{run.truncated?' · 输出已截断':''}}</p><pre v-if="run" aria-label="命令输出">{{run.output_tail}}</pre>
  <button v-if="run&&run.status!=='running'" @click="attach">将此输出加入模型上下文</button><button v-if="proposal&&startUnknown" :disabled="running" @click="recordUnknown">记录启动失败或状态未知</button><p role="status">{{notice}}</p>
</section></template>
<style scoped>.controlled-command{overflow:auto;padding:8px;font:12px/1.5 system-ui;color:#c9d2e2}.command-controls,.parameters{display:flex;gap:8px;flex-wrap:wrap}.parameters label{display:flex;flex-direction:column}.controlled-command button,.controlled-command select,.controlled-command input{font:inherit;background:#202938;color:inherit;border:1px solid #46516a;border-radius:4px;padding:4px}.controlled-command button:disabled{opacity:.45}.controlled-command pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:240px;overflow:auto}.controlled-command p{margin:6px 0}.warning{color:#ffd18b}</style>
