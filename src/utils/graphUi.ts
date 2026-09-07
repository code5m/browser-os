// src/utils/graphUi.ts
// M5-9 图谱 UI **纯逻辑层**：无 invoke、无 DOM、无 store 依赖，可被 node 直接 import 断言
// （见 scripts/check-graph-ui-logic.mjs）。
//
// 契约源：src-tauri/src/domain.rs（Lane A7，W5）：GraphNode / GraphEdge /
//   GraphNodeKind / GraphEdgeKind / GRAPH_MAX_* 常量。
// 边界（board M5-W6 Hard Stop）：不调用任何 graph_* 后端命令、不触发 agent 消费、
// 不构建/重建图、不引入新依赖；stores 有界；不显示 props 正文（K7）。

import type {
  GraphEdge,
  GraphEdgeKind,
  GraphNode,
  GraphNodeKind,
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
