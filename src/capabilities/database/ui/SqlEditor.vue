<script setup lang="ts">
import { computed, ref, watch, nextTick } from 'vue';
import { lexSql, sqlCompletions, selectedSql, formatSql } from '../model/studioSql.mjs';
const props=withDefaults(defineProps<{modelValue:string;disabled?:boolean;objects?:string[];columns?:string[];sourceLabel?:string}>(),{disabled:false,objects:()=>[],columns:()=>[],sourceLabel:''});
const emit=defineEmits<{'update:modelValue':[value:string];run:[value:{text:string;batch:boolean}]}>();
const input=ref<HTMLTextAreaElement|null>(null), backdrop=ref<HTMLElement|null>(null);
const suggestions=ref<any[]>([]), chosen=ref(0), localError=ref(''), composing=ref(false);
const findText=ref(''), replacement=ref(''), previousFormat=ref<string|null>(null);
const parsed=computed(()=>{try{return lexSql(props.modelValue);}catch{return {tokens:[{kind:'plain',text:props.modelValue}],issues:[{offset:0,code:'SQL_TEXT_TOO_LARGE'}]};}});
const highlighted=computed(()=>{
  if(parsed.value.tokens.length>4000)return [{kind:'plain',text:props.modelValue}];
  const output:{kind:string;text:string}[]=[];
  for(const token of parsed.value.tokens){const kind=['keyword','string','number','comment'].includes(token.kind)?token.kind:'plain';const last=output.at(-1);if(last?.kind===kind)last.text+=token.text;else output.push({kind,text:token.text});}
  return output;
});
const diagnosticNames:Record<string,string>={UNCLOSED_COMMENT:'注释未结束',UNCLOSED_STRING:'字符串或标识符未闭合',UNMATCHED_CLOSE:'多余的右括号',UNCLOSED_PAREN:'左括号未闭合',SQL_TEXT_TOO_LARGE:'SQL 超过 64 KiB 上限'};
function syncScroll(){if(input.value&&backdrop.value){backdrop.value.scrollTop=input.value.scrollTop;backdrop.value.scrollLeft=input.value.scrollLeft;}}
function change(event:Event){const element=event.target as HTMLTextAreaElement;if(new TextEncoder().encode(element.value).length>65536){localError.value='SQL 超过 64 KiB，请缩小脚本。';element.value=props.modelValue;return;}localError.value='';emit('update:modelValue',element.value);suggestions.value=[];}
function complete(){if(props.disabled||composing.value||!input.value)return;try{suggestions.value=sqlCompletions(props.modelValue,input.value.selectionStart,props.objects,props.columns);chosen.value=0;}catch(e){localError.value=String(e);}}
async function insert(text:string,start:number,end:number){if(props.disabled)return;const updated=props.modelValue.slice(0,start)+text+props.modelValue.slice(end);if(new TextEncoder().encode(updated).length>65536){localError.value='SQL 超过 64 KiB。';return;}emit('update:modelValue',updated);await nextTick();input.value?.focus();input.value?.setSelectionRange(start+text.length,start+text.length);suggestions.value=[];}
function choose(index=chosen.value){const item=suggestions.value[index];if(item)void insert(item.label,item.from,item.to);}
function run(batch=false){if(props.disabled||composing.value)return;try{const element=input.value;const text=batch?props.modelValue:selectedSql(props.modelValue,element?.selectionStart??0,element?.selectionEnd??0);if(!text.trim()){localError.value='当前没有可执行语句。';return;}emit('run',{text,batch});}catch(e){localError.value=String(e);}}
function format(){if(props.disabled)return;try{const next=formatSql(props.modelValue);previousFormat.value=props.modelValue;emit('update:modelValue',next);}catch(e){localError.value=String(e);}}
function undoFormat(){if(previousFormat.value!==null){emit('update:modelValue',previousFormat.value);previousFormat.value=null;}}
function jump(offset:number){input.value?.focus();input.value?.setSelectionRange(offset,Math.min(offset+1,props.modelValue.length));}
function find(){const element=input.value;if(!element||!findText.value)return;const from=element.selectionEnd;let at=props.modelValue.indexOf(findText.value,from);if(at<0)at=props.modelValue.indexOf(findText.value);if(at<0){localError.value='没有找到匹配文本。';return;}element.focus();element.setSelectionRange(at,at+findText.value.length);}
function replace(){const element=input.value;if(!element||!findText.value||props.disabled)return;const {selectionStart:start,selectionEnd:end}=element;if(props.modelValue.slice(start,end)===findText.value)void insert(replacement.value,start,end);else find();}
function key(event:KeyboardEvent){
  if(composing.value||event.isComposing)return;
  if((event.ctrlKey||event.metaKey)&&event.key==='Enter'){event.preventDefault();run(event.shiftKey);return;}
  if((event.ctrlKey||event.metaKey)&&event.code==='Space'){event.preventDefault();complete();return;}
  if(suggestions.value.length){
    if(event.key==='Escape'){event.preventDefault();suggestions.value=[];return;}
    if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();chosen.value=(chosen.value+(event.key==='ArrowDown'?1:-1)+suggestions.value.length)%suggestions.value.length;return;}
    if(event.key==='Enter'||event.key==='Tab'){event.preventDefault();choose();return;}
  }
  if(event.key==='Tab'&&!props.disabled){event.preventDefault();const element=input.value;if(element)void insert('  ',element.selectionStart,element.selectionEnd);}
}
watch(()=>props.sourceLabel,()=>{suggestions.value=[];previousFormat.value=null;});
watch(()=>props.modelValue,()=>void nextTick(syncScroll));
defineExpose({focus:()=>input.value?.focus(),get selectionStart(){return input.value?.selectionStart??0;},get selectionEnd(){return input.value?.selectionEnd??0;},setSelectionRange:(a:number,b:number)=>input.value?.setSelectionRange(a,b),run});
</script>
<template>
<section class="sql-editor" aria-label="SQL 编辑器">
  <header class="editor-tools"><strong>{{sourceLabel||'尚未绑定数据源'}}</strong><button :disabled="disabled" @click="run(false)">运行当前 / 选中语句</button><button :disabled="disabled" @click="run(true)">运行脚本</button><button :disabled="disabled" @click="complete">补全</button><button :disabled="disabled" @click="format">格式化</button><button :disabled="disabled||previousFormat===null" @click="undoFormat">撤销格式化</button></header>
  <div class="editor-surface"><pre ref="backdrop" class="highlight" aria-hidden="true"><span v-for="(token,index) in highlighted" :key="index" :class="token.kind">{{token.text}}</span>{{'\n'}}</pre><textarea ref="input" :value="modelValue" :disabled="disabled" spellcheck="false" autocapitalize="off" autocomplete="off" aria-label="SQL 文本" aria-describedby="sql-editor-help" wrap="off" @input="change" @keydown="key" @scroll="syncScroll" @compositionstart="composing=true" @compositionend="composing=false"/></div>
  <div v-if="suggestions.length" class="completions" role="listbox" aria-label="SQL 补全候选"><button v-for="(item,index) in suggestions" :key="item.label" role="option" :aria-selected="chosen===index" @mousedown.prevent="choose(index)">{{item.label}}</button></div>
  <div class="find"><input v-model="findText" type="search" placeholder="查找文本" aria-label="查找 SQL 文本" @keydown.enter.prevent="find"/><button @click="find">下一个</button><input v-model="replacement" placeholder="替换为" aria-label="替换文本"/><button :disabled="disabled" @click="replace">替换当前匹配</button></div>
  <p id="sql-editor-help" class="hint">Ctrl / ⌘ + Enter：当前或选中语句；加 Shift：脚本；Ctrl + Space：元数据补全。诊断为词法检查，数据库仍会再次校验。</p>
  <p v-if="localError" class="error" role="alert">{{localError}}</p><div v-if="parsed.issues.length" class="diagnostics"><button v-for="(issue,index) in parsed.issues" :key="index" @click="jump(issue.offset)">{{diagnosticNames[issue.code]||issue.code}} · 行 {{modelValue.slice(0,issue.offset).split('\n').length}}</button></div>
</section>
</template>
<style scoped>
.sql-editor{display:flex;flex-direction:column;min-width:0;border:1px solid #cbd7e4;border-radius:6px;background:#fff}.editor-tools,.find{display:flex;align-items:center;gap:6px;flex-wrap:wrap;padding:7px}.editor-tools strong{margin-right:auto;overflow-wrap:anywhere}.editor-surface{position:relative;min-height:180px;height:250px;resize:vertical;overflow:hidden;border-top:1px solid #e1e8f0;border-bottom:1px solid #e1e8f0}.highlight,.editor-surface textarea{position:absolute;inset:0;box-sizing:border-box;margin:0;border:0;padding:10px 12px;font:13px/1.7 ui-monospace,SFMono-Regular,Consolas,monospace;letter-spacing:normal;tab-size:2;white-space:pre;overflow:auto;width:100%;height:100%;border-radius:0}.highlight{pointer-events:none;color:#263a50;scrollbar-width:none}.highlight::-webkit-scrollbar{display:none}.editor-surface textarea{background:transparent;color:transparent;caret-color:#142c48;resize:none}.editor-surface textarea::selection{background:#aacbf080}.editor-surface textarea:disabled{caret-color:transparent}.keyword{color:#7748a4;font-weight:600}.string{color:#217044}.number{color:#176b9c}.comment{color:#7c8795;font-style:italic}.completions{display:flex;gap:4px;flex-wrap:wrap;max-height:120px;overflow:auto;padding:7px;background:#f5f8fc}.completions button[aria-selected=true]{background:#dfeafb}.find input{min-width:110px;flex:1}.hint{font-size:11px;color:#6b7b8b;margin:5px 8px}.error{color:#9b2929}.diagnostics{display:flex;gap:5px;flex-wrap:wrap;padding:6px}button,input{font:inherit;border:1px solid #c4d2e1;background:#fff;border-radius:4px;padding:4px 7px;color:inherit}button{cursor:pointer}button:disabled{opacity:.45;cursor:not-allowed}
</style>
