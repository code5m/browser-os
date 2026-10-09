#!/usr/bin/env node
/**
 * BrowserOS Rust runtime semantic evidence: derives observations from real Rust/TS sources.
 * Existing Native Command + AppState checkers remain the authority. No second owner registry.
 *
 * This intentionally does not assert that an observed TS generic equals a Rust Serde DTO:
 * these are syntax-level signatures, not cross-language proof of structural equivalence.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = p => fs.readFileSync(path.join(root,p),"utf8");
const filePaths = (directory, ext) => {
  const result=[];
  function walk(d) {
    for(const item of fs.readdirSync(path.join(root,d),{withFileTypes:true})) {
      const p=path.posix.join(d,item.name);
      if(item.isDirectory()) walk(p);
      else if(item.name.endsWith(ext)) result.push(p);
    }
  }
  walk(directory);
  return result.sort();
};
function literalCalls(code, regexp) {return [...code.matchAll(regexp)].map(x=>({name:x[1],offset:x.index}));}
function eventInventory(rust, bridge) {
  const emitters=[];
  for(const [source,content] of rust) {
    for(const m of [
       ...literalCalls(content,/\bemit\s*\(\s*"([^"]+)"/g),
       ...literalCalls(content,/\bemit_to\s*\(\s*[^,]+,\s*"([^"]+)"/g),
     ]) {
      emitters.push({name:m.name,source});
    }
  }
  const tsListeners=literalCalls(bridge,/\blisten(?:<[^>]*>)?\s*\(\s*"([^"]+)"/g).map(x=>x.name);
  const rustNames=new Set(emitters.map(x=>x.name));
  const listeners=[...new Set(tsListeners)].sort();
  const observed=[...rustNames].sort();
  return {emitted:observed,listened: listeners,
    rustEmitCallsites:emitters.length,
    both:observed.filter(e=>listeners.includes(e)),
    notObservedInBridge:observed.filter(e=>!listeners.includes(e)),
    listenerWithoutObservedRustEmit:listeners.filter(e=>!rustNames.has(e)),
    // Partial coverage by design: other TS listeners and dynamically named Rust emits
    // are outside this specific bridge-only projection.
    scope:"literal Rust emit/emit_to/emit_filter and literal src/bridge.ts listen calls",
  };
}
function ipcInventory(bridge, nativeInventory) {
 const registered=new Set(nativeInventory?.registered ?? []);
 // check-native-command-inventory handles authority, this independent projection
 // checks only source-observed wrappers, not dynamically constructed commands.
 const invokes=literalCalls(bridge,/\binvoke(?:<[^(\n]*>)?\s*\(\s*"([^"]+)"/g).map(x=>x.name);
 const unique=[...new Set(invokes)].sort();
 const unresolved=unique.filter(x=>!registered.has(x));
 const dormantFlag=/export\s+const\s+AGENT_SKILL_COMMANDS_AVAILABLE\s*=\s*false\b/.test(bridge);
 const dormant=dormantFlag ? unresolved.filter(x=>/^(agent_|skill_|confirm_agent_|confirm_skill_)/.test(x)) : [];
 const unregistered=unresolved.filter(x=>!dormant.includes(x));
 return {observedLiteralInvocations:unique,occurrences:invokes.length,
   intentionallyDisabled: dormant,unregisteredLiteralInvocations:unregistered,
   scope:"literal invokes in src/bridge.ts; disabled Agent/Skill command family classified only if capability feature flag is false; generic TS types are NOT proof of Serde equivalence"};
}
function selfTest() {
 const sample=eventInventory([["test.rs",'app.emit("ready",123); app.emit_to("window","done",())']], 'listen("ready",cb);listen("orphan",cb)');
 if(sample.emitted.join(",")!=="done,ready" || sample.both.join(",")!=="ready"||sample.listenerWithoutObservedRustEmit.join(",")!=="orphan") return false;
 const native={registered:["hello"]};
 const good=ipcInventory('invoke<string>("hello")',native);
 const bad=ipcInventory('invoke<number>("unknown")',native);
 const disabled=ipcInventory('export const AGENT_SKILL_COMMANDS_AVAILABLE = false;invoke("agent_chat")',native);
 return good.unregisteredLiteralInvocations.length===0&&bad.unregisteredLiteralInvocations[0]==="unknown"&&disabled.intentionallyDisabled[0]==="agent_chat"&&disabled.unregisteredLiteralInvocations.length===0;
}
if(process.argv.includes("--self-test")){
 const ok=selfTest();console.log("RUST_RUNTIME_SEMANTIC_SELF_TEST="+(ok?"PASS":"FAIL"));process.exit(ok?0:1);
}
const inventoryRun=spawnSync(process.execPath,["scripts/check-native-command-inventory.mjs","--json"],{cwd:root,encoding:"utf8"});
if(inventoryRun.status!==0){console.error("RUST_RUNTIME_SEMANTICS_RESULT=FAIL native command authority failed");process.exit(1)}
let native;
try{native=JSON.parse(inventoryRun.stdout)}catch{console.error("RUST_RUNTIME_SEMANTICS_RESULT=FAIL malformed native inventory");process.exit(1)}
const rust=filePaths("src-tauri/src",".rs").map(p=>[p,read(p)]);
const bridge=read("src/bridge.ts");
const events=eventInventory(rust,bridge);
const rustMain=read("src-tauri/src/main.rs");
const handlers=[...rustMain.matchAll(/generate_handler!\[([\s\S]*?)\]/g)]
  .flatMap(x=>x[1].replace(/\/\/[^\n]*/g,"").split(",").map(s=>s.trim()).filter(s=>/^\w+(?:::\w+)*$/.test(s)).map(s=>s.split("::").at(-1)));
const ipc=ipcInventory(bridge,{registered:[...new Set(handlers)]});
const errors=[];
if(!native.gate.PASS)errors.push("native inventory gate failed");
if(ipc.unregisteredLiteralInvocations.length)errors.push("unregistered IPC commands: "+ipc.unregisteredLiteralInvocations.join(","));
const report={schemaVersion:1,source:"code-derived",rustFiles:rust.length,
  native:{registered:native.counts.TOTAL_REGISTERED,appState:native.counts.APPSTATE_STRUCT_FIELDS,gate:native.gate.PASS},
  events,ipc,limitations:[
   "Only literal event strings are included; dynamic events are unverified",
   "src/bridge.ts is not the sole possible frontend listener; missing listener observations are not failures",
   "TypeScript generic and Rust Serde DTO structural equivalence is NOT proven by this report",
   "Resource ownership/lifecycle evidence belongs to the Native Physical Boundary Matrix and existing lifecycle gate"
 ]};
const index=process.argv.indexOf("--report");
if(index!==-1){if(!process.argv[index+1])errors.push("--report missing path");else{
 const target=path.resolve(root,process.argv[index+1]);
 fs.mkdirSync(path.dirname(target),{recursive:true});
 fs.writeFileSync(target,JSON.stringify(report,null,2)+"\n");
}}
if(process.argv.includes("--json"))console.log(JSON.stringify(report,null,2));
for(const err of errors)console.error(err);
console.log("RUST_RUNTIME_SEMANTICS_RESULT="+(errors.length?"FAIL":"PASS")+" rust_files="+rust.length+" emits="+events.emitted.length+" bridge_listens="+events.listened.length+" literal_invokes="+ipc.observedLiteralInvocations.length+" abi_structural_equivalence=UNVERIFIED");
process.exit(errors.length?1:0);
