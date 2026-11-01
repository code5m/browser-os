#!/usr/bin/env node
// Local Ubuntu runtime evidence: per-Grid child PID, RSS/PSS and restarts.
// Read-only /proc sampler. No app privileges, process termination or network.
import {readFileSync,readdirSync,existsSync} from 'node:fs';
import {setTimeout as sleep} from 'node:timers/promises';
function parseStat(text){
 const end=text.lastIndexOf(') ');
 if(end<0)return null;
 const columns=text.slice(end+2).trim().split(/\s+/);
 return columns.length>=20?{ppid:Number(columns[1]),start:columns[19]}:null;
}
function kb(text,name){
 const m=new RegExp('^'+name+':\\s+(\\d+)\\s+kB','m').exec(text);
 return m?Number(m[1]):null;
}
function collect(parent){
 const processes=new Map();
 for(const item of readdirSync('/proc')){
  if(!/^\d+$/.test(item))continue;
  const pid=Number(item);
  try{
   const stat=parseStat(readFileSync('/proc/'+pid+'/stat','utf8'));
   if(stat)processes.set(pid,stat);
  }catch{}
 }
 const children=new Map();
 for(const [pid,info] of processes){
  const entries=children.get(info.ppid)||[];
  entries.push(pid);children.set(info.ppid,entries);
 }
 const descendants=[parent],visited=new Set(descendants);
 for(let i=0;i<descendants.length;i++){
  for(const pid of children.get(descendants[i])||[]){
   if(!visited.has(pid)){visited.add(pid);descendants.push(pid);}
  }
 }
 const grids=[];
 for(const pid of descendants){
  let command='';
  try{command=readFileSync('/proc/'+pid+'/cmdline','utf8').replaceAll('\0',' ');}catch{}
  if(!/--grid-child\s+\d+/.test(command))continue;
  const index=Number(/--grid-child\s+(\d+)/.exec(command)[1]);
  let rss=null,pss=null;
  try{rss=kb(readFileSync('/proc/'+pid+'/status','utf8'),'VmRSS');}catch{}
  try{pss=kb(readFileSync('/proc/'+pid+'/smaps_rollup','utf8'),'Pss');}catch{}
  grids.push({index,pid,start:processes.get(pid)?.start||null,rss_kb:rss,pss_kb:pss});
 }
 return grids.sort((a,b)=>a.index-b.index);
}
const args=process.argv.slice(2);
if(args.includes('--self-test')){
 const x=parseStat('123 (a b) S 17 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 123456');
 if(!x||x.ppid!==17||x.start!=='123456')throw Error('GRID_PROC_STAT_INVALID');
 if(kb('VmRSS:\t123 kB\n','VmRSS')!==123)throw Error('GRID_RSS_PARSE_INVALID');
 console.log('GRID_RUNTIME_SAMPLER=PASS');
}else{
 const pos=args.indexOf('--pid'),pid=Number(pos>=0?args[pos+1]:0);
 const durationAt=args.indexOf('--seconds');
 const seconds=Math.min(1800,Math.max(1,Number(durationAt>=0?args[durationAt+1]:30)));
 if(!Number.isInteger(pid)||pid<1||!existsSync('/proc/'+pid)){
  console.log(JSON.stringify({schema:'browseros-grid-runtime-v1',classification:'UNKNOWN',reason:'Target PID required'}));
 }else{
  const last=new Map(),restart=new Map();
  for(let i=0;i<seconds;i++){
   const cells=collect(pid);
   for(const cell of cells){
    const previous=last.get(cell.index);
    if(previous && previous!==cell.pid+':'+cell.start)
     restart.set(cell.index,(restart.get(cell.index)||0)+1);
    last.set(cell.index,cell.pid+':'+cell.start);
   }
   console.log(JSON.stringify({schema:'browseros-grid-runtime-v1',classification:'MEASURED',
    timestamp:new Date().toISOString(),main_pid:pid,grid_processes:cells,
    observed_restarts:Object.fromEntries(restart)}));
   if(i+1<seconds)await sleep(1000);
  }
 }
}
