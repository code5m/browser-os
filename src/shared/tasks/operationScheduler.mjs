// Bounded shared scheduling for long-running BrowserOS native operations.
// This coordinates work without owning a domain's connection, PTY, SQL transaction
// or Git write approval. Cancellation/confirmation commands bypass this queue.
export function createOperationScheduler({limits={database:2,git:2,terminal:2,ai:2},queueLimit=96}={}){
 const lanes=new Map(Object.entries(limits).map(([name,limit])=>{
  if(!Number.isInteger(limit)||limit<1||limit>16)throw Error('SCHEDULER_LANE_CONFIG_INVALID');
  return [name,{limit,active:0,queue:[],scopes:new Set(),latencies:[],completed:0,rejected:0}];
 }));
 let closed=false;
 function record(lane,ms){
  lane.completed++;lane.latencies.push(ms);
  if(lane.latencies.length>256)lane.latencies.shift();
 }
 function pump(lane){
  if(closed)return;
  while(lane.active<lane.limit){
   const index=lane.queue.findIndex(item=>!lane.scopes.has(item.scope));
   if(index<0)break;
   const item=lane.queue.splice(index,1)[0];
   lane.active++;lane.scopes.add(item.scope);
   const start=Date.now();
   Promise.resolve().then(item.work).then(item.resolve,item.reject).finally(()=>{
    record(lane,Math.max(0,Date.now()-start));
    lane.scopes.delete(item.scope);lane.active--;pump(lane);
   });
  }
 }
 function run(domain,scope,work){
  const lane=lanes.get(domain);
  if(closed||!lane) return Promise.reject(Error('SCHEDULER_UNAVAILABLE'));
  if(typeof scope!=='string'||!scope||typeof work!=='function')return Promise.reject(Error('SCHEDULER_INVALID_REQUEST'));
  if(lane.queue.length>=queueLimit){
    lane.rejected++;return Promise.reject(Error('SCHEDULER_BACKPRESSURE'));
  }
  return new Promise((resolve,reject)=>{
    lane.queue.push({scope,work,resolve,reject});pump(lane);
  });
 }
 function inspect(){
  return [...lanes].map(([name,lane])=>{
   const samples=[...lane.latencies].sort((a,b)=>a-b);
   const q=p=>samples.length?samples[Math.floor((samples.length-1)*p/100)]:null;
   return {name,limit:lane.limit,active:lane.active,pending:lane.queue.length,
     completed:lane.completed,rejected:lane.rejected,p95_ms:q(95),p99_ms:q(99)};
  });
 }
 return Object.freeze({run,inspect,dispose(){
  closed=true;
  for(const lane of lanes.values()){
   for(const item of lane.queue)item.reject(Error('SCHEDULER_DISPOSED'));
   lane.queue.length=0;
  }
 }});
}
export const sharedOperationScheduler=createOperationScheduler();
