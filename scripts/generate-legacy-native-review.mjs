#!/usr/bin/env node
// Each legacy native command receives a physically grounded source/argument/
// consent evidence row. Installed GUI effects remain explicitly unverified.
import assert from 'node:assert/strict';
import {readFileSync,readdirSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
const root=process.cwd();
const names=JSON.parse(readFileSync(join(root,'docs/architecture/native-boundary/native-permission-debt-baseline.json'),'utf8')).pending_side_effect_reviews;
const registry=readFileSync(join(root,'docs/architecture/native-boundary/native-commands.yaml'),'utf8');
const evidenceFile=join(root,'docs/architecture/native-boundary/legacy-native-58-real-effect-review.json');
function files(path){return readdirSync(path,{withFileTypes:true}).flatMap(ent=>ent.isDirectory()?files(join(path,ent.name)):ent.name.endsWith('.rs')?[[join(path,ent.name),readFileSync(join(path,ent.name),'utf8')]]:[])}
const rust=files(join(root,'src-tauri/src'));
const confirmed=new Map([['session_delete','bound-one-shot-token-and-confirmed-ui'],['snippet_remove','bound-one-shot-token-and-confirmed-ui'],['set_default_browser','bound-one-shot-token-after-two-step-ui']]);
const evidence=[];
for(const name of names){
 const found=[];
 for(const [path,source] of rust){
  const pattern=new RegExp(`#\\[tauri::command\\]\\s*pub\\s+(?:async\\s+)?fn\\s+${name}\\s*\\(`);
  const match=pattern.exec(source);if(match)found.push({path,source,position:match.index});
 }
 assert.equal(found.length,1,`${name}: must have one Tauri command owner`);
 const {path,source,position}=found[0];
 const open=source.indexOf('(',position);let depth=0,end=-1;
 for(let i=open;i<source.length;i++){if(source[i]==='(')depth++;if(source[i]===')'&&--depth===0){end=i;break;}}
 assert.ok(end>open,`${name} signature`);
 const params=source.slice(open+1,end);let body=source.slice(end,source.indexOf('\n}',end)+2);
 // The receipt wrappers retain the same command identity and call the exact
 // guarded implementation. Verify both edges; do not treat a helper's name
 // alone as evidence of origin/argument validation.
 const receiptWrapped=new Set(['tab_activate','tab_close','tab_go_back','tab_go_forward','tab_new','tab_open','tab_reload']);
 if(receiptWrapped.has(name)){
  const dispatch=new RegExp('super::tab_mutation::legacy\\(\\s*webview,\\s*"'+name+'",\\s*move\\s*\\|webview\\|\\s*(?:\\{\\s*)?'+name+'_impl\\(webview,\\s*app(?:,\\s*[a-z_]+)*\\)\\s*(?:\\}\\s*)?\\)\\s*\\.await');
  assert.match(body,dispatch,name+': receipt wrapper must dispatch exactly the guarded implementation');
  const owner=rust.find(([file])=>file.endsWith('/bridge/tab_mutation.rs'));
  assert.ok(owner,name+': native receipt owner missing');
  const admissionStart=owner[1].indexOf('pub(super) async fn legacy<');
  assert.ok(admissionStart>=0,name+': legacy admission missing');
  const admission=owner[1].slice(admissionStart);
  assert.match(admission,/crate::shared::native_legacy_policy::require_main\(&webview,\s*command\)\?;\s*let ticket[\s\S]*?let mut state = owner\(\)\?;/,name+': receipt admission must validate main caller before taking owner');
  const helper=new RegExp('pub\\(crate\\)\\s+fn\\s+'+name+'_impl\\s*\\(').exec(source);
  assert.ok(helper,name+': exact native implementation missing');
  body=source.slice(helper.index,source.indexOf('\n}',helper.index)+2);
 }
 const remote=name==='report_title'||name==='report_resources';
 const originGuard=remote?'remote-origin-policy':'main-webview-guard';
 assert.ok(body.includes(remote?'check_invocation_source(':`native_legacy_policy::require_main(&webview, "${name}")`),`${name}: missing ${originGuard}`);
 const primitiveFields=[...params.matchAll(/\b([a-z_][a-z0-9_]*)\s*:\s*(?:Option<\s*)?(String|f64)/g)].map(m=>({name:m[1],type:m[2]}));
 const uncovered=primitiveFields.filter(field=>{
  if(remote)return false;
  if(field.name==='intent' && (name==='session_delete'||name==='snippet_remove'||name==='set_default_browser'))return !body.includes('.consume(');
  return !body.includes(`native_legacy_policy::bound_${field.type==='f64'?'dimension':'text'}("${name}", "${field.name}"`);
 });
 assert.equal(uncovered.length,0,`${name} missing primitive argument bound: ${JSON.stringify(uncovered)}`);
 const m=new RegExp(`^  ${name}:\\s*\\n(?:(?!^  [a-z_]+:)[\\s\\S])*?^    side_effect: "([^"]+)"`,'m').exec(registry);
 assert.ok(m,`${name} effect registry missing`);
 const confirmation=confirmed.get(name)??(remote?'remote-telemetry-only':'main-ui-owner-action');
 if(confirmation==='bound-one-shot-token-and-confirmed-ui'||confirmation==='bound-one-shot-token-after-two-step-ui'){
  assert.match(body,/\.consume\(/);
  assert.match(body,/NATIVE_USER_CONFIRMATION_REQUIRED/);
 }
 evidence.push({command:name,physicalFile:path.slice(root.length+1),declaredEffect:m[1],sourcePolicy:originGuard,
  boundedPrimitiveInputs:primitiveFields.map(field=>field.name),
  opaqueOwnerInputValidation:!remote && /\bVec<|GraphQueryRequest|ScriptParam|Option<.*Value/.test(params)?'owner-specific-validation-not-proven-by-static-test':'none-or-not-required',
  consentMode:confirmation, sourceAndPrimitiveStaticCheck:'PASS',installedGuiSideEffectEvidence:'GUI_PENDING'});
}
const summary={commands:names.length,sourceGuardVerified:evidence.length,mainOnly:evidence.filter(e=>e.sourcePolicy==='main-webview-guard').length,remoteTelemetry:evidence.filter(e=>e.sourcePolicy==='remote-origin-policy').length,oneShotConsent:evidence.filter(e=>e.consentMode==='bound-one-shot-token-and-confirmed-ui'||e.consentMode==='bound-one-shot-token-after-two-step-ui').length,guiEvidenceCompleted:0,limitation:'Source and primitive input checks do not prove authorized UI gestures, complex parameter validation, actual native effects or packaged GUI. All remain GUI_PENDING.'};
const content=JSON.stringify({summary,commands:evidence},null,2)+'\n';
if(process.argv.includes('--generate')){writeFileSync(evidenceFile,content);console.log('NATIVE_58_REVIEW_GENERATED=PASS',JSON.stringify(summary));}
else if(process.argv.includes('--check')){assert.equal(readFileSync(evidenceFile,'utf8'),content,'Native effects ledger drifted from real command code');console.log('NATIVE_58_REVIEW_CHECK=PASS',JSON.stringify(summary));}
else {console.error('usage: --generate | --check');process.exit(2)}
