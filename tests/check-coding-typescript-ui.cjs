// Actual Monaco compiler worker; no language-service fixtures or host processes.
const assert=require('node:assert/strict');
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const base=process.env.CODING_UI_URL||'http://127.0.0.1:1433';
(async()=>{
 const source=await(await fetch(base+'/src/capabilities/agent/ui/CodingCodeEditor.vue')).text();
 const vue=source.match(/from ["']([^"']+\/vue\.js\?[^"']+)["']/)?.[1];assert.ok(vue);
 const browser=await chromium.launch({executablePath:'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});
 try{
  const page=await browser.newPage(),errors=[],externalRequests=[];page.on('pageerror',error=>errors.push(error.message));
  page.on('request',request=>{if(/^https?:/.test(request.url())&&!request.url().startsWith(base+'/'))externalRequests.push(request.url());});
  await page.route('**/__typescript_test',route=>route.fulfill({contentType:'text/html',body:`<style>html,body,#app{height:100%;margin:0}</style><div id="app"></div><script type="module">
   import {createApp,h,ref} from '${vue}';import Editor from '/src/capabilities/agent/ui/CodingCodeEditor.vue';
   const NativeWorker=window.Worker,workers=new Set();window.Worker=class extends NativeWorker{constructor(...args){super(...args);workers.add(this);}terminate(){workers.delete(this);return super.terminate();}};
   const path=ref('main.ts'),value=ref('import { absent } from "external-package";\\nconst count: number = "bad";\\nconst person = { name: "Ada", age: 30 };\\nperson.name;'),openPaths=ref(['main.ts']);
   const makeApp=()=>createApp({setup:()=>()=>h(Editor,{path:path.value,openPaths:openPaths.value,modelValue:value.value,navigateOpenBuffer:async target=>{window.fixture.navigation=target;return true;},'onUpdate:modelValue':v=>value.value=v})});const app=makeApp();app.mount('#app');window.fixture={app,makeApp,path,value,openPaths,workers};
  </script>`}));
  await page.goto(base+'/__typescript_test');await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  await page.evaluate(async()=>{window.fixtureApi=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));});
  const result=await page.evaluate(async()=>{
   const url=performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name));
   const api=await import(url),model=api.editor.getModels().find(item=>item.uri.scheme==='browseros-coding');
   const serviceUrl=performance.getEntriesByType('resource').map(item=>item.name).find(name=>name.includes('/codingEditorServices.mjs'));
   const services=await import(serviceUrl),service=await services.ensureTypeScriptServices();
   await services.syncOpenScriptModels([model]);
   const worker=await(await service.getTypeScriptWorker())(model.uri);
   return {diagnostics:await worker.getSemanticDiagnostics(model.uri.toString()),completion:await worker.getCompletionsAtPosition(model.uri.toString(),model.getValue().lastIndexOf('person.')+7),hover:await worker.getQuickInfoAtPosition(model.uri.toString(),model.getValue().lastIndexOf('person.name')+8)};
  });
  assert.ok(result.diagnostics.some(item=>item.code===2322),'compiler must reject a string assigned to number');
  assert.ok(result.diagnostics.some(item=>item.code===2307),'unavailable packages must be reported rather than downloaded');
  assert.ok(result.completion.entries.some(item=>item.name==='name'),'completion uses the actual object type');
  assert.ok(result.hover.displayParts.some(item=>item.text==='string'),'hover reports actual property type');
  const beforeFormat=await page.evaluate(async()=>{const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));const editor=api.editor.getEditors().find(item=>item.getModel()?.uri.path==='/main.ts');const text=editor.getModel().getValue();editor.getModel().setValue('const formatted={name:"Ada",age:30};');editor.focus();return text;});
  await page.keyboard.press('Alt+Shift+f');
  await page.waitForFunction(()=>{const api=window.fixtureApi;return api.editor.getModels().some(model=>model.uri.path==='/main.ts'&&model.getValue().includes('name: "Ada"'));},null,{timeout:5000});
  await page.evaluate(text=>{window.fixture.value.value=text;},beforeFormat);
  await page.waitForFunction(text=>window.fixtureApi.editor.getModels().some(model=>model.uri.path==='/main.ts'&&model.getValue()===text),beforeFormat);
  await page.evaluate(async()=>{const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));const editor=api.editor.getEditors().find(item=>item.getModel()?.uri.path==='/main.ts');editor.setPosition({lineNumber:3,column:8});editor.focus();});
  await page.keyboard.press('F2');
  await page.locator('.rename-box[widgetid]').waitFor({state:'visible',timeout:5000});
  await page.keyboard.press('Escape');
  await page.evaluate(async()=>{const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));const editor=api.editor.getEditors().find(item=>item.getModel()?.uri.path==='/main.ts');editor.setPosition({lineNumber:4,column:8});editor.focus();});
  await page.keyboard.press('Control+Space');
  await page.locator('.suggest-widget.visible').getByText('name',{exact:true}).first().waitFor();
  await page.keyboard.press('Escape');
  await page.evaluate(()=>{window.fixture.openPaths.value=['main.ts','util.ts'];window.fixture.value.value='export const meaning = 42;';window.fixture.path.value='util.ts';});
  await page.waitForFunction(()=>document.querySelector('.coding-code[data-editor-ready="true"]')&&window.fixture.path.value==='util.ts');
  await page.waitForFunction(()=>{const api=window.fixtureApi;return api.editor.getEditors().some(editor=>editor.getModel()?.uri.path==='/util.ts');});
  await page.evaluate(()=>{window.fixture.value.value='import { meaning } from "./util";\nmeaning;';window.fixture.path.value='main.ts';});
  await page.waitForFunction(()=>{const api=window.fixtureApi;return api.editor.getEditors().some(editor=>editor.getModel()?.uri.path==='/main.ts'&&editor.getModel().getValue().includes('import { meaning }'));});
  const definition=await page.evaluate(async()=>{
   const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));
   const models=api.editor.getModels().filter(item=>item.uri.scheme==='browseros-coding'),model=models.find(item=>item.uri.path==='/main.ts');
   const services=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>name.includes('/codingEditorServices.mjs')));await services.syncOpenScriptModels(models);
   const service=await services.ensureTypeScriptServices(),worker=await(await service.getTypeScriptWorker())(...models.map(item=>item.uri));
   return worker.getDefinitionAtPosition(model.uri.toString(),model.getValue().lastIndexOf('meaning')+2);
  });
  assert.ok(definition.some(item=>item.fileName.endsWith('/util.ts')),'definition resolves another opened file');
  await page.evaluate(async()=>{const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));const editor=api.editor.getEditors().find(item=>item.getModel()?.uri.path==='/main.ts');editor.setPosition({lineNumber:2,column:3});editor.focus();});
  await page.keyboard.press('F12');
  await page.waitForFunction(()=>window.fixture.navigation?.path==='util.ts');
  await page.evaluate(()=>{window.fixture.openPaths.value=['second.js'];window.fixture.value.value='/** @type {number} */ let value = "bad";';window.fixture.path.value='second.js';});
  await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  await page.waitForFunction(()=>{
   const api=window.fixtureApi;
   const models=api.editor.getModels().filter(item=>item.uri.scheme==='browseros-coding');
   return models.length===1&&models[0].uri.path==='/second.js'&&models[0].getLanguageId()==='javascript';
  });
  const jsErrors=await page.evaluate(async()=>{
   const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));
   const model=api.editor.getModels().find(item=>item.uri.path==='/second.js');
   const services=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>name.includes('/codingEditorServices.mjs')));
   await services.syncOpenScriptModels([model]);const service=await services.ensureTypeScriptServices();
   return (await(await service.getJavaScriptWorker())(model.uri)).getSemanticDiagnostics(model.uri.toString());
  });
  assert.ok(jsErrors.some(item=>item.code===2322),'JavaScript checkJs should report the JSDoc type mismatch');
  await page.evaluate(()=>window.fixture.app.unmount());await page.waitForTimeout(1000);
  const remaining=await page.evaluate(async()=>{const api=await import(performance.getEntriesByType('resource').map(item=>item.name).find(name=>/monaco-editor.*editor.*api/.test(name)));return api.editor.getModels().filter(item=>item.uri.scheme==='browseros-coding').length;});
  assert.equal(remaining,0,'closed editor models must be released');
  assert.equal(await page.evaluate(()=>window.fixture.workers.size),0,'worker processes must terminate after the last editor closes');
  await page.evaluate(()=>{window.fixture.app=window.fixture.makeApp();window.fixture.app.mount('#app');});
  await page.locator('.coding-code[data-editor-ready="true"]').waitFor();
  await page.waitForFunction(()=>window.fixture.workers.size>0);
  await page.evaluate(()=>window.fixture.app.unmount());await page.waitForTimeout(1000);
  assert.equal(await page.evaluate(()=>window.fixture.workers.size),0,'reopened editor workers must also terminate');
  assert.deepEqual(errors,[]);assert.deepEqual(externalRequests,[],'language services must not download project dependencies');console.log('TYPESCRIPT_WORKER=PASS TS/JS diagnostics, completion, hover, cross-file navigation callback, model cleanup; actual browser compiler, not installed-client acceptance');
 }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
