// src/stores/useGraphStore.ts
// M5-9 图谱面板状态机。
//
// 约束（board M5-W6 + A7 DTO）：
//   - 组件不得直接 invoke，一切经 `bridge.ts`；
//   - 后端 graph_* 命令由 A7 后续落地；未落地前 `backendReady=false`，
//     **所有动作 no-op 且零 invoke**（LIMITED START 口径）；
//   - 不触发 agent 消费、不重建图、不引入新依赖（W6 Hard Stop）；
//   - stores 有界（GRAPH_MAX_NODES / GRAPH_MAX_EDGES）；props 正文不进 store 展示（K7）。

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { GRAPH_COMMANDS_AVAILABLE, bridge } from "../bridge";
import type { GraphEdge, GraphNode, GraphNodeKind } from "../types";
import {
  boundedInsert,
  capacityState,
  estimateCapacity,
  filterEdges,
  filterNodes,
  panelStateGraph,
  summarizeEdge,
  summarizeNode,
  type GraphFilterState,
} from "../utils/graphUi";

const MAX_NODES = 5000; // = GRAPH_MAX_NODES
const MAX_EDGES = 20000; // = GRAPH_MAX_EDGES

export const useGraphStore = defineStore("graph", () => {
  const nodes = ref<Map<string, GraphNode>>(new Map());
  const edges = ref<Map<string, GraphEdge>>(new Map());
  const selectedNodeId = ref<string | null>(null);
  const selectedEdgeIdx = ref<number | null>(null); // 边可能重 key，用可见列表索引
  const loading = ref(false);
  const error = ref<string | null>(null);
  // 后端命令可用性。A7 落地 graph_query 后置 true（见 `bridge.GRAPH_COMMANDS_AVAILABLE`）。
  const backendReady = ref<boolean>(GRAPH_COMMANDS_AVAILABLE);

  // 过滤状态（UI 受控）
  const filter = ref<GraphFilterState>({ query: "", kinds: [] });

  const nodeList = computed(() => [...nodes.value.values()]);
  const edgeList = computed(() => [...edges.value.values()]);

  const visibleNodes = computed(() => filterNodes(nodeList.value, filter.value));
  const visibleEdges = computed(() => filterEdges(edgeList.value, nodeList.value, filter.value));

  const capacity = computed(() => estimateCapacity(nodes.value.size, edges.value.size));
  const capState = computed(() => capacityState(capacity.value));
  // 只读壳标志：后端 graph 命令未落地时为 true（预期态，非失败）。
  const readOnly = computed(() => !backendReady.value);

  // 当前选择的可读播报（供 GraphViewer/详情区 aria-live 公告，确定性拼接）
  const selectionText = computed(() => {
    if (selectedNode.value) {
      const s = summarizeNode(selectedNode.value);
      return `已选择${s.kindLabel}节点 ${s.label}`;
    }
    if (selectedEdge.value) {
      const s = summarizeEdge(selectedEdge.value);
      return `已选择${s.kindLabel}边（${s.from} → ${s.to}）`;
    }
    return "";
  });

  const selectedNode = computed(() =>
    selectedNodeId.value ? nodes.value.get(selectedNodeId.value) ?? null : null,
  );
  // 选择发生在可见列表，故按 visibleEdges 索引还原
  const selectedEdge = computed(() => {
    if (selectedEdgeIdx.value === null) return null;
    return visibleEdges.value[selectedEdgeIdx.value] ?? null;
  });

  const state = computed(() =>
    panelStateGraph({
      loading: loading.value,
      count: nodes.value.size,
      error: error.value,
      backendReady: backendReady.value,
    }),
  );

  /// 后端未就绪时统一拦截：**一条 invoke 都不发**，避免对不存在的命令反复报错。
  /// 注意：不写入 `error`——"未就绪"是预期态（只读壳），不是失败；真正的错误才进 `error`，
  /// 以免把"尚未实现"误报成红色错误横幅（W9 修正：与 readOnly 标志分流）。
  function guard(): boolean {
    if (backendReady.value) return true;
    return false;
  }
  function fail(e: unknown): void {
    error.value = e instanceof Error ? e.message : String(e ?? "未知错误");
  }

  async function loadGraph(): Promise<void> {
    if (!guard()) return; // 后端未就绪：零 invoke
    loading.value = true;
    error.value = null;
    try {
      // 命令落地后在此调用 bridge.graphQuery() 并 boundedInsert；当前因 guard 不会到达。
      const res = await bridge.graphQuery();
      nodes.value = boundedInsert(nodes.value, res.nodes, MAX_NODES);
      edges.value = boundedInsert(edges.value, res.edges, MAX_EDGES);
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  function selectNode(id: string): void {
    selectedNodeId.value = selectedNodeId.value === id ? null : id;
    selectedEdgeIdx.value = null;
  }
  function selectEdge(idx: number): void {
    selectedEdgeIdx.value = selectedEdgeIdx.value === idx ? null : idx;
    selectedNodeId.value = null;
  }
  function setFilter(f: Partial<GraphFilterState>): void {
    filter.value = { ...filter.value, ...f };
  }
  function toggleKind(k: GraphNodeKind): void {
    const has = filter.value.kinds.includes(k);
    filter.value = {
      ...filter.value,
      kinds: has ? filter.value.kinds.filter((x) => x !== k) : [...filter.value.kinds, k],
    };
  }
  function clearFilter(): void {
    filter.value = { query: "", kinds: [] };
  }

  return {
    nodes,
    edges,
    selectedNodeId,
    selectedEdgeIdx,
    loading,
    error,
    backendReady,
    readOnly,
    filter,
    nodeList,
    edgeList,
    visibleNodes,
    visibleEdges,
    capacity,
    capState,
    selectionText,
    selectedNode,
    selectedEdge,
    state,
    loadGraph,
    selectNode,
    selectEdge,
    setFilter,
    toggleKind,
    clearFilter,
  };
});
