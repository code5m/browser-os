#!/usr/bin/env node
// A5 · M5-W18-R3 数据库工作台（浏览器优先外壳内）合成状态/布局模型
// ---------------------------------------------------------------------------
// 派发：M5-W18-R3B（`M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A5），MODE=RESEARCH_AND_PROTOTYPE，W19=CLOSED。
// 本波为 R3B 收口：对齐 A0 裁决的规范几何（top chrome 60 / status 24 / activity strip 28）；
// 采纳六个验收尺寸（含 900×600 产品最小窗、1200×800）；Collapse All 含固定窗口、Restore 还原可见/尺寸/固定态（A3/A4 裁决）。
// 性质：纯研究资产。不 import 任何 src/ 代码、不发 IPC、不读用户数据、不改产品代码。
//
// 本文件是 A5-R3 的**规范源（normative source）**：
//   1) 布局常量（CHROME/TOOLS/TREE_LIMITS）与 `A5-R3-prototype.html` 内 CSS/JS 常量必须逐值一致；
//      G7 断言组会读取同目录 HTML 做逐项同步校验（防止「文档说一套、原型画另一套」）。
//   2) 全局验收约束（任务卡 §Global acceptance constraints）中可被算术判定的条款，
//      在此以断言形式固化：顶部 chrome ≤2 行且 ≤80px；1440×900 收起态活动内容
//      ≥85% 后标题栏高度且 ≥92% 宽度；专注态归还全视口；每边默认仅一个主工具窗口。
//   3) 右键菜单 = 命令注册表单一真源：每个动作必须有 id / 标签 / 键盘或无障碍路径 /
//      禁用原因 / 安全分级 / 审计分级 / 确认档位 / 后端依赖标注。
//
// 运行：node logs/research/M5-W18/A5-R3-state-model.mjs
// 退出码 0 = 全部断言通过。

import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

// ===========================================================================
// §1 布局常量（与 A5-R3-prototype.html 的 --* CSS 变量 / LAYOUT 常量逐值一致）
// ===========================================================================

/** 外壳 chrome：浏览器优先 = 第 1 行统一标签条（兼作标题栏），第 2 行导航/地址行。 */
export const CHROME = {
  /** 行 1：统一标签条（浏览器页 + 模块文档），兼作标题栏基准。 */
  tabStrip: 28,
  /** 行 2：导航 / 地址 / 全局搜索行。 */
  navRow: 32,
  /** 底部状态栏（含各边工具窗口开关，收起态下是「恢复」入口之一）。A0 裁决：24px。 */
  statusBar: 24,
  /** 左侧工具图标细条（收起态仍在，作为唯一常驻工具入口）。 */
  railLeft: 28,
  /** 数据库文档内部的 SQL 子标签条：仅当 SQL 文档数 ≥2 时占位。 */
  subTabs: 26,
  /** SQL 控制台状态行（运行/取消/execId/快捷键提示）。 */
  consoleStatus: 22,
};

/** 顶部 chrome 行数（外壳级，不含文档内部子标签）。 */
export const TOP_CHROME_ROWS = 2;

/** 工具窗口尺寸（默认/最小/最大）。bottom 用比例上限，避免小窗口下挤死内容。 */
export const TOOLS = {
  left: { def: 260, min: 180, max: 480 },
  right: { def: 280, min: 200, max: 420 },
  bottom: { def: 240, min: 120, maxRatio: 0.8 },
};

/** 层级视图（连接/库表树、属性树）展开上限。 */
export const TREE_LIMITS = { maxVisibleNodes: 250, maxDepth: 6 };

/** 验收目标尺寸（R3B A0 裁决：1920×1080 / 1440×900 / 1366×768 / 1200×800 / 1024×720 / 900×600；800×600 低于产品最小窗，弃用）。 */
export const TARGET_SIZES = [
  { label: "1920×1080", width: 1920, height: 1080 },
  { label: "1440×900", width: 1440, height: 900 },
  { label: "1366×768", width: 1366, height: 768 },
  { label: "1200×800", width: 1200, height: 800 },
  { label: "1024×720", width: 1024, height: 720 },
  { label: "900×600", width: 900, height: 600 },
];

/** 预算红线：仅 1440×900 收起态被任务卡显式约束，其余尺寸按同一红线自查。 */
export const BUDGET = { minHeightPct: 85, minWidthPct: 92, maxTopChromePx: 80 };

/** 后端已冻结的数据库命令（`src-tauri/src/bridge.rs` 实测，2026-09-08）。 */
export const FROZEN_DB_COMMANDS = ["db_connect", "db_query", "db_disconnect"];

/** 后端**未注册**、但 UX 需要的命令（原型内一律渲染为禁用 + 原因）。 */
export const MISSING_DB_COMMANDS = [
  "db_cancel",
  "db_list_connections",
  "db_list_schema",
  "db_export_result",
  "db_query_history",
  "db_row_write",
];

// ===========================================================================
// §2 工具窗口 / 布局状态机（纯函数，可直接移植到 useLayoutStore + useDatabaseStore）
// ===========================================================================

/**
 * 建立初始外壳状态。
 * 语义要点：
 *  - 每边是一个 tab 组，但**默认只有一个成员**（任务卡：每边默认仅一个主工具窗口）。
 *  - `mode` = docked | overlay(自动隐藏) | hidden；只有 docked 才消耗视口。
 *  - `pinned` 决定同边打开另一个工具窗口时「替换」还是「共存为同边标签」。
 */
export function createShell() {
  return {
    view: "normal", // normal | collapsed | focus
    docCount: 1,
    edges: {
      left: { members: ["db.tool.connections"], visible: "db.tool.connections", mode: "docked", size: TOOLS.left.def, pinned: [] },
      right: { members: [], visible: null, mode: "hidden", size: TOOLS.right.def, pinned: [] },
      bottom: { members: ["db.tool.result"], visible: "db.tool.result", mode: "docked", size: TOOLS.bottom.def, pinned: [] },
    },
    /** 收起全部时保存的上一次布局快照（恢复上次布局的唯一来源）。 */
    previousLayout: null,
    /** 展开活动窗口前的尺寸，用于反向还原。 */
    expandedFrom: null,
    /** 专注态返回动作（任务卡：必须保留一个明显的返回动作）。 */
    focusReturnAction: null,
  };
}

function cloneEdges(edges) {
  return JSON.parse(JSON.stringify(edges));
}

/** 打开某边的工具窗口：未固定则替换在位者，已固定则共存为同边标签。 */
export function openTool(shell, edge, toolId) {
  const e = shell.edges[edge];
  const incumbent = e.visible;
  if (incumbent && incumbent !== toolId && !e.pinned.includes(incumbent)) {
    e.members = e.members.filter((m) => m !== incumbent); // 未固定 → 关闭并替换
  }
  if (!e.members.includes(toolId)) e.members.push(toolId);
  e.visible = toolId;
  if (e.mode === "hidden") e.mode = "docked";
  return shell;
}

export function closeTool(shell, edge) {
  const e = shell.edges[edge];
  if (!e.visible) return shell;
  e.members = e.members.filter((m) => m !== e.visible);
  e.pinned = e.pinned.filter((m) => m !== e.visible);
  e.visible = e.members.length ? e.members[e.members.length - 1] : null;
  if (!e.visible) e.mode = "hidden";
  return shell;
}

export function pinTool(shell, edge) {
  const e = shell.edges[edge];
  if (e.visible && !e.pinned.includes(e.visible)) e.pinned.push(e.visible);
  if (e.mode === "overlay") e.mode = "docked"; // 固定即不再自动隐藏
  return shell;
}

export function unpinTool(shell, edge) {
  const e = shell.edges[edge];
  e.pinned = e.pinned.filter((m) => m !== e.visible);
  return shell;
}

/** 自动隐藏：转悬浮层，不再消耗视口（固定态拒绝自动隐藏）。 */
export function autoHideTool(shell, edge) {
  const e = shell.edges[edge];
  if (!e.visible) return shell;
  if (e.pinned.includes(e.visible)) return shell; // 固定优先级更高，幂等拒绝
  e.mode = "overlay";
  return shell;
}

export function resizeTool(shell, edge, next, contentHeightForRatio) {
  const e = shell.edges[edge];
  if (edge === "bottom") {
    const max = Math.floor((contentHeightForRatio ?? 0) * TOOLS.bottom.maxRatio);
    e.size = Math.min(Math.max(next, TOOLS.bottom.min), Math.max(max, TOOLS.bottom.min));
  } else {
    const spec = TOOLS[edge];
    e.size = Math.min(Math.max(next, spec.min), spec.max);
  }
  return shell;
}

/** 收起全部：先存快照再全隐（快照是「恢复上次布局」的唯一来源）。 */
export function collapseAll(shell) {
  shell.previousLayout = { view: shell.view, edges: cloneEdges(shell.edges) };
  for (const edge of Object.keys(shell.edges)) {
    shell.edges[edge].mode = "hidden";
  }
  shell.view = "collapsed";
  return shell;
}

export function restorePreviousLayout(shell) {
  if (!shell.previousLayout) return shell;
  shell.edges = cloneEdges(shell.previousLayout.edges);
  shell.view = shell.previousLayout.view;
  shell.previousLayout = null;
  return shell;
}

/** 展开活动工具窗口（IDEA maximize tool window）；可反向还原。 */
export function expandActive(shell, edge, contentHeight) {
  const e = shell.edges[edge];
  if (!e.visible || e.mode !== "docked") return shell;
  shell.expandedFrom = { edge, size: e.size };
  if (edge === "bottom") e.size = Math.floor(contentHeight * TOOLS.bottom.maxRatio);
  else e.size = TOOLS[edge].max;
  return shell;
}

export function shrinkActive(shell) {
  if (!shell.expandedFrom) return shell;
  const { edge, size } = shell.expandedFrom;
  shell.edges[edge].size = size;
  shell.expandedFrom = null;
  return shell;
}

/** 专注模式：隐藏所有工具 chrome，仅保留标题栏级标签条 + 一个明显返回动作。 */
export function enterFocus(shell) {
  shell.previousLayout = { view: shell.view, edges: cloneEdges(shell.edges) };
  for (const edge of Object.keys(shell.edges)) shell.edges[edge].mode = "hidden";
  shell.view = "focus";
  shell.focusReturnAction = { id: "db.toolwindow.focusMode", label: "退出专注", keyboard: "Esc / F11" };
  return shell;
}

export function exitFocus(shell) {
  shell.focusReturnAction = null;
  return restorePreviousLayout(shell);
}

/** 当前是否有任何工具 chrome 可见（专注态必须为 false）。 */
export function toolChromeVisible(shell) {
  if (shell.view === "focus") return false;
  return true; // 收起态仍有 rail + 状态栏（这是刻意保留的恢复入口，不算「工具窗口占位」）
}

// ===========================================================================
// §3 视口预算计算（外壳内容区 + 严格活动面）
// ===========================================================================

/**
 * @param {{width:number,height:number}} size
 * @param {ReturnType<typeof createShell>} shell
 * 返回 docked 尺寸、内容区、严格活动面与百分比。overlay/hidden 边一律计 0。
 */
export function computeLayout(size, shell) {
  const focus = shell.view === "focus";
  const dock = (edge) => (shell.edges[edge].mode === "docked" && !focus ? shell.edges[edge].size : 0);

  const topChromeHeight = CHROME.tabStrip + CHROME.navRow;
  const railW = focus ? 0 : CHROME.railLeft;
  const statusH = focus ? 0 : CHROME.statusBar;

  // 后标题栏高度：以行 1（统一标签条）为标题栏基准。
  const postTitlebarHeight = size.height - CHROME.tabStrip;

  const contentWidth = size.width - railW - dock("left") - dock("right");
  const contentHeight = size.height - CHROME.tabStrip - (focus ? 0 : CHROME.navRow) - statusH - dock("bottom");

  // 严格活动面：再扣掉文档内部子标签条（仅 ≥2 文档时）与控制台状态行；专注态两者都隐藏。
  const subTabs = !focus && shell.docCount >= 2 ? CHROME.subTabs : 0;
  const consoleStatus = focus ? 0 : CHROME.consoleStatus;
  const activeHeight = contentHeight - subTabs - consoleStatus;
  const activeWidth = contentWidth;

  const pct = (a, b) => Math.round((a / b) * 10000) / 100;
  return {
    topChromeRows: TOP_CHROME_ROWS,
    topChromeHeight,
    postTitlebarHeight,
    contentWidth,
    contentHeight,
    activeWidth,
    activeHeight,
    contentHeightPct: pct(contentHeight, postTitlebarHeight),
    contentWidthPct: pct(contentWidth, size.width),
    activeHeightPct: pct(activeHeight, postTitlebarHeight),
    activeWidthPct: pct(activeWidth, size.width),
    // A0 规范几何：collapsed active height = (H-60-24)/H；active width = (W-28)/W。
    canonicalHeightPct: pct(contentHeight, size.height),
    canonicalWidthPct: pct(contentWidth, size.width),
  };
}

// ===========================================================================
// §4 层级视图行为（全部折叠 / 展开一层 / 有界全展开）
// ===========================================================================

/** 合成树：一个刻意超限的分支（400 个表）用于演示有界展开的截断提示。 */
export function makeSyntheticTree() {
  const tables = Array.from({ length: 400 }, (_, i) => ({ id: `t${i}`, label: `table_${i}`, children: [] }));
  return {
    id: "root",
    label: "连接",
    children: [
      {
        id: "conn.local",
        label: "local-sqlite",
        children: [
          { id: "conn.local.main", label: "main", children: [{ id: "conn.local.main.tables", label: "表", children: tables.slice(0, 3) }] },
        ],
      },
      {
        id: "conn.replica",
        label: "prod-replica",
        children: [
          { id: "conn.replica.public", label: "public", children: [{ id: "conn.replica.public.tables", label: "表", children: tables }] },
          { id: "conn.replica.info", label: "information_schema", children: [] },
        ],
      },
    ],
  };
}

export function collapseAllNodes() {
  return { expanded: new Set(), depth: 0, truncated: false, notice: "" };
}

/** 展开一层：把当前已展开的最深一层的子节点纳入展开集合（深度 +1，受 maxDepth 约束）。 */
export function expandOneLevel(tree, state) {
  if (state.depth >= TREE_LIMITS.maxDepth) {
    return { ...state, truncated: true, notice: `已达最大层级 ${TREE_LIMITS.maxDepth}，未继续展开` };
  }
  const expanded = new Set(state.expanded);
  const walk = (node, d) => {
    if (d <= state.depth) expanded.add(node.id);
    if (d < state.depth) node.children.forEach((c) => walk(c, d + 1));
  };
  walk(tree, 0);
  return { expanded, depth: state.depth + 1, truncated: false, notice: "" };
}

/** 有界全展开：BFS 到节点上限即停并显式说明（禁止无界展开冻结 UI）。 */
export function boundedExpandAll(tree) {
  const expanded = new Set();
  let visible = 1;
  let truncated = false;
  const queue = [{ node: tree, depth: 0 }];
  while (queue.length) {
    const { node, depth } = queue.shift();
    if (depth >= TREE_LIMITS.maxDepth) {
      if (node.children.length) truncated = true;
      continue;
    }
    if (visible + node.children.length > TREE_LIMITS.maxVisibleNodes) {
      truncated = true;
      continue;
    }
    if (node.children.length) {
      expanded.add(node.id);
      visible += node.children.length;
      node.children.forEach((c) => queue.push({ node: c, depth: depth + 1 }));
    }
  }
  return {
    expanded,
    visible,
    truncated,
    notice: truncated ? `已展开 ${visible} 个节点，达到 ${TREE_LIMITS.maxVisibleNodes} 上限；未完全展开（点击具体节点继续）` : "",
  };
}

// ===========================================================================
// §5 SQL 多文档状态（沿用 A5-R2B 已验证算法：execId 陈旧守卫 + 取消幂等）
// ===========================================================================

export function createSqlDoc(id, title, connId = null) {
  return { id, title, sql: "", connId, execId: null, seq: 0, status: "idle", result: null, error: null, dirty: false };
}

export function startRun(doc, sql, connId, makeExecId) {
  if (doc.status === "running") return doc;
  return { ...doc, sql, connId, execId: makeExecId(), seq: doc.seq + 1, status: "running", error: null };
}

export function applyDocResult(doc, execId, result, error) {
  if (doc.execId !== execId) return doc; // 陈旧响应丢弃：切换文档时旧请求不得覆盖新文档
  const status = error
    ? "error"
    : result && result.state === "timeout"
      ? "timeout"
      : result && result.state === "cancelled"
        ? "cancelled"
        : "success";
  return { ...doc, result: result ?? null, error: error ?? null, status };
}

export function cancelDoc(doc) {
  if (doc.status !== "running") return doc;
  return { ...doc, status: "cancelled", execId: null };
}

/** 结果告警（与产品 `src/utils/dbUi.ts:buildResultView` 的文案口径一致，此处为合成镜像）。 */
export const DB_LIMITS_MIRROR = { maxRows: 1000, maxResultBytes: 4 * 1024 * 1024, maxTextFieldBytes: 64 * 1024, defaultTimeoutSecs: 30 };

export function resultWarnings(result) {
  const w = [];
  if (result.truncated) w.push(`结果已被截断（命中行数上限：${DB_LIMITS_MIRROR.maxRows} 行 / 4.00 MiB），展示的不是完整结果集`);
  if (result.field_truncated) w.push(`存在超过 64.00 KiB 的长字段被截断`);
  if (result.state === "cancelled") w.push("查询已被取消，结果不完整");
  if (result.state === "timeout") w.push("查询超时被终止，结果不完整");
  if (result.state === "failed") w.push("查询执行失败，结果不完整");
  return w;
}

// ===========================================================================
// §6 右键菜单 = 命令注册表（对象作用域内聚，禁止大杂烩）
// ===========================================================================

const A11Y_MENU = "Shift+F10 打开菜单 → ↑/↓ 选择 → Enter 执行 → Esc 关闭";

/**
 * safety：READ（只读）｜LOCAL（仅本地 UI/文档状态）｜DB_WRITE（可改库数据/结构）｜DESTRUCTIVE（不可逆）
 * confirm：none ｜ single（单次确认）｜ typed（键入对象名确认）
 * audit：none ｜ audit（须进审计，脱敏后）
 * backend：已冻结命令名 ｜ null（纯前端）｜ MISSING:<命令名>（后端未注册 → 原型内禁用）
 */
function A(id, label, opts) {
  return {
    id,
    label,
    keyboard: opts.keyboard ?? "",
    a11yPath: A11Y_MENU,
    safety: opts.safety,
    confirm: opts.confirm ?? "none",
    audit: opts.audit ?? "none",
    backend: opts.backend ?? null,
    precondition: opts.precondition,
    disabledReason: opts.disabledReason,
  };
}

export function buildDbCommandRegistry() {
  return [
    {
      scope: "db.connection",
      label: "连接节点（左侧连接树）",
      actions: [
        A("db.connection.connect", "连接", { keyboard: "Enter", safety: "READ", backend: "db_connect", precondition: "该连接已启用且当前未连接", disabledReason: "连接已禁用或已处于连接态" }),
        A("db.connection.disconnect", "断开", { safety: "LOCAL", backend: "db_disconnect", audit: "audit", precondition: "存在活动连接", disabledReason: "当前未连接，无可断开的会话" }),
        A("db.connection.newConsole", "新建 SQL 控制台", { keyboard: "Ctrl/Cmd+Alt+N", safety: "LOCAL", precondition: "已选中连接", disabledReason: "未选中连接，无法确定控制台归属" }),
        A("db.connection.edit", "编辑连接", { keyboard: "F2", safety: "LOCAL", precondition: "无在途查询", disabledReason: "该连接有查询在途，先取消或等待完成" }),
        A("db.connection.toggleWrite", "切换写权限", { safety: "DB_WRITE", confirm: "single", audit: "audit", precondition: "生产判定为非生产", disabledReason: "生产判定为生产或未知（未知按生产处理），禁止开启写权限" }),
        A("db.connection.refreshSchema", "刷新结构", { keyboard: "F5", safety: "READ", backend: "MISSING:db_list_schema", precondition: "后端提供结构枚举命令", disabledReason: "依赖未注册命令 db_list_schema，当前无法枚举库表" }),
        A("db.connection.copyLabel", "复制连接标签（脱敏）", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "已选中连接", disabledReason: "未选中连接" }),
        A("db.connection.remove", "移除连接", { keyboard: "Delete", safety: "DESTRUCTIVE", confirm: "typed", audit: "audit", backend: "MISSING:db_list_connections", precondition: "后端提供连接登记簿写命令", disabledReason: "依赖未注册命令 db_list_connections（连接登记簿仅在后端，前端无增删通道）" }),
      ],
    },
    {
      scope: "db.schema",
      label: "库 / 模式节点",
      actions: [
        A("db.schema.expandOneLevel", "展开一层", { keyboard: "→", safety: "READ", precondition: "节点有未展开子节点且未达最大层级", disabledReason: "该节点无子节点或已达最大层级 6" }),
        A("db.schema.collapseAll", "全部折叠", { keyboard: "Ctrl/Cmd+-", safety: "READ", precondition: "树中存在已展开节点", disabledReason: "树已全部折叠" }),
        A("db.schema.expandAllBounded", "有界全展开", { keyboard: "Ctrl/Cmd+Shift+=", safety: "READ", precondition: "节点数未超 250 上限", disabledReason: "子树超过 250 节点上限，仅能逐层展开（避免卡死 UI）" }),
        A("db.schema.newConsole", "在此模式打开控制台", { safety: "LOCAL", precondition: "所属连接已连接", disabledReason: "所属连接未连接" }),
        A("db.schema.copyName", "复制名称", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "已选中节点", disabledReason: "未选中节点" }),
      ],
    },
    {
      scope: "db.table",
      label: "表 / 视图节点",
      actions: [
        A("db.table.previewRows", "预览前 100 行", { keyboard: "Enter", safety: "READ", backend: "db_query", precondition: "已连接且无在途查询", disabledReason: "未连接或已有查询在途" }),
        A("db.table.generateSelect", "生成 SELECT 到控制台", { safety: "LOCAL", precondition: "存在活动 SQL 文档或可新建", disabledReason: "无可写入的控制台文档" }),
        A("db.table.showProperties", "查看属性", { keyboard: "Ctrl/Cmd+I", safety: "READ", backend: "MISSING:db_list_schema", precondition: "后端提供结构枚举命令", disabledReason: "依赖未注册命令 db_list_schema，属性只能显示结果集列元数据" }),
        A("db.table.copyQualifiedName", "复制限定名", { keyboard: "Ctrl/Cmd+Shift+C", safety: "READ", precondition: "已选中表/视图", disabledReason: "未选中表/视图" }),
        A("db.table.truncate", "清空表", { safety: "DESTRUCTIVE", confirm: "typed", audit: "audit", backend: "db_query", precondition: "后端放行 DDL（当前恒不放行）", disabledReason: "TRUNCATE 属 DDL，后端 require_write_confirmation 对 Ddl/Admin 一律返回 WriteDenied（security_policy.rs:1218），与写权限开关无关" }),
      ],
    },
    {
      scope: "db.column",
      label: "列节点",
      actions: [
        A("db.column.copyName", "复制列名", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "已选中列", disabledReason: "未选中列" }),
        A("db.column.generateWhere", "生成 WHERE 条件", { safety: "LOCAL", precondition: "存在活动 SQL 文档", disabledReason: "无活动 SQL 文档" }),
        A("db.column.showProperties", "查看列属性", { keyboard: "Ctrl/Cmd+I", safety: "READ", backend: "MISSING:db_list_schema", precondition: "后端提供结构枚举命令", disabledReason: "依赖未注册命令 db_list_schema" }),
      ],
    },
    {
      scope: "db.console",
      label: "SQL 文档标签",
      actions: [
        A("db.console.run", "运行", { keyboard: "Ctrl/Cmd+Enter", safety: "READ", backend: "db_query", audit: "audit", precondition: "已连接、SQL 非空且 ≤64KiB、无在途查询", disabledReason: "未连接 / SQL 为空或超 64KiB / 已有查询在途" }),
        A("db.console.cancel", "取消查询", { keyboard: "Ctrl/Cmd+F2", safety: "READ", backend: "MISSING:db_cancel", precondition: "有在途查询且后端提供取消命令", disabledReason: "依赖未注册命令 db_cancel；当前只能等默认超时 30s 或断开连接" }),
        A("db.console.close", "关闭文档", { keyboard: "Ctrl/Cmd+W", safety: "LOCAL", confirm: "single", precondition: "无在途查询；有未保存内容时需确认", disabledReason: "查询在途，先取消再关闭（避免留下孤儿请求）" }),
        A("db.console.closeOthers", "关闭其它文档", { safety: "LOCAL", confirm: "single", precondition: "存在 ≥2 个文档且其余无在途查询", disabledReason: "仅一个文档，或其它文档有查询在途" }),
        A("db.console.rename", "重命名文档", { keyboard: "F2", safety: "LOCAL", precondition: "已选中文档标签", disabledReason: "未选中文档标签" }),
        A("db.console.duplicate", "复制为新文档", { safety: "LOCAL", precondition: "已选中文档标签", disabledReason: "未选中文档标签" }),
        A("db.console.switchConnection", "切换连接", { keyboard: "Ctrl/Cmd+Alt+K", safety: "LOCAL", precondition: "无在途查询", disabledReason: "查询在途，切换连接会使结果归属不明" }),
      ],
    },
    {
      scope: "db.editor",
      label: "SQL 编辑器 / 选区",
      actions: [
        A("db.editor.runSelection", "运行选中片段", { keyboard: "Ctrl/Cmd+Enter", safety: "READ", backend: "db_query", audit: "audit", precondition: "有非空选区且已连接", disabledReason: "无选区或未连接" }),
        A("db.editor.copy", "复制", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "有非空选区", disabledReason: "编辑器内无非空选区" }),
        A("db.editor.commentLines", "注释 / 取消注释", { keyboard: "Ctrl/Cmd+/", safety: "LOCAL", precondition: "光标在编辑器内", disabledReason: "焦点不在编辑器" }),
        A("db.editor.insertTableName", "插入选中表名", { safety: "LOCAL", precondition: "树中已选中表", disabledReason: "树中未选中表" }),
      ],
    },
    {
      scope: "db.grid.cell",
      label: "结果单元格",
      actions: [
        A("db.grid.cell.copy", "复制单元格", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "已选中单元格", disabledReason: "未选中单元格" }),
        A("db.grid.cell.viewFull", "查看完整值", { keyboard: "Shift+Enter", safety: "READ", precondition: "值未被 64KiB 单字段上限截断", disabledReason: "该字段已被 64.00 KiB 单字段上限截断，完整值不在前端（需缩小取数范围重查）" }),
        A("db.grid.cell.edit", "编辑值", { safety: "DB_WRITE", confirm: "single", audit: "audit", backend: "MISSING:db_row_write", precondition: "后端提供行级写通道", disabledReason: "首片不做结果集直接编辑（蓝图 §4 J2 明确排除全表编辑），且无行级写命令 db_row_write" }),
      ],
    },
    {
      scope: "db.grid.column",
      label: "结果列头",
      actions: [
        A("db.grid.column.sortAsc", "按此列升序（仅已加载行）", { safety: "LOCAL", precondition: "结果非空", disabledReason: "结果为空" }),
        A("db.grid.column.sortDesc", "按此列降序（仅已加载行）", { safety: "LOCAL", precondition: "结果非空", disabledReason: "结果为空" }),
        A("db.grid.column.copyColumn", "复制整列（已加载行）", { safety: "READ", precondition: "结果非空", disabledReason: "结果为空" }),
        A("db.grid.column.hide", "隐藏此列", { safety: "LOCAL", precondition: "列数 ≥2", disabledReason: "仅剩一列，不能全部隐藏" }),
      ],
    },
    {
      scope: "db.grid.row",
      label: "结果行",
      actions: [
        A("db.grid.row.copyRowCsv", "复制该行为 CSV", { keyboard: "Ctrl/Cmd+Shift+C", safety: "READ", precondition: "已选中行", disabledReason: "未选中行" }),
        A("db.grid.row.openInProperties", "在属性中查看该行", { safety: "READ", precondition: "已选中行", disabledReason: "未选中行" }),
        A("db.grid.row.delete", "删除该行", { safety: "DESTRUCTIVE", confirm: "typed", audit: "audit", backend: "MISSING:db_row_write", precondition: "后端提供行级写通道且连接已开写权限", disabledReason: "依赖未注册命令 db_row_write；首片不开放结果集写回" }),
      ],
    },
    {
      scope: "db.result",
      label: "结果工具窗口",
      actions: [
        A("db.result.copyAllCsv", "复制全部为 CSV", { keyboard: "Ctrl/Cmd+Shift+C", safety: "READ", precondition: "结果非空", disabledReason: "结果为空" }),
        A("db.result.exportFile", "导出为文件", { safety: "READ", audit: "audit", backend: "MISSING:db_export_result", precondition: "后端提供导出命令", disabledReason: "依赖未注册命令 db_export_result（16MiB 导出上限当前无消费方）" }),
        A("db.result.clear", "清空结果", { safety: "LOCAL", precondition: "存在结果或错误", disabledReason: "无结果可清空" }),
        A("db.result.rerun", "重新执行本文档", { keyboard: "Ctrl/Cmd+Enter", safety: "READ", backend: "db_query", precondition: "已连接且无在途查询", disabledReason: "未连接或查询在途" }),
      ],
    },
    {
      scope: "db.toolwindow",
      label: "工具窗口标题栏（任意边）",
      actions: [
        A("db.toolwindow.pin", "固定", { safety: "LOCAL", precondition: "该边有可见工具窗口且未固定", disabledReason: "该边无可见工具窗口，或已固定" }),
        A("db.toolwindow.unpin", "取消固定", { safety: "LOCAL", precondition: "该边可见窗口已固定", disabledReason: "该边可见窗口未固定" }),
        A("db.toolwindow.autoHide", "自动隐藏（悬浮）", { safety: "LOCAL", precondition: "该边可见窗口未固定", disabledReason: "已固定的窗口不参与自动隐藏（先取消固定）" }),
        A("db.toolwindow.close", "关闭", { keyboard: "Shift+Esc", safety: "LOCAL", precondition: "该边有可见工具窗口", disabledReason: "该边无可见工具窗口" }),
        A("db.toolwindow.expandActive", "展开活动窗口", { keyboard: "Ctrl/Cmd+Shift+'", safety: "LOCAL", precondition: "该边窗口为停靠态", disabledReason: "悬浮或隐藏的窗口无停靠尺寸可展开" }),
        A("db.toolwindow.resize", "调整大小", { keyboard: "Alt+←/→/↑/↓", safety: "LOCAL", precondition: "该边窗口为停靠态", disabledReason: "悬浮或隐藏的窗口不可调整停靠尺寸" }),
        A("db.toolwindow.collapseAll", "收起全部工具窗口", { keyboard: "Ctrl/Cmd+Shift+F12", safety: "LOCAL", precondition: "至少一个边为停靠/悬浮态", disabledReason: "已全部收起" }),
        A("db.toolwindow.restoreLayout", "恢复上次布局", { keyboard: "Ctrl/Cmd+Alt+F12", safety: "LOCAL", precondition: "存在收起前的布局快照", disabledReason: "没有可恢复的布局快照" }),
        A("db.toolwindow.focusMode", "专注模式 / 退出专注", { keyboard: "F11（退出 Esc）", safety: "LOCAL", precondition: "存在活动文档", disabledReason: "无活动文档，专注模式无内容可放大" }),
      ],
    },
    {
      scope: "db.history",
      label: "查询历史条目",
      actions: [
        A("db.history.openInConsole", "在控制台打开", { keyboard: "Enter", safety: "LOCAL", precondition: "已选中历史条目", disabledReason: "未选中历史条目" }),
        A("db.history.copySql", "复制 SQL", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "已选中历史条目", disabledReason: "未选中历史条目" }),
        A("db.history.rerun", "重新执行", { safety: "READ", backend: "db_query", confirm: "single", audit: "audit", precondition: "已连接、条目所属连接仍存在且无在途查询", disabledReason: "未连接、原连接已不存在，或查询在途" }),
        A("db.history.clear", "清空历史", { safety: "LOCAL", confirm: "single", precondition: "历史非空", disabledReason: "历史为空" }),
        A("db.history.persistToggle", "持久化历史（默认关）", { safety: "LOCAL", confirm: "single", audit: "audit", backend: "MISSING:db_query_history", precondition: "后端提供历史落盘命令且用户显式开启", disabledReason: "依赖未注册命令 db_query_history；SQL 全文默认不落盘（蓝图 §5 持久化红线）" }),
      ],
    },
    {
      scope: "db.properties",
      label: "属性工具窗口",
      actions: [
        A("db.properties.copyValue", "复制属性值", { keyboard: "Ctrl/Cmd+C", safety: "READ", precondition: "已选中属性行", disabledReason: "未选中属性行" }),
        A("db.properties.refresh", "刷新属性", { keyboard: "F5", safety: "READ", backend: "MISSING:db_list_schema", precondition: "后端提供结构枚举命令", disabledReason: "依赖未注册命令 db_list_schema；当前属性仅来自结果集列元数据" }),
      ],
    },
  ];
}

// ===========================================================================
// §7 断言
// ===========================================================================

let failures = 0;
let checks = 0;
function group(name) {
  console.log(`\n[${name}]`);
}
function check(name, cond) {
  checks++;
  if (cond) console.log("  ok   " + name);
  else {
    console.log("  FAIL " + name);
    failures++;
  }
}

// --- G1 顶部 chrome 预算 ---
group("G1 顶部 chrome（≤2 行 / ≤80px）");
{
  const shell = createShell();
  const l = computeLayout({ width: 1440, height: 900 }, shell);
  check("顶部 chrome 恰好 2 行", l.topChromeRows === 2);
  check(`顶部 chrome 高度 ${l.topChromeHeight}px ≤ ${BUDGET.maxTopChromePx}px`, l.topChromeHeight <= BUDGET.maxTopChromePx);
  check("行 1（标签条）与行 2（导航行）都是紧凑行 ≤32px", CHROME.tabStrip <= 32 && CHROME.navRow <= 32);
}

// --- G2 收起态视口预算（六尺寸，对齐 A0 规范几何公式） ---
// 规范几何（A0 裁决）：collapsed active height = (innerH - 60 - 24)/innerH；active width = (innerW - 28)/innerW。
// 本模型 computeLayout 的 canonicalHeightPct/canonicalWidthPct 即该规范值（statusBar=24 时 contentHeight = H-84）。
group("G2 收起态活动面预算（≥85% 高 / ≥92% 宽，规范公式，六尺寸）");
for (const size of TARGET_SIZES) {
  const shell = collapseAll(createShell());
  shell.docCount = 2; // 最不利：多文档，子标签条占位
  const l = computeLayout(size, shell);
  check(
    `${size.label} 收起态 活动面高 ${l.contentHeight}px = ${l.canonicalHeightPct}% ≥ 85%（规范 (H-60-24)/H）`,
    l.canonicalHeightPct >= BUDGET.minHeightPct,
  );
  check(
    `${size.label} 收起态 活动面宽 ${l.contentWidth}px = ${l.canonicalWidthPct}% ≥ 92%（规范 (W-28)/W）`,
    l.canonicalWidthPct >= BUDGET.minWidthPct,
  );
  // 严格内部量测（再扣文档内子标签条 + 控制台状态行）仅作信息打印，不计入红线门槛。
  console.log("       · 严格活动面（扣文档内 chrome）:" + l.activeHeight + "px = " + l.activeHeightPct + "%");
}

// --- G3 专注态归还全视口 ---
group("G3 专注模式（全视口 + 唯一返回动作）");
{
  let shell = createShell();
  shell.docCount = 2;
  shell = enterFocus(shell);
  const l = computeLayout({ width: 1440, height: 900 }, shell);
  check("专注态宽度 = 100%", l.activeWidthPct === 100 && l.activeWidth === 1440);
  check("专注态高度 = 100% 后标题栏高度", l.activeHeightPct === 100 && l.activeHeight === 872);
  check("专注态无任何工具 chrome", toolChromeVisible(shell) === false);
  check("专注态保留唯一返回动作（含快捷键）", !!shell.focusReturnAction && /Esc/.test(shell.focusReturnAction.keyboard));
  shell = exitFocus(shell);
  check("退出专注恢复到 normal 且左/底停靠回位", shell.view === "normal" && shell.edges.left.mode === "docked" && shell.edges.bottom.mode === "docked");
}

// --- G4 工具窗口行为 ---
group("G4 工具窗口（每边唯一 / 固定共存 / 自动隐藏不吃视口 / 收起-恢复 / 展开活动 / 调整大小）");
{
  let shell = createShell();
  openTool(shell, "left", "db.tool.schema");
  check("同边打开另一个工具窗口：未固定者被替换（成员仍为 1）", shell.edges.left.members.length === 1 && shell.edges.left.visible === "db.tool.schema");

  pinTool(shell, "left");
  openTool(shell, "left", "db.tool.connections");
  check("已固定则共存为同边标签（成员 2，可见为新窗口）", shell.edges.left.members.length === 2 && shell.edges.left.visible === "db.tool.connections");
  check("固定项仍在成员中", shell.edges.left.members.includes("db.tool.schema"));

  // 自动隐藏：不消耗视口
  const before = computeLayout({ width: 1440, height: 900 }, shell);
  autoHideTool(shell, "left");
  const after = computeLayout({ width: 1440, height: 900 }, shell);
  check(`自动隐藏后内容宽度增大（${before.contentWidth} → ${after.contentWidth}）`, after.contentWidth > before.contentWidth);
  check("悬浮态不计入停靠宽度（内容宽 = 全宽 - rail）", after.contentWidth === 1440 - CHROME.railLeft);

  // 固定优先于自动隐藏
  let s2 = createShell();
  pinTool(s2, "bottom");
  autoHideTool(s2, "bottom");
  check("已固定的窗口拒绝自动隐藏（幂等）", s2.edges.bottom.mode === "docked");

  // 收起全部 + 恢复上次布局（往返一致）
  let s3 = createShell();
  openTool(s3, "right", "db.tool.properties");
  resizeTool(s3, "left", 320);
  const snapshot = JSON.stringify(s3.edges);
  collapseAll(s3);
  const collapsed = computeLayout({ width: 1440, height: 900 }, s3);
  check("收起全部后三边皆隐藏", Object.values(s3.edges).every((e) => e.mode === "hidden"));
  check(`收起态内容宽 ${collapsed.contentWidth}px = 全宽 - rail`, collapsed.contentWidth === 1440 - CHROME.railLeft);
  restorePreviousLayout(s3);
  check("恢复上次布局逐字段还原（含自定义尺寸 320px）", JSON.stringify(s3.edges) === snapshot);
  check("快照消费后清空（不可重复恢复）", s3.previousLayout === null);

  // 展开活动窗口 + 反向还原
  let s4 = createShell();
  const base = computeLayout({ width: 1440, height: 900 }, s4);
  const contentRegion = 900 - CHROME.tabStrip - CHROME.navRow - CHROME.statusBar;
  expandActive(s4, "bottom", contentRegion);
  check(`展开活动底部窗口至比例上限 ${TOOLS.bottom.maxRatio}`, s4.edges.bottom.size === Math.floor(contentRegion * TOOLS.bottom.maxRatio));
  shrinkActive(s4);
  check("还原展开后回到默认 240px", s4.edges.bottom.size === TOOLS.bottom.def && computeLayout({ width: 1440, height: 900 }, s4).contentHeight === base.contentHeight);

  // 调整大小钳位
  let s5 = createShell();
  resizeTool(s5, "left", 5000);
  check(`左侧尺寸上钳位到 ${TOOLS.left.max}`, s5.edges.left.size === TOOLS.left.max);
  resizeTool(s5, "left", 10);
  check(`左侧尺寸下钳位到 ${TOOLS.left.min}`, s5.edges.left.size === TOOLS.left.min);
  resizeTool(s5, "bottom", 5000, contentRegion);
  check("底部尺寸按内容区比例上钳位", s5.edges.bottom.size === Math.floor(contentRegion * TOOLS.bottom.maxRatio));

  // 关闭
  let s6 = createShell();
  closeTool(s6, "bottom");
  check("关闭最后一个成员后该边转 hidden 且 visible=null", s6.edges.bottom.mode === "hidden" && s6.edges.bottom.visible === null);
}

// --- G5 层级视图 ---
group("G5 树行为（全部折叠 / 展开一层 / 有界全展开）");
{
  const tree = makeSyntheticTree();
  let st = collapseAllNodes();
  check("全部折叠后展开集合为空", st.expanded.size === 0 && st.depth === 0);
  st = expandOneLevel(tree, st);
  check("展开一层：仅根展开（深度 1）", st.depth === 1 && st.expanded.has("root") && !st.expanded.has("conn.replica"));
  st = expandOneLevel(tree, st);
  check("再展开一层：连接级展开（深度 2）", st.depth === 2 && st.expanded.has("conn.replica"));

  let deep = collapseAllNodes();
  for (let i = 0; i < TREE_LIMITS.maxDepth; i++) deep = expandOneLevel(tree, deep);
  const over = expandOneLevel(tree, deep);
  check(`达最大层级 ${TREE_LIMITS.maxDepth} 后拒绝继续并给出说明`, over.truncated === true && over.notice.includes("最大层级"));

  const bounded = boundedExpandAll(tree);
  check(`有界全展开节点数 ${bounded.visible} ≤ ${TREE_LIMITS.maxVisibleNodes}`, bounded.visible <= TREE_LIMITS.maxVisibleNodes);
  check("400 表的分支触发截断且有显式提示（非静默）", bounded.truncated === true && bounded.notice.length > 0);
  check("小分支仍被展开（截断不是整体放弃）", bounded.expanded.has("conn.local.main.tables"));
}

// --- G6 多文档 / 取消 / 截断 / 错误 ---
group("G6 SQL 多文档（独立 / 陈旧守卫 / 取消幂等 / 截断与错误非静默）");
{
  const gen = (() => {
    let n = 0;
    return () => "exec-" + ++n;
  })();
  let a = createSqlDoc("a", "orders.sql", "conn.replica");
  let b = createSqlDoc("b", "users-report.sql", "conn.local");
  a = startRun(a, "select 1", "conn.replica", gen);
  const aExec = a.execId;
  const b0 = b;
  a = applyDocResult(a, aExec, { state: "completed", rows: [[1]], row_count: 1 });
  check("文档 A 完成后 success，文档 B 完全未受影响", a.status === "success" && b === b0 && b.status === "idle");

  let x = createSqlDoc("x", "slow.sql");
  let y = createSqlDoc("y", "fast.sql");
  x = startRun(x, "select slow", "c1", gen);
  const stale = x.execId;
  y = startRun(y, "select fast", "c1", gen);
  const late = applyDocResult(y, stale, { state: "completed", rows: [["OLD"]] });
  check("陈旧 execId 的迟到结果被丢弃（不覆盖新文档）", late === y && y.status === "running");
  y = applyDocResult(y, y.execId, { state: "completed", rows: [["NEW"]], row_count: 1 });
  check("新文档收到自己的结果", y.status === "success" && y.result.rows[0][0] === "NEW");

  let z = createSqlDoc("z", "z.sql");
  z = startRun(z, "select 2", "c1", gen);
  const c1 = cancelDoc(z);
  const c2 = cancelDoc(c1);
  check("取消幂等（重复取消无副作用）", c1.status === "cancelled" && c2 === c1);
  const retried = startRun(c1, "select 2", "c1", gen);
  check("取消后可重试并获得新 execId", retried.status === "running" && retried.execId !== null && retried.execId !== stale);

  const wTrunc = resultWarnings({ truncated: true, field_truncated: true, state: "completed" });
  check("行/字节截断 + 长字段截断各有显式告警", wTrunc.length === 2 && wTrunc[0].includes("1000 行") && wTrunc[1].includes("64.00 KiB"));
  check("取消态结果显式标注不完整", resultWarnings({ state: "cancelled" })[0].includes("不完整"));
  check("超时态结果显式标注不完整", resultWarnings({ state: "timeout" })[0].includes("不完整"));
  check("失败态结果显式标注不完整", resultWarnings({ state: "failed" })[0].includes("不完整"));
}

// --- G7 命令注册表完整性 ---
group("G7 右键菜单 / 命令注册表（身份 / 禁用原因 / 键盘或无障碍 / 安全分级）");
{
  const reg = buildDbCommandRegistry();
  const all = reg.flatMap((s) => s.actions);
  const ids = all.map((a) => a.id);
  check(`作用域数 ${reg.length} ≥ 12（连接/模式/表/列/文档/编辑器/单元格/列头/行/结果/工具窗口/历史/属性）`, reg.length >= 12);
  check(`动作总数 ${all.length} ≥ 45`, all.length >= 45);
  check("命令 id 全局唯一", new Set(ids).size === ids.length);
  check("每个动作 id 以其作用域为前缀（作用域内聚，无大杂烩）", reg.every((s) => s.actions.every((a) => a.id.startsWith(s.scope + "."))));
  check("每个动作都有非空标签", all.every((a) => a.label.length > 0));
  check("每个动作都有键盘快捷键或无障碍菜单路径", all.every((a) => (a.keyboard && a.keyboard.length > 0) || (a.a11yPath && a.a11yPath.length > 0)));
  check("每个动作都有非空禁用原因", all.every((a) => a.disabledReason && a.disabledReason.length >= 4));
  check("每个动作都有前置条件描述", all.every((a) => a.precondition && a.precondition.length >= 4));
  check("安全分级取值受限（READ/LOCAL/DB_WRITE/DESTRUCTIVE）", all.every((a) => ["READ", "LOCAL", "DB_WRITE", "DESTRUCTIVE"].includes(a.safety)));
  check("DB_WRITE 与 DESTRUCTIVE 一律要求确认", all.filter((a) => a.safety === "DB_WRITE" || a.safety === "DESTRUCTIVE").every((a) => a.confirm !== "none"));
  check("DESTRUCTIVE 一律要求键入式确认 + 审计", all.filter((a) => a.safety === "DESTRUCTIVE").every((a) => a.confirm === "typed" && a.audit === "audit"));
  check("READ/LOCAL 动作不得声明 typed 确认（避免确认疲劳）", all.filter((a) => a.safety === "READ" || a.safety === "LOCAL").every((a) => a.confirm !== "typed"));

  const backends = all.map((a) => a.backend).filter(Boolean);
  check(
    "后端标注只能是已冻结命令或 MISSING:<未注册命令>",
    backends.every((b) => FROZEN_DB_COMMANDS.includes(b) || (b.startsWith("MISSING:") && MISSING_DB_COMMANDS.includes(b.slice(8)))),
  );
  const missingActions = all.filter((a) => (a.backend ?? "").startsWith("MISSING:"));
  check(`依赖未注册命令的动作 ${missingActions.length} 个，全部在禁用原因中点名该命令或写明排除理由`, missingActions.every((a) => /db_[a-z_]+|不做|排除/.test(a.disabledReason)));
  check("取消动作明确点名 db_cancel 未注册且给出替代（超时/断开）", all.find((a) => a.id === "db.console.cancel").disabledReason.includes("db_cancel"));
  check("写权限切换绑定生产判定（未知按生产处理）", all.find((a) => a.id === "db.connection.toggleWrite").disabledReason.includes("未知"));
  check("工具窗口作用域覆盖 pin/unpin/autoHide/close/expand/resize/collapseAll/restore/focus 九项", reg.find((s) => s.scope === "db.toolwindow").actions.length === 9);
}

// --- G8 原型同步（HTML 与本模型逐项一致） ---
group("G8 原型同步（A5-R3-prototype.html 与本规范源一致）");
{
  const here = dirname(fileURLToPath(import.meta.url));
  const htmlPath = join(here, "A5-R3-prototype.html");
  if (!existsSync(htmlPath)) {
    check("A5-R3-prototype.html 存在", false);
  } else {
    const html = readFileSync(htmlPath, "utf8");
    check("A5-R3-prototype.html 存在", true);
    check("原型自包含（无外链 script/style/字体）", !/<script[^>]+src=/i.test(html) && !/<link[^>]+stylesheet/i.test(html) && !/https?:\/\/(?!www\.w3\.org)/i.test(html));
    check("原型声明合成数据 / 零 IPC", /合成/.test(html) && /零\s*IPC|不发\s*IPC|无真实\s*IPC/.test(html));

    const constEntries = [
      ["tabStrip", CHROME.tabStrip],
      ["navRow", CHROME.navRow],
      ["statusBar", CHROME.statusBar],
      ["railLeft", CHROME.railLeft],
      ["subTabs", CHROME.subTabs],
      ["consoleStatus", CHROME.consoleStatus],
    ];
    check(
      "六个 chrome 常量在原型内逐值一致",
      constEntries.every(([k, v]) => new RegExp(`${k}\\s*:\\s*${v}\\b`).test(html)),
    );
    check(
      "工具窗口默认/最小/最大在原型内一致",
      new RegExp(`def:\\s*${TOOLS.left.def}[\\s\\S]{0,40}min:\\s*${TOOLS.left.min}[\\s\\S]{0,40}max:\\s*${TOOLS.left.max}`).test(html) &&
        new RegExp(`def:\\s*${TOOLS.bottom.def}`).test(html),
    );
    check("树上限常量一致", new RegExp(`maxVisibleNodes\\s*:\\s*${TREE_LIMITS.maxVisibleNodes}`).test(html) && new RegExp(`maxDepth\\s*:\\s*${TREE_LIMITS.maxDepth}`).test(html));
    check("六个目标尺寸全部提供预设", TARGET_SIZES.every((s) => html.includes(s.label)));

    const reg = buildDbCommandRegistry();
    const ids = reg.flatMap((s) => s.actions.map((a) => a.id));
    const missingInHtml = ids.filter((id) => !html.includes(id));
    check(`全部 ${ids.length} 个命令 id 均出现在原型内（缺失：${missingInHtml.join(", ") || "无"}）`, missingInHtml.length === 0);
    const scopes = reg.map((s) => s.scope);
    check("全部作用域在原型内可右键触达", scopes.every((s) => html.includes(`data-scope="${s}"`)));

    const states = ["空结果", "加载中", "错误", "禁用", "已截断"];
    check(`结果区状态齐备（${states.join(" / ")}）`, states.every((s) => html.includes(s)));
    const modes = ["收起全部", "恢复上次布局", "展开活动", "专注模式", "自动隐藏", "固定"];
    check(`布局动作齐备（${modes.join(" / ")}）`, modes.every((s) => html.includes(s)));
    check("原型内含实测视口占比读数（DOM 量测）", /offsetWidth/.test(html) && /offsetHeight/.test(html) && /85/.test(html) && /92/.test(html));
    check("原型不含 tauri invoke / fetch / localStorage 等运行时通道", !/invoke\(|__TAURI__|fetch\(|localStorage/.test(html));
    // R3B-02：原型 UI 不得出现捐赠方品牌字体/名称（报告可引用，原型文本不可见）。
    check("原型不含捐赠方品牌字体/名称（JetBrains/DataGrip/IntelliJ/Rebased）", !/(JetBrains|DataGrip|IntelliJ|Rebased)/.test(html));
  }
}

// --- G9 R3B 收起/恢复语义（对齐 A0/A3/A4 裁决） ---
group("G9 R3B 收起全部含固定窗口 / 恢复还原可见+尺寸+固定态 / 快照确定性");
{
  // A0 裁决：Collapse All hides every tool window, including pinned；Restore Layout restores visibility, sizes AND pin states。
  let s = createShell();
  pinTool(s, "left");
  pinTool(s, "bottom");
  resizeTool(s, "left", 360);
  const pre = JSON.stringify({ view: s.view, edges: s.edges });
  collapseAll(s);
  check("收起全部后，已固定的左边窗口也被隐藏", s.edges.left.mode === "hidden" && s.edges.left.pinned.length === 1);
  check("收起全部后，已固定的底边窗口也被隐藏", s.edges.bottom.mode === "hidden" && s.edges.bottom.pinned.length === 1);
  check("收起态所有边均 hidden（含已固定窗口）", Object.values(s.edges).every((e) => e.mode === "hidden"));
  restorePreviousLayout(s);
  check("恢复后左边回到 docked 且自定义尺寸 360 还原", s.edges.left.mode === "docked" && s.edges.left.size === 360);
  check("恢复后左边固定态还原（pinned 含 1 项）", s.edges.left.pinned.length === 1);
  check("恢复后底边固定态还原（pinned 含 1 项）", s.edges.bottom.pinned.length === 1);
  check("恢复后布局逐字段等于收起前快照", JSON.stringify({ view: s.view, edges: s.edges }) === pre);
  check("快照消费后清空（不可重复恢复）", s.previousLayout === null);

  // A4 裁决：快照只含 view + edges，不持久化瞬时字段（focusReturnAction/expandedFrom/hover/overlay）。
  let s2 = createShell();
  s2.focusReturnAction = { id: "x" };
  s2.expandedFrom = { edge: "bottom", size: 240 };
  collapseAll(s2);
  check("快照仅含 view/edges（不含 focusReturnAction/expandedFrom 等瞬时字段）", JSON.stringify(Object.keys(s2.previousLayout).sort()) === JSON.stringify(["edges", "view"]));

  // 确定性往返：collapse -> restore -> collapse -> restore 结果一致（A4 要求确定性）。
  let s3 = createShell();
  pinTool(s3, "right");
  resizeTool(s3, "right", 400);
  const a = JSON.stringify(s3.edges);
  collapseAll(s3);
  restorePreviousLayout(s3);
  collapseAll(s3);
  restorePreviousLayout(s3);
  check("collapse/restore 往返确定性（两次后布局一致）", JSON.stringify(s3.edges) === a);

  // 专注模式退出后活动文档/结果面回到正常停靠（focus mode restores the active editor/result surface）。
  let s4 = createShell();
  enterFocus(s4);
  check("专注态无任何工具 chrome（activity strip 以外）", toolChromeVisible(s4) === false);
  exitFocus(s4);
  check("退出专注恢复到 normal 且左/底停靠回位（活动编辑/结果面还原）", s4.view === "normal" && s4.edges.left.mode === "docked" && s4.edges.bottom.mode === "docked");
}

console.log(
  failures === 0
    ? `\nA5-R3 state-model: ALL_PASS (${checks} checks)`
    : `\nA5-R3 state-model: ${failures}/${checks} FAIL`,
);
process.exit(failures === 0 ? 0 : 1);
