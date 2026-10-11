#!/usr/bin/env node
// Opt-in sustained RSS/PSS/CPU sampling of an already running BrowserOS PID.
// NEVER turn CI first-paint sampling into fake 5/30 minute idle evidence.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
const script = join(dirname(fileURLToPath(import.meta.url)), "measure-app-resources.mjs");
const args=process.argv.slice(2);
const val=(flag,defaultValue)=>{const at=args.indexOf(flag);return at<0?defaultValue:args[at+1];};
function numbers(samples,key) {return samples.map(s=>s[key]).filter(v=>typeof v==="number"&&Number.isFinite(v)&&v>=0).sort((a,b)=>a-b);}
function percentile(n,q){return n.length?n[Math.min(n.length-1,Math.ceil(n.length*q)-1)]:null;}
function summarize(samples,needed){
  const keys=["rss_kb","pss_kb","cpu_percent_one_core"];
  const metrics={};
  for(const key of keys){
    const vals=numbers(samples,key);
    metrics[key]={available:vals.length,median:percentile(vals,.5),p95:percentile(vals,.95),peak:vals.at(-1)??null};
  }
  const valid=samples.filter(x=>x.classification==="MEASURED").length;
  return {schema:"browseros-sustained-resource-v1",
    classification:samples.length===needed&&valid===needed?"MEASURED":"INCOMPLETE",
    expected_samples:needed,obtained_samples:samples.length,valid_samples:valid,metrics,
    limitation:"Only process-tree CPU/RSS/PSS; no frame latency, user interaction timings, or cross-host comparison without matching machines."};
}
if(args.includes("--self-test")){
  const summary=summarize([{classification:"MEASURED",rss_kb:100,cpu_percent_one_core:2},{classification:"MEASURED",rss_kb:120,cpu_percent_one_core:3}],2);
  assert.equal(summary.classification,"MEASURED");
  assert.equal(summary.metrics.rss_kb.p95,120);
  assert.equal(summary.metrics.pss_kb.median,null);
  assert.equal(summarize([],2).classification,"INCOMPLETE");
  console.log("SUSTAINED_RESOURCE_BENCHMARK_SELF_TEST=PASS");
}else{
  const pid=Number(val("--pid",process.env.MVP_BROWSER_PID)),minutes=Number(val("--minutes",5)),interval=Number(val("--interval-ms",1000)),out=val("--out",null);
  if(!Number.isInteger(pid)||pid<=0||![5,30].includes(minutes)||!Number.isInteger(interval)||interval<1000||interval>5000||!out){
    throw Error("Usage: --pid POSITIVE_PID --minutes 5|30 --interval-ms 1000..5000 --out path.json");
  }
  const count=Math.floor(minutes*60000/interval),samples=[];
  for(let i=0;i<count;i++){
    try{const raw=execFileSync(process.execPath,[script,"--pid",String(pid),"--interval-ms",String(interval)],{encoding:"utf8",timeout:interval+10000});
      const value=JSON.parse(raw);if(value.classification!=="MEASURED"){samples.push(value);break;}samples.push(value);
    }catch{samples.push({classification:"UNKNOWN"});break;}
  }
  const result={...summarize(samples,count),target_pid:pid,minutes,interval_ms:interval,sampled_at:new Date().toISOString(),
    provenance:{app_sha:process.env.BROWSEROS_APP_SHA||null,os:process.platform,arch:process.arch}};
  mkdirSync(dirname(resolve(out)),{recursive:true});writeFileSync(out,JSON.stringify(result,null,2)+"\n","utf8");
  console.log("SUSTAINED_RESOURCE_BENCHMARK="+result.classification);
  if(result.classification!=="MEASURED")process.exitCode=1;
}
