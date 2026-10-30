<script setup lang="ts">
import { onBeforeUnmount, ref } from 'vue';
import { bridge } from '../../../bridge';
import { operationResultText, mcpFailureCode, mcpFailureHint } from '../model/codingWorkbench.mjs';
import CodingCommandPanel from './CodingCommandPanel.vue';
type Operation={id:string;kind:'command'|'mcp'|'git_context'|'database_context'|'browser_context'|'code_diagnostics';input:any;status:string;result?:string};
const props=defineProps<{operation:Operation;root:string;claimOperation:(id:string)=>Promise<void>;diagnosticsAvailable?:boolean;runDiagnostics?:(input:any)=>Promise<any>}>();
const emit=defineEmits<{(event:'resolve',value:{id:string;status:string;result:string}):void}>();
const approved=ref(false),busy=ref(false),consumed=ref(false),result=ref(''),notice=ref('');
const outcome=ref('succeeded');
let disposed=false;
onBeforeUnmount(()=>{disposed=true;});
async function executeOperation(){
  if(!approved.value||busy.value||consumed.value)return;
  if(props.operation.kind==='code_diagnostics'&&(!props.diagnosticsAvailable||!props.runDiagnostics))return;
  approved.value=false;busy.value=true;consumed.value=true;
  try{await props.claimOperation(props.operation.id);}catch{busy.value=false;notice.value='无法确认执行状态，请恢复会话核对；请求未发出。';return;}
  if(disposed){busy.value=false;return;}
  try{
    const input=props.operation.input;
    let response:any;
    if(props.operation.kind==='mcp')response=await bridge.codingService({root:props.root,action:'mcp_call',approved:true,server_id:input.server_id,tool:input.tool,arguments:input.arguments});
    else if(props.operation.kind==='code_diagnostics')response=await props.runDiagnostics!(input);
    else if(props.operation.kind==='git_context')response={result:await (await import('../../git/public')).readCodingGitContext(props.root)};
    else if(props.operation.kind==='database_context')response={result:(await import('../../database/public')).readCodingDatabaseContext()};
    else if(props.operation.kind==='browser_context')response={result:(await import('../../browser/public')).readCodingBrowserContext()};
    else throw new Error('OPERATION_KIND_UNSUPPORTED');
    outcome.value=response.result?.isError?'failed':'succeeded';result.value=operationResultText(response.result);notice.value='执行结果尚未发送模型，请先检查，再记录结果。';
  }catch(error){outcome.value='failed';result.value=props.operation.kind==='mcp'?mcpFailureCode(error):'WORKSPACE_OPERATION_FAILED';notice.value=(props.operation.kind==='mcp'?mcpFailureHint(error):'操作失败；确认能力已启用及所需上下文已加载')+'。不会自动重试，请记录失败结果。';}finally{busy.value=false;}
}
function record(){if(!result.value||busy.value)return;emit('resolve',{id:props.operation.id,status:outcome.value,result:result.value});}
function commandResult(value:{status:string;text:string}){emit('resolve',{id:props.operation.id,status:value.status,result:value.text});}
function reject(){if(!busy.value&&!consumed.value)emit('resolve',{id:props.operation.id,status:'rejected',result:'The user rejected this operation.'});}
function recordUnknown(){emit('resolve',{id:props.operation.id,status:'failed',result:'OPERATION_RESULT_UNKNOWN_USER_REVIEW_REQUIRED'});}
</script>
<template><section class="operation-review" :aria-label="'审批操作 '+operation.kind">
 <strong>等待审批 · {{({command:'命令片段',mcp:'MCP 工具',code_diagnostics:'代码诊断',git_context:'Git 状态',database_context:'数据库结构',browser_context:'浏览器标签'} as any)[operation.kind]}}</strong><pre>{{JSON.stringify(operation.input,null,2)}}</pre>
 <p v-if="operation.kind==='git_context'">只读当前编码项目的 Git 状态：{{root}}。不提交、拉取或推送。</p><p v-if="operation.kind==='database_context'">仅读取数据库模块当前已加载的表/视图名称，不连接数据库、不执行 SQL。</p><p v-if="operation.kind==='browser_context'">仅读取现有标签标题和地址；地址移除账号、密码、查询参数和片段，不读取网页内容。</p>
 <p v-if="operation.kind==='code_diagnostics'">请先在编辑器启用本项目语言服务，并处理待审核变更。仅使用你已批准的隔离范围分析 Rust 文本；不运行 Cargo、测试或项目脚本，诊断成功不代表编译通过。</p>
 <template v-if="operation.status==='executing'&&!consumed"><p class="recovery-warning">此操作的执行结果未知，可能已经执行。请检查运行记录或外部系统，再记录状态；不会自动再次执行。</p><button @click="recordUnknown">已核对，记录结果未知</button></template>
 <template v-else>
 <p>{{consumed?'请求已发出，请核实并记录结果。':'这是模型提出的请求，尚未执行。请核对目标和参数。'}}{{operation.kind==='code_diagnostics'?'本次只分析源文本，不修改项目文件或应用修复。':'执行可能修改文件、外部系统或数据。'}}</p>
 <CodingCommandPanel v-if="operation.kind==='command'" :proposal="operation.input" :before-execute="()=>claimOperation(operation.id)" @result="commandResult" @running="busy=$event" @attempt="consumed=true"/>
 <template v-else><label><input v-model="approved" type="checkbox" :disabled="busy||consumed">我批准这一次工作区操作</label><button :disabled="!approved||busy||consumed||(operation.kind==='code_diagnostics'&&(!diagnosticsAvailable||!runDiagnostics))" @click="executeOperation">{{operation.kind==='mcp'?'执行本次 MCP 请求':operation.kind==='code_diagnostics'?'执行本次代码诊断':'读取本次工作区上下文'}}</button><pre v-if="result" aria-label="待记录的操作结果">{{result}}</pre><button v-if="result" :disabled="busy" @click="record">记录结果供智能体续跑</button></template>
 <button :disabled="busy||consumed" @click="reject">拒绝此请求</button></template><p role="status">{{notice}}</p>
</section></template>
<style scoped>.operation-review{border:1px solid #8d7549;border-radius:6px;padding:10px;margin:10px 0;background:#202737}.operation-review pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:200px;overflow:auto;font:11px/1.5 monospace}.operation-review p,.operation-review label{font-size:12px}.recovery-warning{color:#ffd18b}.operation-review button{margin:5px;padding:5px;color:inherit;background:#303f54;border:1px solid #64728b;border-radius:4px}.operation-review button:disabled{opacity:.4}</style>
