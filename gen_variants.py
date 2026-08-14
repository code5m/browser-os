#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
生成 15 个产品原型变体（供领导横向比较）。
原始 prototype.html 不动。

重要：v1 版生成器只换 CSS 变量、共享一套 HTML 骨架，导致 15 版"布局一致毫无新意"。
本版重构为：每个变体拥有【独立的布局骨架 + 独立的默认首屏 + 独立的交互亮点】，
仅复用一份公共 JS 工具库（toast/tab/file/clip/art/term/repo/audit 等），
保证"骨不同、皮不同、交互重点不同"，领导能一眼区分比较。

约定（保证 COMMON JS 能通用）：
- 活动栏容器 #activity，内部 .ic[data-view=xxx]
- 视图容器 .view#view-xxx（.main.view 或 .sidebar.view，由 on 类控制显隐）
- 浏览器主视图 #view-browser、地址栏 #addrInput/#addrGo、页签栏 #tabbar/#tabPlus、视口 #viewport、采集按钮 #collectBtn
- 文件视图 #view-files、宫格 #view-grid(#gridOverlay2)、应用 #view-apps、终端 #view-term、成果 #view-arts、剪贴板 #view-clip、仓库 #view-repo、审计 #view-audit
"""
import os, html

OUT = os.path.join(os.path.dirname(__file__), "variants")
os.makedirs(OUT, exist_ok=True)

# ============================================================
# 公共 CSS 基础库（每个变体都会包含，变体再用 extra_css 叠加差异）
# ============================================================
BASE_CSS = r"""
  * { box-sizing:border-box; margin:0; padding:0; }
  body { font-family:-apple-system,"Segoe UI","Microsoft YaHei",sans-serif; background:var(--bg); color:var(--ink); padding:24px; }
  h1 { font-size:22px; } .sub { color:var(--sub); font-size:13px; margin:4px 0 18px; }
  .pill { background:var(--pillbg); color:var(--blue); border-radius:10px; padding:2px 10px; font-size:11px; margin-left:8px; }
  .banner { max-width:1180px; margin:0 auto 14px; background:var(--bannerbg); border:1px solid var(--line); border-radius:10px; padding:10px 14px; font-size:12px; color:var(--bannertx); display:flex; gap:10px; align-items:center; }
  .banner b { color:var(--accent); }
  .stage { max-width:1180px; margin:0 auto; background:#fff; border-radius:12px; box-shadow:0 4px 20px rgba(0,0,0,.08); overflow:hidden; }
  .titlebar { height:34px; background:linear-gradient(90deg,var(--blue),var(--blue2)); display:flex; align-items:center; padding:0 14px; color:#fff; font-size:13px; }
  .titlebar .dots { display:flex; gap:6px; margin-right:12px; } .titlebar .dots i { width:11px;height:11px;border-radius:50%;background:rgba(255,255,255,.7);display:inline-block; }
  .body { display:flex; height:640px; }
  /* 活动栏（默认左侧竖排；变体可通过 extra_css 改成顶/底/抽屉/分组/隐藏） */
  .activity { width:56px; background:var(--activity-bg); display:flex; flex-direction:column; align-items:center; padding-top:10px; gap:6px; flex-shrink:0; }
  .activity .ic { width:40px;height:40px;border-radius:8px; display:flex; align-items:center; justify-content:center; color:var(--activity-fg); font-size:18px; cursor:pointer; user-select:none; text-align:center; line-height:1.1; font-size:11px; flex-direction:column; }
  .activity .ic span { font-size:9px; margin-top:2px; }
  .activity .ic.active { background:var(--blue); color:#fff; }
  .activity .ic:hover { background:var(--activity-hover); color:#fff; }
  /* 侧栏 */
  .sidebar { width:280px; background:var(--side); border-right:1px solid var(--line); display:flex; flex-direction:column; }
  .sidebar .tabs { display:flex; border-bottom:1px solid var(--line); }
  .sidebar .tabs div { flex:1; text-align:center; padding:10px 0; font-size:13px; cursor:pointer; color:#4e5969; }
  .sidebar .tabs div.on { color:var(--blue); border-bottom:2px solid var(--blue); font-weight:600; }
  .sidebar .content { flex:1; overflow:auto; padding:10px; font-size:13px; }
  .file { padding:6px 8px; border-radius:6px; display:flex; align-items:center; gap:8px; cursor:pointer; }
  .file:hover { background:#eef2f7; } .file .fi { color:#f5a623; }
  .art { padding:8px; border:1px solid var(--line); border-radius:8px; margin-bottom:8px; cursor:pointer; }
  .art .t { font-weight:600; font-size:13px; } .art .src { color:var(--sub); font-size:11px; margin-top:2px; }
  .art.sel { border-color:var(--blue); background:#eef6ff; }
  .sec-h { margin:12px 0 6px; color:var(--sub); font-size:11px; border-top:1px dashed var(--line); padding-top:8px; }
  /* 主区 */
  .main { flex:1; display:flex; flex-direction:column; min-width:0; }
  .addrbar { height:42px; border-bottom:1px solid var(--line); display:flex; align-items:center; gap:8px; padding:0 10px; background:#fafbfc; }
  .addrbar .nav { width:28px;height:28px;border-radius:6px; display:flex;align-items:center;justify-content:center;color:#4e5969;cursor:pointer; user-select:none; }
  .addrbar .nav:hover { background:#eef2f7; }
  .addrbar input { flex:1; height:30px; border:1px solid var(--line); border-radius:16px; padding:0 14px; font-size:13px; outline:none; }
  .addrbar .go { background:var(--blue); color:#fff; border:none; border-radius:6px; padding:6px 14px; cursor:pointer; font-size:13px; }
  .tabbar { height:38px; background:#eef1f5; display:flex; align-items:center; padding:0 6px; gap:2px; border-bottom:1px solid var(--line); }
  .tab { display:flex; align-items:center; gap:6px; padding:6px 10px; background:#fff; border:1px solid #dfe3e8; border-radius:6px 6px 0 0; font-size:12px; cursor:pointer; max-width:160px; }
  .tab.active { border-bottom:2px solid var(--blue); color:var(--ink); }
  .tab .x { color:#b0b6bf; } .tab .x:hover { color:#f53f3f; }
  .tab .fav { width:14px;height:14px;border-radius:3px;background:var(--blue2); }
  .tabbar .plus { width:28px;height:28px;border-radius:6px; display:flex;align-items:center;justify-content:center;cursor:pointer;color:#4e5969; user-select:none; }
  .tabbar .plus:hover { background:#dfe3e8; }
  .viewport { flex:1; background:#fff; position:relative; overflow:hidden; }
  .webpage { position:absolute; inset:12px; border:1px solid var(--line); border-radius:8px; background:linear-gradient(160deg,#e8f1ff,#f7fbff); display:flex; flex-direction:column; }
  .webpage .wh { padding:18px; } .webpage .wh h2 { font-size:18px; color:var(--blue); }
  .webpage .wh p { font-size:13px; color:#4e5969; margin-top:8px; line-height:1.7; }
  .webpage .cards { display:flex; gap:10px; padding:0 18px 18px; } .webpage .card { flex:1; height:70px; background:#fff; border:1px solid var(--line); border-radius:8px; }
  .floating { position:absolute; right:18px; bottom:18px; background:var(--blue); color:#fff; border:none; border-radius:20px; padding:10px 16px; font-size:13px; cursor:pointer; box-shadow:0 4px 12px rgba(43,108,176,.4); }
  #collectHint { position:absolute; left:50%; top:14px; transform:translateX(-50%); background:#1f2733; color:#fff; font-size:12px; padding:6px 12px; border-radius:8px; display:none; }
  .status { height:26px; background:#1f2733; color:#9aa4b2; font-size:11px; display:flex; align-items:center; padding:0 12px; gap:16px; }
  .status .ok { color:#52c41a; }
  .grid-overlay { position:absolute; inset:0; display:none; }
  .grid-cell { position:absolute; border:1px solid #bcd; background:linear-gradient(160deg,#e8f1ff,#f7fbff); border-radius:6px; overflow:hidden; }
  .grid-cell .gh { padding:8px; font-size:12px; color:var(--blue); border-bottom:1px solid #cde; background:#f0f6ff; display:flex; justify-content:space-between; }
  .grid-cell .gh input { flex:1; border:1px solid #cde; border-radius:10px; padding:2px 8px; font-size:11px; margin-right:6px; }
  .grid-cell iframe { width:100%; height:calc(100% - 36px); border:none; }
  .term { position:absolute; inset:0; background:#1f2733; color:#9ae6b4; font-family:monospace; font-size:12.5px; padding:12px; overflow:auto; }
  .term .line { white-space:pre-wrap; } .term .prompt { color:#63b3ed; }
  .term input { background:transparent; border:none; color:#fff; font-family:monospace; font-size:12.5px; outline:none; flex:1; }
  .term .row { display:flex; }
  .term .sim { position:absolute; right:10px; bottom:8px; font-size:10px; color:#5a6b7a; }
  .apps { display:flex; flex-wrap:wrap; gap:14px; padding:18px; align-content:flex-start; }
  .app { width:88px; text-align:center; cursor:pointer; } .app .ai { width:56px;height:56px;border-radius:14px;background:linear-gradient(160deg,var(--blue2),var(--blue));margin:0 auto;display:flex;align-items:center;justify-content:center;font-size:26px;color:#fff; }
  .app:hover .ai { box-shadow:0 4px 12px rgba(43,108,176,.4); } .app .an { font-size:11px; margin-top:6px; color:#4e5969; }
  .repo { padding:14px; font-size:13px; } .repo .row { margin-bottom:10px; } .repo label { display:block; color:var(--sub); font-size:11px; margin-bottom:4px; }
  .repo input { width:100%; height:30px; border:1px solid var(--line); border-radius:6px; padding:0 10px; font-size:13px; }
  .repo .save { background:var(--blue); color:#fff; border:none; border-radius:6px; padding:8px 16px; cursor:pointer; }
  .repo .log { margin-top:12px; font-size:12px; color:#4e5969; border-top:1px dashed var(--line); padding-top:10px; }
  .gate { padding:16px; font-size:13px; } .gate h3 { margin-bottom:10px; } .gate .art { display:flex; align-items:center; gap:8px; }
  .gate .btn { background:var(--blue); color:#fff; border:none; border-radius:6px; padding:8px 16px; cursor:pointer; margin-top:10px; }
  .gate .btn.green { background:#52c41a; } .gate .preview { background:#f7f8fa; border:1px solid var(--line); border-radius:8px; padding:12px; margin-top:10px; display:none; }
  .gate .done { margin-top:12px; color:#52c41a; font-size:13px; display:none; }
  .audit { padding:12px; font-size:12.5px; } .audit .a { padding:8px; border-bottom:1px solid var(--line); } .audit .t { color:var(--sub); font-size:11px; }
  .placeholder { position:absolute; inset:0; display:flex; align-items:center; justify-content:center; color:var(--sub); font-size:14px; flex-direction:column; gap:8px; }
  .view { display:none; } .view.on { display:flex; }
  .toast { position:fixed; bottom:30px; left:50%; transform:translateX(-50%); background:#1f2733; color:#fff; padding:10px 18px; border-radius:8px; font-size:13px; opacity:0; transition:opacity .3s; pointer-events:none; z-index:99; }
  .toast.show { opacity:1; }
  .fb { display:flex; flex-direction:column; height:100%; }
  .pathbar { display:flex; gap:4px; padding:8px; border-bottom:1px solid var(--line); }
  .pathbar button { padding:3px 8px; font-size:12px; }
  .pathbar input { flex:1; min-width:0; padding:4px 8px; font-family:monospace; font-size:11px; border:1px solid var(--line); border-radius:6px; }
  .quick { display:flex; flex-wrap:wrap; gap:6px; padding:8px; border-bottom:1px solid var(--line); }
  .qdir { padding:5px 10px; background:#eef3ff; border-radius:4px; font-size:12px; cursor:pointer; color:var(--blue); }
  .qdir:hover { background:#dbe7ff; }
  .recent { padding:6px 8px; }
  .recent select { width:100%; padding:5px 8px; border:1px solid var(--line); border-radius:6px; font-size:12px; }
  .recent-head { font-size:11px; color:var(--sub); margin-bottom:4px; }
  .flist { list-style:none; padding:0; margin:0; flex:1; overflow:auto; }
  .flist li { display:flex; align-items:center; gap:8px; padding:6px 10px; border-bottom:1px solid #f6f6f6; cursor:pointer; font-size:13px; }
  .flist li:hover { background:#f0f0f0; }
  .flist li .fi { width:20px; text-align:center; }
  .flist li .fn { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .flist li .fs { font-size:10px; color:#bbb; }
  .flist li.is-md .fn { color:#8a5a00; }
  .flist li.new { color:var(--blue); border-style:dashed; font-size:12px; }
  .flist li.new:hover { background:#eafaf0; }
  .feditor { position:absolute; inset:0; background:#fff; display:flex; flex-direction:column; z-index:5; }
  .fe-head { display:flex; align-items:center; justify-content:space-between; padding:8px 10px; border-bottom:1px solid var(--line); font-weight:600; font-size:13px; }
  .fe-acts button { font-size:11px; padding:3px 10px; }
  .fe-body { flex:1; display:flex; flex-direction:column; min-height:0; padding:8px; }
  .md { flex:1; overflow:auto; padding:14px 18px; border:1px solid var(--line); border-radius:6px; line-height:1.7; font-size:14px; }
  .md h1,.md h2,.md h3 { margin:14px 0 8px; } .md h1 { font-size:22px; border-bottom:2px solid #eee; padding-bottom:6px; }
  .md code { background:#f4f4f4; padding:1px 5px; border-radius:4px; font-family:monospace; }
  .md pre { background:#f6f6f6; padding:12px; border-radius:6px; overflow:auto; }
  .md a { color:var(--blue); }
  .feta { flex:1; width:100%; font-family:monospace; font-size:12px; padding:8px; border:1px solid var(--line); border-radius:4px; resize:none; background:#f6f6f6; }
  .ctxmenu { position:fixed; z-index:999; background:#fff; border:1px solid #ddd; box-shadow:0 2px 10px rgba(0,0,0,.2); border-radius:6px; padding:4px; min-width:160px; font-size:13px; }
  .ctxmenu div { padding:7px 12px; cursor:pointer; border-radius:4px; }
  .ctxmenu div:hover { background:#eef3ff; }
  .ctxmenu div.danger { color:#c33; } .ctxmenu div.danger:hover { background:#ffeaea; }
  .ctxmenu .sep { height:1px; background:#eee; margin:3px 0; padding:0; }
  .clip-actions { display:flex; gap:8px; margin-bottom:8px; }
  .clip-actions button { flex:1; }
  .clip-area { flex:1; width:100%; min-height:90px; font-family:monospace; font-size:12px; padding:8px; border:1px solid var(--line); border-radius:6px; resize:vertical; background:#f6f6f6; }
  .clip-tip { font-size:11px; color:var(--sub); margin:6px 0; }
  .clip-history { border-top:1px solid var(--line); padding-top:8px; overflow:auto; flex:1; min-height:0; }
  .clip-item { display:flex; align-items:center; gap:6px; padding:6px 8px; border-bottom:1px solid #f4f4f4; cursor:pointer; border-radius:4px; }
  .clip-item:hover { background:#eef3ff; }
  .clip-item .ct { flex:1; font-size:12px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; color:#333; }
  .clip-item .cc { border:none; background:transparent; color:#666; padding:2px 6px; border-radius:4px; }
  .clip-item .cc:hover { background:#e0eaff; color:var(--blue); }
  .clip-empty { font-size:12px; color:var(--sub); padding:14px; text-align:center; }
  /* 命令面板 */
  .cmd { position:fixed; inset:0; background:rgba(0,0,0,.4); display:none; align-items:flex-start; justify-content:center; padding-top:120px; z-index:200; }
  .cmd.on { display:flex; }
  .cmd-box { width:520px; background:#fff; border-radius:12px; box-shadow:0 8px 30px rgba(0,0,0,.3); overflow:hidden; }
  .cmd-box input { width:100%; border:none; border-bottom:1px solid var(--line); padding:14px 16px; font-size:14px; outline:none; }
  .cmd-list { max-height:300px; overflow:auto; }
  .cmd-list div { padding:10px 16px; font-size:13px; cursor:pointer; }
  .cmd-list div:hover, .cmd-list div.sel { background:#eef3ff; color:var(--blue); }
"""

# ============================================================
# 公共 JS（每个变体都注入，提供 toast/tab/file/clip/art/term/repo/audit/命令面板）
# 各变体通过活动栏 .ic[data-view] 切换视图；首屏默认视图由变体自己的 INIT 决定。
# ============================================================
COMMON_JS = r"""
const state = {
  tabs: [], activeTab: null, tabSeq: 0,
  arts: [ { t:"某技术博客段落", src:"blog.example.com", hash:"a1b2" }, { t:"竞品功能清单", src:"site.example.com", hash:"c3d4" } ],
  audit: [
    { a:"confirm_sync SUCCESS", t:"2026-08-13 14:02" },
    { a:"launch_app firefox", t:"2026-08-13 13:50" },
    { a:"collect_selection", t:"2026-08-13 13:41" },
    { a:"term_spawn bash", t:"2026-08-13 13:38" },
  ],
  apps: [ { n:"终端", i:"💻" },{ n:"Firefox", i:"🔥" },{ n:"VS Code", i:"📝" },{ n:"音乐", i:"🎵" },{ n:"文件", i:"📁" },{ n:"设置", i:"⚙️" },{ n:"日历", i:"📅" } ],
  files: [],
  termLines: [], termCmd: "",
  repo: { url:"gitee.com/xxx/browser-os3", configured:true },
};
const $ = (s,r=document)=>r.querySelector(s);
const $$ = (s,r=document)=>[...r.querySelectorAll(s)];
function toast(msg){ const t=$("#toast"); t.textContent=msg; t.classList.add("show"); clearTimeout(t._t); t._t=setTimeout(()=>t.classList.remove("show"),1600); }

function setView(v){
  $$("#activity .ic").forEach(x=>x.classList.toggle("active", x.dataset.view===v));
  $$(".view").forEach(x=>x.classList.remove("on"));
  const el=$("#view-"+v); if(el) el.classList.add("on");
  if(v==="apps") renderApps();
  if(v==="audit") renderAudit();
  if(v==="repo") renderRepo();
  if(v==="files") renderFiles();
  if(v==="clip"){ renderClip(); clipReadSilent(); toast("clipboard_read · 已同步系统剪贴板"); }
  if(v==="arts") renderArts();
  if(v==="grid") renderGrid(4, "#gridOverlay2");
}
$$("#activity .ic").forEach(ic=>{ ic.addEventListener("click",()=>setView(ic.dataset.view)); });

function hostOf(u){ try{ return new URL(u).hostname.replace(/^www\./,""); }catch(e){ return u.split("/")[0]||"新页签"; } }
function addTab(url){
  url = url && url.trim() ? url : "https://www.baidu.com";
  const id = "tab-"+(++state.tabSeq);
  const tab = { id, url, title: hostOf(url) };
  state.tabs.push(tab); setActive(tab.id); renderTabs();
}
function setActive(id){
  state.activeTab=id; const t=state.tabs.find(x=>x.id===id); if(!t) return;
  $("#addrInput").value=t.url; $("#wpTitle").textContent=t.title;
  $("#wpDesc").textContent="这是内嵌的真实网页（独立 Webview，方案 D 子窗口）。选中文字 → 点右下角“采集选中内容”一键存入成果库。";
  $("#webpage").style.display="flex"; $("#gridOverlay").style.display="none";
  $("#termView").style.display="none"; const cb=$("#collectBtn"); if(cb) cb.style.display="block";
}
function renderTabs(){
  const bar=$("#tabbar"); if(!bar) return; $$(".tab",bar).forEach(t=>t.remove());
  const plus=$("#tabPlus");
  state.tabs.forEach(t=>{
    const d=document.createElement("div"); d.className="tab"+(t.id===state.activeTab?" active":"");
    d.innerHTML=`<span class="fav"></span> ${t.title} <span class="x">✕</span>`;
    d.addEventListener("click",e=>{ if(e.target.classList.contains("x")){ closeTab(t.id); } else setActive(t.id); });
    bar.insertBefore(d, plus);
  });
  const st=$("#stTabs"); if(st) st.textContent="页签 "+state.tabs.length;
}
function closeTab(id){
  const i=state.tabs.findIndex(x=>x.id===id); if(i<0) return;
  state.tabs.splice(i,1);
  if(state.activeTab===id){ if(state.tabs.length){ setActive(state.tabs[Math.max(0,i-1)].id); } else { state.activeTab=null; const wp=$("#webpage"); if(wp) wp.innerHTML='<div class="placeholder">无打开的页签 · 点 ＋ 新建</div>'; } }
  renderTabs();
}
function bindBrowser(){
  const plus=$("#tabPlus"); if(plus) plus.addEventListener("click",()=>addTab(prompt("输入网址（留空=百度）：","https://")));
  const nb=$("#navBack"); if(nb) nb.addEventListener("click",()=>toast("tab_go_back · 后退"));
  const nf=$("#navFwd"); if(nf) nf.addEventListener("click",()=>toast("tab_go_forward · 前进"));
  const nr=$("#navReload"); if(nr) nr.addEventListener("click",()=>toast("tab_reload · 刷新"));
  const go=$("#addrGo"); if(go) go.addEventListener("click",()=>{ const u=$("#addrInput").value.trim(); if(!u) return; if(state.activeTab){ const t=state.tabs.find(x=>x.id===state.activeTab); t.url=u; t.title=hostOf(u); $("#wpTitle").textContent=t.title; renderTabs(); toast("tab_open · 导航到 "+t.title); } else addTab(u); });
  const ai=$("#addrInput"); if(ai) ai.addEventListener("keydown",e=>{ if(e.key==="Enter"){ const g=$("#addrGo"); if(g) g.click(); } });
  const cb=$("#collectBtn"); if(cb) cb.addEventListener("click",()=>{
    const sel=window.getSelection().toString();
    const txt = sel || ($("#wpDesc").textContent.slice(0,20)+"…");
    const hash=Math.random().toString(16).slice(2,6);
    const src=hostOf($("#addrInput").value);
    state.arts.unshift({ t:txt.slice(0,18), src, hash });
    state.audit.unshift({ a:"collect_selection", t:new Date().toISOString().slice(0,16).replace("T"," ") });
    const st=$("#stAudit"); if(st) st.textContent=state.audit.length;
    const h=$("#collectHint"); if(h){ h.textContent="已采集 → 成果库："+txt.slice(0,14); h.style.display="block"; setTimeout(()=>h.style.display="none",1800); }
    toast("collect_selection · 已落库 ("+hash+")");
  });
}
function renderGrid(n, sel){
  const ov=$(sel); if(!ov) return; ov.innerHTML=""; ov.style.display="block";
  const cols=Math.ceil(Math.sqrt(n)), rows=Math.ceil(n/cols);
  const cw=100/cols, ch=100/rows;
  for(let i=0;i<n;i++){
    const r=Math.floor(i/cols), c=i%cols;
    const cell=document.createElement("div"); cell.className="grid-cell";
    cell.style.left=(c*cw)+"%"; cell.style.top=(r*ch)+"%"; cell.style.width=cw+"%"; cell.style.height=ch+"%";
    const useIframe = window.__gridIframe;
    cell.innerHTML=`<div class="gh"><input value="https://site${i+1}.com" /><span>↗</span></div>` + (useIframe ? `<iframe src="https://www.example.com"></iframe>` : `<div style="padding:10px;font-size:11px;color:#4e5969">内嵌网页 ${i+1}</div>`);
    cell.querySelector("input").addEventListener("keydown",e=>{ if(e.key==="Enter") toast("grid_open · "+e.target.value); });
    cell.querySelector("span").addEventListener("click",e=>{ const inp=e.target.previousElementSibling; toast("grid_open · "+inp.value); });
    ov.appendChild(cell);
  }
}
const _g2=$$("#view-grid .go"); _g2.forEach(b=>b.addEventListener("click",()=>renderGrid(+b.dataset.n,"#gridOverlay2")));
function renderApps(){
  const box=$("#appList"); if(!box) return; box.innerHTML="";
  state.apps.forEach(a=>{
    const d=document.createElement("div"); d.className="app";
    d.innerHTML=`<div class="ai">${a.i}</div><div class="an">${a.n}</div>`;
    d.addEventListener("click",()=>{ state.audit.unshift({a:"launch_app "+a.n,t:new Date().toISOString().slice(0,16).replace("T"," ")}); const st=$("#stAudit"); if(st) st.textContent=state.audit.length; toast("launch_app · 启动 "+a.n); });
    box.appendChild(d);
  });
}
const fileState = {
  path:"/home/user/Documents",
  entries:[
    { is_dir:true, name:"项目", size:0, path:"/home/user/Documents/项目" },
    { is_dir:true, name:"截图", size:0, path:"/home/user/Documents/截图" },
    { is_dir:false, name:"需求文档.md", size:2048, path:"/home/user/Documents/需求文档.md" },
    { is_dir:false, name:"方案决策.md", size:4096, path:"/home/user/Documents/方案决策.md" },
    { is_dir:false, name:"流程图.png", size:88123, path:"/home/user/Documents/流程图.png" },
    { is_dir:false, name:"会议纪要.txt", size:1024, path:"/home/user/Documents/会议纪要.txt" },
    { is_dir:false, name:"预算表.xlsx", size:15360, path:"/home/user/Documents/预算表.xlsx" },
  ],
  startDirs:[
    { name:"Home", path:"/home/user" },{ name:"桌面", path:"/home/user/Desktop" },
    { name:"文档", path:"/home/user/Documents" },{ name:"下载", path:"/home/user/Downloads" },
    { name:"成果工作区", path:"/home/user/.browser-os/workspace" },
  ],
  recents:[ { type:"url", title:"https://www.example.com", path:"https://www.example.com" }, { type:"file", title:"方案决策.md", path:"/home/user/Documents/方案决策.md" } ],
  editing:null, ctxEntry:null,
};
function fmtSize(b){ if(b<1024) return b+" B"; if(b<1048576) return (b/1024).toFixed(1)+" KB"; return (b/1048576).toFixed(1)+" MB"; }
function renderFiles(){
  const q=$("#fbQuick"); if(!q) return; q.innerHTML="";
  fileState.startDirs.forEach(d=>{ const e=document.createElement("div"); e.className="qdir"; e.textContent=d.name; e.onclick=()=>{ fileState.path=d.path; renderFiles(); }; q.appendChild(e); });
  const r=$("#fbRecent"); r.innerHTML=`<div class="recent-head">🕒 最近访问</div><select id="fbRecSel"><option value="">— 选择最近访问 —</option>${fileState.recents.map(x=>`<option value="${x.type}|${x.path}">${x.type==='url'?'🌐':'📄'} ${x.title}</option>`).join("")}</select>`;
  $("#fbRecSel").onchange=e=>{ const [t,p]=e.target.value.split("|"); if(t==="url"){ toast("openBrowser: "+p); } else { openFile({is_dir:false,name:p.split("/").pop(),path:p,size:0}); } e.target.value=""; };
  $("#fbPath").value=fileState.path;
  const box=$("#fileList"); box.innerHTML="";
  const sorted=[...fileState.entries].sort((a,b)=> (b.is_dir?1:0)-(a.is_dir?1:0));
  sorted.forEach(f=>{
    const li=document.createElement("li"); const isMd=!f.is_dir && (f.name.endsWith(".md")||f.name.endsWith(".markdown"));
    if(isMd) li.className="is-md";
    li.innerHTML=`<span class="fi">${f.is_dir?"📁":(isMd?"📝":"📄")}</span><span class="fn">${f.name}</span><span class="fs">${f.is_dir?"":fmtSize(f.size)}</span>`;
    li.onclick=()=>openFile(f); li.oncontextmenu=e=>{ e.preventDefault(); showFileCtx(e, f); };
    box.appendChild(li);
  });
  const add=document.createElement("li"); add.className="new"; add.textContent="＋ 在此处新建…"; add.onclick=e=>showFileCtx(e); box.appendChild(add);
  if(!sorted.length){ const e=document.createElement("li"); e.className="new"; e.textContent="空目录"; box.appendChild(e); }
}
function renderMd(src){
  const esc=s=>s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;");
  return src.replace(/\r\n/g,"\n").split("\n").map(raw=>{
    if(raw.startsWith("```")) return ""; let l=esc(raw);
    const h=l.match(/^(#{1,6})\s+(.*)$/); if(h) return `<h${h[1].length}>${h[2]}</h${h[1].length}>`;
    const li=l.match(/^[-*]\s+(.*)$/); if(li) return `<li>${li[1]}</li>`;
    if(!l.trim()) return "<br/>";
    return "<p>"+l.replace(/`([^`]+)`/g,"<code>$1</code>").replace(/\*\*([^*]+)\*\*/g,"<b>$1</b>").replace(/\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g,'<a href="$2" target="_blank">$1</a>')+"</p>";
  }).join("");
}
function openFile(f){
  if(f.is_dir){ fileState.path=f.path; renderFiles(); return; }
  const ext=(f.name.split(".").pop()||"").toLowerCase();
  const isMd=ext==="md"||ext==="markdown";
  const textExts=["txt","json","js","ts","vue","rs","html","css","xml","yaml","yml","toml","csv","log","sh","py","java","go","c","cpp","h","sql","env","gitignore","md","markdown"];
  if(!textExts.includes(ext)){ toast("非文本文件 ("+f.name+")，暂不支持预览"); return; }
  fileState.editing={ name:f.name, path:f.path, isMd };
  const ed=$("#fbEditor"); if(ed){ ed.style.display="flex"; $("#feName").textContent=f.name; }
  const sample=isMd ? "# "+f.name.replace(/\.md$/,"")+"\n\n这是 **Markdown** 文件预览。\n\n- 支持列表\n- 支持 `代码`\n- [链接](https://example.com)" : "（模拟读取 "+f.name+" 的文本内容。）";
  $("#feText").value=sample; $("#feMd").innerHTML=renderMd(sample);
  const pv=$("#fePreview"); if(pv) pv.style.display=isMd?"inline-block":"none";
  const so=$("#feSource"); if(so) so.style.display="none";
  $("#feText").style.display=isMd?"none":"block"; $("#feMd").style.display=isMd?"block":"none";
}
const feClose=$("#feClose"); if(feClose) feClose.onclick=()=>{ const ed=$("#fbEditor"); if(ed) ed.style.display="none"; fileState.editing=null; };
const feSave=$("#feSave"); if(feSave) feSave.onclick=()=>{ if(!fileState.editing) return; toast("write_file · 已保存 "+fileState.editing.name); };
const fePrev=$("#fePreview"); if(fePrev) fePrev.onclick=()=>{ $("#feMd").innerHTML=renderMd($("#feText").value); $("#feMd").style.display="block"; $("#feText").style.display="none"; fePrev.style.display="none"; if($("#feSource")) $("#feSource").style.display="inline-block"; };
const feSrc=$("#feSource"); if(feSrc) feSrc.onclick=()=>{ $("#feText").style.display="block"; $("#feMd").style.display="none"; feSrc.style.display="none"; if($("#fePreview")) $("#fePreview").style.display="inline-block"; };
function showFileCtx(e, entry){
  e.preventDefault(); fileState.ctxEntry=entry||null; const m=$("#fbCtx");
  m.style.display="block"; m.style.left=e.clientX+"px"; m.style.top=e.clientY+"px";
  $("#fbCtxSep").style.display=entry?"block":"none"; $("#fbCtxRename").style.display=entry?"block":"none"; $("#fbCtxDelete").style.display=entry?"block":"none";
}
function closeFileCtx(){ const m=$("#fbCtx"); if(m) m.style.display="none"; fileState.ctxEntry=null; }
const fbCtx=$("#fbCtx"); if(fbCtx){ fbCtx.addEventListener("click",e=>e.stopPropagation());
  $("#fbCtxNewFile").onclick=()=>{ const n=prompt("新文件名："); if(n){ fileState.entries.push({is_dir:false,name:n,size:0,path:fileState.path+"/"+n}); closeFileCtx(); renderFiles(); toast("create_file · "+n); } else closeFileCtx(); };
  $("#fbCtxNewDir").onclick=()=>{ const n=prompt("新目录名："); if(n){ fileState.entries.push({is_dir:true,name:n,size:0,path:fileState.path+"/"+n}); closeFileCtx(); renderFiles(); toast("create_dir · "+n); } else closeFileCtx(); };
  $("#fbCtxRename").onclick=()=>{ const e=fileState.ctxEntry; if(!e) return; const n=prompt("重命名为：",e.name); if(n&&n!==e.name){ e.name=n; e.path=e.path.split("/").slice(0,-1).join("/")+"/"+n; closeFileCtx(); renderFiles(); toast("rename_path · "+n); } else closeFileCtx(); };
  $("#fbCtxDelete").onclick=()=>{ const e=fileState.ctxEntry; if(!e) return; if(confirm("删除「"+e.name+"」？")){ fileState.entries=fileState.entries.filter(x=>x!==e); closeFileCtx(); renderFiles(); toast("delete_path · "+e.name); } else closeFileCtx(); };
}
document.addEventListener("click",closeFileCtx);
const fbUp=$("#fbUp"); if(fbUp) fbUp.onclick=()=>{ const p=fileState.path.split("/").slice(0,-1).join("/")||"/"; fileState.path=p; renderFiles(); };
const fbGo=$("#fbGo"); if(fbGo) fbGo.onclick=()=>{ const p=$("#fbPath").value.trim(); if(p){ fileState.path=p; renderFiles(); } };
const fbPath=$("#fbPath"); if(fbPath) fbPath.addEventListener("keydown",e=>{ if(e.key==="Enter"){ const g=$("#fbGo"); if(g) g.click(); } });
const fbHome=$("#fbHome"); if(fbHome) fbHome.onclick=()=>{ const ws=fileState.startDirs.find(d=>d.name==="成果工作区"); fileState.path=(ws||fileState.startDirs[0]).path; renderFiles(); };

const clipState = { text:"", history:[] };
const CLIP_KEY="browser-os-clipboard";
function loadClipHistory(){ try{ const raw=localStorage.getItem(CLIP_KEY); if(raw) clipState.history=JSON.parse(raw); }catch(e){} }
function saveClipHistory(){ try{ localStorage.setItem(CLIP_KEY, JSON.stringify(clipState.history.slice(0,50))); }catch(e){} }
function clipReadSilent(){
  const samples=["https://developer.mozilla.org/zh-CN/docs/Web/API/Clipboard_API","会议结论：Q3 优先做宫格对比 + 剪贴板同步","SELECT * FROM orders WHERE status='PAID' LIMIT 10","https://www.figma.com/file/xxx/浏览器OS融合原型"];
  const t=samples[clipState.history.length % samples.length];
  if(!t || t===clipState.text) return; clipState.text=t; pushClipHistory(t); renderClip();
}
function pushClipHistory(text){
  const idx=clipState.history.findIndex(c=>c.text===text); if(idx>=0) clipState.history.splice(idx,1);
  clipState.history.unshift({text, at:Date.now()}); saveClipHistory();
}
function renderClip(){
  const area=$("#clipArea"); if(area) area.value=clipState.text;
  const cnt=$("#clipCount"); if(cnt) cnt.textContent=clipState.history.length;
  const box=$("#clipHistory"); if(!box) return; box.innerHTML="";
  if(!clipState.history.length){ box.innerHTML='<div class="clip-empty">暂无历史记录，复制内容后会自动收集</div>'; return; }
  clipState.history.slice(0,30).forEach((it)=>{
    const row=document.createElement("div"); row.className="clip-item";
    row.innerHTML=`<span class="ct" title="${it.text.replace(/"/g,'&quot;')}">${it.text}</span><button class="cc" title="复制此项">📋</button>`;
    row.querySelector(".ct").onclick=()=>{ clipState.text=it.text; renderClip(); toast("已填入编辑框"); };
    row.querySelector(".cc").onclick=e=>{ e.stopPropagation(); clipState.text=it.text; renderClip(); copyClipCurrent(true); };
    box.appendChild(row);
  });
}
function copyClipCurrent(silent){
  if(!clipState.text.trim()){ if(!silent) toast("没有可复制的内容"); return; }
  pushClipHistory(clipState.text); renderClip(); toast("clipboard_write · 已写入系统剪贴板");
}
const cp=$("#clipPaste"); if(cp) cp.onclick=()=>{ clipReadSilent(); toast("clipboard_read · 已粘贴"); };
const cc=$("#clipCopy"); if(cc) cc.onclick=()=>copyClipCurrent(false);
const cl=$("#clipClear"); if(cl) cl.onclick=()=>{ clipState.history.splice(0); saveClipHistory(); renderClip(); toast("已清空历史"); };
const ca=$("#clipArea"); if(ca) ca.addEventListener("input",e=>{ clipState.text=e.target.value; });
loadClipHistory();
function renderArts(){
  const box=$("#artList"); if(!box) return; box.innerHTML="";
  state.arts.forEach(a=>{ const d=document.createElement("div"); d.className="art"; d.innerHTML=`<div class="t">${a.t}</div><div class="src">来源：${a.src} · 哈希 ${a.hash}</div>`; d.addEventListener("click",()=>toast("read_artifact · "+a.t)); box.appendChild(d); });
}
function termInit(el){
  if(!el) return;
  el.innerHTML='<div class="line"><span class="prompt">user@os</span>:~$ <span id="tout"></span></div><div class="row"><span class="prompt">user@os</span>:~$ <input id="tinput" autofocus /></div><span class="sim">（模拟终端 · 真实走 term_spawn + portable_pty）</span>';
  const inp=$("#tinput",el);
  inp.addEventListener("keydown",e=>{ if(e.key==="Enter"){ const cmd=inp.value; inp.value=""; const out=el.querySelector("#tout");
    const res = cmd.trim()==="ls" ? "documents/  downloads/  project/  需求文档.md" : cmd.trim()==="pwd" ? "/home/user" : cmd.trim()==="" ? "" : cmd+": command ok (模拟)";
    el.insertBefore(document.createTextNode("\n"+cmd+"\n"+res+"\n"), el.lastElementChild); el.scrollTop=el.scrollHeight; } });
}
termInit($("#termView")); termInit($("#termFull"));
function renderRepo(){
  const b=$("#repoBody"); if(!b) return;
  b.innerHTML=`
    <div class="row"><label>远程仓库地址</label><input id="repoUrl" value="${state.repo.url}" /></div>
    <div class="row"><label>访问令牌（仅存系统密钥库，不回前端）</label><input id="repoToken" placeholder="输入后存入 KeyringStore" type="password" /></div>
    <button class="save" id="repoSave">配置仓库</button>
    <div class="log" id="repoLog"></div>`;
  $("#repoSave").addEventListener("click",()=>{ state.repo.url=$("#repoUrl").value; state.repo.configured=true; $("#repoLog").innerHTML="✓ configure_repo · 凭据已存入密钥库<br/>request_sync 可生成待确认任务"; state.audit.unshift({a:"configure_repo",t:new Date().toISOString().slice(0,16).replace("T"," ")}); toast("configure_repo · 已配置"); });
}
function renderGate(){
  const b=$("#repoBody"); if(!b) return;
  b.innerHTML+=`
    <div class="gate" style="margin-top:16px;border-top:1px dashed var(--line);padding-top:14px">
      <h3>Git 同步 · 两步闸门</h3>
      <div id="gateArts"></div>
      <button class="btn" id="gateReq">生成同步预览</button>
      <div class="preview" id="gatePrev"></div>
      <button class="btn green" id="gateConf" style="display:none">✓ 确认推送</button>
      <div class="done" id="gateDone">✓ 已推送 ${state.arts.length} 个成果 · 审计已留痕</div>
    </div>`;
  const ga=$("#gateArts"); state.arts.forEach(()=>{ const d=document.createElement("div"); d.className="art sel"; d.innerHTML=`<div class="t">☑ 成果项</div>`; ga.appendChild(d); });
  $("#gateReq").addEventListener("click",()=>{ $("#gatePrev").style.display="block"; $("#gatePrev").innerHTML=`仓库：${state.repo.url}<br/>将推送 <b>${state.arts.length}</b> 个成果<br/><span style="color:var(--sub);font-size:11px">token 由 confirm_sync 从密钥库取出，不回前端</span>`; $("#gateConf").style.display="inline-block"; toast("request_sync · 生成待确认任务"); });
  $("#gateConf").addEventListener("click",()=>{ $("#gateDone").style.display="block"; $("#gateConf").style.display="none"; state.audit.unshift({a:"confirm_sync SUCCESS",t:new Date().toISOString().slice(0,16).replace("T"," ")}); const st=$("#stAudit"); if(st) st.textContent=state.audit.length; toast("confirm_sync · 已推送"); });
}
const _renderRepo=renderRepo; renderRepo=function(){ _renderRepo(); renderGate(); };
function renderAudit(){ const b=$("#auditList"); if(!b) return; b.innerHTML=""; state.audit.forEach(a=>{ const d=document.createElement("div"); d.className="a"; d.innerHTML=`${a.a}<div class="t">${a.t}</div>`; b.appendChild(d); }); }

// 命令面板（Ctrl/Cmd+K）—— 仅当页面存在 #cmd 时启用
if($("#cmd")){
  const CMD_ITEMS=[ {k:"浏览",v:"browser"},{k:"文件",v:"files"},{k:"剪贴板",v:"clip"},{k:"成果",v:"arts"},{k:"宫格",v:"grid"},{k:"应用",v:"apps"},{k:"终端",v:"term"},{k:"仓库",v:"repo"},{k:"审计",v:"audit"},{k:"新建页签",v:"__newtab"},{k:"宫格 4",v:"__grid4"},{k:"采集",v:"__collect"} ];
  function openCmd(){ const c=$("#cmd"); c.classList.add("on"); const list=$("#cmdList"); list.innerHTML=""; CMD_ITEMS.forEach(it=>{ const d=document.createElement("div"); d.textContent="→ "+it.k; d.onclick=()=>runCmd(it.v); list.appendChild(d); }); $("#cmdInput").value=""; $("#cmdInput").focus(); }
  function runCmd(v){ $("#cmd").classList.remove("on");
    if(v==="__newtab"){ addTab(prompt("输入网址：","https://")); return; }
    if(v==="__grid4"){ setView("grid"); renderGrid(4,"#gridOverlay2"); return; }
    if(v==="__collect"){ const cb=$("#collectBtn"); if(cb) cb.click(); return; }
    setView(v);
  }
  $("#cmdInput").addEventListener("keydown",e=>{ if(e.key==="Enter"){ const v=CMD_ITEMS.find(x=>x.k.includes($("#cmdInput").value))?.v; if(v) runCmd(v); } if(e.key==="Escape"){ $("#cmd").classList.remove("on"); } });
  document.addEventListener("keydown",e=>{ if((e.metaKey||e.ctrlKey)&&e.key.toLowerCase()==="k"){ e.preventDefault(); openCmd(); } });
  $("#cmd").addEventListener("click",e=>{ if(e.target.id==="cmd") $("#cmd").classList.remove("on"); });
}
bindBrowser();
addTab("https://www.example.com"); addTab("https://www.baidu.com"); setActive(state.tabs[0].id); renderFiles();
"""

# ============================================================
# 公共 HTML 片段（视图片段，每个变体复用，保证 COMMON_JS 能找到 #view-xxx）
# ============================================================
VIEWS = {
"browser":"""
      <!-- 浏览器主视图 -->
      <div class="main view" id="view-browser">
        <div class="addrbar">
          <div class="nav" id="navBack">←</div><div class="nav" id="navFwd">→</div><div class="nav" id="navReload">⟳</div>
          <input id="addrInput" value="https://www.example.com" />
          <button class="go" id="addrGo">前往</button>
        </div>
        <div class="tabbar" id="tabbar">
          <div class="plus" id="tabPlus">＋</div>
        </div>
        <div class="viewport" id="viewport">
          <div class="webpage" id="webpage">
            <div class="wh"><h2 id="wpTitle">Example 站点</h2><p id="wpDesc">这是内嵌的真实网页（独立 Webview，方案 D 子窗口）。可正常滚动、点击、登录。选中文字 → 点右下角“采集选中内容”一键存入左侧成果库。</p></div>
            <div class="cards"><div class="card"></div><div class="card"></div><div class="card"></div></div>
          </div>
          <div class="grid-overlay" id="gridOverlay"></div>
          <div class="term" id="termView" style="display:none"></div>
          <button class="floating" id="collectBtn">＋ 采集选中内容</button>
          <div id="collectHint"></div>
        </div>
        <div class="status"><span class="ok">● 已连接</span> <span id="stTabs">页签 1</span> · 终端就绪 · 仓库已配置 · 审计 <span id="stAudit">12</span> 条</div>
      </div>""",
"files":"""
      <!-- 文件视图 -->
      <div class="sidebar view" id="view-files" style="width:100%">
        <div class="tabs"><div class="on">文件管理器</div></div>
        <div class="content" style="padding:0">
          <div class="fb">
            <div class="pathbar">
              <button id="fbUp">⬆</button>
              <input id="fbPath" placeholder="输入路径回车跳转（如 /home/you/Documents）" />
              <button id="fbGo">↵</button>
              <button id="fbHome" title="回到起始目录">🏠</button>
            </div>
            <div class="quick" id="fbQuick"></div>
            <div class="recent" id="fbRecent"></div>
            <ul class="flist" id="fileList"></ul>
          </div>
          <div class="feditor" id="fbEditor" style="display:none">
            <div class="fe-head">
              <span id="feName">文件</span>
              <span class="fe-acts">
                <button id="fePreview" style="display:none">👁 预览</button>
                <button id="feSource" style="display:none">✎ 源码</button>
                <button class="primary" id="feSave">💾 保存</button>
                <button id="feClose">✕ 关闭</button>
              </span>
            </div>
            <div class="fe-body">
              <div class="md" id="feMd"></div>
              <textarea class="feta" id="feText"></textarea>
            </div>
          </div>
        </div>
        <div class="ctxmenu" id="fbCtx" style="display:none">
          <div id="fbCtxNewFile">📄 新建文件</div>
          <div id="fbCtxNewDir">📁 新建目录</div>
          <div id="fbCtxSep" class="sep"></div>
          <div id="fbCtxRename">✏ 重命名</div>
          <div id="fbCtxDelete" class="danger">🗑 删除</div>
        </div>
      </div>""",
"clip":"""
      <!-- 剪贴板视图 -->
      <div class="sidebar view" id="view-clip" style="width:100%">
        <div class="tabs"><div class="on">📋 剪贴板</div></div>
        <div class="content" style="padding:10px;display:flex;flex-direction:column">
          <div class="clip-actions">
            <button id="clipPaste">📥 刷新/粘贴</button>
            <button id="clipCopy" class="primary">📤 复制当前</button>
            <button id="clipClear" class="ghost">🗑 清空历史</button>
          </div>
          <textarea id="clipArea" class="clip-area" placeholder="在此编辑文本后点「复制当前」写入系统剪贴板；切回本应用或打开面板时自动同步"></textarea>
          <div class="clip-tip" id="clipTip">切回本应用或打开面板时自动读取系统剪贴板，已保存 <b id="clipCount">0</b> 条历史</div>
          <div class="clip-history" id="clipHistory"></div>
        </div>
      </div>""",
"arts":"""
      <!-- 成果视图 -->
      <div class="sidebar view" id="view-arts" style="width:100%">
        <div class="tabs"><div class="on">成果库</div></div>
        <div class="content" id="artList"></div>
      </div>""",
"grid":"""
      <!-- 宫格视图 -->
      <div class="main view" id="view-grid">
        <div class="addrbar">
          <span style="font-size:13px;color:#4e5969">宫格数量：</span>
          <button class="go" data-n="2">2</button>
          <button class="go" data-n="4">4</button>
          <button class="go" data-n="6">6</button>
          <button class="go" data-n="9">9</button>
          <button class="go" data-n="12">12</button>
          <span style="font-size:12px;color:var(--sub);margin-left:8px">（各宫格独立导航，模拟对比浏览）</span>
        </div>
        <div class="viewport"><div class="grid-overlay" id="gridOverlay2" style="display:block"></div></div>
      </div>""",
"apps":"""
      <!-- 应用启动器视图 -->
      <div class="main view" id="view-apps">
        <div class="viewport"><div class="apps" id="appList"></div></div>
      </div>""",
"term":"""
      <!-- 终端视图 -->
      <div class="main view" id="view-term">
        <div class="viewport"><div class="term" id="termFull"><span class="sim">（模拟终端 · 真实走 term_spawn + portable_pty）</span></div></div>
      </div>""",
"repo":"""
      <!-- 仓库视图 -->
      <div class="sidebar view" id="view-repo" style="width:100%">
        <div class="tabs"><div class="on">Git 仓库</div></div>
        <div class="content"><div class="repo" id="repoBody"></div></div>
      </div>""",
"audit":"""
      <!-- 审计视图 -->
      <div class="sidebar view" id="view-audit" style="width:100%">
        <div class="tabs"><div class="on">审计日志</div></div>
        <div class="content"><div class="audit" id="auditList"></div></div>
      </div>""",
}

# 活动栏（默认 9 模块左侧竖排）；变体可覆盖
ACT_DEFAULT = """
      <div class="activity" id="activity">
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="clip">📋<span>剪贴板</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="ic" data-view="grid">🗂️<span>宫格</span></div>
        <div class="ic" data-view="apps">🚀<span>应用</span></div>
        <div class="ic" data-view="term">💻<span>终端</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
        <div class="ic" data-view="audit">🛡️<span>审计</span></div>
      </div>"""

# 默认主题变量（专业蓝）
THEME_BLUE = dict(BLUE="#2b6cb0",BLUE2="#4299e1",LINE="#e5e6eb",INK="#1f2329",SUB="#8a919f",SIDE="#f7f8fa",BG="#f0f2f5",
  ACTBG="#1f2733",ACTFG="#cbd5e0",ACTW="56",ACTDIR="flex-direction:column;",ACTBORDER="",ACTHOVER="#2f3a47",ACCENT="#2b6cb0",
  PILLBG="#e8f3ff",BANNERBG="#fff",BANNERTX="#4e5969")

# ============================================================
# 变体定义：每个变体 = 独立骨架(activity 覆盖 + 额外 css + 首屏 INIT + 可选额外 DOM)
# ============================================================
VARIANTS = [
  # ---------- 1. 左侧极简图标栏（无文字，hover 显示名） ----------
  dict(f="v01-mini-icon-rail.html", title="变体01 · 极简图标栏",
    sub="左侧仅图标无文字，hover 浮出名称，最大化内容区，最克制的导航。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity" id="activity">
        <div class="ic active" data-view="browser" title="浏览">📁</div>
        <div class="ic" data-view="files" title="文件">📂</div>
        <div class="ic" data-view="clip" title="剪贴板">📋</div>
        <div class="ic" data-view="arts" title="成果">📚</div>
        <div class="ic" data-view="grid" title="宫格">🗂️</div>
        <div class="ic" data-view="apps" title="应用">🚀</div>
        <div class="ic" data-view="term" title="终端">💻</div>
        <div class="ic" data-view="repo" title="仓库">🛰️</div>
        <div class="ic" data-view="audit" title="审计">🛡️</div>
      </div>""",
    extra_css="""
  .activity .ic { width:46px;height:46px;font-size:20px; }
  .activity .ic:hover::after { content:attr(title); position:absolute; left:54px; background:#1f2733; color:#fff; font-size:11px; padding:3px 8px; border-radius:6px; white-space:nowrap; z-index:50; }
  .activity { position:relative; }""",
    init="", focus="布局-极简图标栏"),

  # ---------- 2. 顶部 Tab 全局导航 + 浏览器常驻 ----------
  dict(f="v02-top-tabs.html", title="变体02 · 顶部 Tab 全局导航",
    sub="活动栏转为顶部横排 Tab，浏览器常驻主区，类 Chrome 全屏浏览体验，宽屏最优。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity topmode" id="activity">
        <div class="ic active" data-view="browser">📁 浏览</div>
        <div class="ic" data-view="files">📂 文件</div>
        <div class="ic" data-view="clip">📋 剪贴板</div>
        <div class="ic" data-view="arts">📚 成果</div>
        <div class="ic" data-view="grid">🗂️ 宫格</div>
        <div class="ic" data-view="apps">🚀 应用</div>
        <div class="ic" data-view="term">💻 终端</div>
        <div class="ic" data-view="repo">🛰️ 仓库</div>
        <div class="ic" data-view="audit">🛡️ 审计</div>
      </div>""",
    extra_css="""
  .body { flex-direction:column; }
  .activity.topmode { width:100%; flex-direction:row; height:46px; padding:0; gap:0; background:#1f2733; }
  .activity.topmode .ic { flex-direction:row; width:auto; height:46px; padding:0 16px; font-size:13px; border-radius:0; color:#cbd5e0; }
  .activity.topmode .ic.active { background:var(--blue); color:#fff; }
  .activity.topmode .ic:hover { background:#2f3a47; }""",
    init="", focus="布局-顶部导航"),

  # ---------- 3. 左侧分组导航（带分隔标题） ----------
  dict(f="v03-grouped-nav.html", title="变体03 · 分组导航侧栏",
    sub="左侧导航带分组标题（浏览/创作/系统），信息层级清晰，便于功能归类发现。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity grouped" id="activity">
        <div class="grp">浏览</div>
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="grid">🗂️<span>宫格</span></div>
        <div class="ic" data-view="apps">🚀<span>应用</span></div>
        <div class="grp">创作</div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="clip">📋<span>剪贴板</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="grp">系统</div>
        <div class="ic" data-view="term">💻<span>终端</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
        <div class="ic" data-view="audit">🛡️<span>审计</span></div>
      </div>""",
    extra_css="""
  .activity.grouped { width:96px; gap:2px; padding-top:6px; }
  .activity.grouped .grp { width:100%; text-align:left; padding:8px 10px 2px; font-size:10px; color:var(--activity-fg); opacity:.6; letter-spacing:1px; }
  .activity.grouped .ic { width:80px; flex-direction:row; justify-content:flex-start; gap:8px; font-size:12px; padding-left:12px; }""",
    init="", focus="布局-分组侧栏"),

  # ---------- 4. 可折叠抽屉侧栏 ----------
  dict(f="v04-drawer.html", title="变体04 · 可折叠抽屉侧栏",
    sub="点标题收起仅留图标，hover 滑出完整侧栏，移动端/小屏常用形态。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity drawer" id="activity">
        <div class="drawer-h" id="drawerH">⟨ 收起</div>
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="clip">📋<span>剪贴板</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="ic" data-view="grid">🗂️<span>宫格</span></div>
        <div class="ic" data-view="apps">🚀<span>应用</span></div>
        <div class="ic" data-view="term">💻<span>终端</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
        <div class="ic" data-view="audit">🛡️<span>审计</span></div>
      </div>""",
    extra_css="""
  .activity.drawer { width:140px; transition:width .2s; align-items:stretch; padding-top:0; }
  .activity.drawer .drawer-h { padding:10px; font-size:12px; color:var(--activity-fg); cursor:pointer; border-bottom:1px solid rgba(255,255,255,.1); user-select:none; }
  .activity.drawer .ic { width:auto; flex-direction:row; justify-content:flex-start; gap:10px; padding:0 14px; font-size:13px; border-radius:0; }
  .activity.drawer.collapsed { width:52px; }
  .activity.drawer.collapsed .ic span, .activity.drawer.collapsed .drawer-h { display:none; }
  .activity.drawer.collapsed .ic { justify-content:center; }""",
    init="""
  const dh=$("#drawerH"); if(dh) dh.addEventListener("click",()=>{ const a=$("#activity"); a.classList.toggle("collapsed"); dh.textContent=a.classList.contains("collapsed")?"⟩":"⟨ 收起"; });""",
    focus="布局-抽屉侧栏"),

  # ---------- 5. 暗色工作台（左栏+终端常驻底栏） ----------
  dict(f="v05-dark-workbench.html", title="变体05 · 暗色工作台",
    sub="整体暗色，左栏导航 + 底部常驻终端，偏开发者终端审美，沉浸工作。",
    theme=dict(BLUE="#63b3ed",BLUE2="#90cdf4",LINE="#2d3748",INK="#e2e8f0",SUB="#a0aec0",SIDE="#1a202c",BG="#0f141a",
      ACTBG="#0b0f14",ACTFG="#a0aec0",ACTW="56",ACTDIR="flex-direction:column;",ACTBORDER="",ACTHOVER="#2d3748",ACCENT="#63b3ed",
      PILLBG="#1e2a3a",BANNERBG="#1a202c",BANNERTX="#a0aec0"),
    activity=ACT_DEFAULT,
    extra_css="""
  .stage { background:#0f141a; }
  .sidebar { background:#161c24; }
  .activity .ic.active { background:#63b3ed; }
  .body { height:640px; flex-wrap:wrap; align-content:flex-start; }
  /* 底部常驻终端：把终端视图做成始终在底部的条 */
  #view-term { position:absolute; left:56px; right:0; bottom:0; height:180px; border-top:1px solid var(--line); box-shadow:0 -4px 14px rgba(0,0,0,.3); z-index:6; }
  #view-term .viewport { height:100%; }
  .body { position:relative; }""",
    init="""
  // 暗色工作台：终端常驻底部，默认也打开浏览器
  setView('browser'); const t=$("#view-term"); if(t) t.classList.add("on");""",
    focus="配色-暗色工作台"),

  # ---------- 6. 极简白卡片墙（无侧栏，模块化卡片） ----------
  dict(f="v06-card-wall.html", title="变体06 · 极简卡片墙",
    sub="去掉传统侧栏，首页即能力卡片墙，点卡片进入对应模块，轻量门户感。",
    theme=dict(BLUE="#111827",BLUE2="#374151",LINE="#e5e7eb",INK="#111827",SUB="#6b7280",SIDE="#ffffff",BG="#ffffff",
      ACTBG="#f9fafb",ACTFG="#374151",ACTW="0",ACTDIR="",ACTBORDER="border-right:1px solid var(--line);",ACTHOVER="#eef2f7",ACCENT="#111827",
      PILLBG="#f3f4f6",BANNERBG="#f9fafb",BANNERTX="#6b7280"),
    activity="""
      <div class="activity" id="activity" style="display:none"></div>
      <div class="main view on" id="view-home">
        <div class="viewport" style="padding:30px; overflow:auto">
          <h2 style="font-size:20px;margin-bottom:6px">浏览器OS · 能力中心</h2>
          <p style="color:var(--sub);margin-bottom:20px">点卡片进入模块（无传统侧栏的极简门户形态）</p>
          <div class="cards-wall" id="cardsWall"></div>
        </div>
      </div>""",
    extra_css="""
  .stage { background:#fff; }
  .cards-wall { display:grid; grid-template-columns:repeat(auto-fill,minmax(180px,1fr)); gap:16px; }
  .ccard { border:1px solid var(--line); border-radius:12px; padding:18px; cursor:pointer; transition:.15s; background:#fff; }
  .ccard:hover { box-shadow:0 6px 18px rgba(0,0,0,.08); transform:translateY(-2px); border-color:var(--blue); }
  .ccard .ci { font-size:30px; }
  .ccard .ct { font-weight:600; margin-top:8px; font-size:15px; }
  .ccard .cd { font-size:12px; color:var(--sub); margin-top:4px; }""",
    init="""
  const WALL=[["📁","浏览","内嵌真实网页",'browser'],["📂","文件","管理器+MD预览",'files'],["📋","剪贴板","读取/写入/历史",'clip'],["📚","成果","采集落库",'arts'],["🗂️","宫格","多网页对比",'grid'],["🚀","应用","启动器",'apps'],["💻","终端","PTY",'term'],["🛰️","仓库","Git闸门",'repo'],["🛡️","审计","操作留痕",'audit']];
  const wall=$("#cardsWall"); wall.innerHTML=""; WALL.forEach(([i,t,d,v])=>{ const c=document.createElement("div"); c.className="ccard"; c.innerHTML=`<div class="ci">${i}</div><div class="ct">${t}</div><div class="cd">${d}</div>`; c.onclick=()=>setView(v); wall.appendChild(c); });""",
    focus="架构-卡片墙门户"),

  # ---------- 7. 薄荷绿消费级（圆形活动栏+圆角大卡片） ----------
  dict(f="v07-mint.html", title="变体07 · 薄荷绿消费级",
    sub="主色薄荷绿、圆角更大、图标圆润，偏消费级轻量产品观感。",
    theme=dict(BLUE="#10b981",BLUE2="#34d399",LINE="#d1fae5",INK="#064e3b",SUB="#6b7280",SIDE="#ecfdf5",BG="#f0fdf9",
      ACTBG="#064e3b",ACTFG="#a7f3d0",ACTW="64",ACTDIR="flex-direction:column;",ACTBORDER="",ACTHOVER="#065f46",ACCENT="#10b981",
      PILLBG="#d1fae5",BANNERBG="#ecfdf5",BANNERTX="#6b7280"),
    activity="""
      <div class="activity mint" id="activity">
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="clip">📋<span>剪贴板</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="ic" data-view="grid">🗂️<span>宫格</span></div>
        <div class="ic" data-view="apps">🚀<span>应用</span></div>
        <div class="ic" data-view="term">💻<span>终端</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
        <div class="ic" data-view="audit">🛡️<span>审计</span></div>
      </div>""",
    extra_css="""
  .activity.mint { border-radius:0 16px 16px 0; }
  .activity.mint .ic { border-radius:50%; width:46px; height:46px; }
  .stage, .sidebar, .card, .art, .app .ai { border-radius:16px !important; }
  .addrbar .go, .tab, .clip-actions button, .repo .save, .gate .btn { border-radius:20px !important; }""",
    init="", focus="配色-薄荷绿"),

  # ---------- 8. 紫橙渐变年轻化（毛玻璃+渐变标题栏） ----------
  dict(f="v08-purple-orange.html", title="变体08 · 紫橙渐变年轻化",
    sub="紫主橙辅渐变标题栏 + 毛玻璃活动栏，强视觉冲击，年轻化品牌向。",
    theme=dict(BLUE="#7c3aed",BLUE2="#f97316",LINE="#ede9fe",INK="#1e1b4b",SUB="#6b7280",SIDE="#f5f3ff",BG="#faf5ff",
      ACTBG="#1e1b4b",ACTFG="#c4b5fd",ACTW="56",ACTDIR="flex-direction:column;",ACTBORDER="",ACTHOVER="#312e81",ACCENT="#f97316",
      PILLBG="#ede9fe",BANNERBG="#f5f3ff",BANNERTX="#6b7280"),
    activity=ACT_DEFAULT,
    extra_css="""
  .titlebar { background:linear-gradient(90deg,#7c3aed,#f97316); }
  .activity { backdrop-filter:blur(8px); background:rgba(30,27,75,.92); }
  .floating { background:linear-gradient(90deg,#7c3aed,#f97316); }
  .pill { background:linear-gradient(90deg,#7c3aed,#f97316); color:#fff; }
  h1 .pill { color:#fff; }""",
    init="", focus="配色-紫橙渐变"),

  # ---------- 9. 宫格对比优先（首屏 2×2 真实 iframe，无活动栏） ----------
  dict(f="v09-grid-first.html", title="变体09 · 宫格对比优先",
    sub="首屏即 2×2 真实网页对比（方案 D 核心卖点），活动栏改为顶部极简切换。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity topmode" id="activity">
        <div class="ic active" data-view="grid">🗂️ 宫格对比</div>
        <div class="ic" data-view="browser">📁 浏览</div>
        <div class="ic" data-view="files">📂 文件</div>
        <div class="ic" data-view="arts">📚 成果</div>
        <div class="ic" data-view="term">💻 终端</div>
      </div>""",
    extra_css="""
  .body { flex-direction:column; }
  .activity.topmode { width:100%; flex-direction:row; height:46px; padding:0; background:#1f2733; gap:0; }
  .activity.topmode .ic { flex-direction:row; width:auto; height:46px; padding:0 16px; font-size:13px; border-radius:0; color:#cbd5e0; }
  .activity.topmode .ic.active { background:var(--blue); color:#fff; }""",
    init="window.__gridIframe=true; setView('grid'); renderGrid(4,'#gridOverlay2');", focus="交互-宫格优先"),

  # ---------- 10. 采集优先（首屏网页+右侧常驻采集面板） ----------
  dict(f="v10-collect-first.html", title="变体10 · 采集优先",
    sub="首屏网页 + 右侧常驻采集面板（含 hash 溯源流），强调“网页→成果库”闭环。",
    theme=THEME_BLUE,
    activity=ACT_DEFAULT,
    extra_css="""
  #view-browser { flex-direction:row; }
  #view-browser .main { flex:1; }
  .collect-panel { width:300px; border-left:1px solid var(--line); background:var(--side); display:flex; flex-direction:column; }
  .collect-panel h4 { padding:12px; font-size:13px; border-bottom:1px solid var(--line); }
  .collect-panel .cp-body { flex:1; overflow:auto; padding:10px; }
  .collect-panel .cp-item { padding:8px; border:1px solid var(--line); border-radius:8px; margin-bottom:8px; font-size:12px; }
  .collect-panel .cp-hash { color:var(--blue); font-family:monospace; font-size:11px; }
  .collect-panel .cp-btn { margin:10px; padding:10px; background:var(--blue); color:#fff; border:none; border-radius:8px; cursor:pointer; }""",
    init="""
  // 在浏览器视图内注入右侧常驻采集面板
  const bv=$("#view-browser"); const cpPanel=document.createElement("div"); cpPanel.className="collect-panel";
  cpPanel.innerHTML='<h4>📥 采集面板（网页→成果库）</h4><div class="cp-body" id="cpBody"></div><button class="cp-btn" id="cpBtn">＋ 采集当前网页选区</button>';
  bv.appendChild(cpPanel);
  function renderCp(){ const box=$("#cpBody"); box.innerHTML=""; state.arts.forEach(a=>{ const d=document.createElement("div"); d.className="cp-item"; d.innerHTML=`<div>${a.t}</div><div class="cp-hash">#${a.hash} · ${a.src}</div>`; box.appendChild(d); }); }
  renderCp();
  $("#cpBtn").addEventListener("click",()=>{ const sel=window.getSelection().toString(); const txt=sel||($("#wpDesc").textContent.slice(0,20)+"…"); const hash=Math.random().toString(16).slice(2,6); const src=hostOf($("#addrInput").value); state.arts.unshift({t:txt.slice(0,18),src,hash}); state.audit.unshift({a:"collect_selection",t:new Date().toISOString().slice(0,16).replace("T"," ")}); const st=$("#stAudit"); if(st) st.textContent=state.audit.length; renderCp(); toast("collect_selection · 已落库 #"+hash); });
  // 隐藏浏览器视图里默认的浮动采集按钮（避免重复）
  const fb=$("#collectBtn"); if(fb) fb.style.display="none";""",
    focus="交互-采集优先"),

  # ---------- 11. 命令面板驱动（极简界面，⌘K 唤起一切） ----------
  dict(f="v11-command-palette.html", title="变体11 · 命令面板驱动",
    sub="界面极简，⌘K/Ctrl+K 唤起命令面板直达任意模块/动作，键盘流优先。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity" id="activity" style="width:40px;background:transparent">
        <div class="ic active" data-view="browser" style="font-size:22px;color:var(--blue)">⌘</div>
      </div>""",
    extra_css="""
  .activity { background:transparent !important; }
  .activity .ic { background:#eef3ff; width:36px; height:36px; }
  .activity .ic.active { background:var(--blue); color:#fff; }""",
    init="""
  // 命令面板变体：默认打开命令面板提示，点 ⌘ 也开
  const kic=document.querySelector("#activity .ic"); if(kic) kic.addEventListener("click",()=>$("#cmd")&&$("#cmd").classList.add("on"));
  const hint=document.createElement("div"); hint.className="placeholder"; hint.innerHTML="<div style='font-size:16px'>⌘ / Ctrl + K 唤起命令面板</div><div style='font-size:12px;color:var(--sub)'>输入“浏览/文件/宫格/采集…”直达</div>";
  $("#view-browser").appendChild(hint);
  setTimeout(()=>{ const c=$("#cmd"); if(c) c.classList.add("on"); },300);""",
    focus="交互-命令面板"),

  # ---------- 12. 极简 4 模块（仅 浏览/文件/成果/仓库） ----------
  dict(f="v12-minimal-4.html", title="变体12 · 极简 4 模块",
    sub="信息架构瘦身：仅保留 浏览 / 文件 / 成果 / 仓库 四个核心模块，降低认知负荷。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity" id="activity">
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
      </div>""",
    extra_css="",
    init="", focus="架构-极简4模块"),

  # ---------- 13. 全功能 9 模块 + 命令面板入口（双栏+分组） ----------
  dict(f="v13-full-ia.html", title="变体13 · 全功能 9 模块",
    sub="完整 9 模块 + 分组侧栏 + ⌘K 命令面板入口，功能最全，重度用户向。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity grouped" id="activity">
        <div class="grp">浏览</div>
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="grid">🗂️<span>宫格</span></div>
        <div class="ic" data-view="apps">🚀<span>应用</span></div>
        <div class="grp">创作</div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="clip">📋<span>剪贴板</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="grp">系统</div>
        <div class="ic" data-view="term">💻<span>终端</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
        <div class="ic" data-view="audit">🛡️<span>审计</span></div>
      </div>
      <div style="position:absolute;right:18px;top:44px;z-index:7"><button class="go" id="cmdOpen" style="border-radius:16px">⌘K 命令</button></div>""",
    extra_css="""
  .activity.grouped { width:104px; }
  .activity.grouped .grp { width:100%; text-align:left; padding:8px 10px 2px; font-size:10px; color:var(--activity-fg); opacity:.6; letter-spacing:1px; }
  .activity.grouped .ic { width:88px; flex-direction:row; justify-content:flex-start; gap:8px; font-size:12px; padding-left:12px; }
  .body { position:relative; }""",
    init="""
  const co=$("#cmdOpen"); if(co) co.addEventListener("click",()=>{ const c=$("#cmd"); if(c) c.classList.add("on"); });""",
    focus="架构-全功能9模块"),

  # ---------- 14. 响应式底部 Tab 栏（手机 App 形态） ----------
  dict(f="v14-mobile-tabbar.html", title="变体14 · 移动端底栏",
    sub="手机 App 形态：底部 Tab 栏导航，内容区全屏，适配小窗/平板触屏。",
    theme=THEME_BLUE,
    activity="""
      <div class="activity bottombar" id="activity">
        <div class="ic active" data-view="browser">📁<span>浏览</span></div>
        <div class="ic" data-view="files">📂<span>文件</span></div>
        <div class="ic" data-view="clip">📋<span>剪贴板</span></div>
        <div class="ic" data-view="arts">📚<span>成果</span></div>
        <div class="ic" data-view="grid">🗂️<span>宫格</span></div>
        <div class="ic" data-view="term">💻<span>终端</span></div>
        <div class="ic" data-view="repo">🛰️<span>仓库</span></div>
        <div class="ic" data-view="audit">🛡️<span>审计</span></div>
      </div>""",
    extra_css="""
  .stage { max-width:480px; margin:0 auto; }
  .body { flex-direction:column; height:560px; }
  .activity.bottombar { width:100%; flex-direction:row; height:60px; order:2; background:#1f2733; justify-content:space-around; padding-top:0; }
  .activity.bottombar .ic { width:auto; flex:1; font-size:18px; }
  .activity.bottombar .ic span { font-size:9px; }
  .main.view, .sidebar.view { order:1; }""",
    init="", focus="布局-移动底栏"),

  # ---------- 15. 专家改进版（宫格 iframe + 终端标注 + 原型≠成品横幅） ----------
  dict(f="v15-expert-fixes.html", title="变体15 · 专家评审改进版",
    sub="集成 11 位专家意见：宫格嵌真实 iframe + 终端标注“模拟” + “原型≠成品”横幅 + 采集含 hash 溯源。",
    theme=dict(BLUE="#2b6cb0",BLUE2="#4299e1",LINE="#e5e6eb",INK="#1f2329",SUB="#8a919f",SIDE="#f7f8fa",BG="#f0f2f5",
      ACTBG="#1f2733",ACTFG="#cbd5e0",ACTW="56",ACTDIR="flex-direction:column;",ACTBORDER="",ACTHOVER="#2f3a47",ACCENT="#2b6cb0",
      PILLBG="#e8f3ff",BANNERBG="#fff7e6",BANNERTX="#8a5a00"),
    activity=ACT_DEFAULT,
    extra_css="",
    banner="<div class='banner'>⚠️ <b>原型演示稿</b>：非成品，所有命令为前端模拟；真实交互见 Tauri 应用（bridge.rs）。已补：宫格嵌真实 iframe / 终端标注“模拟” / 采集含 hash 溯源。</div>",
    init="window.__gridIframe=true;", focus="综合-专家改进"),
]

def build(v):
    t = v["theme"]
    activity = v.get("activity", ACT_DEFAULT)
    extra_css = v.get("extra_css", "")
    banner = v.get("banner", "")
    init = v.get("init", "")
    # 收集所有被 activity 引用的 view（若变体把关掉某些模块，则省略对应 view）
    # 简单做法：始终包含全部 9 个 view，未引用的只是不会出现在活动栏
    views_html = "\n".join(VIEWS[k] for k in ["browser","files","clip","arts","grid","apps","term","repo","audit"])
    # 命令面板 + toast 容器
    tail = """
  <div class="cmd" id="cmd"><div class="cmd-box"><input id="cmdInput" placeholder="⌘ 输入命令（如 grid 4 / open 文件 / collect）…" /><div class="cmd-list" id="cmdList"></div></div></div>
  <div class="toast" id="toast"></div>"""
    html = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>{v['title']}</title>
<style>
  :root {{
    --blue:{t['BLUE']}; --blue2:{t['BLUE2']}; --line:{t['LINE']}; --ink:{t['INK']}; --sub:{t['SUB']}; --side:{t['SIDE']};
    --bg:{t['BG']}; --activity-bg:{t['ACTBG']}; --activity-fg:{t['ACTFG']}; --activity-hover:{t['ACTHOVER']}; --accent:{t['ACCENT']};
    --pillbg:{t['PILLBG']}; --bannerbg:{t['BANNERBG']}; --bannertx:{t['BANNERTX']};
  }}
{BASE_CSS}
{extra_css}
</style>
</head>
<body>
  <h1>{v['title']} <span class="pill">方案 D：单窗口多 Webview</span></h1>
  <div class="sub">{v['sub']}</div>
  {banner}
  <div class="stage">
    <div class="titlebar"><span class="dots"><i></i><i></i><i></i></span> 浏览器OS融合</div>
    <div class="body">
      {activity}
      {views_html}
    </div>
  </div>
  <div class="sub" style="text-align:center;max-width:1180px;margin:14px auto 0">
    提示：所有交互均为前端模拟，对应 bridge.rs 命令（tab_new / tab_open / create_grid / term_spawn / collect_selection / request_sync / confirm_sync / list_dir / read_file / write_file / clipboard_read / clipboard_write 等）。剪贴板历史存 localStorage（前 50 条）。
  </div>
{tail}
<script>
{COMMON_JS}
{init}
</script>
</body>
</html>
"""
    return html

for v in VARIANTS:
    with open(os.path.join(OUT, v["f"]), "w", encoding="utf-8") as f:
        f.write(build(v))
    print("generated:", v["f"], "·", v["focus"])

# ---------- 索引画廊页 ----------
cards = "".join(
    f'<a class="card" href="variants/{html.escape(v["f"])}" target="_blank"><div class="ct">{html.escape(v["title"])}</div><div class="cf">{html.escape(v["focus"])}</div><div class="cs">{html.escape(v["sub"])}</div></a>'
    for v in VARIANTS
)
index = f"""<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="UTF-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>浏览器OS 原型变体画廊（15 版 + 原始）</title>
<style>
*{{box-sizing:border-box;margin:0;padding:0}}
body{{font-family:-apple-system,"Segoe UI","Microsoft YaHei",sans-serif;background:#f0f2f5;color:#1f2329;padding:28px}}
h1{{font-size:22px}} .sub{{color:#8a919f;font-size:13px;margin:6px 0 18px}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(260px,1fr));gap:16px;max-width:1180px;margin:0 auto}}
.card{{display:block;text-decoration:none;background:#fff;border:1px solid #e5e6eb;border-radius:12px;padding:16px;transition:.15s;box-shadow:0 2px 8px rgba(0,0,0,.05)}}
.card:hover{{box-shadow:0 6px 20px rgba(43,108,176,.18);transform:translateY(-2px);border-color:#2b6cb0}}
.ct{{font-weight:600;font-size:15px;color:#1f2329}}
.cf{{display:inline-block;margin:8px 0;font-size:11px;background:#e8f3ff;color:#2b6cb0;border-radius:8px;padding:2px 8px}}
.cs{{font-size:12px;color:#6b7280;line-height:1.6}}
.note{{max-width:1180px;margin:0 auto 16px;background:#fff7e6;border:1px solid #ffe0a3;border-radius:10px;padding:12px 16px;font-size:12px;color:#8a5a00}}
a.orig{{color:#2b6cb0;font-weight:600}}
.legend{{max-width:1180px;margin:0 auto 16px;background:#fff;border:1px solid #e5e6eb;border-radius:10px;padding:12px 16px;font-size:12px;color:#4e5969;line-height:1.8}}
.legend b{{color:#1f2329}}
</style></head>
<body>
<h1>浏览器OS 融合 · 产品原型比较画廊（15 版差异化）</h1>
<div class="sub">共 16 个版本：原始综合版 + 15 个【骨架/布局/交互各不同】的变体。V2 重写：每个变体有独立信息架构与首屏交互，不再是换皮。</div>
<div class="note">📌 原始综合版（方案 D 完整原型，9 模块齐全）在此：<a class="orig" href="../prototype.html" target="_blank">../prototype.html</a> —— 本任务中未改动。下方 15 个为差异化比较稿。</div>
<div class="legend"><b>15 版差异速览：</b> ① 极简图标栏 ② 顶部Tab导航 ③ 分组侧栏 ④ 可折叠抽屉 ⑤ 暗色工作台(终端常驻底栏) ⑥ 极简卡片墙门户 ⑦ 薄荷绿消费级 ⑧ 紫橙渐变年轻化 ⑨ 宫格对比优先(2×2真实iframe) ⑩ 采集优先(右侧常驻面板) ⑪ 命令面板驱动(⌘K) ⑫ 极简4模块 ⑬ 全功能9模块+命令面板 ⑭ 移动端底栏 ⑮ 专家改进版(原型≠成品横幅)</div>
<div class="grid">
<a class="card" href="../prototype.html" target="_blank"><div class="ct">★ 原始综合版（基准）</div><div class="cf">基准-全功能</div><div class="cs">方案 D 完整原型，9 模块齐全，作为比较基准，本任务未改。</div></a>
{cards}
</div>
</body></html>
"""
with open(os.path.join(os.path.dirname(__file__), "prototype-index.html"), "w", encoding="utf-8") as f:
    f.write(index)
print("generated: prototype-index.html (gallery)")
print("total variants:", len(VARIANTS))
