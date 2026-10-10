#!/usr/bin/env node
// Linux /proc one-shot measurements scoped ONLY to BrowserOS's process tree.
// CPU is process-tree utilization over a real sample interval, not system-wide CPU.
import { existsSync, readFileSync, readlinkSync, readdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { performance } from "node:perf_hooks";
function parseStat(s) {
  const right=s.lastIndexOf(") ");
  if(right<0)return null;
  const rest=s.slice(right+2).trim().split(/\s+/);
  if(rest.length<13)return null;
  return {ppid:Number(rest[1]),cpuTicks:Number(rest[11])+Number(rest[12])};
}
function parseKb(s,label) {
  const match=new RegExp("^"+label+":\\s+(\\d+)\\s+kB","m").exec(s);
  return match?Number(match[1]):null;
}
function collect(pid) {
  const stats=new Map();
  for(const name of readdirSync("/proc")) {
    if(!/^\d+$/.test(name))continue;
    try {const st=parseStat(readFileSync("/proc/"+name+"/stat","utf8"));if(st)stats.set(Number(name),st);}
    catch { /* process exited / access denied */ }
  }
  if(!stats.has(pid))return null;
  const all=[pid];
  for(let i=0;i<all.length;i++)for(const [child,s] of stats){
    if(child!==pid && s.ppid===all[i] && !all.includes(child))all.push(child);
  }
  let rssKb=0,pssKb=0,pssComplete=true,webkitProcesses=0,ptyDescriptors=0,cpuTicks=0;
  for(const id of all) {
    const base="/proc/"+id;
    cpuTicks+=stats.get(id)?.cpuTicks||0;
    try{rssKb+=parseKb(readFileSync(base+"/status","utf8"),"VmRSS")||0;}catch{}
    try{const pss=parseKb(readFileSync(base+"/smaps_rollup","utf8"),"Pss");if(pss==null)pssComplete=false;else pssKb+=pss;}catch{pssComplete=false;}
    try{
      const cmd=readFileSync(base+"/cmdline","utf8").replaceAll("\0"," ");
      if(/webkit|gtkwebkit|chromium|wry|tauri.*webview/i.test(cmd))webkitProcesses++;
    }catch{}
    try{
      for(const fd of readdirSync(base+"/fd")){
        try{if(/^\/dev\/pts\/\d+$/.test(readlinkSync(base+"/fd/"+fd)))ptyDescriptors++;}catch{}
      }
    }catch{}
  }
  return {pids:all,processCount:all.length,rssKb,pssKb:pssComplete?pssKb:null,
    webkitProcesses,ptyDescriptors,cpuTicks};
}
const args=process.argv.slice(2);
if(args.includes("--self-test")){
  const fixture="123 (browser os app) S 1 0 0 0 0 0 0 0 0 0 123 45 0 0 0 0";
  const a=parseStat(fixture);
  if(!a||a.ppid!==1||a.cpuTicks!==168)throw Error("stat parsing invariant");
  if(parseKb("VmRSS:\t2048 kB\nPss:\t1200 kB","Pss")!==1200)throw Error("Pss fixture");
  console.log("NATIVE_RESOURCE_SAMPLER_SELF_TEST=PASS");
} else {
  const i=args.indexOf("--pid"),pid=i>=0?Number(args[i+1]):Number(process.env.MVP_BROWSER_PID||0);
  const interval=Math.max(250,Math.min(5000,Number(args[args.indexOf("--interval-ms")+1])||1000));
  if(!Number.isInteger(pid)||pid<=0||!existsSync("/proc/"+pid)){
    console.log(JSON.stringify({schema:"browseros-native-resource-v1",classification:"UNKNOWN",
      reason:"No running target pid; no made-up process, CPU or memory measurements."},null,2));
  }else{
    const hz=Number(execFileSync("getconf",["CLK_TCK"],{encoding:"utf8"}).trim());
    const before=collect(pid),started=performance.now();
    await new Promise(resolve=>setTimeout(resolve,interval));
    const after=collect(pid),seconds=(performance.now()-started)/1000;
    const valid=before&&after&&Number.isFinite(hz)&&hz>0;
    console.log(JSON.stringify({schema:"browseros-native-resource-v1",classification:valid?"MEASURED":"UNKNOWN",
      targetPid:pid,processCount:after?.processCount??null,
      rss_kb:after?.rssKb??null,pss_kb:after?.pssKb??null,
      webkit_processes:after?.webkitProcesses??null,pty_descriptors:after?.ptyDescriptors??null,
      cpu_percent_one_core:valid?Math.max(0,(after.cpuTicks-before.cpuTicks)/hz/seconds*100):null,
      sample_ms:Math.round(seconds*1000),
      caveat:"RSS sums shared pages across processes; PSS may be unavailable. CPU may exceed 100% with multiple cores. First-paint sample is NOT 5/30-minute idle or p95 evidence."},null,2));
  }
}
