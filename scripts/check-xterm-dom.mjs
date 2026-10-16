#!/usr/bin/env node
// Real Chromium DOM comparison: original locked vendor vs shipped terminal chunk.
// Additional coverage; this does not replace packaged WebKit/Tauri GUI checks.
import {mkdtempSync,readFileSync,readdirSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';import {join,resolve,dirname} from 'node:path';
import {createRequire} from 'node:module';import {fileURLToPath} from 'node:url';
import {spawn,spawnSync} from 'node:child_process';
const require=createRequire(import.meta.url),root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const browser=process.env.CHROME_BIN||['chromium','google-chrome','google-chrome-stable','google-chrome-browser'].map(name=>spawnSync('which',[name],{encoding:'utf8'})).find(r=>r.status===0)?.stdout.trim();
if(!browser)throw Error('XTERM_DOM_BROWSER_REQUIRED: install Chromium/Chrome or set CHROME_BIN');
const terminal=resolve(dirname(require.resolve('@xterm/xterm')),'xterm.mjs');
const addon=resolve(dirname(require.resolve('@xterm/addon-fit')),'addon-fit.mjs');
const css=resolve(dirname(terminal),'../css/xterm.css');
const chunks=readdirSync(join(root,'dist/assets')).filter(n=>/^xterm-[\w-]+\.js$/.test(n));
if(chunks.length!==1)throw Error('XTERM_DOM_EXACTLY_ONE_VENDOR_CHUNK_REQUIRED');
const optimized=join(root,'dist/assets',chunks[0]);
const url=p=>JSON.stringify('data:text/javascript;base64,'+Buffer.from(readFileSync(p,'utf8')).toString('base64'));
const stylesheet=readFileSync(css,'utf8');
const temp=mkdtempSync(join(tmpdir(),'browseros-terminal-dom-'));
try{
 const html=`<!doctype html><meta charset="utf-8"><style>${stylesheet}</style><style>body{margin:20px}.fixture{width:720px;height:220px;position:relative;margin-bottom:16px}#result{white-space:pre-wrap}</style><div id="original" class="fixture"></div><div id="optimized" class="fixture"></div><pre id="result">PENDING</pre><script type="module">
 const output=document.getElementById('result');
 try {
  const [{Terminal},{FitAddon},compact]=await Promise.all([import(${url(terminal)}),import(${url(addon)}),import(${url(optimized)})]);
  const CompactTerminal=Object.values(compact).find(value=>typeof value==='function'&&value.prototype?.write&&value.prototype?.resize);
  const CompactFit=Object.values(compact).find(value=>typeof value==='function'&&value.prototype?.fit&&value.prototype?.proposeDimensions);
  const pause=()=>new Promise(resolve=>setTimeout(resolve,80));
  async function exercise(T,F,id){
   const host=document.getElementById(id),t=new T({cursorBlink:false,fontSize:14,scrollback:30,allowProposedApi:true});
   const fit=new F();t.loadAddon(fit);const events=[];const subscription=t.onData(data=>events.push(data));
   const snapshots=[];
   const capture=()=>({cols:t.cols,rows:t.rows,selection:t.getSelection(),lines:Array.from({length:t.buffer.active.length},(_,i)=>t.buffer.active.getLine(i).translateToString()),text:host.querySelector('.xterm-rows')?.textContent||'',spans:Array.from(host.querySelectorAll('.xterm-rows span')).map(span=>[span.textContent,getComputedStyle(span).color,getComputedStyle(span).backgroundColor]),dimensions:fit.proposeDimensions()});
   t.open(host);fit.fit();await pause();snapshots.push(capture());
   const text='hello terminal\\r\\n\\x1b[31;1mred bold\\x1b[0m \\u4e2d\\u6587 \\u{1f642}\\r\\n'+Array.from({length:18},(_,i)=>'row '+i+'\\r\\n').join('');
   await new Promise(resolve=>t.write(text,resolve));await pause();snapshots.push(capture());
   t.select(0,0,5);t.paste('paste\\nvalue');await pause();snapshots.push(capture());
   host.style.width='560px';fit.fit();t.scrollToTop();await pause();snapshots.push(capture());
   t.scrollToBottom();t.options.theme={foreground:'#eeeeee',background:'#111111',cursor:'#abcdef'};t.focus();await pause();snapshots.push(capture());
   t.textarea.dispatchEvent(new KeyboardEvent('keydown',{key:'a',code:'KeyA',keyCode:65,which:65,bubbles:true}));
   subscription.dispose();t.dispose();return {snapshots,events,disposed:host.querySelector('.xterm')===null};
  }
  const baseline=await exercise(Terminal,FitAddon,'original');const actual=await exercise(CompactTerminal,CompactFit,'optimized');
  if(JSON.stringify(baseline)!==JSON.stringify(actual))throw Error('Differential mismatch '+JSON.stringify({baseline,actual}));
  if(!actual.disposed||!actual.events.some(text=>text.includes('paste')))throw Error('DOM behavior was not exercised');
  output.textContent='XTERM_DOM_DIFFERENTIAL=PASS '+JSON.stringify({snapshots:actual.snapshots.length,paste:true,resize:true,selection:true,theme:true,disposed:actual.disposed});
 }catch(error){output.textContent='XTERM_DOM_DIFFERENTIAL=FAIL '+String(error.stack||error);}
 </script>`;
 const child=spawn(browser,['--headless','--no-sandbox','--disable-gpu','--disable-dev-shm-usage','--disable-background-networking','--no-first-run',`--user-data-dir=${join(temp,'profile')}`,'--remote-debugging-pipe'],{stdio:['ignore','ignore','pipe','pipe','pipe'],detached:process.platform!=='win32'});
 const pending=new Map();let sequence=0,buffer='',stderr='';
 child.stderr.on('data',chunk=>{stderr=(stderr+chunk.toString()).slice(-4000);});
 child.stdio[4].on('data',chunk=>{
  buffer+=chunk.toString();let end;
  while((end=buffer.indexOf('\0'))>=0){const raw=buffer.slice(0,end);buffer=buffer.slice(end+1);if(!raw)continue;const reply=JSON.parse(raw),waiter=pending.get(reply.id);if(waiter){pending.delete(reply.id);clearTimeout(waiter.timer);reply.error?waiter.reject(Error(JSON.stringify(reply.error))):waiter.resolve(reply.result);}}
 });
 const fail=error=>{for(const waiter of pending.values()){clearTimeout(waiter.timer);waiter.reject(error);}pending.clear();};
 child.on('error',fail);child.on('exit',()=>fail(Error('XTERM_DOM_BROWSER_EXIT '+stderr)));
 function call(method,params={},sessionId){
  return new Promise((resolve,reject)=>{
   const id=++sequence,timer=setTimeout(()=>{pending.delete(id);reject(Error('XTERM_DOM_CDP_TIMEOUT '+method+' '+stderr));},20000);
   pending.set(id,{resolve,reject,timer});child.stdio[3].write(JSON.stringify({id,method,params,...sessionId?{sessionId}:{}})+'\0');
  });
 }
 let report='';
 try{
  const target=await call('Target.createTarget',{url:'about:blank'});
  const attached=await call('Target.attachToTarget',{targetId:target.targetId,flatten:true});
  const session=attached.sessionId;await call('Page.enable',{},session);
  const frame=await call('Page.getFrameTree',{},session);
  await call('Page.setDocumentContent',{frameId:frame.frameTree.frame.id,html},session);
  const deadline=Date.now()+20000;
  while(Date.now()<deadline){
   const evaluated=await call('Runtime.evaluate',{expression:"document.getElementById('result')?.textContent || ''",returnByValue:true},session);
   report=evaluated.result?.value||'';
   if(report.startsWith('XTERM_DOM_DIFFERENTIAL='))break;
   await new Promise(resolve=>setTimeout(resolve,100));
  }
  if(!report.startsWith('XTERM_DOM_DIFFERENTIAL=PASS'))throw Error(report.slice(0,15000)||stderr||'XTERM_DOM_NO_REPORT');
 }finally{
  fail(Error('XTERM_DOM_TEST_CLOSED'));
  try{if(process.platform!=='win32')process.kill(-child.pid,'SIGKILL');else child.kill('SIGKILL');}catch{}
 }
 console.log(report);
}finally{rmSync(temp,{recursive:true,force:true});}
