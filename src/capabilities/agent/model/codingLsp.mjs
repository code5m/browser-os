// LSP uses zero-based UTF-16 coordinates, just like Monaco's one-based columns.
export function editorRange(range) {
  const start=range?.start,end=range?.end;
  if(!start||!end||![start.line,start.character,end.line,end.character].every(n=>Number.isSafeInteger(n)&&n>=0&&n<=262144))return null;
  if(end.line<start.line||(end.line===start.line&&end.character<start.character))return null;
  return {startLineNumber:start.line+1,startColumn:start.character+1,endLineNumber:end.line+1,endColumn:end.character+1};
}
export function completionItems(result,wordRange) {
  const kinds=[18,18,0,1,2,3,4,5,7,8,9,12,13,15,17,28,19,20,21,23,16,14,6,10,11,24];
  const items=Array.isArray(result)?result:result?.items;
  if(!Array.isArray(items))return [];
  return items.slice(0,200).filter(item=>typeof item?.label==='string').map(item=>({
    label:item.label,kind:kinds[item.kind]??18,detail:typeof item.detail==='string'?item.detail:undefined,
    insertText:typeof item.textEdit?.newText==='string'?item.textEdit.newText:typeof item.insertText==='string'?item.insertText:item.label,
    insertTextRules:item.insertTextFormat===2?4:0,
    range:editorRange(item.textEdit?.range||item.textEdit?.replace)||wordRange,
    sortText:item.sortText,filterText:item.filterText,
  }));
}
export function diagnosticMarkers(items) {
  if(!Array.isArray(items))return [];
  return items.slice(0,200).flatMap(item=>{
    const range=editorRange(item?.range);
    return range&&typeof item.message==='string'?[{...range,message:item.message.slice(0,8000),severity:({1:8,2:4,3:2,4:1})[item.severity]||2,source:'rust-analyzer'}]:[];
  });
}
export function definitionRanges(result) {
  return (Array.isArray(result)?result:result?[result]:[]).flatMap(item=>{
    // The sandbox mirrors only the current document. Never open a server-supplied host URI.
    if((item?.uri||item?.targetUri)!=='file:///work/document.rs')return [];
    const range=editorRange(item.range||item.targetSelectionRange||item.targetRange);
    return range?[range]:[];
  });
}
export function plainHover(text) {
  // Server content is data, not trusted Markdown, links, images or HTML.
  return String(text).slice(0,16000).replace(/[\\`*_{}[\]()<>!#|]/g,'\\$&');
}
export function mappedDefinitionTargets(response,currentPath) {
  return (Array.isArray(response?.definition_targets)?response.definition_targets:[]).slice(0,32).flatMap(item=>{
    if(typeof item?.path!=='string'||!item.path.endsWith('.rs')||item.path.startsWith('/')||item.path.includes('\\')||item.path.split('/').some(part=>!part||part==='.'||part==='..'))return [];
    if(!/^[a-f0-9]{64}$/.test(item.hash||''))return [];
    const range=editorRange(item.range);if(!range)return [];
    const expected=new URL('file:///work/');expected.pathname='/work/'+item.path;
    if(item.uri!==expected.href&&!(item.path===currentPath&&item.uri===response.document_uri))return [];
    return [{path:item.path,hash:item.hash,range}];
  });
}
export function diagnosticContext(response,path) {
  if(!Array.isArray(response?.diagnostics)||!/^[a-f0-9]{64}$/.test(response?.document_hash||'')||!['isolated_current_rust_document','isolated_rust_module_sources'].includes(response?.scope))throw Error('LSP_DIAGNOSTIC_RESULT_INVALID');
  const valid=response.diagnostics.filter(item=>editorRange(item?.range)&&typeof item?.message==='string');
  return {kind:'rust_language_analysis',path,document_hash:response.document_hash,scope:response.scope,analysis_completed:true,compilation_verified:false,position_encoding:'UTF-16; lines and columns are 1-based',diagnostic_count:response.diagnostics.length,errors:valid.filter(item=>item.severity===1).length,warnings:valid.filter(item=>item.severity===2).length,skipped_invalid:response.diagnostics.length-valid.length,truncated:valid.length>20||valid.some(item=>item.message.length>512),diagnostics:valid.slice(0,20).map(item=>({range:editorRange(item.range),severity:item.severity||3,message:item.message.slice(0,512)}))};
}
export function lspFailureHint(error) {
  const code=String(error).replace(/^Error: /,'');
  return ({LSP_BUSY:'语言服务正忙，请稍后重试',LSP_TIMEOUT:'语言服务超时，请稍后重试',LSP_TIMEOUT_OR_CLOSED:'语言服务超时或已关闭',LSP_SERVER_UNAVAILABLE:'语言服务不可用，请重新检测',LSP_APPROVAL_REQUIRED:'请先批准当前项目的语言服务',LSP_MODULE_APPROVAL_REQUIRED:'请先批准读取 Rust 模块树'})[code] || '语言服务请求失败，请检查配置或重新检测';
}

/** Fixed, non-sensitive recovery wording shared by the native status panel. */
export function sandboxRecoveryHint(status) {
  switch (status?.state) {
    case 'ready': return '隔离执行已就绪';
    case 'recovering': return '正在恢复上次未完成的隔离资源，请稍后重新检查';
    case 'pending': return '隔离资源尚未释放，可重试安全恢复';
    case 'review_required': return '隔离执行暂不可用：存在需要核对的恢复记录';
    case 'stopping': return '应用正在退出，隔离执行已停止接收任务';
    case 'not_initialized': return '隔离恢复存储未就绪，请检查启动诊断后重启';
    case 'unsupported': return '';
    default: return '隔离恢复状态未知，请重新检查';
  }
}
