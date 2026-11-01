<script setup lang="ts">
import {computed,defineAsyncComponent,onBeforeUnmount,onMounted,ref} from 'vue';
import {useLayoutStore} from '../../../stores/useLayoutStore';
import {aiService} from '../../../settings/public';
import {browserAgentAdapter} from '../../browser/public';
import {databaseAgentAdapter} from '../../database/public';
import {gitAgentAdapter} from '../../git/public';
import {codingAgentAdapter} from '../public';
import {createAgentTaskRuntime} from '../../../shared/agent/taskRuntime.mjs';
const layout=useLayoutStore();
type Pane='code'|'database'|'git'|'terminal'|'browser'|'agents';
const pane=ref<Pane>('code');
const pages:Record<Pane,string>={code:'编码',database:'数据库',git:'Git',terminal:'终端',browser:'浏览器',agents:'智能体'};
const panels={
 code:defineAsyncComponent(()=>import('./CodingWorkbench.vue')),
 database:defineAsyncComponent(()=>import('../../database/public').then(m=>m.loadDatabaseStudioSurface())),
 git:defineAsyncComponent(()=>import('../../git/public').then(m=>m.loadGitStudioSurface())),
 terminal:defineAsyncComponent(()=>import('../../terminal/public').then(m=>m.loadTerminalStudioSurface())),
};
const selectedPanel=computed(()=>panels[pane.value as keyof typeof panels] || null);
const runtime=createAgentTaskRuntime({chat:(scope:string,msg:string,model:string)=>aiService.chat(scope as any,msg,model)});
for(const adapter of [browserAgentAdapter,databaseAgentAdapter,gitAgentAdapter,codingAgentAdapter])runtime.register(adapter);
const workload=ref('coding'),instruction=ref(''),root=ref(''),withContext=ref(false),modelId=ref('');
const tasks=ref(runtime.inspect()),models=ref<{id:string;name:string}[]>([]),notice=ref('');
const unsubscribe=runtime.subscribe(()=>{tasks.value=runtime.inspect()});
const current=computed(()=>[...tasks.value].reverse().find(t=>t.workload===workload.value));
async function refresh(){try{const cfg=await aiService.list();models.value=cfg.profiles.filter(x=>x.kind==='chat').map(x=>({id:x.id,name:x.name}));notice.value='';}catch{notice.value='请先配置 AI 模型';}}
function send(){try{const scope=['git','coding'].includes(workload.value)?root.value:'';runtime.submit({workload:workload.value,instruction:instruction.value,scope,modelId:modelId.value,withContext:withContext.value});tasks.value=runtime.inspect();instruction.value='';notice.value='';}catch(e){notice.value=String(e);}}
function cancel(){if(current.value){runtime.cancel(current.value.id);tasks.value=runtime.inspect();}}
onMounted(refresh);onBeforeUnmount(()=>{unsubscribe();runtime.dispose();});
</script>
<template><section class="developer-studio" aria-label="Developer Studio">
  <header><strong>Developer Studio</strong><span>统一编辑器 · Agent Channel · 能力组合</span></header>
  <nav aria-label="开发工具"><button v-for="(label,id) in pages" :key="id" :class="{active:pane===id}" @click="pane=id">{{label}}</button></nav>
  <div class="content">
    <section class="surface"><component v-if="selectedPanel" :is="selectedPanel"/>
      <div v-else-if="pane==='browser'" class="info"><strong>Browser Host 独立拥有原生 WebView</strong><p>不复制 WebView 或创建另一套布局调度。</p><button @click="layout.activateBrowser()">打开浏览器</button></div>
      <div v-else class="info"><strong>公共 Agent Channel</strong><p>右侧四类工作负载复用同一任务队列、授权上下文与统一 AI 配置。</p></div>
    </section>
    <aside class="agent" aria-label="Agent Channel">
      <strong>Agent Channel</strong>
      <label>类型<select v-model="workload"><option value="coding">Coding Agent</option><option value="browser">Browser Agent</option><option value="database">Database Agent</option><option value="git">Git Agent</option></select></label>
      <label v-if="workload==='coding'||workload==='git'">项目路径<input v-model="root" placeholder="/home/.../repo"/></label>
      <label>模型<select v-model="modelId"><option value="">使用统一配置</option><option v-for="m in models" :key="m.id" :value="m.id">{{m.name}}</option></select></label>
      <label class="check"><input v-model="withContext" type="checkbox"/>明确授权读取并发送当前业务上下文</label>
      <textarea v-model="instruction" rows="5" aria-label="Agent 任务" maxlength="12000" placeholder="描述你的任务"/>
      <div class="actions"><button :disabled="!instruction.trim()" @click="send">运行</button><button :disabled="!current||!['queued','running'].includes(current.state)" @click="cancel">取消</button><button @click="refresh">刷新模型</button></div>
      <p v-if="notice" role="alert">{{notice}}</p><div v-if="current" class="result" role="status"><strong>{{current.workload}} · {{current.state}}</strong><pre>{{current.answer||current.error||'处理中'}}</pre></div>
      <small>Agent 仅提出建议，不能直接执行 SQL、终端、Git、网页脚本或文件写入。写操作必须经过各能力原有审核。取消后网络请求不保证立即中断。</small>
    </aside>
  </div>
</section></template>
<style scoped>
.developer-studio{display:flex;flex-direction:column;height:100%;min-height:0;background:#141b27;color:#dae5f4}
header{display:flex;gap:15px;align-items:center;padding:10px;border-bottom:1px solid #354056}header span,small{font-size:11px;opacity:.7}
nav{display:flex;gap:4px;padding:6px;overflow-x:auto}nav button{background:transparent;color:inherit;border:0;padding:7px 12px;cursor:pointer}nav button.active{border-bottom:2px solid #88b5fd}
.content{display:flex;flex:1;min-height:0}.surface{flex:1;min-height:0;min-width:0;overflow:hidden}.agent{display:flex;flex-direction:column;gap:10px;width:min(340px,38vw);min-width:250px;overflow:auto;padding:12px;border-left:1px solid #354056;box-sizing:border-box}
.agent label{display:flex;flex-direction:column;gap:4px;font-size:12px}.agent label.check{flex-direction:row;align-items:center}
select,input:not([type=checkbox]),textarea{background:#202a39;color:inherit;border:1px solid #53617a;padding:7px;max-width:100%;box-sizing:border-box;font:inherit}
.actions{display:flex;flex-wrap:wrap;gap:6px}.actions button{padding:6px}.actions button:disabled{opacity:.5}.result pre{max-height:40vh;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px}.info{padding:30px;max-width:500px}
</style>