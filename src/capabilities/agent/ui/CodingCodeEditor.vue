<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { languageForPath } from '../model/codingWorkbench.mjs';
import {completionItems,definitionRanges,diagnosticMarkers,editorRange,plainHover,mappedDefinitionTargets} from '../model/codingLsp.mjs';
import {ensureEditorWorkers,ensureTypeScriptServices,syncOpenScriptModels,retainEditorServices} from '../model/codingEditorServices.mjs';
const releaseServices=retainEditorServices();
const props=defineProps<{path:string;modelValue:string;scope?:string;openPaths?:string[];original?:string;diff?:boolean;readOnly?:boolean;lspRequest?:(method:string,path:string,content:string,line:number,character:number,cancelled?:()=>boolean)=>Promise<any>;navigateDefinition?:(target:any)=>Promise<boolean>;navigateOpenBuffer?:(target:any)=>Promise<boolean>;location?:{path:string;line:number;column:number;id:number}}>();
const emit=defineEmits<{(event:'update:modelValue',value:string):void;(event:'edit',value:{path:string;content:string;scope?:string}):void;(event:'save'):void;(event:'diagnostics',value:string[]):void;(event:'ready',value:boolean):void}>();
const host=ref<HTMLElement|null>(null), error=ref(''), ready=ref(false);
let monaco:any, editor:any, model:any, originalModel:any, markerSubscription:any;
let editorModeDiff = false;
const models=new Map<string,any>(), viewStates=new Map<string,any>();
const retiringModels=new Map<string,any>();
const modelSubscriptions=new Map<any,any>();
let mountedPath='',locationApplied=0;
let disposed=false, applying=false, initialized=false, generation=0;
const pendingDiffs=new Set<Promise<void>>();
const languageProviders:any[]=[];
const definitionTargets=new Map<string,any>(),namespace=crypto.randomUUID();
let diagnosticTimer:ReturnType<typeof setTimeout>|undefined, lspGeneration=0;
async function languageRequest(method:string,target:any,position:any,token?:any,full=false){
  const request=props.lspRequest,version=target.getVersionId(),epoch=lspGeneration,path=props.path;
  if(!request||props.readOnly||props.diff||target!==model||!path.endsWith('.rs')||token?.isCancellationRequested)return null;
  const response=await request(method,path,target.getValue(),position.lineNumber-1,position.column-1,()=>disposed||epoch!==lspGeneration||target.isDisposed()||target!==model||version!==target.getVersionId()||!!token?.isCancellationRequested);
  if(disposed||epoch!==lspGeneration||request!==props.lspRequest||target!==model||target.isDisposed()||version!==target.getVersionId()||path!==props.path||token?.isCancellationRequested)return null;
  monaco.editor.setModelMarkers(target,'coding-lsp',diagnosticMarkers(response?.diagnostics));
  return full?response:response?.result;
}
function scheduleDiagnostics(){
  clearTimeout(diagnosticTimer);
  if(!props.lspRequest||!model||props.diff||props.readOnly)return;
  const target=model;
  diagnosticTimer=setTimeout(()=>{void languageRequest('diagnostics',target,{lineNumber:1,column:1}).catch(()=>{});},900);
}
function detachEditor(){if(editor){editor.setModel(null);editor.dispose();editor=null;}}
function pruneClosedModels(){
  if(!props.openPaths)return;
  const open=new Set(props.openPaths);
  for(const [path,cached] of models){
    if(open.has(path)||cached===model)continue;
    models.delete(path);viewStates.delete(path);retiringModels.set(path,cached);
    void Promise.allSettled([...pendingDiffs]).then(()=>{
      if(retiringModels.get(path)===cached){retiringModels.delete(path);modelSubscriptions.get(cached)?.dispose();modelSubscriptions.delete(cached);cached.dispose();}
    });
  }
}
function applyLocation(){const value=props.location;if(value?.path!==props.path||mountedPath!==props.path||value.id===locationApplied||!editor)return;const target=editor.getModifiedEditor?.()||editor;target.setPosition({lineNumber:value.line,column:value.column});target.revealPositionInCenter(target.getPosition());target.focus();locationApplied=value.id;}
function renameSymbol(){if(!ready.value||props.diff||props.readOnly||mountedPath!==props.path)return;editor?.focus();void editor?.getAction('editor.action.rename')?.run();}
defineExpose({renameSymbol});
async function mountEditor(){
  if(!host.value || !monaco || !initialized || disposed)return;
  const version=++generation;++lspGeneration;definitionTargets.clear();clearTimeout(diagnosticTimer);ready.value=false;emit('ready',false);
  if(editor&&mountedPath)viewStates.set(mountedPath,(editor.getModifiedEditor?.()||editor).saveViewState());
  const reuseNormalEditor = !!editor && !editorModeDiff && !props.diff;
  if(!reuseNormalEditor)detachEditor();
  if(originalModel){const previous=originalModel;Promise.allSettled([...pendingDiffs]).then(()=>previous.dispose());originalModel=null;}
  model=null;pruneClosedModels();
  if(['typescript','javascript'].includes(languageForPath(props.path)))await ensureTypeScriptServices();
  if(disposed||version!==generation)return;
  model=models.get(props.path);
  if(!model&&retiringModels.has(props.path)){model=retiringModels.get(props.path);retiringModels.delete(props.path);models.set(props.path,model);}
  if(!model){model=monaco.editor.createModel(props.modelValue,languageForPath(props.path),monaco.Uri.from({scheme:'browseros-coding',authority:namespace,path:'/'+props.path}));models.set(props.path,model);}
  else if(model.getValue()!==props.modelValue)model.setValue(props.modelValue);
  mountedPath=props.path;
  const options={theme:'vs-dark',automaticLayout:true,editContext:false,fontSize:14,fontFamily:'"Cascadia Code", "Fira Code", monospace',minimap:{enabled:false},scrollBeyondLastLine:false,readOnly:props.readOnly,tabSize:2,wordWrap:'off',padding:{top:12},bracketPairColorization:{enabled:true}};
  if(props.diff){originalModel=monaco.editor.createModel(props.original??'',languageForPath(props.path));editor=monaco.editor.createDiffEditor(host.value,options);editor.setModel({original:originalModel,modified:model});}
  else if(reuseNormalEditor){editor.setModel(model);editor.updateOptions({readOnly:props.readOnly});}
  else {editor=monaco.editor.create(host.value,{...options,model});}
  editorModeDiff=!!props.diff;
  const observedModel=model,observedPath=mountedPath,observedScope=props.scope;
  if(!modelSubscriptions.has(observedModel))modelSubscriptions.set(observedModel,observedModel.onDidChangeContent(()=>{if(!applying){const content=observedModel.getValue();emit('edit',{path:observedPath,content,scope:observedScope});if(observedModel===model)emit('update:modelValue',content);}if(observedModel===model)scheduleDiagnostics();}));
  const target=props.diff?editor.getModifiedEditor():editor;
  if(viewStates.has(props.path))target.restoreViewState(viewStates.get(props.path));
  applyLocation();
  if(!reuseNormalEditor){
    target.addCommand(monaco.KeyMod.CtrlCmd|monaco.KeyCode.KeyS,()=>emit('save'));
    target.addCommand(monaco.KeyMod.Alt|monaco.KeyMod.Shift|monaco.KeyCode.KeyF,()=>{void target.getAction('editor.action.formatDocument')?.run();});
  }
  if(props.diff){const result=editor.waitForDiff();pendingDiffs.add(result);try{await result;}finally{pendingDiffs.delete(result);}}
  try{await syncOpenScriptModels([...models.values()]);}catch{if(!disposed&&version===generation)emit('diagnostics',['内置语言服务初始化失败，请重新打开文件。']);}
  if(!disposed&&version===generation){ready.value=true;emit('ready',true);scheduleDiagnostics();}
}
onMounted(async()=>{
  try{
    await ensureEditorWorkers();
    monaco=await import('monaco-editor/editor/editor.api.js');
    // TypeScript's worker manager loads editor contributions lazily. Their
    // singleton services must exist before the first standalone editor starts.
    await Promise.all([import('monaco-editor/editor/contrib/codelens/browser/codeLensCache.js'),import('monaco-editor/editor/common/services/treeViewsDndService.js'),import('monaco-editor/editor/contrib/documentSymbols/browser/outlineModel.js')]);
    await Promise.all([import('monaco-editor/features/gotoLine/register.js'),import('monaco-editor/features/quickCommand/register.js')]);
    await Promise.all([import('monaco-editor/editor/contrib/find/browser/findController.js'),import('monaco-editor/editor/contrib/folding/browser/folding.js'),import('monaco-editor/editor/contrib/bracketMatching/browser/bracketMatching.js'),import('monaco-editor/editor/contrib/cursorUndo/browser/cursorUndo.js'),import('monaco-editor/editor/contrib/multicursor/browser/multicursor.js')]);
    await Promise.all([import('monaco-editor/editor/contrib/suggest/browser/suggestController.js'),import('monaco-editor/editor/contrib/hover/browser/hoverContribution.js'),import('monaco-editor/editor/contrib/gotoSymbol/browser/goToCommands.js'),import('monaco-editor/editor/contrib/rename/browser/rename.js'),import('monaco-editor/editor/contrib/format/browser/formatActions.js')]);
    await Promise.all([import('monaco-editor/languages/definitions/typescript/register.js'),import('monaco-editor/languages/definitions/javascript/register.js'),import('monaco-editor/languages/definitions/rust/register.js'),import('monaco-editor/languages/definitions/python/register.js'),import('monaco-editor/languages/definitions/sql/register.js'),import('monaco-editor/languages/definitions/markdown/register.js'),import('monaco-editor/languages/definitions/html/register.js'),import('monaco-editor/languages/definitions/css/register.js'),import('monaco-editor/languages/definitions/java/register.js'),import('monaco-editor/languages/definitions/shell/register.js')]);
    markerSubscription=monaco.editor.onDidChangeMarkers(()=>{if(model)emit('diagnostics',monaco.editor.getModelMarkers({resource:model.uri}).map((m:any)=>`${m.startLineNumber}:${m.startColumn} ${m.message}`));});
    languageProviders.push(monaco.languages.registerCompletionItemProvider('rust',{triggerCharacters:['.',':'],provideCompletionItems:async(target:any,position:any,_context:any,token:any)=>{
      try{const result=await languageRequest('completion',target,position,token);const word=target.getWordUntilPosition(position);return {suggestions:completionItems(result,{startLineNumber:position.lineNumber,endLineNumber:position.lineNumber,startColumn:word.startColumn,endColumn:word.endColumn})};}catch{return {suggestions:[]};}
    }}));
    languageProviders.push(monaco.languages.registerDefinitionProvider('rust',{provideDefinition:async(target:any,position:any,token:any)=>{
      try{const response=await languageRequest('definition',target,position,token,true);if(!response)return [];
        const mapped=mappedDefinitionTargets(response,props.path);if(mapped.length)return mapped.map(item=>{const uri=item.path===props.path?target.uri:monaco.Uri.from({scheme:'browseros-coding',authority:namespace,path:'/'+item.path});definitionTargets.set(uri.toString(),item);return {uri,range:item.range};});
        return definitionRanges(response.result).map(range=>({uri:target.uri,range}));}catch{return [];}
    }}));
    languageProviders.push(monaco.editor.registerEditorOpener({openCodeEditor:async(source:any,resource:any,selection:any)=>{
      if(source!==(editor?.getModifiedEditor?.()||editor))return false;
      const item=definitionTargets.get(resource.toString());
      if(item&&item.path!==props.path&&props.lspRequest&&props.navigateDefinition)return props.navigateDefinition(item);
      if(resource.scheme!=='browseros-coding'||resource.authority!==namespace||!props.navigateOpenBuffer)return false;
      const entry=[...models].find(([,cached])=>cached.uri.toString()===resource.toString());
      if(!entry||entry[0]===props.path)return false;
      return props.navigateOpenBuffer({path:entry[0],line:selection?.startLineNumber||selection?.lineNumber||1,column:selection?.startColumn||selection?.column||1});
    }}));
    languageProviders.push(monaco.languages.registerHoverProvider('rust',{provideHover:async(target:any,position:any,token:any)=>{
      try{const result=await languageRequest('hover',target,position,token);if(!result)return null;const contents=Array.isArray(result.contents)?result.contents:[result.contents];return {range:editorRange(result.range)||undefined,contents:contents.flatMap((item:any)=>{const text=typeof item==='string'?item:item?.value;return typeof text==='string'?[{value:plainHover(text),isTrusted:false,supportHtml:false}]:[];})};}catch{return null;}
    }}));
    initialized=true;await mountEditor();
  }catch(e){error.value=`编辑器加载失败：${String(e)}`;}
});
watch(()=>[props.path,props.diff],()=>mountEditor());
watch(()=>props.modelValue,value=>{if(model&&mountedPath===props.path&&model.getValue()!==value){applying=true;model.setValue(value);applying=false;}},{flush:'post'});
watch(()=>props.original,value=>{if(originalModel&&originalModel.getValue()!==(value??''))originalModel.setValue(value??'');});
watch(()=>props.readOnly,value=>editor?.updateOptions({readOnly:value}));
watch(()=>props.location,()=>applyLocation());
watch(()=>props.openPaths,()=>pruneClosedModels());
watch(()=>props.lspRequest,()=>{++lspGeneration;definitionTargets.clear();(editor?.getModifiedEditor?.()||editor)?.getContribution('editor.contrib.contentHover')?.hideContentHover();for(const cached of models.values())monaco?.editor.setModelMarkers(cached,'coding-lsp',[]);scheduleDiagnostics();});
onBeforeUnmount(()=>{disposed=true;++generation;++lspGeneration;clearTimeout(diagnosticTimer);for(const provider of languageProviders)provider.dispose();for(const subscription of modelSubscriptions.values())subscription.dispose();modelSubscriptions.clear();markerSubscription?.dispose();detachEditor();const cachedModels=[...models.values(),...retiringModels.values()],original=originalModel;models.clear();retiringModels.clear();Promise.allSettled([...pendingDiffs]).then(()=>{for(const cached of cachedModels)cached.dispose();original?.dispose();releaseServices();});});
</script>
<template><div class="coding-code" :data-editor-ready="ready"><div v-if="error" role="alert">{{error}}</div><div ref="host" class="coding-monaco" aria-label="代码编辑器"/></div></template>
<style scoped>.coding-code,.coding-monaco{height:100%;width:100%;min-height:0;overflow:hidden}.coding-code{position:relative}.coding-code>[role=alert]{padding:16px;color:#ffb4a9}</style>
