#!/usr/bin/env node
// Offline signed DATA resource package carrier. Never runs/imports plugin code.
// PLUGIN_NO_EXEC_SURFACE and native Plugin Stage-I remain unchanged.
import assert from "node:assert/strict";
import { closeSync,constants,existsSync,fstatSync,fsyncSync,lstatSync,mkdirSync,mkdtempSync,openSync,readFileSync,renameSync,rmSync,symlinkSync,unlinkSync,writeFileSync } from "node:fs";
import { dirname,join,resolve,sep } from "node:path";
import { tmpdir } from "node:os";
import { createHash,createPublicKey,generateKeyPairSync,sign,verify } from "node:crypto";

const S="browseros-signed-data-v1",STATE="browseros-signed-data-state-v1",MAX=4*1024*1024;
const ID=/^[a-z][a-z0-9]*(?:\.[a-z][a-z0-9-]*)+$/;
const VER=/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const HASH=/^[a-f0-9]{64}$/;
const EXT=/\.(?:md|txt|json|csv|png|jpg|jpeg|webp)$/;
const fail=(code)=>{throw Error(code);};
const hash=b=>createHash("sha256").update(b).digest("hex");
function canonical(v){
  if(Array.isArray(v))return "["+v.map(canonical).join(",")+"]";
  if(v&&typeof v==="object")return "{"+Object.keys(v).sort().map(k=>JSON.stringify(k)+":"+canonical(v[k])).join(",")+"}";
  return JSON.stringify(v);
}
function shape(o,keys,code){
  if(!o||typeof o!=="object"||Array.isArray(o)||Object.keys(o).sort().join(",")!==keys.slice().sort().join(","))fail(code);
}
const validId=s=>typeof s==="string"&&s.length<=96&&ID.test(s);
function validPath(p){
  if(typeof p!=="string"||p.length>180||!p.startsWith("assets/")||!/^[a-z0-9._/-]+$/.test(p)||!EXT.test(p)||
     p.split("/").some(x=>!x||x==="."||x===".."||x.startsWith(".")))fail("PACK_INVALID_PATH");
}
function manifest(m){
  shape(m,["schema","id","version","kind","host_api","dependencies","files","signature"],"PACK_SCHEMA");
  if(m.schema!==S||m.kind!=="data-only"||m.host_api!=="1"||!validId(m.id)||!VER.test(m.version))fail("PACK_UNSUPPORTED");
  if(!Array.isArray(m.files)||!m.files.length||m.files.length>32||!Array.isArray(m.dependencies)||m.dependencies.length>16)fail("PACK_LIMIT");
  for(const d of m.dependencies){shape(d,["id","version"],"PACK_DEPENDENCY_SCHEMA");if(!validId(d.id)||!VER.test(d.version)||d.id===m.id)fail("PACK_DEPENDENCY");}
  if(new Set(m.dependencies.map(x=>x.id)).size!==m.dependencies.length)fail("PACK_DUPLICATE_DEPENDENCY");
  let bytes=0;
  for(const f of m.files){
    shape(f,["path","bytes","sha256"],"PACK_FILE_SCHEMA");validPath(f.path);
    if(!Number.isSafeInteger(f.bytes)||f.bytes<0||f.bytes>MAX||typeof f.sha256!=="string"||!HASH.test(f.sha256))fail("PACK_FILE_LIMIT");
    bytes+=f.bytes;
  }
  if(bytes>16*1024*1024||new Set(m.files.map(f=>f.path)).size!==m.files.length||m.files.some((f,i,a)=>i>0&&f.path<=a[i-1].path))fail("PACK_FILE_ORDER_OR_QUOTA");
  shape(m.signature,["algorithm","key_id","value"],"PACK_SIGNATURE_SCHEMA");
  if(m.signature.algorithm!=="Ed25519"||typeof m.signature.key_id!=="string"||!/^[a-zA-Z0-9_-]{1,64}$/.test(m.signature.key_id))fail("PACK_SIGNATURE_FORMAT");
  return m;
}
function signedBytes(m){const {signature,...body}=m;return Buffer.from(S+"\n"+canonical(body),"utf8");}
function strict64(value){
  if(typeof value!=="string"||!/^[A-Za-z0-9+/]+={0,2}$/.test(value))fail("PACK_SIGNATURE_FORMAT");
  const b=Buffer.from(value,"base64");
  if(b.length!==64||b.toString("base64")!==value)fail("PACK_SIGNATURE_FORMAT");
  return b;
}
function verifySignature(m,trust){
  manifest(m);shape(trust,["schema","keys"],"PACK_TRUST_SCHEMA");
  if(trust.schema!=="browseros-trusted-keys-v1"||!trust.keys||typeof trust.keys!=="object"||Array.isArray(trust.keys))fail("PACK_TRUST_SCHEMA");
  if(!Object.hasOwn(trust.keys,m.signature.key_id))fail("PACK_UNKNOWN_KEY");
  const k=trust.keys[m.signature.key_id];shape(k,["spki_base64","revoked"],"PACK_KEY_SCHEMA");
  if(k.revoked!==false)fail("PACK_KEY_REVOKED");
  let pk;
  try{
    const bytes=Buffer.from(k.spki_base64,"base64");
    if(bytes.toString("base64")!==k.spki_base64)fail("PACK_BAD_KEY");
    pk=createPublicKey({key:bytes,format:"der",type:"spki"});
    if(pk.asymmetricKeyType!=="ed25519")fail("PACK_BAD_KEY");
  }catch{fail("PACK_BAD_KEY");}
  if(!verify(null,signedBytes(m),pk,strict64(m.signature.value)))fail("PACK_SIGNATURE_INVALID");
}
function regular(path,limit){
  const s=lstatSync(path);
  if(!s.isFile()||s.isSymbolicLink()||s.size>limit||s.nlink!==1)fail("PACK_UNSAFE_FILE");
  const fd=openSync(path,constants.O_RDONLY|(constants.O_NOFOLLOW||0));
  try{
    const t=fstatSync(fd);
    if(!t.isFile()||t.size>limit||t.nlink!==1||s.dev!==t.dev||s.ino!==t.ino)fail("PACK_UNSAFE_FILE");
    return readFileSync(fd);
  }finally{closeSync(fd);}
}
function asset(root,path){
  validPath(path);
  const parts=path.split("/");
  let loc=root;
  for(const p of parts.slice(0,-1)){
    loc=join(loc,p);const s=lstatSync(loc);
    if(!s.isDirectory()||s.isSymbolicLink())fail("PACK_SYMLINK");
  }
  return regular(join(loc,parts.at(-1)),MAX);
}
function json(path,size=65536){try{return JSON.parse(regular(path,size).toString("utf8"));}catch(e){if(e.message?.startsWith("PACK_"))throw e;fail("PACK_INVALID_JSON");}}
function readSource(dir,trust){
  const s=lstatSync(dir);
  if(!s.isDirectory()||s.isSymbolicLink())fail("PACK_UNSAFE_SOURCE");
  const m=manifest(json(join(dir,"manifest.json")));verifySignature(m,trust);
  const files=m.files.map(f=>{
    const content=asset(dir,f.path);
    if(content.length!==f.bytes||hash(content)!==f.sha256)fail("PACK_PAYLOAD_MISMATCH");
    return {path:f.path,content};
  });
  return {m,files};
}
function verifiedDirectory(parent,component,create=false){
  const path=join(parent,component);
  if(create&&!existsSync(path))mkdirSync(path,{mode:0o700});
  const st=lstatSync(path);
  if(!st.isDirectory()||st.isSymbolicLink())fail("PACK_STORE_SYMLINK");
  return path;
}
function storeDir(path){
  const abs=resolve(path);
  if(abs===resolve(sep))fail("PACK_STORE_ROOT");
  // Never call mkdir recursive before checking parents: that could follow an
  // attacker-controlled symlink and create files OUTSIDE the intended store.
  let dir=resolve(sep);
  for(const p of abs.split(sep).filter(Boolean))dir=verifiedDirectory(dir,p,true);
  return abs;
}
function stateOf(dir){
  const p=join(dir,"state.json");
  if(!existsSync(p))return {schema:STATE,current:{},history:{}};
  const s=json(p,131072);shape(s,["schema","current","history"],"PACK_STATE_SCHEMA");
  if(s.schema!==STATE||!s.current||typeof s.current!=="object"||Array.isArray(s.current)||!s.history||typeof s.history!=="object"||Array.isArray(s.history))fail("PACK_STATE_SCHEMA");
  return s;
}
function atomic(dir,s){
  const p=join(dir,".state-"+process.pid+"-"+Math.random().toString(16).slice(2));
  const fd=openSync(p,constants.O_CREAT|constants.O_EXCL|constants.O_WRONLY,0o600);
  try{writeFileSync(fd,JSON.stringify(s,null,2)+"\n");fsyncSync(fd);}finally{closeSync(fd);}
  renameSync(p,join(dir,"state.json"));
}
function locked(dir,fn){
  const root=storeDir(dir),p=join(root,".lock");
  let fd;try{fd=openSync(p,constants.O_CREAT|constants.O_EXCL|constants.O_WRONLY,0o600);}catch{fail("PACK_STORE_BUSY");}
  try{return fn(root);}finally{closeSync(fd);unlinkSync(p);}
}
function installed(root,id,digest,trust){
  if(!validId(id)||typeof digest!=="string"||!HASH.test(digest))fail("PACK_INVALID_STATE");
  const loc=verifiedDirectory(verifiedDirectory(verifiedDirectory(root,"packages"),id),digest);
  const m=manifest(json(join(loc,"manifest.json")));
  if(m.id!==id||hash(Buffer.from(canonical(m)))!==digest)fail("PACK_RECORD_MISMATCH");
  verifySignature(m,trust);
  for(const f of m.files){const data=asset(loc,f.path);if(data.length!==f.bytes||hash(data)!==f.sha256)fail("PACK_RECORD_TAMPER");}
  return m;
}
function checkGraph(root,state,trust){
  const all=new Map();
  for(const [id,digest] of Object.entries(state.current))all.set(id,installed(root,id,digest,trust));
  for(const m of all.values())for(const dep of m.dependencies)
    if(all.get(dep.id)?.version!==dep.version)fail("PACK_DEPENDENCY_UNSATISFIED");
}
function install(root,source,trust){
  const {m,files}=readSource(source,trust);
  return locked(root,dir=>{
    const digest=hash(Buffer.from(canonical(m))),home=join(dir,"packages",m.id,digest),state=stateOf(dir);
    const parent=verifiedDirectory(verifiedDirectory(dir,"packages",true),m.id,true);
    if(!existsSync(home)){
      const stage=mkdtempSync(join(dir,".staging-"));
      try{
        writeFileSync(join(stage,"manifest.json"),JSON.stringify(m,null,2)+"\n",{flag:"wx",mode:0o600});
        for(const f of files){const p=join(stage,f.path);mkdirSync(dirname(p),{recursive:true,mode:0o700});writeFileSync(p,f.content,{flag:"wx",mode:0o600});}
        // Protect destination from a racy replacement of its parent.
        verifiedDirectory(verifiedDirectory(dir,"packages"),m.id);
        renameSync(stage,home);
      }finally{if(existsSync(stage))rmSync(stage,{recursive:true,force:true});}
    }
    installed(dir,m.id,digest,trust);
    if(state.current[m.id]===digest)return {id:m.id,version:m.version,status:"already-installed",execution:"FORBIDDEN"};
    const was=state.current[m.id]??null;
    state.current[m.id]=digest;
    checkGraph(dir,state,trust);
    (state.history[m.id]??=[]).push(was);
    atomic(dir,state);
    return {id:m.id,version:m.version,digest,status:"signed-data-installed",execution:"FORBIDDEN"};
  });
}
function uninstall(root,id,trust){
  if(!validId(id))fail("PACK_BAD_ID");
  return locked(root,dir=>{
    const state=stateOf(dir),previous=state.current[id];
    if(!previous)fail("PACK_NOT_INSTALLED");
    delete state.current[id];checkGraph(dir,state,trust);
    (state.history[id]??=[]).push(previous);
    atomic(dir,state);
    return {id,status:"data-pointer-uninstalled",rollback_preserved:true,execution:"FORBIDDEN"};
  });
}
function rollback(root,id,trust){
  if(!validId(id))fail("PACK_BAD_ID");
  return locked(root,dir=>{
    const state=stateOf(dir),hist=state.history[id];
    if(!Array.isArray(hist)||!hist.length)fail("PACK_NO_HISTORY");
    const previous=hist.at(-1);
    if(previous===null)delete state.current[id];else state.current[id]=previous;
    checkGraph(dir,state,trust);hist.pop();atomic(dir,state);
    return {id,status:"data-pointer-rolled-back",execution:"FORBIDDEN"};
  });
}
function list(root,trust){
  const dir=storeDir(root),state=stateOf(dir);checkGraph(dir,state,trust);
  return Object.entries(state.current).sort(([a],[b])=>a.localeCompare(b)).map(([id,digest])=>{
    const m=installed(dir,id,digest,trust);
    return {id,version:m.version,sha256:digest,verified:true,asset_bytes:m.files.reduce((n,f)=>n+f.bytes,0),runtime_loaded:false,execution:"FORBIDDEN"};
  });
}
function selfTest(){
  const temp=mkdtempSync(join(tmpdir(),"browseros-pack-test-")),root=join(temp,"store"),source=join(temp,"package"),file=join(source,"assets","guide.md");
  const {publicKey,privateKey}=generateKeyPairSync("ed25519");
  const trust={schema:"browseros-trusted-keys-v1",keys:{test:{spki_base64:publicKey.export({format:"der",type:"spki"}).toString("base64"),revoked:false}}};
  const expect=(code,fn)=>assert.throws(fn,e=>e.message===code);
  function make(version="1.0.0",deps=[]){
    mkdirSync(dirname(file),{recursive:true});const b=Buffer.from("BrowserOS signed docs "+version);writeFileSync(file,b);
    const m={schema:S,id:"org.example.guide",version,kind:"data-only",host_api:"1",dependencies:deps,files:[{path:"assets/guide.md",bytes:b.length,sha256:hash(b)}],signature:{algorithm:"Ed25519",key_id:"test",value:""}};
    m.signature.value=sign(null,signedBytes(m),privateKey).toString("base64");
    writeFileSync(join(source,"manifest.json"),JSON.stringify(m));return m;
  }
  try{
    make();assert.equal(install(root,source,trust).status,"signed-data-installed");
    assert.equal(install(root,source,trust).status,"already-installed");
    assert.equal(list(root,trust)[0].runtime_loaded,false);
    writeFileSync(file,"tamper");expect("PACK_PAYLOAD_MISMATCH",()=>install(root,source,trust));
    make("2.0.0");install(root,source,trust);assert.equal(list(root,trust)[0].version,"2.0.0");
    rollback(root,"org.example.guide",trust);assert.equal(list(root,trust)[0].version,"1.0.0");
    uninstall(root,"org.example.guide",trust);assert.equal(list(root,trust).length,0);
    rollback(root,"org.example.guide",trust);assert.equal(list(root,trust).length,1);
    const bad=make("3.0.0");bad.files[0].path="assets/../escape.md";expect("PACK_INVALID_PATH",()=>manifest(bad));
    make("3.0.0");rmSync(file);writeFileSync(join(temp,"outside.md"),"x");symlinkSync(join(temp,"outside.md"),file);
    expect("PACK_UNSAFE_FILE",()=>install(root,source,trust));unlinkSync(file);make("3.0.0");
    const changed=json(join(source,"manifest.json"));changed.files[0].bytes++;writeFileSync(join(source,"manifest.json"),JSON.stringify(changed));
    expect("PACK_SIGNATURE_INVALID",()=>install(root,source,trust));
    make("3.0.0");const revoked=structuredClone(trust);revoked.keys.test.revoked=true;
    expect("PACK_KEY_REVOKED",()=>install(root,source,revoked));
    expect("PACK_UNKNOWN_KEY",()=>install(root,source,{schema:"browseros-trusted-keys-v1",keys:{}}));
    const js=make("3.0.0");js.files[0].path="assets/run.js";expect("PACK_INVALID_PATH",()=>manifest(js));
    make("3.0.0",[{id:"org.missing.dep",version:"1.0.0"}]);expect("PACK_DEPENDENCY_UNSATISFIED",()=>install(root,source,trust));
    const row=list(root,trust)[0];writeFileSync(join(root,"packages",row.id,row.sha256,"assets","guide.md"),"tampered");
    expect("PACK_RECORD_TAMPER",()=>list(root,trust));
    // The storage root may not follow symlinks, including parent components
    // that did not previously exist (mkdir recursive would escape first).
    const outside=join(temp,"outside-dir"),redirect=join(temp,"redirect");
    mkdirSync(outside);symlinkSync(outside,redirect,"dir");
    expect("PACK_STORE_SYMLINK",()=>list(join(redirect,"evil-store"),trust));
    assert.equal(existsSync(join(outside,"evil-store")),false);
    console.log("SIGNED_RESOURCE_PACKAGE_SELF_TEST=PASS");
  }finally{rmSync(temp,{force:true,recursive:true});}
}
async function main(){
  if(process.argv.includes("--self-test")){selfTest();return;}
  const args=process.argv.slice(2),op=args[0],get=k=>{const i=args.indexOf(k);return i<0?null:args[i+1];};
  const root=get("--store"),trustPath=get("--trust");
  if(!root||!trustPath)fail("PACK_USAGE_STORE_TRUST_REQUIRED");
  const trust=json(trustPath,131072);
  let result;
  if(op==="install"){if(!get("--source"))fail("PACK_USAGE_SOURCE_REQUIRED");result=install(root,resolve(get("--source")),trust);}
  else if(op==="uninstall")result=uninstall(root,get("--id"),trust);
  else if(op==="rollback")result=rollback(root,get("--id"),trust);
  else if(op==="list")result=list(root,trust);
  else fail("PACK_USAGE_COMMAND");
  console.log(JSON.stringify(result,null,2));
}
main().catch(e=>{console.error("SIGNED_RESOURCE_PACKAGE_ERROR="+(e.message?.startsWith("PACK_")?e.message:"PACK_OPERATION_FAILED"));process.exitCode=1;});
