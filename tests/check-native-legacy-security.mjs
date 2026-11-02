import assert from 'node:assert/strict'
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
const root = new URL('../', import.meta.url).pathname
const names=JSON.parse(readFileSync(join(root,'docs/architecture/native-boundary/native-permission-debt-baseline.json'),'utf8')).pending_side_effect_reviews
const exceptions={tab_new:'native internal Rust startup / main-only Tauri ACL',tab_close:'native internal Rust cleanup / main-only Tauri ACL',tab_list:'internal Browser Public API / main-only Tauri ACL',report_title:'remote-collect telemetry + origin check',report_resources:'remote-collect telemetry + origin check'}
function rustPaths(dir){return readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?rustPaths(join(dir,e.name)):e.name.endsWith('.rs')?[join(dir,e.name)]:[])}
const sourceFiles=rustPaths(join(root,'src-tauri/src')).map(p=>[p,readFileSync(p,'utf8')])
let guarded=0,remote=0,internal=0
for(const name of names){
  const hits=[]
  for(const [path,text] of sourceFiles){
    const m=new RegExp(`#\\[tauri::command\\]\\s*pub\\s+(?:async\\s+)?fn\\s+${name}\\s*\\(`).exec(text)
    if(m)hits.push({path,text,offset:m.index})
  }
  assert.equal(hits.length,1,`${name}: expected exactly one native command`)
  const {path,text,offset}=hits[0],segment=text.slice(offset,offset+1900)
  if(name==='report_title'||name==='report_resources') {
    assert.match(segment,/check_invocation_source\(/,`${name}: must validate the untrusted remote origin`)
    remote++;continue
  }
  if(Object.hasOwn(exceptions,name)) {internal++;continue}
  assert.match(segment,new RegExp(`native_legacy_policy::require_main\\(&webview, "${name}"\\)`),`${name} missing Rust source guard at ${path}`)
  assert.match(segment,/webview:\s*(?:tauri::)?Webview/,`${name}: no Webview caller context`)
  guarded++
}
assert.deepEqual({guarded,remote,internal},{guarded:53,remote:2,internal:3})
const full=sourceFiles.filter(x=>x[0].endsWith('native_legacy_policy.rs'))[0][1]
assert.match(full,/bound_text\(/)
assert.match(full,/bound_dimension\(/)
assert.match(readFileSync(join(root,'src/capabilities/script/state/useSnippetStore.ts'),'utf8'),/window\.confirm\(/)
assert.match(readFileSync(join(root,'src/capabilities/session/state/useSessionStore.ts'),'utf8'),/window\.confirm\(/)
const serverSession=sourceFiles.find(([path])=>path.endsWith('/capabilities/session/commands.rs'))[1]
const serverBridge=sourceFiles.find(([path])=>path.endsWith('/bridge.rs'))[1]
assert.ok(serverSession.includes('session_delete:{id}') && serverSession.includes('.consume('))
assert.ok(serverBridge.includes('snippet_remove:{id}') && serverBridge.includes('.consume('))
assert.match(serverBridge,/is_confirmed_effect/)
const uiBridge=readFileSync(join(root,'src/bridge.ts'),'utf8')
assert.match(uiBridge,/nativeIssueIntent/)
assert.match(uiBridge,/sessionDelete: \(id: string, intent: string\)/)
assert.match(uiBridge,/snippetRemove: \(id: string, intent: string\)/)
const acl=JSON.parse(readFileSync(join(root,'src-tauri/capabilities/default.json'),'utf8'))
assert.deepEqual(acl.windows,['main'])
assert.ok(!acl.remote)
console.log(`NATIVE_LEGACY_SECURITY=PASS registered=${names.length} rust_source_guard=${guarded} remote_origin_guard=${remote} main_acl_internal=${internal} irreversible_ui_confirms=2`)
