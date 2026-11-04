// Only browser workers and already-open Monaco models. No filesystem, tsconfig,
// package scripts, plugins, automatic type downloads or external language process.
let users=0,basePromise,typescriptPromise,EditorWorker,TypeScriptWorker,services;
const workers=new Set();
function track(WorkerType){
  // Monaco can ask for a worker after its final editor is removed. Do not
  // leave a live worker behind or throw from its asynchronous callback.
  if(!users){const stale=new WorkerType();stale.terminate();return stale;}
  const worker=new WorkerType(),terminate=worker.terminate.bind(worker);
  worker.terminate=()=>{workers.delete(worker);terminate();};workers.add(worker);return worker;
}
export function retainEditorServices(){
  // The previous Monaco worker-manager may have cached a terminated worker.
  // Reset its compiler configuration before reopening an editor.
  if(++users===1&&services)for(const defaults of [services.typescriptDefaults,services.javascriptDefaults])defaults.setCompilerOptions({...defaults.getCompilerOptions()});
  let active=true;
  return ()=>{if(!active)return;active=false;if(--users)return;
    // Do not mutate TypeScript defaults during teardown: setCompilerOptions
    // schedules asynchronous Monaco worker creation after the last owner exits.
    for(const worker of [...workers])worker.terminate();
  };
}
export async function ensureEditorWorkers(){
  if(!basePromise)basePromise=import('monaco-editor/editor/editor.worker.js?worker').then(module=>{EditorWorker=module.default;}).catch(error=>{basePromise=undefined;throw error;});
  await basePromise;
  if(!users)return; // Owner unmounted during the worker module import.
  globalThis.MonacoEnvironment={...globalThis.MonacoEnvironment,getWorker:(_module,label)=>{
    if(label==='typescript'||label==='javascript'){
      if(!TypeScriptWorker)throw Error('TYPESCRIPT_WORKER_NOT_READY');
      return track(TypeScriptWorker);
    }
    return track(EditorWorker);
  }};
}
export async function ensureTypeScriptServices(){
  await ensureEditorWorkers();
  if(!typescriptPromise)typescriptPromise=(async()=>{
    TypeScriptWorker=(await import('monaco-editor/languages/features/typescript/ts.worker.js?worker')).default;
    services=await import('monaco-editor/languages/features/typescript/register.js');
    if(!users)return services; // Do not configure a service after its owner closes.
    const compiler={allowNonTsExtensions:true,allowJs:true,noEmit:true,strict:true,target:services.ScriptTarget.ES2020,module:services.ModuleKind.ESNext,moduleResolution:services.ModuleResolutionKind.NodeJs,allowImportingTsExtensions:true,jsx:services.JsxEmit.Preserve,types:[],typeRoots:[]};
    services.typescriptDefaults.setCompilerOptions(compiler);
    services.javascriptDefaults.setCompilerOptions({...compiler,strict:false,checkJs:true,noImplicitAny:false});
    for(const defaults of [services.typescriptDefaults,services.javascriptDefaults]){
      defaults.setDiagnosticsOptions({noSemanticValidation:false,noSyntaxValidation:false,onlyVisible:true});
      // Only active/open TS/JS models are synchronized by syncOpenScriptModels.
      // Avoid cloning every Monaco model into each worker on startup.
      defaults.setEagerModelSync(false);
    }
    return services;
  })().catch(error=>{typescriptPromise=undefined;throw error;});
  return typescriptPromise;
}
export async function syncOpenScriptModels(models){
  const scripts=models.filter(model=>!model.isDisposed()&&['typescript','javascript'].includes(model.getLanguageId()));
  if(!scripts.length)return;
  const service=await ensureTypeScriptServices(),uris=scripts.map(model=>model.uri);
  if(!users||scripts.every(model=>model.isDisposed()))return;
  // Monaco registers its mode asynchronously after the first model is created.
  // Waiting here prevents switching files from racing that registration.
  async function getRegisteredWorker(getter){
    for(let attempt=0;attempt<100;attempt++){
      if(!users||scripts.every(model=>model.isDisposed()))return null;
      try{return await getter();}catch(error){
        if(!String(error).includes('not registered'))throw error;
        await new Promise(resolve=>setTimeout(resolve,30));
      }
    }
    throw Error('TypeScript language service did not initialize');
  }
  if(scripts.some(model=>model.getLanguageId()==='typescript')){const worker=await getRegisteredWorker(service.getTypeScriptWorker);if(worker)await worker(...uris);}
  if(scripts.some(model=>model.getLanguageId()==='javascript')){const worker=await getRegisteredWorker(service.getJavaScriptWorker);if(worker)await worker(...uris);}
}
