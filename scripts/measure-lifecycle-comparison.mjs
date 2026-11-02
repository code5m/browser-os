#!/usr/bin/env node
// Honest same-machine baseline vs current for TS Capability orchestration only.
// This NEVER substitutes for installed-GUI, process-tree CPU/RSS/PSS or 5/30m idle checks.
import { execFileSync, spawnSync } from 'node:child_process'
import { transformSync } from 'esbuild'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { cpus, arch, release, totalmem } from 'node:os'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { performance } from 'node:perf_hooks'
const ROOT=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const BASE='dd32cc828e21f923e61aca0ed66b647e8c957e41'
const args=process.argv.slice(2)
const get=(flag,defaultValue)=>{const i=args.indexOf(flag); return i>=0?args[i+1]:defaultValue}
function percentile(values,q){const n=[...values].sort((a,b)=>a-b);return n[Math.max(0,Math.ceil(q*n.length)-1)]}
function statPss(){try {const m=/^Pss:\s+(\d+) kB/m.exec(readFileSync('/proc/self/smaps_rollup','utf8'));return m?Number(m[1]):null}catch{return null}}
function cap(){return {id:'perf',name:'Perf',category:'CAPABILITY',provides:[],dependsOn:[],optionalDependencies:[],lifecycle:{supported:['ACTIVE','SUSPENDED'],default:'ACTIVE',activatable:true,resident:false,onActivate:()=>{},onSuspend:()=>{}},resources:{class:['LIGHT'],suspendable:true,destroyable:true},permissions:[],persistence:{scope:'runtime_only',sensitive:false},entrypoint:'perf',semanticOwner:'perf',governanceStatus:'GOVERNED',status:'COMPATIBILITY_WRAPPED'}}
async function worker(version,iterations,samples){
  const source=version==='baseline'?execFileSync('git',['show',`${BASE}:src/capability/runtime.ts`],{cwd:ROOT,encoding:'utf8',maxBuffer:5_000_000}):readFileSync(resolve(ROOT,'src/capability/runtime.ts'),'utf8')
  const js=transformSync(source,{loader:'ts',format:'esm',target:'es2020'}).code
  const {createCapabilityRuntime}=await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'))
  const runtime=createCapabilityRuntime();runtime.register(cap());runtime.resolve('perf')
  async function batch(){for(let i=0;i<iterations;i++){await runtime.activateAsync('perf');await runtime.suspendAsync('perf')}}
  await batch(); // same warm-up for both versions
  const durations=[], cpuUsages=[], rssValues=[], pssValues=[]
  for(let k=0;k<samples;k++){
    const cpu=process.cpuUsage(),t=performance.now();await batch();const ms=performance.now()-t
    const used=process.cpuUsage(cpu)
    durations.push(ms/iterations);cpuUsages.push((used.user+used.system)/1000/iterations)
    rssValues.push(process.memoryUsage().rss/1024)
    const pss=statPss();if(pss!=null)pssValues.push(pss)
  }
  const result={version,iterations,samples,operation:'activateAsync+suspendAsync (Owner hooks no-op)',ms_per_cycle:{median:percentile(durations,.5),p95:percentile(durations,.95)},cpu_ms_per_cycle:{median:percentile(cpuUsages,.5),p95:percentile(cpuUsages,.95)},rss_kb:{median:percentile(rssValues,.5),p95:percentile(rssValues,.95)},pss_kb:pssValues.length?{median:percentile(pssValues,.5),p95:percentile(pssValues,.95)}:null}
  process.stdout.write(JSON.stringify(result))
}
if(args.includes('--self-test')) {
  if(percentile([4,2,1,3],.5)!==2)throw Error('median fixture mismatch')
  if(percentile([4,2,1,3],.95)!==4)throw Error('p95 fixture mismatch')
  process.stdout.write('LIFECYCLE_COMPARISON_SELF_TEST=PASS\n')
} else if(args.includes('--worker')) {
  await worker(get('--worker','current'),Number(get('--iterations',1500)),Number(get('--samples',6)))
} else {
  const iterations=Number(get('--iterations',1500)),samples=Number(get('--samples',6))
  const out=resolve(ROOT,get('--out','artifacts/perf/lifecycle-comparison-20261102.json'))
  if(!Number.isInteger(iterations)||iterations<100||iterations>20_000||!Number.isInteger(samples)||samples<4||samples>12)throw Error('Invalid benchmark bounds')
  const measurements={}
  for(const version of ['current','baseline']) {
    const child=spawnSync(process.execPath,[fileURLToPath(import.meta.url),'--worker',version,'--iterations',String(iterations),'--samples',String(samples)],{cwd:ROOT,encoding:'utf8',timeout:45000,maxBuffer:1_000_000})
    if(child.status!==0)throw Error(`${version} benchmark failed: ${child.stderr}`)
    measurements[version]=JSON.parse(child.stdout)
  }
  const ratio=(m)=>measurements.current[m].median/measurements.baseline[m].median
  const result={schema:'browseros-lifecycle-headless-compare-v1',classification:'MEASURED_HEADLESS_ONLY',baseline_sha:BASE,current_worktree:true,scenario:measurements.current.operation,measurements,ratios:{latency:ratio('ms_per_cycle'),cpu:ratio('cpu_ms_per_cycle')},same_machine:{os_release:release(),arch:arch(),cpu_model:cpus()[0]?.model,totalmem_bytes:totalmem(),node:process.version},limitations:'No real BrowserOS GUI, native PTY/WebView/DB, startup ready, or application process-tree RSS/PSS. Memory median is benchmark Node worker RSS/PSS, not BrowserOS. Do not treat as production perf evidence.'}
  mkdirSync(dirname(out),{recursive:true});writeFileSync(out,JSON.stringify(result,null,2)+'\n')
  console.log(`LIFECYCLE_HEADLESS_COMPARISON=MEASURED baseline=${result.baseline_sha.slice(0,8)} latency_ratio=${result.ratios.latency.toFixed(2)} cpu_ratio=${result.ratios.cpu.toFixed(2)} out=${out}`)
}
