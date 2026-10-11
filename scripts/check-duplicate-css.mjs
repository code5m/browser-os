#!/usr/bin/env node
// Advisory CSS dedup inventory. Identical bodies are NOT proof that selectors
// can be merged without changing cascade specificity, scoping or order.
import { readdirSync,readFileSync } from "node:fs";
import { join,relative,resolve,dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
function walk(p){
  return readdirSync(p,{withFileTypes:true}).flatMap(x=>{
    if(x.isDirectory())return walk(join(p,x.name));
    if(!/\.(css|vue)$/.test(x.name))return [];
    return [join(p,x.name)];
  });
}
function collect(text){
  const stripped=text.replace(/\/\*[\s\S]*?\*\//g,"");
  return [...stripped.matchAll(/([^{}]+)\{([^{}]+)\}/g)].map(x=>({
    selector:x[1].trim().replace(/\s+/g," "),
    body:x[2].trim().replace(/\s+/g," "),
  })).filter(x=>x.body.includes(";")&&!x.selector.startsWith("@"));
}
function audit(sources){
  const bodies=new Map();
  for(const [file,source] of sources){
    for(const r of collect(source)){
      if(r.body.length<25)continue;
      const k=r.body;
      bodies.set(k,[...(bodies.get(k)||[]),{file,selector:r.selector}]);
    }
  }
  return [...bodies].filter(([,items])=>items.length>1)
    .map(([body,items])=>({declarations:body,occurrences:items}))
    .sort((a,b)=>b.occurrences.length-a.occurrences.length||b.declarations.length-a.declarations.length);
}
if(process.argv.includes("--self-test")){
  const f=audit([["a.css",".a{display: flex; color: red;} .b{display: flex; color: red;}"]]);
  assert.equal(f.length,1);assert.equal(f[0].occurrences.length,2);
  console.log("CSS_DUPLICATE_INVENTORY_SELF_TEST=PASS");
}else{
  const files=[...walk(join(root,"src")),...walk(join(root,"packages"))];
  const data=audit(files.map(file=>[relative(root,file),readFileSync(file,"utf8")]));
  console.log(JSON.stringify({schema:"browseros-css-duplicate-inventory-v1",scanned_files:files.length,
    candidates:data,advisory:"Exact text matches only; CSS cascade and scoped styles must be visual-regression tested before deletion."},null,2));
}
