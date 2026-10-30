<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import { aiService, type AiScope, type AiProfile } from '../../settings/public';
const props=defineProps<{scope:AiScope;context?:string}>();
const profiles=ref<AiProfile[]>([]),selected=ref(''),prompt=ref(''),includeContext=ref(false),busy=ref(false),answer=ref(''),error=ref('');
let revision=0;
watch(()=>props.context,()=>{revision++;answer.value='';includeContext.value=false;});
async function load(){try{const config=await aiService.list();profiles.value=config.profiles.filter(p=>p.kind==='chat');selected.value=config.bindings[props.scope]||config.bindings.default||'';}catch(e){error.value=String(e);}}
async function send(){if(busy.value||!prompt.value.trim())return;busy.value=true;error.value='';const turn=revision;try{const message=prompt.value+(includeContext.value&&props.context?`\n\nUser-approved context:\n${props.context.slice(0,12000)}`:'');const result=await aiService.chat(props.scope,message,selected.value);if(turn===revision)answer.value=result.content;}catch(e){if(turn===revision)error.value=String(e);}finally{busy.value=false;}}
onMounted(load);
</script>
<template><section class="unified-ai" aria-label="工作区 AI 助手"><header><strong>AI 助手</strong><select v-model="selected" :disabled="busy" aria-label="AI 模型"><option value="">使用统一配置</option><option v-for="p in profiles" :key="p.id" :value="p.id">{{p.name}}</option></select><button :disabled="busy" @click="load">刷新配置</button></header><p>在设置 → 统一 AI 配置中一键导入模型。回复作为建议展示；发送前请检查内容。</p><textarea v-model="prompt" rows="3" maxlength="12000" :disabled="busy" placeholder="输入需要 AI 协助的问题" aria-label="AI 问题"/><label v-if="context"><input type="checkbox" v-model="includeContext" :disabled="busy"/>附带下方当前上下文（将发送给所选供应商）</label><details v-if="context"><summary>查看可附带上下文</summary><pre>{{context.slice(0,12000)}}</pre></details><button :disabled="busy||!prompt.trim()" @click="send">{{busy?'正在请求…':'发送'}}</button><p v-if="error" role="alert">{{error}}</p><pre v-if="answer" aria-label="AI 回复">{{answer}}</pre></section></template>
<style scoped>
.unified-ai{display:flex;flex-direction:column;gap:10px;padding:14px;min-width:0;max-height:65vh;overflow:auto;border:1px solid #8295ac66;border-radius:6px;background:var(--ui-surface,#fff);color:var(--ui-text,#333)}header{display:flex;align-items:center;gap:8px;flex-wrap:wrap}header strong{margin-right:auto}textarea,select,button{font:inherit;padding:7px;max-width:100%;background:var(--ui-surface,#fff);color:inherit;border:1px solid #8295ac88;border-radius:4px}textarea{resize:vertical;flex-shrink:0}p{margin:0;font-size:12px;line-height:1.5}pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:380px;overflow:auto;font:13px/1.65 monospace}button{cursor:pointer}button:disabled{opacity:.5}
</style>
