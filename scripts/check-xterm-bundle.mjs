#!/usr/bin/env node
// Differential tests against the exact installed vendor, not a reimplemented
// terminal or a mock parser. DOM rendering is separately verified by GUI gates.
import assert from 'node:assert/strict';
import { readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { compactXtermPrivateChunk, xtermPublicNames, preserveLiteralNames } from './compact-xterm-private-chunk.mjs';
const require=createRequire(import.meta.url);
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const names=xtermPublicNames();let assertions=0;
function equal(actual,expected){assert.deepEqual(actual,expected);assertions++;}
const sample=new Set();
preserveLiteralNames({type:'Program',body:[{type:'Literal',value:'reflectedName'},{type:'TemplateElement',value:{cooked:'dynamicTemplateName'}}]},sample);
equal([...sample].sort(),['dynamicTemplateName','reflectedName']);
equal(await compactXtermPrivateChunk().renderChunk('untouched',{name:'application'}),null);
equal(await compactXtermPrivateChunk().renderChunk('stub',{name:'xterm'}),null);
for(const name of ['write','buffer','active','getLine','getCell','translateToString','getFgColor','onData','onResize','registerCsiHandler','registerOscHandler','fit','proposeDimensions','_core']){equal(names.has(name),true);}
const assets=resolve(root,'dist/assets');const candidates=readdirSync(assets).filter(name=>/^xterm-[\w-]+\.js$/.test(name));
equal(candidates.length,1);
const compiled=await import(pathToFileURL(resolve(assets,candidates[0])).href);
const vendor=await import(pathToFileURL(resolve(dirname(require.resolve('@xterm/xterm')),'xterm.mjs')).href);
const addon=await import(pathToFileURL(resolve(dirname(require.resolve('@xterm/addon-fit')),'addon-fit.mjs')).href);
const CompactTerminal=Object.values(compiled).find(value=>typeof value==='function'&&value.prototype?.write&&value.prototype?.resize);
const CompactFit=Object.values(compiled).find(value=>typeof value==='function'&&value.prototype?.fit&&value.prototype?.proposeDimensions);
equal(typeof CompactTerminal,'function');equal(typeof CompactFit,'function');
function publicShape(instance){
 const shape=new Set();
 for(let value=instance;value&&value!==Object.prototype;value=Object.getPrototypeOf(value))for(const key of Object.getOwnPropertyNames(value))if(names.has(key)&&!key.startsWith('_'))shape.add(key);
 return [...shape].sort();
}
function snapshot(terminal){
 const buffer=terminal.buffer.active;const lines=[];
 for(let y=0;y<buffer.length;y++){
   const line=buffer.getLine(y);const cells=[];
   for(let x=0;x<line.length;x++){
     const cell=line.getCell(x);
     cells.push([cell.getChars(),cell.getWidth(),cell.getCode(),cell.getFgColor(),cell.getBgColor(),cell.isBold(),cell.isItalic(),cell.isUnderline(),cell.isInverse()]);
   }
   lines.push({text:line.translateToString(),wrapped:line.isWrapped,cells});
 }
 return {cols:terminal.cols,rows:terminal.rows,type:buffer.type,cursorX:buffer.cursorX,cursorY:buffer.cursorY,baseY:buffer.baseY,viewportY:buffer.viewportY,lines};
}
async function write(terminal,text){
 let timer;
 try{await Promise.race([new Promise(resolve=>terminal.write(text,resolve)),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('TERMINAL_WRITE_TIMEOUT')),3000);})]);}
 finally{clearTimeout(timer);}
}
async function exercise(Terminal,Fit){
 const terminal=new Terminal({cols:24,rows:6,scrollback:40,convertEol:true,allowProposedApi:true});
 const captures=[],events=[],disposables=[];let addonDisposed=0;
 try{
   captures.push({terminal:publicShape(terminal),buffer:publicShape(terminal.buffer),active:publicShape(terminal.buffer.active),parser:publicShape(terminal.parser),unicode:publicShape(terminal.unicode)});
   disposables.push(terminal.onData(data=>events.push(['data',data])));
   disposables.push(terminal.onResize(event=>events.push(['resize',event.cols,event.rows])));
   disposables.push(terminal.onBell(()=>events.push(['bell'])));
   disposables.push(terminal.parser.registerCsiHandler({final:'z'},parameters=>{events.push(['custom-csi',parameters]);return true;}));
   disposables.push(terminal.parser.registerOscHandler(777,text=>{events.push(['custom-osc',text]);return true;}));
   terminal.loadAddon({activate(owner){equal(owner,terminal);},dispose(){addonDisposed++;}});
   const fit=new Fit();terminal.loadAddon(fit);captures.push({fit:publicShape(fit),unopenedFit:fit.proposeDimensions()});
   for(const text of ['plain text\r\n','\x1b[31;1mred bold\x1b[0m normal\r\n','\u4e2d\u6587 wide \u{1f642} combining e\u0301\r\n','\x1b[2;3Hcursor\x1b[3D!\x1b[0K','\x1b[?1049halt screen\r\n\x1b[32mgreen\x1b[0m','\x1b[?1049l\x07\x1b[12;34z\x1b]777;sample\x07',Array.from({length:12},(_,i)=>'line '+i+'\r\n').join('')]){
     await write(terminal,text);captures.push(snapshot(terminal));
   }
   terminal.resize(18,8);captures.push(snapshot(terminal));
   terminal.input('typed input',true);
   terminal.options.fontSize=16;terminal.options.theme={foreground:'#eeeeee',background:'#111111',cursor:'#abcdef'};
   captures.push({fontSize:terminal.options.fontSize,theme:terminal.options.theme,unicode:terminal.unicode.activeVersion});
   terminal.scrollToTop();captures.push(snapshot(terminal));terminal.scrollToBottom();captures.push(snapshot(terminal));
   terminal.clear();captures.push(snapshot(terminal));terminal.reset();captures.push(snapshot(terminal));
   const line=terminal.buffer.active.getLine(0);captures.push({line:publicShape(line),cell:publicShape(line.getCell(0))});
   return {captures,events};
 }finally{for(const disposable of disposables)disposable.dispose();terminal.dispose();equal(addonDisposed,1);}
}
const baseline=await exercise(vendor.Terminal,addon.FitAddon);
const compacted=await exercise(CompactTerminal,CompactFit);
equal(compacted,baseline);
console.log(`XTERM_BUNDLE_DIFFERENTIAL=PASS assertions=${assertions} snapshots=${baseline.captures.length} public_api_and_real_parser=true DOM_RENDERING=GUI_GATE_REQUIRED`);
