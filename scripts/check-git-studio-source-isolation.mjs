#!/usr/bin/env node
// Executes the actual Vue SFC setup and watchers with an in-memory renderer.
// Only native IO and adjacent capability owners are fixtures; this is not GUI E2E.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve, dirname } from 'node:path';
import { build } from 'esbuild';
import { parse, compileScript } from '@vue/compiler-sfc';
import { createRenderer, reactive, nextTick } from 'vue';

const ROOT = fileURLToPath(new URL('../', import.meta.url));
const SOURCE = resolve(ROOT, 'src/capabilities/git/ui/GitStudio.vue');
const NAMES = ['info','pane','branchTarget','stashTarget','worktreeTarget','revision',
  'logSelection','message','pr','prText','resolution','notice','commits','stashes',
  'worktrees','blame','selectedLines','selected','activeFile','patch','file',
  'conflict','remote','prDetail','offset','error','preview','acknowledged',
  'writeBusy','refresh','prepare','confirm','selectRepo'];
const flush = async () => { for (let i=0;i<8;i++) { await Promise.resolve(); await nextTick(); } };
const status = id => ({root:'/fixture/'+id,branch:'main-'+id,state:'Clean',files:[],branches:[['main-'+id,'HEAD',''] ]});

async function mountSource(text,props={}) {
  const {descriptor,errors}=parse(text,{filename:SOURCE});
  assert.deepEqual(errors,[], 'production component must parse');
  // Expose bindings to the harness; no production expression is replaced.
  const instrumented=text.replace('</script>', '\ndefineExpose({'+NAMES.join(',')+'});\n</script>');
  const exposed=parse(instrumented,{filename:SOURCE}).descriptor;
  const compiled=compileScript(exposed,{id:'git-source-isolation'});
  const fixture={active:true,binding:null,cancelled:[],previews:[],confirmations:[],readOverride:null};
  fixture.git=reactive({repoId:'A',async selectRepo(id){this.repoId=id;}});
  fixture.repos={repos:[]};
  fixture.bridge={
    async gitStudioRead(source,input){
      if(fixture.readOverride){const result=fixture.readOverride(source,input);if(result)return result;}
      if(input.action==='status')return status(source.repo_id || source.path);
      return {commits:[],has_more:false,text:'',patch:'',prs:[],runs:[]};
    },
    async gitStudioPreview(source,input){const job={job_id:'ticket-'+(fixture.previews.length+1),source,input};fixture.previews.push(job);return job;},
    async gitStudioCancel(id){fixture.cancelled.push(id);return true;},
    async gitStudioConfirm(id,ack){fixture.confirmations.push({id,ack});return {success:true};}
  };
  const key='__browserosGitSourceFixture_'+process.pid+'_'+Math.random().toString(36).slice(2);
  globalThis[key]=fixture;
  const access='globalThis['+JSON.stringify(key)+']';
  const virtual={
    bridge:`export const bridge=${access}.bridge;`,
    git:`export const useGitStore=()=>${access}.git;`,
    repos:`export const useRepoStore=()=>${access}.repos;`,
    lifecycle:`export const gitLifecycleSnapshot=()=>({active:${access}.active});export const registerGitLifecycleBinding=b=>{${access}.binding=b;return ()=>{${access}.unregistered=true;}};`
  };
  const output=resolve(ROOT,'scripts/.git-source-isolation-'+process.pid+'-'+Math.random().toString(36).slice(2)+'.mjs');
  let app;
  try {
    const bundled=await build({stdin:{contents:compiled.content,loader:'ts',resolveDir:dirname(SOURCE),sourcefile:SOURCE},bundle:true,write:false,format:'esm',platform:'node',external:['vue'],plugins:[{name:'native-io-fixtures',setup(builder){
      builder.onResolve({filter:/^(\.\.\/\.\.\/\.\.\/bridge|\.\.\/public|\.\.\/\.\.\/workspace\/public|\.\.\/lifecycle)$/},args=>({namespace:'git-fixture',path:args.path.endsWith('/bridge')?'bridge':args.path==='../public'?'git':args.path.includes('workspace')?'repos':'lifecycle'}));
      builder.onLoad({filter:/.*/,namespace:'git-fixture'},args=>({contents:virtual[args.path],loader:'js'}));
    }}]});
    writeFileSync(output,bundled.outputFiles[0].text);
    const component=(await import(pathToFileURL(output).href)).default;
    // Avoid claiming browser rendering; all production setup/watch logic is retained.
    component.render=()=>null;
    const renderer=createRenderer({
      createElement:type=>({type,children:[]}),createText:text=>({text}),createComment:text=>({comment:text}),
      insert(child,parent){child.parent=parent;parent.children.push(child);},
      remove(child){if(child.parent)child.parent.children=child.parent.children.filter(x=>x!==child);},
      setText(node,text){node.text=text;},setElementText(node,text){node.text=text;},
      parentNode:node=>node.parent||null,nextSibling:()=>null,patchProp(){},
    });
    app=renderer.createApp(component,props);
    const vm=app.mount({children:[]});
    await flush();
    return {vm,fixture,template:descriptor.template.content,dispose:async()=>{app.unmount();await flush();delete globalThis[key];}};
  } catch(error) {if(app)app.unmount();delete globalThis[key];throw error;}
  finally {try{unlinkSync(output);}catch(error){if(error.code!=='ENOENT')throw error;}}
}

export async function checkGitStudioSourceIsolation({sourceText=readFileSync(SOURCE,'utf8')}={}) {
  let assertions=0;
  const eq=(actual,expected,label)=>{assert.deepEqual(actual,expected,label);assertions++;};
  const mounted=await mountSource(sourceText);
  const {vm,fixture,template}=mounted;
  try {
    eq(vm.info.root,'/fixture/A','initial status is associated with A');
    eq(template.includes('v-for="group in groups"'),true,'actual template displays real change groups');
    eq(template.includes('selected.length'),true,'actual template displays explicit selected-file scope');
    const blank=['branchTarget','stashTarget','worktreeTarget','revision','logSelection',
      'message','prText','resolution','notice'];
    for(const name of blank)vm[name]='A-bound-'+name;
    vm.pr=17;vm.commits=[{oid:'a'.repeat(40)}];vm.stashes=[['stash@{0}']];vm.worktrees='/fixture/A';
    vm.blame=[{line:1}];vm.selectedLines=[7];vm.selected=['A.txt'];vm.offset=50;
    vm.pane='stash';
    fixture.git.repoId='B';
    // This assertion intentionally precedes nextTick: no same-event stale action.
    eq(vm.branchTarget,'','branch target invalidated synchronously on source change');
    await flush();
    for(const name of blank)eq(vm[name],'',name+' must not leak into another repository');
    eq(vm.pr,null,'PR number must not leak');
    eq(vm.pane,'changes','new source returns to its default view rather than showing stale unloaded lists');
    for(const name of ['commits','stashes','blame','selectedLines','selected'])eq(vm[name],[],name+' results must not leak');
    eq(vm.worktrees,'','worktree output must not leak');eq(vm.offset,0,'history paging reset');
    eq(vm.info.root,'/fixture/B','new repository status loaded');
    vm.branchTarget='new-branch-B';vm.message='draft B';
    await vm.refresh();
    eq(vm.branchTarget,'new-branch-B','same-source refresh preserves explicit target');
    eq(vm.message,'draft B','same-source refresh preserves draft');
    await vm.prepare('branch_create');
    eq(fixture.previews.at(-1).source,{repo_id:'B'},'preview uses current repository');
    eq(fixture.previews.at(-1).input.target,'new-branch-B','preview uses current explicit target');
    await vm.confirm();eq(fixture.confirmations.length,0,'unacknowledged write never confirms');
    const ticket=vm.preview.job_id;
    fixture.git.repoId='C';await flush();
    eq(vm.preview,null,'cross-source preview discarded');
    eq(fixture.cancelled.includes(ticket),true,'old preview cancelled in native owner');
    vm.acknowledged=true;await vm.confirm();
    eq(fixture.confirmations.length,0,'old ticket never confirms after source change');
    vm.writeBusy=true;vm.selectRepo('D');
    eq(fixture.git.repoId,'C','UI blocks source selection during a confirmed write');
    vm.writeBusy=false;
    let finishOld;
    fixture.readOverride=(source,input)=>source.repo_id==='C'&&input.action==='status'?new Promise(resolve=>{finishOld=resolve;}):null;
    const oldRefresh=vm.refresh();
    eq(typeof finishOld,'function','delayed old-source read created');
    fixture.git.repoId='D';await flush();
    eq(vm.info.root,'/fixture/D','new-source read completed first');
    finishOld(status('C'));await oldRefresh;await flush();
    eq(vm.info.root,'/fixture/D','late response cannot restore old repository');
    fixture.readOverride=null;
    vm.branchTarget='target-D';vm.pr=42;fixture.active=false;fixture.binding.cleanup();await flush();
    eq(vm.branchTarget,'','lifecycle cleanup uses same target invalidation');
    eq(vm.pr,null,'lifecycle cleanup clears PR target');eq(vm.info,null,'cleanup removes old status');
  } finally {await mounted.dispose();}
  const history=await mountSource(sourceText,{mode:'history'});
  try {
    eq(history.vm.pane,'log','history entry starts in history mode');
    history.vm.pane='stash';history.fixture.git.repoId='B';await flush();
    eq(history.vm.pane,'log','source reset preserves the history entry contract');
  } finally {await history.dispose();}
  return {assertions,scope:'real Vue SFC setup and watchers; native IO fixtures; no browser/GUI or real Git writes'};
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const index=process.argv.indexOf('--source');
  const options=index<0?{}:{sourceText:readFileSync(resolve(process.argv[index+1]),'utf8')};
  const result=await checkGitStudioSourceIsolation(options);
  console.log('GIT_STUDIO_SOURCE_ISOLATION=PASS '+JSON.stringify(result));
}
