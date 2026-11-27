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
 const receiptActions=new Map([
  ['tab_activate',['Activate','id']],['tab_close',['Close','id']],
  ['tab_go_back',['Back','id']],['tab_go_forward',['Forward','id']],
  ['tab_new',['New','url']],['tab_open',['Open','id, url']],['tab_reload',['Reload','id']]
 ]);
 if(receiptActions.has(name)){
  const [variant,fields]=receiptActions.get(name);
  const dispatch=new RegExp('super::tab_mutation::legacy_action\\(\\s*webview,\\s*app,\\s*super::tab_mutation::Action::'+variant+'\\s*\\{\\s*'+fields.replace(', ', ',\\s*')+'\\s*\\}\\s*,?\\s*\\)\\s*\\.await\\s*\\}');
  assert.match(body,dispatch,name+': wrapper must dispatch the exact typed action and arguments');
  const owner=rust.find(([file])=>file.endsWith('/bridge/tab_mutation.rs'));
  assert.ok(owner,name+': native receipt owner missing');
  const cut=(from,to)=>{const a=owner[1].indexOf(from),b=owner[1].indexOf(to,a+from.length);assert.ok(a>=0&&b>a,name+': missing receipt region '+from);return owner[1].slice(a,b)};
  const command=cut('fn command(&self)', 'fn target(&self)');
  assert.match(command,new RegExp('Self::'+variant+'\\s*\\{\\s*\\.\\.\\s*\\}\\s*=>\\s*"'+name+'"'));
  const authorize=cut('fn authorize(&self,', 'fn is_document_mutation(');
  assert.match(authorize,/else\s*\{\s*self\.command\(\)\s*\};\s*require_main\(webview,\s*cmd\)\?;/);
  if(variant==='New') assert.match(authorize,/Self::New\s*\{\s*url\s*\}\s*=>\s*bound_text\(cmd,\s*"url",\s*url\)/);
  else if(variant==='Open') assert.match(authorize,/Self::Open\s*\{\s*id,\s*url\s*\}\s*=>\s*\{\s*bound_text\(cmd,\s*"id",\s*id\)\?;\s*bound_text\(cmd,\s*"url",\s*url\)/);
  else assert.match(authorize,/Self::Close\s*\{\s*id\s*\}\s*\|\s*Self::Activate\s*\{\s*id\s*\}\s*\|\s*Self::Back\s*\{\s*id\s*\}\s*\|\s*Self::Forward\s*\{\s*id\s*\}\s*\|\s*Self::Reload\s*\{\s*id\s*\}\s*=>\s*bound_text\(cmd,\s*"id",\s*id\)/);
  const admission=cut('pub(super) async fn legacy_action<','pub(crate) fn tab_mutation_diagnostic(');
  assert.match(admission,/action\.authorize\(&webview\)\?;\s*let ticket[\s\S]*?let mut state = owner\(\)\?;/);
  assert.match(admission,/let binding = Binding::capture\(&app,\s*&action\)\?;[\s\S]*?if binding\.current\(&app,\s*&action\)\s*\{\s*action\s*\.execute\(webview,\s*app\.clone\(\),\s*&binding\)/);
  // Validate the worker's generation-bound routes as well as the compatibility
  // helper: newer document bindings must not silently fall back to a replacement.
  assert.match(admission,/let entered = \{[\s\S]*?matching\(&mut state, &ticket\)\.is_ok_and\(Slot::enter\)[\s\S]*?if !entered \{[\s\S]*?TAB_MUTATION_CANCELLED_BEFORE_EXECUTION/);
  const execute=cut('fn execute(self,','fn agent_action(');
  assert.match(execute,/self\.authorize\(&webview\)\?;/);
  assert.match(execute,new RegExp('Self::'+variant+'\\s*\\{\\s*'+fields.replace(', ', ',\\s*')+'\\s*\\}\\s*=>\\s*(?:serde_json::to_value\\()?'+name+'_impl\\(webview,\\s*app,\\s*'+fields.replace(', ', ',\\s*')+'\\)'));
  if(variant==='Close'){
   assert.match(execute,/if let Self::Close \{ ref id \} = self \{[\s\S]*?Binding::LiveDocument\(document\)[\s\S]*?TabCloseAdmission::EmbeddedDocument\(document\)[\s\S]*?Binding::IsolatedDocument\(document\)[\s\S]*?TabCloseAdmission::IsolatedDocument\(\*document\)[\s\S]*?return super::close_tab_admitted\(&app, id, Some\(admitted\)\)/);
  }
  if(variant==='Open'){
   assert.match(execute,/if let Binding::LiveDocument\(document\) = binding \{[\s\S]*?tab_open_document\(&app, &id, &url, document\)/);
   assert.match(execute,/if let Binding::IsolatedDocument\(document\) = binding \{[\s\S]*?tab_open_admitted\(webview, app.clone\(\), id, url, Some\(\*document\)\)/);
  }
  if(variant==='Activate'){
   assert.match(execute,/if let Binding::IsolatedActivation\(expected\) = binding \{[\s\S]*?tab_activate_admitted\(webview, app.clone\(\), id.clone\(\), Some\(\*expected\)\)/);
  }
  if(['Back','Forward','Reload'].includes(variant)){
   assert.match(execute,/if let Binding::LiveDocument\(document\) = binding \{[\s\S]*?\.eval_navigation_document\(&id, document, js\)/);
   const code={Back:'if (history.length > 1) { history.back(); }',Forward:'history.forward();',Reload:'location.reload();'}[variant];
   const literal=code.replace(/[.*+?^$(){}|[\]\\]/g,'\\$&');
   assert.match(execute,new RegExp('Self::'+variant+'\\s*\\{\\s*id\\s*\\}\\s*=>\\s*(?:\\{\\s*)?super::isolated_tabs::eval_bound\\(\\s*&app,\\s*&id,\\s*"'+literal+'"\\s*,\\s*\\*document\\s*,?\\s*\\)'));
  }
  const helper=new RegExp('pub\\(crate\\)\\s+fn\\s+'+name+'_impl\\s*\\(').exec(source);
  assert.ok(helper,name+': exact native implementation missing');
  body=source.slice(helper.index,source.indexOf('\n}',helper.index)+2);
  if(name==='tab_activate'||name==='tab_open'){
   assert.match(body,new RegExp(name+'_admitted\\(webview,\\s*app,\\s*'+fields.replace(', ', ',\\s*')+',\\s*None\\)'));
   const admitted=new RegExp('pub\\(super\\)\\s+fn\\s+'+name+'_admitted\\s*\\(').exec(source);
   assert.ok(admitted,name+': admitted helper missing');
   body=source.slice(admitted.index,source.indexOf('\n}',admitted.index)+2);
  }
 }
 const remote=name==='report_title'||name==='report_resources';
 const originGuard=remote?'remote-origin-policy':'main-webview-guard';
 const mainGuard=new RegExp('native_legacy_policy::require_main\\(\\s*&webview,\\s*"'+name+'"\\s*,?\\s*\\)');
 assert.ok(remote?/check_invocation_source\s*\(/.test(body):mainGuard.test(body),`${name}: missing ${originGuard}`);
 const primitiveFields=[...params.matchAll(/\b([a-z_][a-z0-9_]*)\s*:\s*(?:Option<\s*)?(String|f64)/g)].map(m=>({name:m[1],type:m[2]}));
 const uncovered=primitiveFields.filter(field=>{
  if(remote)return false;
  if(field.name==='intent' && (name==='session_delete'||name==='snippet_remove'||name==='set_default_browser'))return !body.includes('.consume(');
  const bound=new RegExp('native_legacy_policy::bound_'+(field.type==='f64'?'dimension':'text')+'\\(\\s*"'+name+'",\\s*"'+field.name+'"');
  return !bound.test(body);
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
