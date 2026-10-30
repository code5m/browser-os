export function languageForPath(path) {
  const extension = path.split('.').pop()?.toLowerCase();
  return ({ts:'typescript',tsx:'typescript',js:'javascript',jsx:'javascript',rs:'rust',py:'python',java:'java',vue:'html',html:'html',css:'css',scss:'scss',json:'json',md:'markdown',sql:'sql',sh:'shell',yaml:'yaml',yml:'yaml',toml:'ini',xml:'xml'})[extension] || 'plaintext';
}
export function selectChatModel(profiles, bindings, current = '') {
  const available = profiles.filter(profile => profile.kind === 'chat');
  const preferred = [current, bindings?.workbench, bindings?.default];
  return preferred.find(id => id && available.some(profile => profile.id === id)) || available[0]?.id || '';
}
export function newBuffer(path, content, hash = null) { return {path, content, saved:content, original:content, hash}; }
export function isDirtyBuffer(buffer) { return buffer.hash === null || buffer.content !== buffer.saved; }
export function dirtyBuffers(buffers) { return buffers.filter(isDirtyBuffer); }
export function clampPane(value, min, max) { return Math.max(min, Math.min(max, value)); }
export function entryPath(parent, entry) { return entry.path || `${parent ? parent.replace(/\/$/,'')+'/' : ''}${entry.name}`; }
export function sessionMessages(session) { return (session.messages || []).filter(message => ['user','assistant'].includes(message.role) && typeof message.content === 'string').map(({role,content}) => ({role,content})); }
export function canApplyChanges(buffers, changes) { return !buffers.some(buffer => isDirtyBuffer(buffer) && changes.some(change => change.path === buffer.path)); }
export function proposalFor(buffer) { return {action:'propose',path:buffer.path,content:buffer.content,expected_hash:buffer.hash}; }
export function parseMcpJson(argsText,argumentsText) {
  let args,argumentsValue;
  try{args=JSON.parse(argsText);argumentsValue=JSON.parse(argumentsText);}catch{throw Error('MCP 参数不是有效 JSON');}
  if(!Array.isArray(args)||args.some(item=>typeof item!=='string'))throw Error('MCP 启动参数必须是字符串数组');
  if(argumentsValue===null||typeof argumentsValue!=='object'||Array.isArray(argumentsValue))throw Error('MCP 工具参数必须是 JSON 对象');
  return {args,argumentsValue};
}
export function reconcileSavedBuffer(buffer, attempted, file) {
  if (file.content !== attempted) return false;
  buffer.saved=file.content;buffer.hash=file.hash;return true;
}
export async function readRecoveryFile(call, path) {
  try { return await call({action:'read',path}); }
  catch (error) {
    if (String(error).replace(/^Error: /, '') === 'AGENT_PATH_NOT_FOUND') return null;
    throw error;
  }
}
export function limitUtf8Text(text,tail=false,limit=16000) {
  const encoder=new TextEncoder(),bytes=encoder.encode(text);
  if(bytes.length<=limit)return text;
  const marker=tail?'[输出已截断]\n':'\n[输出已截断]',available=limit-encoder.encode(marker).length;
  if(available<0)throw Error('WORKSPACE_OPERATION_LIMIT_INVALID');
  if(tail){
    let start=bytes.length-available;
    while(start<bytes.length&&(bytes[start]&0xc0)===0x80)start++;
    return marker+new TextDecoder().decode(bytes.subarray(start));
  }
  let end=available;
  while(end>0&&(bytes[end]&0xc0)===0x80)end--;
  return new TextDecoder().decode(bytes.subarray(0,end))+marker;
}
export function operationResultText(value) {
  const serialized=JSON.stringify(value);
  if(typeof serialized!=='string')throw Error('WORKSPACE_OPERATION_RESULT_INVALID');
  return limitUtf8Text(serialized);
}
const mcpFailureHints={
    MCP_SERVER_NOT_FOUND:'找不到该 MCP 服务，请刷新服务列表并检查配置',
    MCP_HTTP_NOT_IMPLEMENTED:'当前尚不支持 HTTP MCP 服务',
    MCP_EXECUTABLE_NOT_FOUND:'找不到 MCP 服务程序，请检查配置',
    MCP_ABSOLUTE_EXECUTABLE_REQUIRED:'MCP 程序路径无效，请检查配置',
    MCP_CREDENTIAL_MISSING:'无法读取 MCP 凭据，请检查系统密钥库',
    MCP_CREDENTIAL_REF_INVALID:'MCP 凭据引用无效，请检查配置',
    MCP_TIMEOUT_OR_CLOSED:'MCP 服务超时或已关闭，请检查服务状态',
    MCP_WRITE_TIMEOUT:'MCP 服务请求写入超时，请检查服务状态',
    MCP_REMOTE_ERROR:'MCP 服务返回错误，请检查服务状态',
    MCP_EXPLICIT_APPROVAL_REQUIRED:'本次 MCP 操作需要明确批准',
    MCP_PROJECT_ENV_REQUIRES_KEYRING_REFS:'项目 MCP 配置不能直接包含环境凭据，请改用密钥库引用',
    MCP_PLAINTEXT_ENV_REJECTED:'MCP 配置不能直接包含明文环境凭据',
    MCP_SHELL_COMMAND_REJECTED:'不允许使用 Shell 启动 MCP 服务',
    MCP_START_FAILED:'无法启动 MCP 服务，请检查程序和权限',
};
export function mcpFailureCode(error) {
  const code=String(error).match(/\bMCP_[A-Z_]+\b/)?.[0];
  return code&&Object.hasOwn(mcpFailureHints,code)?code:'WORKSPACE_OPERATION_FAILED';
}
export function mcpFailureHint(error) {
  const code=mcpFailureCode(error);
  return Object.hasOwn(mcpFailureHints,code)?mcpFailureHints[code]+'（'+code+'）':'操作失败；请检查 MCP 服务配置和状态';
}
export function providerFailureHint(error) {
  const match=String(error).match(/AGENT_PROVIDER_FAILED: (AI_[A-Z_]+)/);
  if(!match)return '模型调用失败，请检查配置';
  const hints={
    AI_NOT_CONFIGURED:'请先选择并配置模型',
    AI_CONFIG_BUSY:'模型配置正在使用，请稍后重试',
    AI_CONFIG_READ_FAILED:'模型配置无法读取，请检查配置',
    AI_CONFIG_UNSAFE:'模型配置文件不安全，请检查文件权限',
    AI_CONFIG_INVALID:'模型配置格式无效，请重新检查',
    AI_ENDPOINT_INVALID:'模型服务地址无效，请检查配置',
    AI_PROFILE_INVALID:'模型配置无效，请重新检查',
    AI_MODEL_NOT_CHAT:'所选模型不支持对话，请切换模型',
    AI_PROVIDER_BUSY:'模型请求正在处理中，请稍后重试',
    AI_PROVIDER_TIMEOUT:'模型请求超时，请检查服务状态后重试',
    AI_PROVIDER_DNS_FAILED:'模型服务域名解析失败，请检查 DNS 和网络',
    AI_PROVIDER_CONNECT_FAILED:'无法连接模型服务，请检查网络和服务地址',
    AI_PROVIDER_HTTP_ERROR:'模型服务返回 HTTP 错误，请检查模型名称、账号权限及服务状态',
    AI_PROVIDER_AUTH_FAILED:'模型服务拒绝认证或访问，请检查密钥和账号权限',
    AI_PROVIDER_ENDPOINT_NOT_FOUND:'模型服务接口不存在，请检查服务地址和接口路径',
    AI_PROVIDER_RATE_LIMITED:'模型服务限流或配额不足，请稍后重试或检查服务商配额',
    AI_PROVIDER_SERVER_ERROR:'模型服务暂时异常，请稍后重试',
    AI_PROVIDER_TLS_FAILED:'模型服务 TLS 连接失败，请检查证书和网络代理',
    AI_CREDENTIAL_UNAVAILABLE:'无法读取模型凭据，请检查系统密钥库',
    AI_CREDENTIAL_INVALID:'模型凭据无效，请重新配置',
    AI_CONTEXT_LIMIT:'上下文过长，请减少本轮附加内容',
    AI_CONTEXT_CREDENTIAL_DENIED:'上下文包含凭据信息，请移除后重试',
    AI_HTTP_CLIENT_UNAVAILABLE:'本机网络请求工具不可用',
    AI_HTTP_IO:'无法与本机网络请求工具通信',
    AI_PROVIDER_REQUEST_FAILED:'模型请求失败，请检查网络、服务地址和账号权限',
    AI_RESPONSE_LIMIT:'模型响应过大，请缩小本轮任务范围',
    AI_RESPONSE_INVALID:'模型返回格式无法解析',
    AI_RESPONSE_EMPTY:'模型没有返回有效内容',
  };
  return hints[match[1]]?hints[match[1]]+'（'+match[1]+'）':'模型调用失败，请检查配置';
}
export function sessionNotice(session) {
  if(session.status==='step_limit')return '已达到本轮工具步数上限；会话保留，可继续本轮任务';
  if(session.status==='approval_required')return '等待你审批工作区操作';
  if(session.status==='cancelled')return '本轮已停止';
  if(session.changes?.length)return '变更已生成，请查看差异后应用';
  return session.status==='complete'?'本轮已完成':'本轮状态已同步，请核对会话结果';
}
