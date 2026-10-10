#!/usr/bin/env node
// Real Vite dist evidence. Does not infer module install size or startup bytes.
import { readdirSync, readFileSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join, relative, resolve, dirname } from "node:path";
import { gzipSync } from "node:zlib";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
function scanDist(folder) {
  function walk(p) {
    return readdirSync(p,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(join(p,e.name)):[join(p,e.name)]);
  }
  const html=readFileSync(join(folder,"index.html"),"utf8");
  const initialRefs=new Set([...html.matchAll(/(?:src|href)=["']([^"']+)["']/g)].map(m=>m[1].replace(/^\//,"")));
  const assets=walk(folder).sort().map(file=>{
    const path=relative(folder,file).replaceAll("\\","/");
    const bytes=readFileSync(file);
    return {path,bytes:bytes.length,gzip_bytes:gzipSync(bytes,{level:9}).length,
      referenced_by_index_html:initialRefs.has(path),
      type:path.endsWith(".js")?"js":path.endsWith(".css")?"css":"other"};
  });
  return {schema:"browseros-dist-assets-v1",total_bytes:assets.reduce((n,x)=>n+x.bytes,0),
    js_total_bytes:assets.filter(x=>x.type==="js").reduce((n,x)=>n+x.bytes,0),
    initial_html_referenced_bytes:assets.filter(x=>x.referenced_by_index_html).reduce((n,x)=>n+x.bytes,0),
    assets,limitation:"Initial HTML references are not runtime network loading; shared chunks cannot be wholly attributed to a single capability. No .deb, RSS, PSS or CPU inference."};
}
if(process.argv.includes("--self-test")){
  const d=mkdtempSync(join(tmpdir(),"browseros-dist-fixture-"));
  try {
    mkdirSync(join(d,"assets"));
    writeFileSync(join(d,"index.html"),'<script src="/assets/a.js"></script><link href="/assets/b.css">');
    writeFileSync(join(d,"assets","a.js"),"test");
    writeFileSync(join(d,"assets","b.css"),"body{}");
    const report=scanDist(d),actual=readFileSync(join(d,"index.html")).length+4+6;
    if(report.total_bytes!==actual||report.assets.filter(x=>x.referenced_by_index_html).length!==2)throw Error("sample mismatch: "+JSON.stringify(report));
    console.log("BUILD_ASSETS_SELF_TEST=PASS");
  } finally { rmSync(d,{force:true,recursive:true}); }
} else {
  const dist=process.argv[2]?resolve(process.argv[2]):join(root,"dist");
  console.log(JSON.stringify(scanDist(dist),null,2));
}
