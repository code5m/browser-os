#!/usr/bin/env node
// BrowserOS dependency evidence: advisory inventory, NEVER fake installed sizes.
// First-party relative imports only; no attempt to infer byte attribution.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
function walk(dir) {
  return readdirSync(dir,{withFileTypes:true}).flatMap(ent=>{
    if(ent.name.startsWith(".")||["node_modules","dist","target"].includes(ent.name))return [];
    const p=join(dir,ent.name);
    return ent.isDirectory()?walk(p):/\.(ts|vue|mjs)$/.test(ent.name)?[p]:[];
  });
}
function owner(path) {
  const normalized=path.split("\\").join("/");
  const m=/^src\/capabilities\/([^/]+)\//.exec(normalized);
  if(m)return "capability:"+m[1];
  if(normalized.startsWith("src/shared/pure/"))return "shared:pure";
  if(normalized.startsWith("src/utils/"))return "legacy:utils";
  return "other";
}
function imports(text) {
  const out=[];
  const staticRx=/(?:import|export)\s+(?:[\s\S]*?\s+from\s+)?["']([^"']+)["']/g;
  const dynamicRx=/\bimport\s*\(\s*["']([^"']+)["']\s*\)/g;
  // Keep line anchored to avoid spanning arbitrary source segments or comments.
  const lineRx=/^\s*(?:import|export)\s+(?:[^\n;]*?\bfrom\s+)?["']([^"']+)["']/gm;
  for(const m of text.matchAll(lineRx))out.push({specifier:m[1],kind:"static"});
  for(const m of text.matchAll(dynamicRx))out.push({specifier:m[1],kind:"dynamic"});
  return out;
}
function analyze(){
  const paths=walk(join(root,"src")).sort();
  const graph=[];
  const topLevel=new Map();
  for(const file of paths){
    const from=relative(root,file).replaceAll("\\","/");
    const content=readFileSync(file,"utf8");
    const fOwner=owner(from);
    topLevel.set(fOwner,(topLevel.get(fOwner)||0)+1);
    for(const im of imports(content)){
      if(!im.specifier.startsWith("."))continue;
      const target=relative(root,resolve(dirname(file),im.specifier)).replaceAll("\\","/");
      graph.push({from,to:target,kind:im.kind,fromOwner:fOwner,toOwner:owner(target)});
    }
  }
  const cross=graph.filter(x=>x.fromOwner.startsWith("capability:")&&x.toOwner.startsWith("capability:")&&x.fromOwner!==x.toOwner);
  const pureViolations=graph.filter(x=>x.fromOwner==="shared:pure"&&x.toOwner!=="shared:pure");
  return {schema:"browseros-semantic-dependency-v1",sourceFiles:paths.length,trackedEdges:graph.length,
    sourceOwners:Object.fromEntries([...topLevel].sort((a,b)=>a[0].localeCompare(b[0]))),
    crossCapabilityImports:cross,sharedPureExternalImports:pureViolations,
    limitation:"Static relative imports; alias/dynamic expressions and built output bytes are not inferred. This is inventory, not a PASS gate or proof of no cycles."};
}
if(process.argv.includes("--self-test")){
  const cases=[
    ["src/capabilities/bookmark/manifest.ts","capability:bookmark"],
    ["src/shared/pure/time/relative.ts","shared:pure"],
    ["src/utils/taskUi.ts","legacy:utils"],
    ["src/components/layout/ActivityBar.vue","other"]];
  for(const [p,want] of cases)if(owner(p)!==want)throw Error("owner regression "+p);
  const fixture='import {x} from "../shared/pure/time/relative";\nexport {x} from "./x";\nconst y=import("./screen.vue");';
  const got=imports(fixture);
  if(got.length!==3||got.filter(x=>x.kind==="dynamic").length!==1)throw Error("import parser regression "+JSON.stringify(got));
  console.log("SEMANTIC_DEPENDENCY_INVENTORY_SELF_TEST=PASS");
} else console.log(JSON.stringify(analyze(),null,2));
