// src/utils/graphUi.ts
// M5-9 图谱 UI **纯逻辑层**：无 invoke、无 DOM、无 store 依赖，可被 node 直接 import 断言
// （见 scripts/check-graph-ui-logic.mjs）。
//
// 契约源：src-tauri/src/domain.rs（Lane A7，W5）：GraphNode / GraphEdge /
//   GraphNodeKind / GraphEdgeKind / GRAPH_MAX_* 常量。
// 边界（board M5-W6 Hard Stop）：不调用任何 graph_* 后端命令、不触发 agent 消费、
// 不构建/重建图、不引入新依赖；stores 有界；不显示 props 正文（K7）。
//
// M5-W12 增量：新增 3 个纯函数 + 1 个常量 + 2 个类型，与 A7 §3.4 / §5 / §6 对齐。
//  - abortableDebounce：300ms 消抖 + AbortController 取消上一请求（前端 cancellation 唯一来源）
//  - formatGraphStableError：稳定错误码 → 用户可读中文（零 secret echo，零原始 error.message 透传）
//  - applyGraphErrorView：错误对象 → GraphErrorView（一致 UI 态输入）
//  - GRAPH_TRUNCATION_NOTICE：UI 截断提示文案（与 A7 §4 truncated/applied 信号配套）
//  - GraphQueryView / GraphStatsView：UI 内部窄类型，供 store/组件消费
//  - viewToNode / viewToEdge：view → 本地窄模型（删 props 是单向：从 store 的 View 拉下来后
//    即便外部强塞 props 也不会被还原；这是 K7 双闸的前端第三层）。

import type {
  GraphEdge,
  GraphEdgeKind,
  GraphNode,
  GraphNodeKind,
  // M5-W12 View DTO（删 props，K7 双闸；A7 §7.5 镜像）
  GraphEdgeView,
  GraphErrorView,
  GraphNodeView,
  GraphQueryLimits,
  GraphQueryRequest,
  GraphQueryResult,
  GraphStableErrorCode,
  GraphStats,
} from "../types";

// ====== 常量（与后端 domain.rs 对齐；前端不可自行放宽） ======
export const GRAPH_NODE_KINDS: ReadonlyArray<GraphNodeKind> = [
  "file", "dir", "tab", "script", "skill", "agent", "tag", "topic",
];
export const GRAPH_EDGE_KINDS: ReadonlyArray<GraphEdgeKind> = [
  "in_dir", "references", "related_to", "tagged_with", "uses", "a2a_with", "memorizes",
];

// 容量上限（与 domain.rs 一致；供 UI 容量计/守卫）。
export const GRAPH_MAX_NODES = 5000;
export const GRAPH_MAX_EDGES = 20000;
export const GRAPH_MAX_DEPTH = 4;
export const GRAPH_QUERY_LIMIT = 1000;

// ====== 展示用标签/配色 ======
export const NODE_KIND_LABEL: Readonly<Record<GraphNodeKind, string>> = {
  file: "文件", dir: "目录", tab: "页签", script: "脚本",
  skill: "技能", agent: "智能体", tag: "标签", topic: "主题",
};
export const EDGE_KIND_LABEL: Readonly<Record<GraphEdgeKind, string>> = {
  in_dir: "归属", references: "引用", related_to: "相关",
  tagged_with: "标注", uses: "使用", a2a_with: "A2A 协作", memorizes: "记忆",
};
export const NODE_KIND_COLOR: Readonly<Record<GraphNodeKind, string>> = {
  file: "#7c8cff", dir: "#5b8def", tab: "#13c2c2", script: "#fa8c16",
  skill: "#52c41a", agent: "#eb2f96", tag: "#faad14", topic: "#9254de",
};

// ====== 判别/展示 ======
export type GraphElement = GraphNode | GraphEdge;

export function elementKind(d: unknown): "node" | "edge" | "unknown" {
  if (!d || typeof d !== "object") return "unknown";
  const o = d as Record<string, unknown>;
  if (typeof o.id === "string" && "kind" in o && "label" in o && !("from" in o)) return "node";
  if (typeof o.from === "string" && typeof o.to === "string" && "kind" in o) return "edge";
  return "unknown";
}

export function nodeKindLabel(k: GraphNodeKind): string {
  return NODE_KIND_LABEL[k] ?? String(k);
}
export function edgeKindLabel(k: GraphEdgeKind): string {
  return EDGE_KIND_LABEL[k] ?? String(k);
}
export function nodeColor(k: GraphNodeKind): string {
  return NODE_KIND_COLOR[k] ?? "#8c8c8c";
}

// 节点摘要（K7：剔除 props 正文，摘要中绝不含 props 字段）
export interface NodeSummary {
  id: string;
  kind: GraphNodeKind;
  kindLabel: string;
  label: string;
  isAgentConsumption: boolean;
}
export function summarizeNode(n: GraphNode): NodeSummary {
  return {
    id: n.id,
    kind: n.kind,
    kindLabel: nodeKindLabel(n.kind),
    label: n.label,
    isAgentConsumption: n.kind === "agent" || n.kind === "skill",
  };
}

export interface EdgeSummary {
  from: string;
  to: string;
  kind: GraphEdgeKind;
  kindLabel: string;
  weight: number;
  agentConsumption: boolean;
}
export function summarizeEdge(e: GraphEdge): EdgeSummary {
  return {
    from: e.from,
    to: e.to,
    kind: e.kind,
    kindLabel: edgeKindLabel(e.kind),
    weight: e.weight,
    agentConsumption: e.kind === "uses" || e.kind === "a2a_with" || e.kind === "memorizes",
  };
}

// Agent 消费子视图：边是否把 Agent 连到 Skill/记忆/A2A（对应官方 M5-9 (b)）
export function isAgentConsumptionEdge(e: GraphEdge): boolean {
  return e.kind === "uses" || e.kind === "a2a_with" || e.kind === "memorizes";
}

// 边的稳定标识（from|to|kind）：用于确定性选择/高亮，避免依赖可见列表索引
// （索引随过滤变化而不稳定，W10 修正选中边漂移）。
export function edgeKey(e: GraphEdge): string {
  return `${e.from}|${e.to}|${e.kind}`;
}

// 确定性选择解析：在给定边集合中按稳定标识 `edgeKey` 解析选中边。
// 过滤变化后只要该边仍在可见集合即可还原，选择不随可见列表索引漂移。
// W11 收口为单一真源（store 的 selectedEdge 复用，杜绝选择逻辑散落）。
export function resolveEdgeByKey(
  edges: readonly GraphEdge[],
  key: string | null,
): GraphEdge | null {
  if (!key) return null;
  return edges.find((e) => edgeKey(e) === key) ?? null;
}

// ====== 布局（纯 TS，无 D3；确定性，无随机，便于测试） ======
export type LayoutKind = "tree" | "force" | "cluster";
export function layoutKindOf(edges: GraphEdge[]): LayoutKind {
  const hasInDir = edges.some((e) => e.kind === "in_dir");
  const hasCluster = edges.some((e) => e.kind === "tagged_with" || e.kind === "related_to");
  if (hasInDir && !hasCluster) return "tree";
  if (hasCluster) return "cluster";
  return "force";
}

export interface LayoutPoint { id: string; x: number; y: number; }
export function layoutPositions(nodes: GraphNode[], edges: GraphEdge[]): LayoutPoint[] {
  const kind = layoutKindOf(edges);
  const n = nodes.length || 1;
  if (kind === "tree") {
    // 按入度（作为"父"的被指向次数）分层；退化时按索引
    const childCount = new Map<string, number>();
    for (const e of edges) childCount.set(e.to, (childCount.get(e.to) ?? 0) + 1);
    return nodes.map((nd, i) => {
      const depth = Math.min(GRAPH_MAX_DEPTH, childCount.get(nd.id) ?? 0);
      const angle = (i / n) * Math.PI * 2;
      const radius = 80 + depth * 70;
      return { id: nd.id, x: 400 + Math.cos(angle) * radius, y: 300 + Math.sin(angle) * radius };
    });
  }
  if (kind === "cluster") {
    const cols = Math.ceil(Math.sqrt(n));
    return nodes.map((nd, i) => ({
      id: nd.id,
      x: 60 + (i % cols) * 120,
      y: 60 + Math.floor(i / cols) * 110,
    }));
  }
  // force -> 放射环
  return nodes.map((nd, i) => {
    const angle = (i / n) * Math.PI * 2;
    return { id: nd.id, x: 400 + Math.cos(angle) * 220, y: 300 + Math.sin(angle) * 220 };
  });
}

// ====== 容量/守卫 ======
export interface CapacityView {
  nodeCount: number;
  edgeCount: number;
  nodeUsedPct: number; // 0..100（基于 GRAPH_MAX_NODES）
  edgeUsedPct: number;
  withinLimit: boolean;
  maxDepth: number;
  queryLimit: number;
}
export function estimateCapacity(nodeCount: number, edgeCount: number): CapacityView {
  const nodeUsedPct = Math.min(100, Math.round((Math.max(0, nodeCount) / GRAPH_MAX_NODES) * 100));
  const edgeUsedPct = Math.min(100, Math.round((Math.max(0, edgeCount) / GRAPH_MAX_EDGES) * 100));
  return {
    nodeCount: Math.max(0, nodeCount),
    edgeCount: Math.max(0, edgeCount),
    nodeUsedPct,
    edgeUsedPct,
    withinLimit: nodeCount <= GRAPH_MAX_NODES && edgeCount <= GRAPH_MAX_EDGES,
    maxDepth: GRAPH_MAX_DEPTH,
    queryLimit: GRAPH_QUERY_LIMIT,
  };
}

// 容量健康度（确定性、无随机）：over=超上限，near=接近上限(>=90%)，ok=正常。
// 用于 UI 超量态横幅，避免对超限数据无提示。
export interface CapacityState {
  level: "ok" | "near" | "over";
  message: string;
}
export function capacityState(cap: CapacityView): CapacityState {
  if (!cap.withinLimit || cap.nodeUsedPct >= 100 || cap.edgeUsedPct >= 100) {
    return { level: "over", message: "图谱已达容量上限，仅展示已载入部分。" };
  }
  if (cap.nodeUsedPct >= 90 || cap.edgeUsedPct >= 90) {
    return { level: "near", message: "图谱接近容量上限，部分数据可能未载入。" };
  }
  return { level: "ok", message: "" };
}

// ====== 搜索/过滤 ======
export interface GraphFilterState {
  query: string;
  kinds: GraphNodeKind[]; // 空数组=不过滤
}
export function filterNodes(nodes: GraphNode[], f: GraphFilterState): GraphNode[] {
  const q = f.query.trim().toLowerCase();
  return nodes.filter((nd) => {
    if (f.kinds.length && !f.kinds.includes(nd.kind)) return false;
    if (q && !`${nd.label} ${nd.id} ${nodeKindLabel(nd.kind)}`.toLowerCase().includes(q)) return false;
    return true;
  });
}
export function filterEdges(edges: GraphEdge[], nodes: GraphNode[], f: GraphFilterState): GraphEdge[] {
  const visible = new Set(filterNodes(nodes, f).map((nd) => nd.id));
  const q = f.query.trim().toLowerCase();
  return edges.filter((e) => {
    if (!visible.has(e.from) || !visible.has(e.to)) return false;
    if (q && !edgeKindLabel(e.kind).toLowerCase().includes(q)) return false;
    return true;
  });
}

// ====== 有界合并（store 用：超过上限丢弃最旧） ======
export function boundedInsert<T extends { id: string }>(
  map: Map<string, T>,
  items: T[],
  max: number,
): Map<string, T> {
  const next = new Map(map);
  for (const it of items) {
    next.set(it.id, it);
    if (next.size > max) {
      const oldest = next.keys().next().value;
      if (typeof oldest === "string") next.delete(oldest);
    }
  }
  return next;
}

// ====== 渲染有界（UI 安全网）======
// 与数据有界(MAX_NODES/MAX_EDGES)同源：渲染护栏口径必须与数据上限严格一致，
// 故直接引用 GRAPH_MAX_NODES/GRAPH_MAX_EDGES（W11 修正 R-W9-3 源扩散，不得再写字面量）。
export const RENDER_NODE_CAP = GRAPH_MAX_NODES;
export const RENDER_EDGE_CAP = GRAPH_MAX_EDGES;
export interface RenderClamp<T> {
  items: T[];
  truncated: boolean;
  total: number;
}
export function clampRender<T>(items: readonly T[], cap: number): RenderClamp<T> {
  const total = items.length;
  if (total <= cap) return { items: items as T[], truncated: false, total };
  return { items: items.slice(0, cap) as T[], truncated: true, total };
}

// ====== 面板三态（空 / 错误 / 加载） ======
export interface PanelStateView {
  state: "loading" | "empty" | "ready" | "error";
  message: string;
}
export function panelStateGraph(opts: {
  loading: boolean;
  count: number;
  error: string | null;
  backendReady: boolean;
}): PanelStateView {
  if (opts.error) return { state: "error", message: opts.error };
  if (opts.loading) return { state: "loading", message: "加载中…" };
  if (opts.count === 0) {
    return {
      state: "empty",
      message: opts.backendReady
        ? "暂无图谱数据。点击「刷新」从后端载入。"
        : "后端 graph 命令尚未就绪，当前为只读壳。",
    };
  }
  return { state: "ready", message: "" };
}

// 错误格式化（不泄露内部栈）
export function formatGraphError(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  return "图谱加载失败";
}

// ====== M5-W12 增量：与 A7 §3.4 / §5 / §6 对齐的纯函数 ======

// 前端 300ms debounce 窗口（与 A7 §5 cancellation + W10 "cancel last query" 范式一致）。
// 仅消费方统一时延；后端本身是 bounded async stateless，无 IO 等待，故无需更复杂调度。
export const GRAPH_DEBOUNCE_MS = 300;

// 截断提示文案（A7 §4 truncated/applied 信号配套；不依赖 props / secret / URL）
export const GRAPH_TRUNCATION_NOTICE =
  "结果已按配置上限截断；如需查看完整数据，请缩小起始节点范围。";

// 稳定错误码 → 用户可读中文文案（与 A7 §3.4 1:1 镜像；**绝不**回显 label / props /
// 查询体 / 路径 / URL / token / cookie / Authorization / secret）。UI 直接消费，
// 不再做 e.message 透传（避免后端未来扩展文案时把不可控内容带到 UI）。
const GRAPH_STABLE_ERROR_CN: Readonly<Record<GraphStableErrorCode, string>> = {
  GRAPH_INVALID_ID: "节点 ID 格式不合法。",
  GRAPH_REF_ID_NOT_HEX: "引用 ID 必须为 64 位十六进制字符串。",
  GRAPH_LABEL_TOO_LONG: "节点标签超过长度上限。",
  GRAPH_PROP_KEY_TOO_LONG: "属性键名超过长度上限（不应抵达 UI；仅展示）。",
  GRAPH_PROP_VALUE_TOO_LONG: "属性值超过长度上限（不应抵达 UI；仅展示）。",
  GRAPH_PROP_COUNT_EXCEEDED: "属性数量超过上限（不应抵达 UI；仅展示）。",
  GRAPH_SECRET_IN_PROPS: "检测到疑似敏感字段，已阻断渲染。",
  GRAPH_NODE_CAPACITY_EXCEEDED: "节点容量已达上限。",
  GRAPH_EDGE_CAPACITY_EXCEEDED: "边容量已达上限。",
  GRAPH_DUPLICATE_NODE: "节点已存在。",
  GRAPH_DUPLICATE_EDGE: "边已存在。",
  GRAPH_STORE_LOAD_FAILED: "图谱快照载入失败，已回退为空视图。",
  GRAPH_UNKNOWN_ERROR: "图谱命令尚未就绪，请稍候刷新或联系维护者。",
};

/** 1:1 识别后端稳定码（与 A7 §3.4 13+1 表镜像）。 */
const GRAPH_STABLE_ERROR_KEYS = Object.keys(GRAPH_STABLE_ERROR_CN) as GraphStableErrorCode[];

export function isGraphStableErrorCode(s: string): s is GraphStableErrorCode {
  return (GRAPH_STABLE_ERROR_KEYS as string[]).includes(s);
}

/** 把任意错误对象（Error / 字符串 / 未知）映射到稳定 GraphErrorView。 */
export function applyGraphErrorView(e: unknown, fallbackCode: GraphStableErrorCode = "GRAPH_UNKNOWN_ERROR"): GraphErrorView {
  // 1) 优先识别 Error.code（Tauri reject 路径）
  if (e && typeof e === "object" && "code" in e) {
    const c = (e as { code?: unknown }).code;
    if (typeof c === "string" && isGraphStableErrorCode(c)) {
      return { code: c, message: GRAPH_STABLE_ERROR_CN[c] };
    }
  }
  // 2) 识别字符串 body（部分后端 reject 直接给字符串；强校验稳定码再回退 fallback）
  if (typeof e === "string" && isGraphStableErrorCode(e)) {
    return { code: e, message: GRAPH_STABLE_ERROR_CN[e] };
  }
  // 3) 兜底：丢弃原始 message（零 secret echo）
  return { code: fallbackCode, message: GRAPH_STABLE_ERROR_CN[fallbackCode] };
}

/** 仅生成 message（绝大多数 UI 只需 message）。零 props/secret/URL 透传。 */
export function formatGraphStableError(e: unknown, fallbackCode: GraphStableErrorCode = "GRAPH_UNKNOWN_ERROR"): string {
  return applyGraphErrorView(e, fallbackCode).message;
}

// 工具：把 GraphNodeView 拉成 UI 内部"窄节点"（删 props 已是 View 层的语义，但本函数再
// 做一次**显式字段提取**作为前端第三闸：即便上游误传含 props 字段的伪造对象，
// 落到 store 后也只保留 id/kind/label 三字段）。这是 K7 双闸在 UI 层的**纵深防御**：
//   ① 后端 GraphNodeView::from(GraphNode) drop props（编译期）
//   ② serde 序列化层（运行期）
//   ③ 前端 viewToNode 字段白名单（UI 层）
export interface UiNode {
  id: string;
  kind: GraphNodeKind;
  label: string;
}
export function viewToNode(v: GraphNodeView): UiNode {
  return { id: v.id, kind: v.kind, label: v.label };
}

export interface UiEdge {
  from: string;
  to: string;
  kind: GraphEdgeKind;
}
export function viewToEdge(e: GraphEdgeView): UiEdge {
  return { from: e.from, to: e.to, kind: e.kind };
}

/** 把 GraphQueryResult 的 View[] 拉成 UI 窄集合（确定性 + 删 props）。 */
export interface GraphQueryView {
  found: boolean;
  nodes: UiNode[];
  edges: UiEdge[];
  truncated: boolean;
  applied: GraphQueryLimits;
  nodeCount: number;
  edgeCount: number;
}
export function viewToQueryResult(r: GraphQueryResult): GraphQueryView {
  return {
    found: r.found,
    nodes: r.nodes.map(viewToNode),
    edges: r.edges.map(viewToEdge),
    truncated: r.truncated,
    applied: r.applied,
    nodeCount: r.node_count,
    edgeCount: r.edge_count,
  };
}

/** 把 GraphStats 拉成 UI 内部"窄容量视图"（与 estimateCapacity 对齐）。 */
export function viewToStats(s: GraphStats): CapacityView {
  return estimateCapacity(s.node_count, s.edge_count);
}

/** 判断 stats 是否逼近容量（≥90% 黄牌），供 banner 触发。 */
export function isApproachingCapacity(s: GraphStats): boolean {
  return s.approaching_node_capacity || s.approaching_edge_capacity;
}

// ====== 取消上一个请求的简易 debouncer（A7 §5 落地）======
// 与 useGraphStore / W10 W11 searchGraphNodes 范式同源：
//   - 调用 schedule(fn) 时若存在上一 token 则 abort；再起新 fn，返回新 token。
//   - fn 必须接受一个 AbortSignal 形参（用于未来桥接 invoke signal）。
//   - 不依赖任何 Tauri / 浏览器 DOM，仅 token + Promise（Node 端可测）。
// 边界：
//   - 仅 debounce 时序控制；**不**保证 fn 必然被调用（用户在窗口内连击 → 仅最后一次触发）。
//   - 取消通过 `token.cancelled` 标记；fn 内部自行检查（也可把 AbortSignal 传过去供 invoke 复用）。
// 不可用副作用：纯函数 + 一次性内部状态（闭包），多次实例互不干扰。
export interface AbortableToken {
  id: number;
  cancelled: boolean;
  signal: AbortSignal;
}
export interface AbortableDebouncer {
  schedule: (fn: (token: AbortableToken) => void | Promise<void>) => AbortableToken;
  cancelAll: () => void;
  pending: () => AbortableToken | null;
}
export function makeAbortableDebouncer(delayMs: number = GRAPH_DEBOUNCE_MS): AbortableDebouncer {
  let current: AbortableToken | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let seq = 0;
  return {
    schedule(fn) {
      if (current) current.cancelled = true;
      // 跨环境构造 abort signal：
      //  - 浏览器 / 现代 Node 全局 AbortController；
      //  - 测试桩 / 极旧运行时无 AbortController 时自管 minimal signal（仅 cancelled + noop abort）。
      // 这样既能让 bridge.graphQuery(req, signal) 走 Tauri v2 的 AbortSignal 路径，
      // 又能在 Node 测试脚本里独立跑通（无副作用）。
      const signals: AbortableToken["signal"] = (() => {
        const ac = (globalThis as { AbortController?: new () => AbortController }).AbortController;
        if (typeof ac === "function") {
          const c = new ac();
          return c.signal as unknown as AbortableToken["signal"];
        }
        return { aborted: false, abort() { this.aborted = true; } } as unknown as AbortSignal;
      })();
      const token: AbortableToken = { id: ++seq, cancelled: false, signal: signals };
      current = token;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        if (token.cancelled) return;
        Promise.resolve(fn(token)).catch(() => { /* swallow; UI 显示在调用方处理 */ });
      }, Math.max(0, delayMs));
      return token;
    },
    cancelAll() {
      if (current) {
        current.cancelled = true;
        if (current.signal && typeof (current.signal as { abort?: () => void }).abort === "function") {
          (current.signal as { abort: () => void }).abort();
        }
      }
      current = null;
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
    },
    pending() {
      return current;
    },
  };
}

// ====== request_id 关联（避免乱序回包覆盖；A7 §5 契约）======
// 后端 GraphQueryRequest.request_id 字段（optional）。前端 store 在发起新 query 时把
// request_id 记到一张 map；回包时（resolve 后）查 map 命中即清掉，不命中则视为过时回包
// 丢弃（不打乱当前视图）。这是 cancellation 之外的"乱序"维度。
let __reqIdSeq = 0;
export function newGraphRequestId(): string {
  // 32-bit 单调递增序列化为 8-hex 短串（与后端 request_id? 字符串类型一致；不冲突即可）
  __reqIdSeq = (__reqIdSeq + 1) | 0;
  return `req-${__reqIdSeq.toString(16).padStart(8, "0")}`;
}

/** 把 GraphQueryRequest 的 start_id / depth / limit 应用 GRAPH_MAX_* / GRAPH_QUERY_LIMIT 裁剪。 */
export function normalizeGraphQueryRequest(req: GraphQueryRequest): {
  start_id: string;
  depth: number;
  limit: number;
  request_id: string;
} {
  const depth = Math.max(0, Math.min(GRAPH_MAX_DEPTH, req.depth ?? 2));
  const limit = Math.max(1, Math.min(GRAPH_QUERY_LIMIT, req.limit ?? GRAPH_QUERY_LIMIT));
  return {
    start_id: req.start_id,
    depth,
    limit,
    request_id: req.request_id ?? newGraphRequestId(),
  };
}
