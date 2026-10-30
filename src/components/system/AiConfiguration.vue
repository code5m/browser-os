<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { aiService, type AiConfiguration, type AiProfile, type AiScope } from '../../settings/public';
const config=ref<AiConfiguration>({profiles:[],bindings:{},plugins:[],import_sources:[]});
const busy=ref(false),notice=ref(''),credential=ref(''),editing=ref<AiProfile|null>(null),testAnswer=ref('');
const scopes:{id:AiScope;label:string}[]=[{id:'default',label:'全局默认'},{id:'database',label:'数据库'},{id:'git',label:'Git'},{id:'browser',label:'浏览器'},{id:'workbench',label:'工作台'}];
async function load(){config.value=await aiService.list();}
async function run(action:()=>Promise<void>){if(busy.value)return;busy.value=true;notice.value='';try{await action();}catch(e){notice.value=String(e);}finally{busy.value=false;credential.value='';}}
function edit(profile?:AiProfile){credential.value='';editing.value=profile?{...profile}:{id:crypto.randomUUID().replaceAll('-','').slice(0,24),name:'',model:'',base_url:'',kind:'chat',source:'manual',credential_ref:''};}
function importLocal(){void run(async()=>{const result=await aiService.importLocal();await load();notice.value=`新增 ${result.imported} 个模型，共 ${result.total} 个；插件凭据 ${result.plugin_credentials} 项。已保存的配置保持不变。`;});}
function save(){void run(async()=>{if(!editing.value)return;await aiService.save(editing.value,credential.value);editing.value=null;await load();notice.value='已保存；密钥保存在系统凭据库。';});}
function bind(scope:AiScope,event:Event){const id=(event.target as HTMLSelectElement).value;void run(async()=>{await aiService.bind(scope,id);await load();notice.value='模型分配已保存。';});}
function remove(id:string){void run(async()=>{const result=await aiService.remove(id);await load();notice.value=result.credential_removed?'已移除模型和凭据。':'模型已移除；系统凭据尚未删除，可在系统密钥管理器中清理。';});}
function test(id:string){void run(async()=>{testAnswer.value='';const result=await aiService.chat('default','Reply with OK only.',id);testAnswer.value=result.content;notice.value=`连接测试已收到 ${result.model} 的真实回复。`;});}
onMounted(()=>{void run(load);});
</script>
<template>
<section class="ai-config" aria-label="统一 AI 配置">
  <header><strong>统一 AI 配置</strong><button :disabled="busy" @click="importLocal">一键导入本机 AI 配置</button><button :disabled="busy" @click="edit()">添加模型</button><button :disabled="busy" @click="run(load)">刷新</button></header>
  <p>从 CodeBuddy、WorkBuddy 的本机配置和生图技能导入可识别的模型与插件凭据。密钥存入系统凭据库；各模块共用下方配置。首次请求会将你填写的内容发送到所选供应商。</p>
  <div class="bindings"><label v-for="scope in scopes" :key="scope.id">{{scope.label}}<select :aria-label="scope.label+' AI 模型'" :disabled="busy" :value="config.bindings[scope.id]||''" @change="bind(scope.id,$event)"><option value="">{{scope.id==='default'?'请选择聊天模型':'继承全局默认'}}</option><option v-for="p in config.profiles.filter(p=>p.kind==='chat')" :key="p.id" :value="p.id">{{p.name}}</option></select></label></div>
  <form v-if="editing" @submit.prevent="save">
    <label>名称<input v-model="editing.name" required maxlength="120"/></label><label>模型 ID<input v-model="editing.model" required maxlength="160"/></label>
    <label>API 基础地址<input v-model="editing.base_url" type="url" required placeholder="https://provider.example/v1"/></label>
    <label>用途<select v-model="editing.kind"><option value="chat">聊天（OpenAI 兼容）</option><option value="image">图像</option><option value="video">视频</option><option value="unsupported">其他协议</option></select></label>
    <label>API 密钥<input v-model="credential" type="password" autocomplete="new-password" placeholder="新配置必填；编辑留空保留原密钥"/></label>
    <div><button :disabled="busy" type="submit">保存</button><button type="button" @click="editing=null;credential=''">取消</button></div>
  </form>
  <div class="models"><article v-for="p in config.profiles" :key="p.id"><div><strong>{{p.name}}</strong><small>{{p.model}} · {{p.kind}} · {{p.base_url}}</small></div><button :disabled="busy" @click="edit(p)">编辑</button><button v-if="p.kind==='chat'" :disabled="busy" @click="test(p.id)">测试连接</button><button :disabled="busy" @click="remove(p.id)">移除</button></article></div>
  <p v-if="!config.profiles.length">尚未配置模型。点击一键导入，或手动添加 API 地址、模型 ID 和密钥。</p>
  <details v-if="config.plugins.length"><summary>插件凭据目录（{{config.plugins.length}}）</summary><p>凭据已保存。插件自身的运行协议、工具权限和执行器仍需逐项适配；导入不会启动插件。</p><div v-for="p in config.plugins" :key="p.source+p.name">{{p.name}} · {{p.transport}} · {{p.credential_refs.length}} 个凭据引用</div></details>
  <p>聊天模型可供各工作区使用。图像、视频模型保存为专用模型配置，当前助手不执行生成任务。</p>
  <p role="status">{{busy?'正在处理…':notice}}</p><pre v-if="testAnswer">{{testAnswer}}</pre>
</section>
</template>
<style scoped>
.ai-config{display:flex;flex-direction:column;gap:12px;font-size:13px;color:var(--text-primary,#26384a)}header,article{display:flex;align-items:center;gap:8px;flex-wrap:wrap}header strong{margin-right:auto}.bindings,form{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:12px}label{display:flex;flex-direction:column;gap:6px}input,select,button{font:inherit;padding:7px;border:1px solid #8295ac;border-radius:5px;background:var(--bg-secondary,#f5f7fa);color:inherit}button{cursor:pointer}button:disabled{opacity:.5}article{padding:10px;border-bottom:1px solid #8295ac55}article div{flex:1;min-width:180px}small{display:block;overflow-wrap:anywhere;margin-top:4px}p{line-height:1.6;margin:0}pre{white-space:pre-wrap}form{padding:12px;border:1px solid #8295ac;border-radius:6px}.models{max-height:420px;overflow:auto}
</style>
