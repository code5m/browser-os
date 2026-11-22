import assert from 'node:assert/strict';
import { lspFailureHint } from '../src/capabilities/agent/model/codingLsp.mjs';
assert.match(lspFailureHint('LSP_BUSY'),/忙/);
assert.doesNotMatch(lspFailureHint('LSP_BUSY secret-value'),/secret-value/);
assert.doesNotMatch(lspFailureHint('unexpected secret-value'),/secret-value/);
assert.match(lspFailureHint('LSP_DIAGNOSTICS_UNSUPPORTED'),/更新 rust-analyzer/);
assert.match(lspFailureHint('LSP_DIAGNOSTICS_CAPABILITY_INVALID'),/能力信息无效/);
assert.match(lspFailureHint('LSP_DIAGNOSTICS_REPORT_INVALID'),/不完整.*结果不可用/);
assert.match(lspFailureHint('LSP_DOCUMENT_VERSION_EXHAUSTED'),/关闭并重新开启/);
assert.doesNotMatch(lspFailureHint('LSP_DIAGNOSTICS_REPORT_INVALID secret-value'),/secret-value/);

for (const code of ['LSP_BUNDLE_DIRECTORY_UNAVAILABLE','LSP_BUNDLE_MANIFEST_UNAVAILABLE','LSP_BUNDLE_EXECUTABLE_UNAVAILABLE']) {
  assert.match(lspFailureHint(code),/修复或重新安装 BrowserOS/);
}
for (const code of ['LSP_BUNDLE_MANIFEST_INVALID','LSP_BUNDLE_MANIFEST_MISMATCH','LSP_BUNDLE_EXECUTABLE_MISMATCH']) {
  assert.match(lspFailureHint(code),/完整性校验未通过.*BrowserOS/);
  assert.doesNotMatch(lspFailureHint(code+' private-path secret-value'),/private-path|secret-value/);
}
assert.match(lspFailureHint('LSP_RUSTC_TOOLCHAIN_MISSING'),/工具链缺少 rustc/);
assert.match(lspFailureHint('LSP_RUSTC_CFG_PEER_MISSING'),/工具链缺少 rustc/);
assert.match(lspFailureHint('LSP_RUSTC_CFG_PEER_UNSAFE'),/路径未通过安全检查/);

import {editorRange,completionItems,diagnosticMarkers,definitionRanges,plainHover,mappedDefinitionTargets,diagnosticContext} from '../src/capabilities/agent/model/codingLsp.mjs';
const range={start:{line:0,character:2},end:{line:0,character:5}};
assert.deepEqual(editorRange(range),{startLineNumber:1,startColumn:3,endLineNumber:1,endColumn:6});
assert.equal(editorRange({start:{line:-1,character:0},end:{line:0,character:0}}),null);
assert.equal(editorRange({start:{line:1,character:0},end:{line:0,character:0}}),null);
assert.equal(completionItems({items:[{label:'square',kind:3,textEdit:{newText:'square($0)',range},insertTextFormat:2}]},{} )[0].insertText,'square($0)');
assert.equal(completionItems(null,{}).length,0);
assert.equal(completionItems([{label:'square',kind:3}],{})[0].kind,1,'LSP Function is Monaco Function, not Constructor');
assert.equal(completionItems([{label:'snippet',kind:15}],{})[0].kind,28);
assert.equal(diagnosticMarkers([{range,message:'syntax',severity:1}])[0].severity,8);
assert.equal(definitionRanges([{uri:'file:///etc/passwd',range}]).length,0);
assert.equal(definitionRanges([{targetUri:'file:///work/document.rs',targetSelectionRange:range}]).length,1);
assert.equal(definitionRanges({uri:'file:///work/document.rs',range}).length,1);
assert.equal(plainHover('![x](https://example.test)'), '\\!\\[x\\]\\(https://example.test\\)');
const targets={definition_targets:[{path:'src/math.rs',uri:'file:///work/src/math.rs',hash:'a'.repeat(64),range}]};
assert.equal(mappedDefinitionTargets(targets,'src/main.rs')[0].path,'src/math.rs');
assert.equal(mappedDefinitionTargets({definition_targets:[{...targets.definition_targets[0],path:'../outside.rs'}]},'main.rs').length,0);
assert.equal(mappedDefinitionTargets({definition_targets:[{...targets.definition_targets[0],uri:'file:///etc/passwd'}]},'main.rs').length,0);
assert.equal(mappedDefinitionTargets({definition_targets:[{...targets.definition_targets[0],path:'src/工具 #?.rs',uri:'file:///work/src/%E5%B7%A5%E5%85%B7%20%23%3F.rs'}]},'main.rs').length,1);
const diagnostics=diagnosticContext({scope:'isolated_current_rust_document',document_hash:'a'.repeat(64),diagnostics:Array.from({length:25},()=>({range,severity:1,message:'x'.repeat(600)}))},'main.rs');
assert.equal(diagnostics.compilation_verified,false);assert.equal(diagnostics.errors,25);assert.equal(diagnostics.diagnostics.length,20);assert.equal(diagnostics.truncated,true);assert.ok(JSON.stringify(diagnostics).length<16000);
assert.throws(()=>diagnosticContext({diagnostics:[]},'main.rs'));

const { sandboxRecoveryHint } = await import('../src/capabilities/agent/model/codingLsp.mjs');
assert.match(sandboxRecoveryHint({state:'review_required'}), /需要核对/);
assert.match(sandboxRecoveryHint({state:'pending'}), /重试/);
assert.equal(sandboxRecoveryHint({state:'unsupported'}), '');
assert.match(sandboxRecoveryHint({state:'not_initialized'}), /重启/);
assert.ok(!sandboxRecoveryHint({state:'unexpected',lastCode:'secret-token'}).includes('secret-token'));

console.log('CODING_LSP_CONVERSION_PASS');
