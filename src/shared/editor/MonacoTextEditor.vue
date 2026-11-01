<script setup lang="ts">
import {onBeforeUnmount,onMounted,ref,watch} from 'vue';
import {loadMonacoEditor} from './loadMonaco.mjs';
import {createDocumentModel,editorLanguage} from './documentModel.mjs';
import {retainEditorServices,ensureTypeScriptServices,syncOpenScriptModels} from './monacoWorkerServices.mjs';
const props=withDefaults(defineProps<{path:string;modelValue:string;readOnly?:boolean}>(),{readOnly:false});
const emit=defineEmits<{ 'update:modelValue':[value:string];save:[] }>();
const host=ref<HTMLElement|null>(null),error=ref(''),ready=ref(false);
const namespace=crypto.randomUUID(),release=retainEditorServices();
let editor:any,model:any,subscription:any,monaco:any;
let disposed=false,revision=0,applying=false;
async function mount(){
  const token=++revision;
  if(!host.value||disposed)return;
  ready.value=false;error.value='';
  try {
    const api=await loadMonacoEditor();
    if(disposed||token!==revision)return;
    monaco=api;
    const language=editorLanguage(props.path);
    if(language==='typescript'||language==='javascript')await ensureTypeScriptServices();
    if(disposed||token!==revision)return;
    subscription?.dispose();subscription=null;
    if(editor){editor.setModel(null);editor.dispose();editor=null;}
    model?.dispose();model=null;
    model=createDocumentModel(monaco,{path:props.path,content:props.modelValue,language,namespace});
    subscription=model.onDidChangeContent(()=>{if(!applying)emit('update:modelValue',model.getValue());});
    editor=monaco.editor.create(host.value,{model,theme:'vs-dark',automaticLayout:true,
      readOnly:props.readOnly,editContext:false,fontSize:14,minimap:{enabled:false},
      wordWrap:'off',scrollBeyondLastLine:false,bracketPairColorization:{enabled:true}});
    editor.addCommand(monaco.KeyMod.CtrlCmd|monaco.KeyCode.KeyS,()=>emit('save'));
    if(language==='typescript'||language==='javascript')await syncOpenScriptModels([model]);
    if(!disposed&&token===revision)ready.value=true;
  } catch(e){if(!disposed&&token===revision)error.value='编辑器初始化失败：'+String(e);}
}
onMounted(()=>{void mount();});
watch(()=>props.path,()=>{void mount();});
watch(()=>props.modelValue,value=>{if(model&&model.getValue()!==value){applying=true;try{model.setValue(value);}finally{applying=false;}}});
watch(()=>props.readOnly,value=>editor?.updateOptions({readOnly:value}));
onBeforeUnmount(()=>{disposed=true;++revision;subscription?.dispose();if(editor){editor.setModel(null);editor.dispose();}model?.dispose();release();});
</script>
<template><div class="shared-monaco-text" :data-editor-ready="ready"><div v-if="error" class="editor-error" role="alert">{{error}}</div><div ref="host" class="monaco-host" aria-label="代码编辑器"/></div></template>
<style scoped>.shared-monaco-text,.monaco-host{width:100%;height:100%;min-height:0;min-width:0;overflow:hidden}.shared-monaco-text{position:relative}.editor-error{position:absolute;top:0;left:0;z-index:1;padding:8px;background:#202637;color:#ffb4a9}</style>
