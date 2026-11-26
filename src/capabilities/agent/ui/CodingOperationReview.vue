<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { bridge } from '../../../bridge';
import { operationResultText, mcpFailureHint } from '../model/codingWorkbench.mjs';
import CodingCommandPanel from './CodingCommandPanel.vue';
import McpExecutionReview from './McpExecutionReview.vue';
type Operation={id:string;kind:'command'|'mcp'|'git_context'|'database_context'|'browser_context'|'code_diagnostics';input:any;status:string;result?:string;review_revision?:string;can_reconcile?:boolean;execution?:{authority?:{effect_id:string;authority:{account:string;namespace:string}}}};
const props=defineProps<{operation:Operation;root:string;claimOperation:(id:string)=>Promise<void>;executeOwned:(id:string,grant:Record<string,unknown>)=>Promise<void>;diagnosticsAvailable?:boolean;runDiagnostics?:(input:any)=>Promise<any>}>();
const emit=defineEmits<{(event:'resolve',value:{id:string;status:string;result:string;review_revision?:string}):void}>();
const approved=ref(false),busy=ref(false),consumed=ref(false),result=ref(''),notice=ref('');
const outcome=ref('succeeded');
const evidence=ref('');
const executionEnded=ref(false);
const reconciliation=ref<'succeeded'|'not_applied'>('succeeded');
watch(()=>[props.root,props.operation.id,props.operation.review_revision],()=>{
  evidence.value='';executionEnded.value=false;reconciliation.value='succeeded';
},{flush:'sync'});
const resultOwner=ref<{id:string;root:string}|null>(null);
const mcpReview=ref<any>(null),credentialsApproved=ref(false),reviewing=ref(false);
let reviewSequence=0;
const credentialScopeApproved=computed(()=>!mcpReview.value?.credential_refs?.length&&!mcpReview.value?.environment_names?.length||credentialsApproved.value);
watch(()=>[props.root,props.operation.id,JSON.stringify(props.operation.input)],()=>{reviewSequence++;mcpReview.value=null;credentialsApproved.value=false;approved.value=false;reviewing.value=false;},{flush:'sync'});
async function reviewMcp(){
  if(busy.value||consumed.value||reviewing.value)return;
  const sequence=++reviewSequence;approved.value=false;credentialsApproved.value=false;mcpReview.value=null;reviewing.value=true;
  const input=props.operation.input;
  try{
    const review=await bridge.codingService({root:props.root,action:'mcp_review',review_action:'mcp_call',server_id:input.server_id,tool:input.tool,arguments:input.arguments});
    if(disposed||sequence!==reviewSequence)return;
    mcpReview.value=review;notice.value='请核对实际程序、参数和凭据范围，再批准本次调用。';
  }catch(error){if(!disposed&&sequence===reviewSequence)notice.value=mcpFailureHint(error);}
  finally{if(sequence===reviewSequence)reviewing.value=false;}
}
let disposed=false;
onBeforeUnmount(()=>{disposed=true;});
async function executeOperation(){
  if(!approved.value||busy.value||consumed.value)return;
  if(props.operation.kind==='mcp'&&(!mcpReview.value||!credentialScopeApproved.value||reviewing.value))return;
  if(props.operation.kind==='code_diagnostics'&&(!props.diagnosticsAvailable||!props.runDiagnostics))return;
  const owner={id:props.operation.id,root:props.root},kind=props.operation.kind,input=JSON.parse(JSON.stringify(props.operation.input)),review=mcpReview.value?JSON.parse(JSON.stringify(mcpReview.value)):null;
  const reviewedInput=JSON.stringify(input),diagnostics=props.runDiagnostics;
  approved.value=false;busy.value=true;consumed.value=true;resultOwner.value=owner;
  if(kind==='mcp'){
    try{await props.executeOwned(owner.id,{review_revision:review.revision,approved_credential_refs:review.credential_refs,approved_environment_names:review.environment_names});}
    catch{notice.value='原生执行结果未确认，请恢复当前会话；不会重试。';}
    finally{busy.value=false;}
    return;
  }
  try{await props.claimOperation(owner.id);}catch{busy.value=false;notice.value='无法确认执行状态，请恢复会话核对；请求未发出。';return;}
  if(disposed){busy.value=false;return;}
  if(owner.root!==props.root||owner.id!==props.operation.id||kind!==props.operation.kind||reviewedInput!==JSON.stringify(props.operation.input)){
    busy.value=false;outcome.value='failed';result.value='OPERATION_REVIEW_CHANGED';notice.value='待执行请求已变化，本次未执行。请回到原会话记录结果后重新审核。';return;
  }
  try{
    let response:any;
    if(kind==='code_diagnostics')response=await diagnostics!(input);
    else if(kind==='git_context')response={result:await (await import('../../git/public')).readCodingGitContext(owner.root)};
    else if(kind==='database_context')response={result:(await import('../../database/public')).readCodingDatabaseContext()};
    else if(kind==='browser_context')response={result:(await import('../../browser/public')).readCodingBrowserContext()};
    else throw new Error('OPERATION_KIND_UNSUPPORTED');
    outcome.value=response.result?.isError?'failed':'succeeded';result.value=operationResultText(response.result);notice.value='执行结果尚未发送模型，请先检查，再记录结果。';
  }catch{outcome.value='failed';result.value='WORKSPACE_OPERATION_FAILED';notice.value='操作失败；确认能力已启用及所需上下文已加载。不会自动重试。';}finally{busy.value=false;}
}
function record(){if(!result.value||busy.value||resultOwner.value?.id!==props.operation.id||resultOwner.value?.root!==props.root)return;emit('resolve',{id:resultOwner.value.id,status:outcome.value,result:result.value});}
function commandResult(value:{status:string;text:string}){emit('resolve',{id:props.operation.id,status:value.status,result:value.text});}
function reject(){if(!busy.value&&!consumed.value)emit('resolve',{id:props.operation.id,status:'rejected',result:'The user rejected this operation.'});}
function recordUnknown(){emit('resolve',{id:props.operation.id,status:'unknown',result:'OPERATION_RESULT_UNKNOWN_USER_REVIEW_REQUIRED'});}
function reconcile(){if(!executionEnded.value||!evidence.value.trim()||!props.operation.review_revision||props.operation.can_reconcile!==true)return;emit('resolve',{id:props.operation.id,status:reconciliation.value==='succeeded'?'reconciled_succeeded':'reconciled_not_applied',result:evidence.value.trim(),review_revision:props.operation.review_revision});}
</script>
<template><section class="operation-review" :aria-label="'审批操作 '+operation.kind">
 <strong>等待审批 · {{({command:'命令片段',mcp:'MCP 工具',code_diagnostics:'代码诊断',git_context:'Git 状态',database_context:'数据库结构',browser_context:'浏览器标签'} as any)[operation.kind]}}</strong><pre>{{JSON.stringify(operation.input,null,2)}}</pre>
 <p v-if="operation.kind==='git_context'">只读当前编码项目的 Git 状态：{{root}}。不提交、拉取或推送。</p><p v-if="operation.kind==='database_context'">仅读取数据库模块当前已加载的表/视图名称，不连接数据库、不执行 SQL。</p><p v-if="operation.kind==='browser_context'">仅读取现有标签标题和地址；地址移除账号、密码、查询参数和片段，不读取网页内容。</p>
 <p v-if="operation.kind==='code_diagnostics'">请先在编辑器启用本项目语言服务，并处理待审核变更。仅使用你已批准的隔离范围分析 Rust 文本；不运行 Cargo、测试或项目脚本，诊断成功不代表编译通过。</p>
 <template v-if="operation.status==='result_ready'"><p>原生执行回执已持久化。核对后再将实际结果交给模型；命令详细输出可在命令运行历史中查看。</p><pre>{{operation.result}}</pre><button :disabled="busy" @click="emit('resolve',{id:operation.id,status:'accepted',result:''})">接受原生回执并记录结果</button></template>
 <template v-else-if="operation.status==='unknown'||(operation.status==='executing'&&!consumed)"><p class="recovery-warning">外部结果尚未确认。超时、取消或通信失败不代表操作未生效；不会自动重试。</p><p v-if="operation.execution?.authority">提供方：{{operation.execution.authority.authority.account}} / {{operation.execution.authority.authority.namespace}}；幂等键：browseros:{{operation.execution.authority.effect_id}}。重新读取当前会话将自动核对独立签名终态，不会执行原请求。</p><button v-if="operation.status==='executing'" @click="recordUnknown">暂时记录为结果未知</button><label>核对外部系统后的结果 <select v-model="reconciliation"><option value="succeeded">已确认生效</option><option value="not_applied">已确认未生效</option></select></label><textarea v-model="evidence" placeholder="填写实际回执、外部系统记录或核对依据（不要包含凭据）" aria-label="对账依据"></textarea><label><input v-model="executionEnded" type="checkbox">已核实原执行者和外部任务均已终止，不存在迟到副作用</label><p v-if="operation.status==='unknown'&&operation.can_reconcile!==true" class="recovery-warning">原始审批或工作区证据不足，只能保留未知记录；不能确认生效、未生效或重放。</p><button :disabled="!executionEnded||!evidence.trim()||busy||operation.status==='executing'||!operation.review_revision||operation.can_reconcile!==true" @click="reconcile">确认并记录对账结果</button></template>
 <template v-else>
 <p>{{consumed?'请求已发出，请核实并记录结果。':'这是模型提出的请求，尚未执行。请核对目标和参数。'}}{{operation.kind==='code_diagnostics'?'本次只分析源文本，不修改项目文件或应用修复。':'执行可能修改文件、外部系统或数据。'}}</p>
 <CodingCommandPanel v-if="operation.kind==='command'" :proposal="operation.input" :execute-owned="(values,revision)=>executeOwned(operation.id,{values,expected_revision:revision})" @result="commandResult" @running="busy=$event" @attempt="consumed=true"/>
 <template v-else><template v-if="operation.kind==='mcp'"><button :disabled="busy||consumed||reviewing" @click="reviewMcp">读取本次执行范围</button><McpExecutionReview :review="mcpReview" v-model:approved="credentialsApproved" :disabled="busy||consumed"/></template><label><input v-model="approved" type="checkbox" :disabled="busy||consumed">我批准这一次工作区操作</label><button :disabled="!approved||busy||consumed||(operation.kind==='mcp'&&(!mcpReview||!credentialScopeApproved||reviewing))||(operation.kind==='code_diagnostics'&&(!diagnosticsAvailable||!runDiagnostics))" @click="executeOperation">{{operation.kind==='mcp'?'执行本次 MCP 请求':operation.kind==='code_diagnostics'?'执行本次代码诊断':'读取本次工作区上下文'}}</button><pre v-if="result" aria-label="待记录的操作结果">{{result}}</pre><button v-if="result" :disabled="busy" @click="record">记录结果供智能体续跑</button></template>
 <button :disabled="busy||consumed" @click="reject">拒绝此请求</button></template><p role="status">{{notice}}</p>
</section></template>
<style scoped>.operation-review{border:1px solid #8d7549;border-radius:6px;padding:10px;margin:10px 0;background:#202737}.operation-review pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:200px;overflow:auto;font:11px/1.5 monospace}.operation-review p,.operation-review label{font-size:12px}.recovery-warning{color:#ffd18b}.operation-review button{margin:5px;padding:5px;color:inherit;background:#303f54;border:1px solid #64728b;border-radius:4px}.operation-review button:disabled{opacity:.4}</style>
