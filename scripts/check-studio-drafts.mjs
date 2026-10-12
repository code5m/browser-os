#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const url=new URL("../src/utils/studioDrafts.ts",import.meta.url);
const { createStudioDraft, validateStudioDraft, parseStudioList, saveStudioList,studioProgress,studioKey }=await import(url.href);
let seq=0;
const memory=new Map();
const storage={setItem:(key,value)=>memory.set(key,value),getItem:(key)=>memory.get(key)??null};
for(const kind of ["agent","skill"]){
  const d=createStudioDraft(kind);
  assert.equal(d.kind,kind); assert.equal(d.steps.length,3);
  d.goal="生成用户可以理解的任务规划";d.steps[0].done=true;
  assert.equal(studioProgress(d),33);
  saveStudioList([d],kind,storage);
  const loaded=parseStudioList(storage.getItem(studioKey(kind)),kind);
  assert.equal(loaded[0].goal,d.goal);assert.equal(loaded[0].steps[0].done,true);
  seq++;
  assert.throws(()=>validateStudioDraft({...d,kind:kind==="agent"?"skill":"agent"},kind));
  assert.throws(()=>validateStudioDraft({...d,description:"Bearer abcdefghijklmnop"},kind));
  assert.throws(()=>saveStudioList(Array.from({length:kind==="agent"?65:201},(_,i)=>({...d,id:"draft-"+i})),kind,storage));
  const original=storage.getItem(studioKey(kind));
  assert.throws(()=>parseStudioList("{broken",kind));
  assert.equal(storage.getItem(studioKey(kind)),original);
  seq+=5;
}
const agent=parseStudioList(storage.getItem(studioKey("agent")),"agent");
const skill=parseStudioList(storage.getItem(studioKey("skill")),"skill");
assert.notEqual(agent[0].kind,skill[0].kind);
const ui=readFileSync(new URL("../src/shared/ui/BlueprintStudio.vue",import.meta.url),"utf8");
assert.ok(ui.includes("verifiedText.value!==definitionText.value"),"必须禁止校验后篡改定义");
assert.ok(ui.includes('file.size>90_000'),"导入容量必须受限");
assert.ok(ui.includes("window.confirm("),"删除必须二次确认");
console.log("STUDIO_DRAFTS_RESULT=PASS assertions="+(seq+4));
