#!/usr/bin/env node
// ---------------------------------------------------------------------------
// M5-9（知识图谱 UI）前端逻辑层自动化测试（headless，无 GUI 依赖）
//
// 直接加载真实的 src/utils/graphUi.ts（纯逻辑层，无 invoke / DOM / store 依赖），
// 只把产品代码行为作为断言对象。不 mock 逻辑层自身。
//
// 用法: node scripts/check-graph-ui-logic.mjs
// 退出码: 0 = 全部通过；1 = 有断言失败
// ---------------------------------------------------------------------------

import * as nodeModule from "node:module";

function resolveWithExt(specifier, context, next) {
  try {
    return next(specifier, context);
  } catch (err) {
    if (specifier.startsWith(".") || specifier.startsWith("/")) {
      for (const ext of [".ts", "/index.ts", ".mjs", ".js"]) {
        try {
          return next(specifier + ext, context);
        } catch {}
      }
    }
    throw err;
  }
}

if (typeof nodeModule.registerHooks === "function") {
  nodeModule.registerHooks({ resolve: resolveWithExt });
} else {
  nodeModule.register(
    "data:text/javascript," +
      encodeURIComponent(
        `export async function resolve(specifier, context, next) {
  return globalThis.__m48Resolve(specifier, context, next);
}`
      )
  );
  globalThis.__m48Resolve = resolveWithExt;
}

// 最小浏览器桩（graphUi 纯逻辑不需要，但保留以兼容潜在 import 链）
globalThis.window = {
  setTimeout: (fn, ms) => setTimeout(fn, ms),
  clearTimeout: (h) => clearTimeout(h),
  setInterval: () => 0,
  clearInterval: () => {},
};
globalThis.localStorage = { getItem: () => null, setItem: () => {}, removeItem: () => {} };

const ROOT = new URL("..", import.meta.url).pathname;
const graphUi = await import(`${ROOT}src/utils/graphUi.ts`);

let passed = 0;
let failed = 0;
function ok(name, cond) {
  if (cond) {
    passed++;
    console.log("  ✓ " + name);
  } else {
    failed++;
    console.error("  ✗ " + name);
  }
}
function eq(name, a, b) {
  ok(name + ` (got=${JSON.stringify(a)})`, JSON.stringify(a) === JSON.stringify(b));
}

const node = (id, kind, label) => ({ id, kind, label, props: {} });
const edge = (from, to, kind, weight = 1) => ({ from, to, kind, weight, props: {} });

// ---- elementKind 判别 ----
ok("elementKind: node", graphUi.elementKind(node("n1", "file", "a")) === "node");
ok("elementKind: edge", graphUi.elementKind(edge("n1", "n2", "references")) === "edge");
ok("elementKind: unknown", graphUi.elementKind({ foo: 1 }) === "unknown");
ok("elementKind: null", graphUi.elementKind(null) === "unknown");

// ---- 标签/配色 ----
eq("nodeKindLabel.agent", graphUi.nodeKindLabel("agent"), "智能体");
eq("edgeKindLabel.uses", graphUi.edgeKindLabel("uses"), "使用");
ok("nodeColor 非空", graphUi.nodeColor("agent").length > 0);

// ---- 摘要（K7：不泄露 props）----
const sn = graphUi.summarizeNode(node("n1", "skill", "S1"));
ok("summarizeNode 不含 props", !("props" in sn));
eq("summarizeNode.isAgentConsumption(skill)", sn.isAgentConsumption, true);
const se = graphUi.summarizeEdge(edge("a", "s", "uses"));
ok("summarizeEdge.agentConsumption(uses)", se.agentConsumption === true);
ok("summarizeEdge 不含 props", !("props" in se));

// ---- isAgentConsumptionEdge ----
ok("isAgentConsumptionEdge uses", graphUi.isAgentConsumptionEdge(edge("a", "s", "uses")) === true);
ok(
  "isAgentConsumptionEdge references",
  graphUi.isAgentConsumptionEdge(edge("a", "s", "references")) === false,
);

// ---- 布局（确定性、无随机、坐标有限）----
eq("layoutKindOf tree(in_dir)", graphUi.layoutKindOf([edge("a", "b", "in_dir")]), "tree");
eq(
  "layoutKindOf cluster(tagged_with)",
  graphUi.layoutKindOf([edge("a", "b", "tagged_with")]),
  "cluster",
);
eq("layoutKindOf force", graphUi.layoutKindOf([edge("a", "b", "references")]), "force");
const ns = [node("a", "file", "A"), node("b", "dir", "B"), node("c", "skill", "C")];
const pts = graphUi.layoutPositions(ns, [edge("a", "b", "references")]);
ok("layoutPositions 数量一致", pts.length === ns.length);
ok(
  "layoutPositions 坐标有限",
  pts.every((p) => Number.isFinite(p.x) && Number.isFinite(p.y)),
);
ok(
  "layoutPositions 稳定（无随机）",
  JSON.stringify(graphUi.layoutPositions(ns, [edge("a", "b", "references")])) ===
    JSON.stringify(pts),
);

// ---- 容量 ----
const cap = graphUi.estimateCapacity(50, 200);
eq("capacity.nodeUsedPct(50/5000=1%)", cap.nodeUsedPct, 1);
ok("capacity.withinLimit(true)", cap.withinLimit === true);
const cap2 = graphUi.estimateCapacity(6000, 10);
ok("capacity.withinLimit(false>max)", cap2.withinLimit === false);
eq("capacity.nodeUsedPct clamp100", cap2.nodeUsedPct, 100);

// ---- 搜索/过滤 ----
const data = [
  node("f1", "file", "README"),
  node("s1", "skill", "Summarizer"),
  node("a1", "agent", "Helper"),
];
eq(
  "filterNodes by query(忽略大小写)",
  graphUi.filterNodes(data, { query: "readme", kinds: [] }).map((n) => n.id),
  ["f1"],
);
eq(
  "filterNodes by kinds",
  graphUi.filterNodes(data, { query: "", kinds: ["skill", "agent"] }).map((n) => n.id),
  ["s1", "a1"],
);
eq(
  "filterNodes by kindLabel query",
  graphUi.filterNodes(data, { query: "技能", kinds: [] }).map((n) => n.id),
  ["s1"],
);
const edgesData = [edge("f1", "s1", "uses"), edge("a1", "s1", "a2a_with")];
eq(
  "filterEdges 保留两端可见(全可见)",
  graphUi.filterEdges(edgesData, data, { query: "", kinds: [] }).length,
  2,
);
eq(
  "filterEdges 仅保留两端可见(按 agent 过滤后为空)",
  graphUi.filterEdges(edgesData, data, { query: "", kinds: ["agent"] }).length,
  0,
);

// ---- 有界合并 ----
let m = new Map();
m = graphUi.boundedInsert(m, [node("1", "file", "a"), node("2", "file", "b")], 2);
ok("boundedInsert 初始大小", m.size === 2);
m = graphUi.boundedInsert(m, [node("3", "file", "c")], 2);
ok("boundedInsert 超上限丢弃最旧", m.size === 2 && !m.has("1"));

// ---- 确定性选择（边稳定标识 edgeKey）----
ok("edgeKey 格式(from|to|kind)", graphUi.edgeKey(edge("a", "b", "uses")) === "a|b|uses");
ok(
  "edgeKey 同边同标识(确定性，避免索引漂移)",
  graphUi.edgeKey(edge("a", "b", "uses")) === graphUi.edgeKey(edge("a", "b", "uses")),
);

// ---- 渲染有界（clampRender：UI 安全网，避免渲染数组无界增长）----
const rc1 = graphUi.clampRender([1, 2, 3], 10);
ok("clampRender 未超限不截断", rc1.truncated === false && rc1.items.length === 3 && rc1.total === 3);
const bigArr = Array.from({ length: 6000 }, (_, i) => i);
const rc2 = graphUi.clampRender(bigArr, 5000);
ok("clampRender 超限封顶(cap=5000)", rc2.truncated === true && rc2.items.length === 5000 && rc2.total === 6000);

// ---- 面板三态 ----
eq(
  "panelStateGraph backendReady=false empty",
  graphUi.panelStateGraph({ loading: false, count: 0, error: null, backendReady: false }).state,
  "empty",
);
ok(
  "panelStateGraph 提示只读壳",
  graphUi.panelStateGraph({ loading: false, count: 0, error: null, backendReady: false }).message.includes(
    "只读壳",
  ),
);
eq(
  "panelStateGraph error",
  graphUi.panelStateGraph({ loading: false, count: 0, error: "boom", backendReady: true }).state,
  "error",
);
eq(
  "panelStateGraph ready",
  graphUi.panelStateGraph({ loading: false, count: 5, error: null, backendReady: true }).state,
  "ready",
);
eq(
  "panelStateGraph backendReady=true 空态文案",
  graphUi.panelStateGraph({ loading: false, count: 0, error: null, backendReady: true }).message,
  "暂无图谱数据。点击「刷新」从后端载入。",
);
ok(
  "panelStateGraph 两态文案可区分(就绪空≠未就绪只读)",
  graphUi.panelStateGraph({ loading: false, count: 0, error: null, backendReady: true }).message !==
    graphUi.panelStateGraph({ loading: false, count: 0, error: null, backendReady: false }).message,
);

// ---- 容量健康度（超量态：ok/near/over，确定性）----
eq("capacityState ok(50/200)", graphUi.capacityState(graphUi.estimateCapacity(50, 200)).level, "ok");
eq(
  "capacityState near(4700/19000≈94%)",
  graphUi.capacityState(graphUi.estimateCapacity(4700, 19000)).level,
  "near",
);
eq(
  "capacityState over(6000/>max)",
  graphUi.capacityState(graphUi.estimateCapacity(6000, 10)).level,
  "over",
);
ok(
  "capacityState over 文案非空",
  graphUi.capacityState(graphUi.estimateCapacity(6000, 10)).message.length > 0,
);

// ---- 确定性搜索 / 无界数组防护 ----
const big = [];
for (let i = 0; i < 6000; i++) big.push(node("n" + i, i % 2 ? "file" : "dir", "Node" + i));
const bigFiltered = graphUi.filterNodes(big, { query: "node1", kinds: [] });
ok("filterNodes 不放大数组(6000→子集)", bigFiltered.length > 0 && bigFiltered.length <= 6000);
const ordered = graphUi.filterNodes(big, { query: "", kinds: ["file"] });
ok("filterNodes 顺序稳定(file 子集首项 n1)", ordered.length === 3000 && ordered[0].id === "n1");
let bm = new Map();
bm = graphUi.boundedInsert(bm, big, 5000);
ok("boundedInsert 6000→封顶 5000(无界防护)", bm.size === 5000 && !bm.has("n0"));

// ---- 结果 ----
console.log(`\n图谱 UI 逻辑测试：通过 ${passed}，失败 ${failed}`);
process.exit(failed === 0 ? 0 : 1);
