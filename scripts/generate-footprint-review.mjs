#!/usr/bin/env node
// Offline, provenance-aware build/package size comparison and visual report.
// Never infer startup memory or per-capability installation bytes from source files.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import assert from "node:assert/strict";

const fields = [
  ["frontend_dist_bytes", "前端构建产物"],
  ["release_binary_bytes", "Rust Release 可执行文件"],
  ["deb_file_bytes", "Linux .deb 安装包"],
  ["deb_installed_size_field_bytes", "Deb 声明安装体积"],
];
const args = process.argv.slice(2);
function value(flag, fallback = null) { const i = args.indexOf(flag); return i < 0 ? fallback : args[i + 1]; }
function read(path) { return JSON.parse(readFileSync(resolve(path), "utf8")); }
function safeNumber(v) { return typeof v === "number" && Number.isFinite(v) && v >= 0 ? v : null; }
function compare(before, after) {
  const rows = fields.map(([key,label]) => {
    const b=safeNumber(before[key]),a=safeNumber(after[key]);
    return {key,label,before_bytes:b,after_bytes:a,
      delta_bytes:b===null||a===null?null:a-b,
      delta_percent:b===null||a===null||b===0?null:Math.round((a-b)/b*10000)/100,
      status:b===null||a===null?"UNKNOWN":"MEASURED"};
  });
  return {schema:"browseros-footprint-comparison-v1",baseline:before.provenance||null,
    candidate:after.provenance||null,rows,
    limitation:"Same-runner comparable sizes only; not a controlled CPU, startup or memory benchmark. Module attribution is not measured."};
}
function escapeHtml(v){return String(v).replace(/[&<>"']/g,ch=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[ch]));}
function human(n){return n===null?"未测量":(n/1048576).toFixed(2)+" MiB";}
function html(summary, bundle, native) {
  const items=(Array.isArray(bundle?.assets)?bundle.assets:[])
    .filter(x=>x&&typeof x.path==="string"&&safeNumber(x.bytes)!==null)
    .sort((a,b)=>b.bytes-a.bytes).slice(0,15);
  const tr = summary.rows.map(r=>'<tr><th>'+escapeHtml(r.label)+'</th><td>'+human(r.before_bytes)+'</td><td>'+human(r.after_bytes)+'</td><td>'+
    (r.delta_bytes===null?"未知":((r.delta_bytes>0?"+":"")+r.delta_bytes.toLocaleString("en-US")+" B / "+r.delta_percent+"%"))+'</td></tr>').join("");
  const chunks=items.map(x=>'<tr><td>'+escapeHtml(x.path)+'</td><td>'+human(x.bytes)+'</td><td>'+human(safeNumber(x.gzip_bytes))+'</td><td>'+escapeHtml(x.referenced_by_index_html?"HTML 引用":"非 HTML 直接引用")+'</td></tr>').join("");
  const runtime=native?.classification==="MEASURED"
    ? "首次渲染后一次性进程采样：RSS "+human(safeNumber(native.rss_kb)===null?null:native.rss_kb*1024)
      +"；PSS "+human(safeNumber(native.pss_kb)===null?null:native.pss_kb*1024)
      +"；采样 CPU "+escapeHtml(native.cpu_percent_one_core)+"%（非 5/30 分钟空闲均值）"
    : "进程 CPU / PSS / RSS：未取得有效一次性采样";
  return '<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">'
    +'<title>BrowserOS 构建与资源审计</title><style>body{font:15px system-ui,sans-serif;max-width:1100px;margin:28px auto;padding:0 16px;color:#1e293b}'
    +'h1{font-size:24px}h2{margin-top:30px}table{border-collapse:collapse;width:100%;font-size:13px}th,td{text-align:left;padding:10px;border-bottom:1px solid #e2e8f0}'
    +'th{font-weight:600}small,aside{color:#64748b}article{padding:16px;background:#f1f5f9;border-radius:12px}code{overflow-wrap:anywhere}</style>'
    +'<h1>BrowserOS 构建资源诊断</h1><p>实测字节数与未知指标严格区分；不会把共享 Chunk 按模块重复计费。</p>'
    +'<article>基线：<code>'+escapeHtml(JSON.stringify(summary.baseline||{}))+'</code><p>候选：<code>'+escapeHtml(JSON.stringify(summary.candidate||{}))+'</code></p></article>'
    +'<h2>构建/安装包对比</h2><table><tr><th>指标</th><th>基线</th><th>候选</th><th>变化</th></tr>'+tr+'</table>'
    +'<h2>前端产物 Top 15（真实文件，不等于首次加载）</h2><table><tr><th>文件</th><th>原始字节</th><th>gzip</th><th>入口关系</th></tr>'+chunks+'</table>'
    +'<h2>原生进程采样</h2><p>'+runtime+'</p><aside>安装包与构建产物、首次网络传输、CPU/PSS/RSS 为不同单位和测量范围。空值不是 0。<br>'+escapeHtml(summary.limitation)+'</aside></html>';
}
if(args.includes("--self-test")){
  const c=compare({frontend_dist_bytes:100,release_binary_bytes:null},{frontend_dist_bytes:80,release_binary_bytes:1});
  assert.equal(c.rows[0].delta_percent,-20);
  assert.equal(c.rows[1].status,"UNKNOWN");
  assert.equal(safeNumber("500"),null);
  assert.ok(html(c,{assets:[{path:"x.js",bytes:100,gzip_bytes:80}]},{classification:"UNKNOWN"}).includes("x.js"));
  assert.ok(escapeHtml('<script>')==="&lt;script&gt;");
  // Baseline is intentionally pinned in the UI; detect any drift from canonical
  // provenance JSON rather than invent a second unguarded measurement source.
  const canonical = read("docs/engineering/footprint-baselines/master-7ae6fa86.json");
  const ui = readFileSync("src/components/system/EngineeringHealthPanel.vue","utf8");
  for(const field of ["frontend_dist_bytes","deb_file_bytes","deb_installed_size_field_bytes"]){
    assert.ok(ui.includes(canonical[field].toLocaleString("en-US")+" B"),"Measured UI value drift: "+field);
  }
  assert.ok(ui.includes(canonical.provenance.sha.slice(0,8))&&ui.includes("runs/"+canonical.provenance.run_id),"Evidence link drift");

  console.log("FOOTPRINT_COMPARISON_SELF_TEST=PASS");
}else{
  const beforePath=value("--before"),afterPath=value("--after"),out=value("--out"),outJson=value("--out-json");
  if(!beforePath||!afterPath||!out||!outJson)throw Error("Usage: --before BASE --after CURRENT --out REPORT.html --out-json REPORT.json");
  const before=read(beforePath),after=read(afterPath),bundle=value("--bundle")?read(value("--bundle")):null;
  const native=value("--native")?read(value("--native")):null;
  if(before.schema!=="browseros-footprint-v1"||after.schema!=="browseros-footprint-v1")throw Error("Unexpected footprint schema");
  const data=compare(before,after);
  mkdirSync(dirname(resolve(out)),{recursive:true});mkdirSync(dirname(resolve(outJson)),{recursive:true});
  writeFileSync(out,html(data,bundle,native),"utf8");
  writeFileSync(outJson,JSON.stringify(data,null,2)+"\n","utf8");
  console.log("FOOTPRINT_REPORT_RESULT=PASS; missing fields remain UNKNOWN");
}
