#!/usr/bin/env node
// Audit-only size thresholds; production bytes, source ownership, pinned deps and SHA256 verification still fail closed.
import { build } from 'vite';
import { relative, join } from 'node:path';
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { compactVueScopes } from './compact-vue-scope-ids.mjs';
import assert from 'node:assert/strict';
import { evaluateEditorBudget, isExclusiveEditorAsset } from './coding-editor-budget.mjs';
const forceSplit=process.argv.includes('--split-css');
const productionCssSplit=/\bcssCodeSplit\s*:\s*true\b/.test(readFileSync('vite.config.ts','utf8'));
const split=forceSplit||productionCssSplit;
const verifyDist=process.argv.includes('--verify-dist');
const auditBudget=process.argv.includes('--audit-budget');
assert.ok(!auditBudget||verifyDist,'Budget audit requires production verification');
assert.ok(process.argv.slice(2).every(arg=>['--split-css','--verify-dist','--audit-budget'].includes(arg)),'Unknown audit option');
const isEditor=id=>/\/node_modules\/(monaco-editor|dompurify|marked)\//.test(id.replaceAll('\\','/'));
const hash=value=>createHash('sha256').update(value).digest('hex');
const workerHashes=new Set();
const workerPlugin=()=>({name:'audit-editor-worker-source-ownership',generateBundle(_options,bundle){for(const item of Object.values(bundle)){if(item.type!=='chunk')continue;const ids=Object.keys(item.modules).filter(id=>!id.startsWith('\0'));if(ids.length&&ids.every(isEditor))workerHashes.add(hash(item.code));}}});
const originalLog=console.log;console.log=(...args)=>console.error(...args);
let result;
try{result=await build({logLevel:'error',worker:{plugins:()=>[workerPlugin()]},build:{write:false,minify:'terser',cssCodeSplit:split}});}finally{console.log=originalLog;}
const outputs=(Array.isArray(result)?result:[result]).flatMap(r=>r.output||[]);
// write:false skips writeBundle. Apply the same production scope compaction in memory.
const texts=outputs.filter(item=>/\.(js|css)$/.test(item.fileName)).map(item=>({path:item.fileName,content:item.type==='chunk'?item.code:Buffer.from(item.source).toString('utf8')}));
const compacted=compactVueScopes(texts);
const compactedByFile=new Map(compacted.output.map(item=>[item.path,item.content]));
for(const item of outputs){const text=compactedByFile.get(item.fileName);if(text!==undefined){if(item.type==='chunk')item.code=text;else item.source=text;}}
const bytes=item=>Buffer.byteLength(item.type==='chunk'?item.code:item.source);
const chunks=outputs.filter(item=>item.type==='chunk').map(item=>{
 const ids=Object.keys(item.modules).filter(id=>!id.startsWith('\0'));
 const monaco=ids.some(isEditor),exclusive=ids.length>0&&ids.every(isEditor);
 return {file:item.fileName,bytes:bytes(item),entry:item.isEntry,editor:monaco,exclusive,imports:item.imports,dynamicImports:item.dynamicImports,css:[...(item.viteMetadata?.importedCss||[])],assets:[...(item.viteMetadata?.importedAssets||[])],other_modules:monaco&&!exclusive?ids.filter(id=>!isEditor(id)).map(id=>relative(process.cwd(),id)):[]};
});
const assets=outputs.filter(item=>item.type==='asset').map(item=>{
 const owners=chunks.filter(c=>c.css.includes(item.fileName)||c.assets.includes(item.fileName));
 const css=item.fileName.endsWith('.css');
 const worker=workerHashes.has(hash(item.source));
 return {file:item.fileName,bytes:bytes(item),exclusive_editor:isExclusiveEditorAsset(owners,worker),owners:owners.map(c=>c.file),worker_source_hash_verified:worker,note:worker?'Exact emitted bytes match a worker whose source modules are all editor-owned.':css?'CSS attribution requires every importing chunk to be editor-exclusive.':'Unattributed/shared assets remain core.'};
});
const total=outputs.reduce((n,item)=>n+bytes(item),0);
const editor=chunks.filter(c=>c.exclusive).reduce((n,c)=>n+c.bytes,0)+assets.filter(a=>a.exclusive_editor).reduce((n,a)=>n+a.bytes,0);
assert.equal(new Set(outputs.map(item=>item.fileName)).size,outputs.length,'Duplicate output would double-count bytes');
assert.equal(chunks.reduce((n,c)=>n+c.bytes,0)+assets.reduce((n,a)=>n+a.bytes,0),total,'Every emitted byte must be accounted for');
assert.ok(editor>=0&&editor<=total,'Invalid editor attribution');
// Dynamic imports are deliberately not followed: this describes the initial JS
// dependency graph, not a claim about browser timings or subsequent navigation.
const byFile=new Map(chunks.map(chunk=>[chunk.file,chunk]));
const initial=new Set();
const visit=file=>{if(initial.has(file))return;const chunk=byFile.get(file);if(!chunk)return;initial.add(file);chunk.imports.forEach(visit);};
chunks.filter(chunk=>chunk.entry).forEach(chunk=>visit(chunk.file));
const initialEditor=[...initial].filter(file=>byFile.get(file).editor);
assert.deepEqual(initialEditor,[],'Editor code must remain outside the initial static JS graph');
const initialGraph={files:[...initial].sort(),js_bytes:[...initial].reduce((sum,file)=>sum+byFile.get(file).bytes,0),editor_js_files:initialEditor,scope:'Static JS imports only; global CSS and runtime loading measured separately'};
let distVerification=null;
if(verifyDist){
 const files=directory=>readdirSync(directory,{withFileTypes:true}).flatMap(item=>item.isDirectory()?files(join(directory,item.name)):[join(directory,item.name)]);
 const diskFiles=files('dist').map(file=>relative('dist',file).replaceAll('\\','/')).sort();
 assert.deepEqual(diskFiles,outputs.map(item=>item.fileName).sort(),'Disk files differ from audited outputs; public/copied assets must not escape accounting');
 for(const item of outputs)assert.equal(hash(readFileSync(join('dist',item.fileName))),hash(item.type==='chunk'?item.code:item.source),`Production content mismatch: ${item.fileName}`);
 distVerification={status:'PASS',files:diskFiles.length,comparison:'SHA256 of every emitted file; exact disk inventory'};
}
const report={schema:'browseros-editor-budget-audit-v1',css_split:split,dist_verification:distVerification,initial_js_graph:initialGraph,post_build_compaction_bytes_saved:compacted.bytesSaved,total_bytes:total,exclusive_editor_bytes:editor,core_and_shared_bytes:total-editor,chunks,assets,limitations:['The same-run master increment is recorded as advisory data, not a merge blocker.','Core includes every shared/unattributed byte.','Worker attribution uses exact content hashes and complete source-module ownership.','This is build attribution, not startup time, memory use or installed package size.']};
if(auditBudget){
 const policy=JSON.parse(readFileSync(new URL('../docs/engineering/coding-editor-budget.json',import.meta.url)));
 const manifest=JSON.parse(readFileSync('package.json')),lock=JSON.parse(readFileSync('package-lock.json'));
 for(const [name,version] of Object.entries(policy.dependencies))assert.equal(lock.packages[`node_modules/${name}`]?.version,version,`Unreviewed ${name} version`);
 assert.equal(manifest.dependencies['monaco-editor'],policy.dependencies['monaco-editor']);
 assert.equal(manifest.overrides?.['monaco-editor']?.dompurify,policy.dependencies.dompurify);
 assert.equal(JSON.parse(readFileSync('node_modules/monaco-editor/package.json')).version,policy.dependencies['monaco-editor']);
 assert.equal(JSON.parse(readFileSync('node_modules/dompurify/package.json')).version,policy.dependencies.dompurify);
 report.approved_budget=evaluateEditorBudget(report,policy);
 if(report.approved_budget.status==='WARN')console.error('::warning title=Frontend size advisory::Approved byte envelopes exceeded: '+report.approved_budget.failures.join(', ')+'; production hash and owner checks remain mandatory.');
}
console.log(JSON.stringify(report,null,2));
