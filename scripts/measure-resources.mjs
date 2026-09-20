#!/usr/bin/env node
// ---------------------------------------------------------------------------
// measure-resources.mjs — 真实资源测量 harness（Phase 8E / Train F）
//
// 诚实分类（Train F 验收 §14）：
//   MEASURED  = 有运行中的目标进程且 /proc 可读 → 真实读数
//   DECLARED  = 能力声明了资源类别（capabilities.yaml resources.class），但未实测
//   UNKNOWN   = 无运行中进程 / /proc 不可读 → 不估算、不乱报
//
// 禁止：用「经验值 / 估算」冒充 MEASURED。读不到就标 UNKNOWN。
//
// 用法:
//   node scripts/measure-resources.mjs                 # 自动探测 MVP_BROWSER_PID / 无则 UNKNOWN
//   node scripts/measure-resources.mjs --pid <PID>
//   node scripts/measure-resources.mjs --json
//
// 输出：各维度 { value, classification }；无运行时进程时 value=null, classification=UNKNOWN。
// ---------------------------------------------------------------------------

import { readdirSync, readFileSync, existsSync, statSync } from "node:fs";
import { join } from "node:path";

function classify(measured) {
  return measured ? "MEASURED" : "UNKNOWN";
}

function readProc(pid) {
  const base = `/proc/${pid}`;
  if (!existsSync(base)) return null;
  let status = "";
  let cmdline = "";
  try {
    status = readFileSync(join(base, "status"), "utf8");
    cmdline = readFileSync(join(base, "cmdline"), "utf8");
  } catch {
    return null;
  }
  const rss = (status.match(/^VmRSS:\s+(\d+)\s*kB/m) || [])[1];
  // 子进程：扫描 /proc/*/stat 的 PPID == pid
  const children = [];
  try {
    for (const d of readdirSync("/proc")) {
      if (!/^\d+$/.test(d)) continue;
      if (d === String(pid)) continue;
      try {
        const st = readFileSync(join("/proc", d, "stat"), "utf8");
        const m = st.match(/^\(.*\)\s+\w+\s+(\d+)/);
        if (m && m[1] === String(pid)) children.push(Number(d));
      } catch { /* ignore */ }
    }
  } catch { /* ignore */ }
  return {
    pid,
    rssKb: rss ? Number(rss) : null,
    cmdline: cmdline.replace(/\0/g, " ").trim(),
    childCount: children.length,
    children,
  };
}

// PTY master 计数：扫描所有进程的 fd 符号链接是否指向 /dev/pts/N
function countPtyMasters() {
  let n = 0;
  try {
    for (const d of readdirSync("/proc")) {
      if (!/^\d+$/.test(d)) continue;
      const fdDir = join("/proc", d, "fd");
      let fds;
      try { fds = readdirSync(fdDir); } catch { continue; }
      for (const fd of fds) {
        try {
          const link = readFileSync(join(fdDir, fd), "utf8");
          if (/^\/dev\/pts\/\d+$/.test(link)) n += 1;
        } catch { /* ignore */ }
      }
    }
  } catch { /* ignore */ }
  return n;
}

// WebView 子进程启发式（cmdline 含 webkit/gtk/chromium/tauri webview 特征）
function countWebViewProcs() {
  let n = 0;
  try {
    for (const d of readdirSync("/proc")) {
      if (!/^\d+$/.test(d)) continue;
      try {
        const cl = readFileSync(join("/proc", d, "cmdline"), "utf8").replace(/\0/g, " ");
        if (/webkit|gtkwebkit|chromium|wry|tauri.*webview/i.test(cl)) n += 1;
      } catch { /* ignore */ }
    }
  } catch { /* ignore */ }
  return n;
}

function main() {
  const argv = process.argv.slice(2);
  const json = argv.includes("--json");
  let pid = null;
  const pi = argv.indexOf("--pid");
  if (pi >= 0 && argv[pi + 1]) pid = Number(argv[pi + 1]);
  if (pid == null) pid = Number(process.env.MVP_BROWSER_PID || "") || null;

  const proc = pid != null ? readProc(pid) : null;
  const measured = proc != null;

  const webview = countWebViewProcs();
  const pty = countPtyMasters();

  const report = {
    target: { pid: pid ?? (process.env.MVP_BROWSER_PID || null), found: measured },
    mainProcessRssKb: { value: proc?.rssKb ?? null, classification: classify(measured) },
    childProcessCount: {
      value: measured ? proc.childCount : null,
      classification: classify(measured),
    },
    webkitProcessCount: { value: measured ? webview : null, classification: classify(measured) },
    ptyMasterCount: { value: measured ? pty : null, classification: classify(measured) },
    // 能力声明（DECLARED，非实测）：来自 capabilities.yaml resources.class
    declaredResources: {
      bookmark: [], workspace: [], browser: ["PROCESS", "WEBVIEW"], terminal: ["PROCESS", "PTY"],
    },
    note: measured
      ? "MEASURED：基于运行中进程 /proc 真实读数"
      : "UNKNOWN：当前无可测量目标进程（CI/headless 无运行中的 mvp-browser-os）。禁止用估算冒充 MEASURED；如需实测，启动应用后设 MVP_BROWSER_PID 或 --pid 复跑。",
  };

  if (json) console.log(JSON.stringify(report, null, 2));
  else {
    console.log(`target.pid=${report.target.pid} found=${report.target.found}`);
    for (const k of ["mainProcessRssKb", "childProcessCount", "webkitProcessCount", "ptyMasterCount"]) {
      const r = report[k];
      console.log(`  ${k}: value=${r.value} [${r.classification}]`);
    }
    console.log(report.note);
  }
  return 0;
}

const isMain = process.argv[1] && process.argv[1].includes("measure-resources.mjs");
if (isMain) process.exit(main());
