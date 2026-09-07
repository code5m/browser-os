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

// ---- W11：单源守卫（RENDER_* 必须 == 数据上限常量，杜绝 R-W9-3 源扩散）----
ok(
  "RENDER_NODE_CAP === GRAPH_MAX_NODES(5000)",
  graphUi.RENDER_NODE_CAP === graphUi.GRAPH_MAX_NODES && graphUi.RENDER_NODE_CAP === 5000,
);
ok(
  "RENDER_EDGE_CAP === GRAPH_MAX_EDGES(20000)",
  graphUi.RENDER_EDGE_CAP === graphUi.GRAPH_MAX_EDGES && graphUi.RENDER_EDGE_CAP === 20000,
);

// ---- W11：有界渲染（边 cap=20000 与节点 cap=5000 同口径，UI 安全网）----
const rcEdge = graphUi.clampRender(Array.from({ length: 25000 }, (_, i) => i), graphUi.GRAPH_MAX_EDGES);
ok("clampRender 边上限封顶(cap=20000)", rcEdge.truncated === true && rcEdge.items.length === 20000 && rcEdge.total === 25000);
const rcNode2 = graphUi.clampRender(Array.from({ length: 6000 }, (_, i) => i), graphUi.GRAPH_MAX_NODES);
ok("clampRender 节点上限封顶(cap=5000)", rcNode2.truncated === true && rcNode2.items.length === 5000 && rcNode2.total === 6000);

// ---- W11：面板三态补充（loading 态；只读壳文案不泄露后端命令名/invoke）----
eq(
  "panelStateGraph loading",
  graphUi.panelStateGraph({ loading: true, count: 0, error: null, backendReady: false }).state,
  "loading",
);
ok(
  "panelStateGraph 只读壳文案不含后端命令名/invoke",
  !/invoke|graph_query|graph_/.test(
    graphUi.panelStateGraph({ loading: false, count: 0, error: null, backendReady: false }).message,
  ),
);

// ---- W11：确定性选择（resolveEdgeByKey 单一真源，过滤后还原选中边，不随索引漂移）----
const e1 = edge("a", "b", "uses");
const e2 = edge("c", "d", "references");
const e3 = edge("a", "d", "in_dir");
const selEdges = [e1, e2, e3];
const selKey = graphUi.edgeKey(e2); // c|d|references
ok("resolveEdgeByKey 命中(from=c)", graphUi.resolveEdgeByKey(selEdges, selKey)?.from === "c");
ok("resolveEdgeByKey 缺失 key 返回 null", graphUi.resolveEdgeByKey(selEdges, "x|y|z") === null);
ok("resolveEdgeByKey null key 返回 null", graphUi.resolveEdgeByKey(selEdges, null) === null);
// 过滤后（仅保留 file/agent 可见节点 a,d）→ 边 a|d|in_dir 两端仍可见，按同一 key 还原
const fNodes = [node("a", "file", "A"), node("b", "dir", "B"), node("c", "skill", "C"), node("d", "agent", "D")];
const visibleAfterFilter = graphUi.filterEdges(selEdges, fNodes, { query: "", kinds: ["file", "agent"] });
ok(
  "resolveEdgeByKey 过滤后仍可还原选中边(不随索引漂移)",
  graphUi.resolveEdgeByKey(visibleAfterFilter, graphUi.edgeKey(e3))?.from === "a",
);

// ---- W11：搜索稳定性（filterEdges 对相同输入确定性输出；按边类型中文标签检索后 key 稳定）----
const detA = graphUi.filterEdges(selEdges, fNodes, { query: "", kinds: [] });
const detB = graphUi.filterEdges(selEdges, fNodes, { query: "", kinds: [] });
ok(
  "filterEdges 确定性(同输入同输出，顺序可复现非随机)",
  detA.map(graphUi.edgeKey).join(",") === detB.map(graphUi.edgeKey).join(","),
);
// 按节点类型过滤后，保留边的 edgeKey 稳定（与可见集合顺序无关）；
// 注意：filterEdges 的 query 同时作用于节点可见性与边类型标签，故边检索经节点类型过滤表达。
const kindFiltered = graphUi.filterEdges(selEdges, fNodes, { query: "", kinds: ["file", "agent"] });
ok(
  "filterEdges 按节点类型过滤后边 key 稳定",
  kindFiltered.length === 1 && graphUi.edgeKey(kindFiltered[0]) === graphUi.edgeKey(e3),
);

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

// ============================================================================
// M5-W12 增量断言（A7 §3.4 / §5 / §6；与 A8 前端消费契约对齐）
// ============================================================================

// ---- W12 稳定错误码识别与映射（与 A7 §3.4 1:1）----
ok("isGraphStableErrorCode 命中 GRAPH_INVALID_ID", graphUi.isGraphStableErrorCode("GRAPH_INVALID_ID") === true);
ok("isGraphStableErrorCode 命中 GRAPH_UNKNOWN_ERROR", graphUi.isGraphStableErrorCode("GRAPH_UNKNOWN_ERROR") === true);
ok("isGraphStableErrorCode 拒绝 13 码之外的串", graphUi.isGraphStableErrorCode("GRAPH_NOT_A_REAL_CODE") === false);
ok("isGraphStableErrorCode 拒绝 props 串", graphUi.isGraphStableErrorCode("token=sk-1234") === false);
eq("isGraphStableErrorCode 拒绝空串", graphUi.isGraphStableErrorCode(""), false);

// applyGraphErrorView：Error.code 路径
const ev1 = graphUi.applyGraphErrorView({ code: "GRAPH_INVALID_ID" });
eq("applyGraphErrorView 码→GRAPH_INVALID_ID", ev1.code, "GRAPH_INVALID_ID");
ok("applyGraphErrorView 消息非空(本地化中文)", ev1.message.length > 0 && !/token|secret|sk-/.test(ev1.message));

// applyGraphErrorView：字符串 body 路径
const ev2 = graphUi.applyGraphErrorView("GRAPH_REF_ID_NOT_HEX");
eq("applyGraphErrorView 字符串 body→GRAPH_REF_ID_NOT_HEX", ev2.code, "GRAPH_REF_ID_NOT_HEX");

// applyGraphErrorView：未知错误 → 兜底（不泄露原始 message）
const ev3 = graphUi.applyGraphErrorView(new Error("内部栈包含 token=sk-abcdef 应被丢弃"));
eq("applyGraphErrorView 未知错误→GRAPH_UNKNOWN_ERROR", ev3.code, "GRAPH_UNKNOWN_ERROR");
ok(
  "applyGraphErrorView 兜底零 secret echo(原始 message 不透传)",
  !/token=sk-abcdef/.test(ev3.message) && !/内部栈/.test(ev3.message),
);

// applyGraphErrorView：非稳定码字符串也走兜底
const ev4 = graphUi.applyGraphErrorView("random error text");
eq("applyGraphErrorView 非稳定码串→兜底", ev4.code, "GRAPH_UNKNOWN_ERROR");

// applyGraphErrorView：null/undefined 走兜底
const ev5 = graphUi.applyGraphErrorView(null);
eq("applyGraphErrorView null→兜底", ev5.code, "GRAPH_UNKNOWN_ERROR");

// formatGraphStableError：仅返回 message（绝大多数 UI 只需 message）
ok("formatGraphStableError 串→非空中文", graphUi.formatGraphStableError("GRAPH_STORE_LOAD_FAILED").includes("图谱快照"));
ok("formatGraphStableError Error→无 secret", !/sk-/.test(graphUi.formatGraphStableError(new Error("sk-abc"))));

// 13 个稳定码全部能映射（A7 §3.4 全覆盖；避免新增码位时漏文案）
const ALL_STABLE = [
  "GRAPH_INVALID_ID", "GRAPH_REF_ID_NOT_HEX", "GRAPH_LABEL_TOO_LONG",
  "GRAPH_PROP_KEY_TOO_LONG", "GRAPH_PROP_VALUE_TOO_LONG", "GRAPH_PROP_COUNT_EXCEEDED",
  "GRAPH_SECRET_IN_PROPS", "GRAPH_NODE_CAPACITY_EXCEEDED", "GRAPH_EDGE_CAPACITY_EXCEEDED",
  "GRAPH_DUPLICATE_NODE", "GRAPH_DUPLICATE_EDGE", "GRAPH_STORE_LOAD_FAILED", "GRAPH_UNKNOWN_ERROR",
];
ok(
  "W12 13 稳定码全覆盖映射(零漏文案)",
  ALL_STABLE.every((c) => {
    const m = graphUi.applyGraphErrorView({ code: c }).message;
    return m.length > 0 && !/token|secret|sk-/.test(m);
  }),
);

// ---- W12 viewToNode/Edge：删 props 第三闸（K7 纵深防御）----
const vNodeIn = { id: "n1", kind: "file", label: "L1", props: { token: "sk-leaked" } }; // 假设上游误传
const vNodeOut = graphUi.viewToNode(vNodeIn);
ok("viewToNode 删 props(无 props 字段)", !("props" in vNodeOut));
ok("viewToNode 显式字段提取(id/kind/label)", vNodeOut.id === "n1" && vNodeOut.kind === "file" && vNodeOut.label === "L1");

const vEdgeIn = { from: "a", to: "b", kind: "uses", props: { secret: "AKIA-leaked" } };
const vEdgeOut = graphUi.viewToEdge(vEdgeIn);
ok("viewToEdge 删 props(无 props 字段)", !("props" in vEdgeOut));
ok("viewToEdge 显式字段提取(from/to/kind)", vEdgeOut.from === "a" && vEdgeOut.to === "b" && vEdgeOut.kind === "uses");

// ---- W12 viewToQueryResult：组合转化 + truncated/applied 透传 + 删 props ----
const vQueryIn = {
  found: true,
  nodes: [{ id: "n1", kind: "file", label: "L1", props: { token: "sk-1" } }],
  edges: [{ from: "n1", to: "n2", kind: "uses", props: { secret: "AKIA-1" } }],
  truncated: true,
  applied: { depth: 2, limit: 100 },
  node_count: 1,
  edge_count: 1,
};
const vQueryOut = graphUi.viewToQueryResult(vQueryIn);
eq("viewToQueryResult found 透传", vQueryOut.found, true);
eq("viewToQueryResult truncated 透传", vQueryOut.truncated, true);
eq("viewToQueryResult applied.depth=2", vQueryOut.applied.depth, 2);
eq("viewToQueryResult applied.limit=100", vQueryOut.applied.limit, 100);
eq("viewToQueryResult nodeCount 透传", vQueryOut.nodeCount, 1);
ok("viewToQueryResult nodes 无 props(零透传)", !vQueryOut.nodes.some((n) => "props" in n));
ok("viewToQueryResult edges 无 props(零透传)", !vQueryOut.edges.some((e) => "props" in e));

// ---- W12 viewToStats：与 estimateCapacity 同口径 ----
const vStatsIn = { node_count: 50, edge_count: 200, node_capacity: 5000, edge_capacity: 20000, approaching_node_capacity: false, approaching_edge_capacity: false };
const vStatsOut = graphUi.viewToStats(vStatsIn);
eq("viewToStats nodeCount 透传", vStatsOut.nodeCount, 50);
eq("viewToStats edgeCount 透传", vStatsOut.edgeCount, 200);
eq("viewToStats nodeUsedPct 1%", vStatsOut.nodeUsedPct, 1);
ok("viewToStats withinLimit true", vStatsOut.withinLimit === true);

// isApproachingCapacity：任一逼近即 true
ok("isApproachingCapacity false 双 false", graphUi.isApproachingCapacity(vStatsIn) === false);
ok(
  "isApproachingCapacity true 节点逼近",
  graphUi.isApproachingCapacity({ ...vStatsIn, approaching_node_capacity: true }) === true,
);
ok(
  "isApproachingCapacity true 边逼近",
  graphUi.isApproachingCapacity({ ...vStatsIn, approaching_edge_capacity: true }) === true,
);

// ---- W12 normalizeGraphQueryRequest：容量裁剪（domain.rs GRAPH_MAX_* 单源）----
const nq1 = graphUi.normalizeGraphQueryRequest({ start_id: "n1" });
eq("normalizeGraphQueryRequest 默认 depth=2", nq1.depth, 2);
eq("normalizeGraphQueryRequest 默认 limit=GRAPH_QUERY_LIMIT(1000)", nq1.limit, 1000);
ok("normalizeGraphQueryRequest 默认 request_id 8-hex", /^req-[0-9a-f]{8}$/.test(nq1.request_id));

const nq2 = graphUi.normalizeGraphQueryRequest({ start_id: "n1", depth: 99, limit: 99999 });
eq("normalizeGraphQueryRequest depth 截 GRAPH_MAX_DEPTH(4)", nq2.depth, 4);
eq("normalizeGraphQueryRequest limit 截 GRAPH_QUERY_LIMIT(1000)", nq2.limit, 1000);

const nq3 = graphUi.normalizeGraphQueryRequest({ start_id: "n1", depth: -5, limit: 0 });
eq("normalizeGraphQueryRequest 负 depth→0", nq3.depth, 0);
eq("normalizeGraphQueryRequest limit=0→最小 1", nq3.limit, 1);

const nq4 = graphUi.normalizeGraphQueryRequest({ start_id: "n1", request_id: "req-custom" });
eq("normalizeGraphQueryRequest request_id 透传", nq4.request_id, "req-custom");

// ---- W12 makeAbortableDebouncer：取消上一个 / cancelAll / pending ----
async function withTimeout(p, ms) {
  return await Promise.race([p, new Promise((r) => setTimeout(() => r("__timeout__"), ms))]);
}

const d1 = graphUi.makeAbortableDebouncer(20);
let d1Calls = 0;
let d1LastToken = null;
d1.schedule((t) => { d1Calls++; d1LastToken = t; });
d1.schedule((t) => { d1Calls++; d1LastToken = t; });
await new Promise((r) => setTimeout(r, 50));
eq("abortableDebounce 两次 schedule 仅末次执行(防抖)", d1Calls, 1);
ok("abortableDebounce 末次 token 未被取消", d1LastToken && d1LastToken.cancelled === false);

const d2 = graphUi.makeAbortableDebouncer(20);
let d2Calls = 0;
d2.schedule(() => { d2Calls++; });
d2.cancelAll();
await new Promise((r) => setTimeout(r, 50));
eq("abortableDebounce cancelAll 阻止执行", d2Calls, 0);
ok("abortableDebounce cancelAll 后 pending=null", d2.pending() === null);

const d3 = graphUi.makeAbortableDebouncer(20);
let d3FirstCancelled = null;
let d3SecondCalled = false;
const d3First = d3.schedule((t) => { d3FirstCancelled = t.cancelled; });
const d3Second = d3.schedule((t) => { d3SecondCalled = true; });
// 直接断言 schedule 返回值（这是文档化契约；比闭包变量更稳）
eq("abortableDebounce 第一次被取消(第一次 schedule 返回值.cancelled=true)", d3First.cancelled, true);
eq("abortableDebounce 第二次未取消(第二次 schedule 返回值.cancelled=false)", d3Second.cancelled, false);
await new Promise((r) => setTimeout(r, 50));
ok("abortableDebounce 第二次正常执行", d3SecondCalled === true);

const d4 = graphUi.makeAbortableDebouncer(20);
let d4ThrowCount = 0;
d4.schedule(() => { throw new Error("internal error"); });
d4.schedule(() => { d4ThrowCount = 1; });
await new Promise((r) => setTimeout(r, 50));
ok("abortableDebounce 错误不冒泡到下一次 schedule", d4ThrowCount === 1);

// ---- W12 request_id 唯一性 ----
const r1 = graphUi.newGraphRequestId();
const r2 = graphUi.newGraphRequestId();
const r3 = graphUi.newGraphRequestId();
ok("newGraphRequestId 唯一性(r1!=r2!=r3)", r1 !== r2 && r2 !== r3 && r1 !== r3);
ok("newGraphRequestId 格式(/^req-[0-9a-f]{8}$/)", /^req-[0-9a-f]{8}$/.test(r1));

// ---- W12 GRAPH_TRUNCATION_NOTICE / GRAPH_DEBOUNCE_MS 单源 ----
ok("GRAPH_TRUNCATION_NOTICE 非空(用户可见)", graphUi.GRAPH_TRUNCATION_NOTICE.length > 0);
eq("GRAPH_DEBOUNCE_MS=300(与 A7 §5 + W10 W11 一致)", graphUi.GRAPH_DEBOUNCE_MS, 300);

// ---- W12 K7 双闸 + 零 props 渲染（panel 文案 / 错误文案全不漏 props）----
const w12Texts = [
  graphUi.GRAPH_TRUNCATION_NOTICE,
  ...ALL_STABLE.map((c) => graphUi.applyGraphErrorView({ code: c }).message),
];
ok(
  "W12 用户可见文案不泄露 props/secret/token 关键字",
  w12Texts.every((t) => !/token|secret|api[_-]?key|password|Bearer|sk-|AKIA/.test(t)),
);

// ---- 结果 ----
console.log(`\n图谱 UI 逻辑测试：通过 ${passed}，失败 ${failed}`);
process.exit(failed === 0 ? 0 : 1);
