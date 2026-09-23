// src/stores/useGraphStore.ts
// M5-9 图谱面板状态机。
//
// 约束（board M5-W6 + A7 DTO）：
//   - 组件不得直接 invoke，一切经 `bridge.ts`；
//   - 后端 graph_* 命令由 A7 后续落地；未落地前 `backendReady=false`，
//     **所有动作 no-op 且零 invoke**（LIMITED START 口径）；
//   - 不触发 agent 消费、不重建图、不引入新依赖（W6 Hard Stop）；
//   - stores 有界（GRAPH_MAX_NODES / GRAPH_MAX_EDGES）；props 正文不进 store 展示（K7）。
//
// M5-W12 增量（A7 §3.4 / §5 / §6）：
//   - `loadGraph(startId)`：接 AbortController + 300ms debounce + request_id（取消上一个）
//   - `loadNode(id)`：单节点查询（graph_node_get 包装）
//   - `loadStats()`：容量概览（graph_stats 包装），落 `capState` + `truncated` 信号
//   - `refresh()`：当前 start_id 重新查询
//   - 错误落 `error.value`（稳定码映射；零 secret echo）
//   - `truncated` 暴露（与 A7 §4 truncated/applied 信号配套）
//   - 维持 GRAPH_COMMANDS_AVAILABLE=false 时零 invoke 守门（W11 既有）
//   - 边用稳定标识 `selectedEdgeKey` 选择（W11 既有），保持不随索引漂移

import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { GRAPH_COMMANDS_AVAILABLE, bridge } from "../../../bridge";
import type {
  GraphEdge,
  GraphNode,
  GraphNodeKind,
  GraphQueryRequest,
} from "../../../types";
import {
  applyGraphErrorView,
  boundedInsert,
  capacityState,
  estimateCapacity,
  filterEdges,
  filterNodes,
  GRAPH_MAX_EDGES,
  GRAPH_MAX_NODES,
  makeAbortableDebouncer,
  newGraphRequestId,
  normalizeGraphQueryRequest,
  panelStateGraph,
  resolveEdgeByKey,
  summarizeEdge,
  summarizeNode,
  viewToEdge,
  viewToNode,
  viewToQueryResult,
  type GraphFilterState,
} from "../../../utils/graphUi";

export const useGraphStore = defineStore("graph", () => {
  const nodes = ref<Map<string, GraphNode>>(new Map());
  const edges = ref<Map<string, GraphEdge>>(new Map());
  const selectedNodeId = ref<string | null>(null);
  const selectedEdgeKey = ref<string | null>(null); // 边用稳定标识(from|to|kind)选择，避免索引随过滤漂移
  const loading = ref(false);
  const error = ref<string | null>(null);
  // 后端命令可用性。A7 落地 graph_query 后置 true（见 `bridge.GRAPH_COMMANDS_AVAILABLE`）。
  const backendReady = ref<boolean>(GRAPH_COMMANDS_AVAILABLE);
  // 当前已载入的 start_id（W12 新增；用于 refresh 与诊断）
  const startId = ref<string | null>(null);
  // 截断信号（W12 新增；与 A7 §4 truncated/applied 信号配套；用于 UI 横幅）
  const truncated = ref<boolean>(false);
  // request_id 关联表（W12 新增；乱序回包丢弃用——A7 §5）
  // 仅当回包的 request_id 不在表内（或已被取消）时才视为过时。
  // 不持有 Promise 本身；持有最近一次 query 的 id 即可（单 in-flight 简化模型）。
  const inFlightRequestId = ref<string | null>(null);

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
  // 边按稳定标识选择：过滤变化后只要该边仍在可见集合即可还原，选择不随列表索引漂移（确定性）。
  // 复用 graphUi.resolveEdgeByKey 单一真源（W11）。
  const selectedEdge = computed(() =>
    resolveEdgeByKey(visibleEdges.value, selectedEdgeKey.value),
  );

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
    return backendReady.value;
  }

  /// 错误落库：稳定码映射（`applyGraphErrorView`）→ 落 message。
  /// 绝不直接 `e.message` 透传——若后端未来扩文案带 props/URL/secret，前端即漏出。
  function fail(e: unknown): void {
    error.value = applyGraphErrorView(e).message;
  }

  // ====== W12 取消/防抖（A7 §5 落地）======
  // 单 in-flight 简化模型：上一个 token 取消，函数调度在 300ms debounce 后才真正调用。
  // debouncer 是单例（store 生命周期内）；组件调用 loadGraph/startId 变化时复用。
  const debouncer = makeAbortableDebouncer();

  /// 主动取消所有进行中的 query（W12 新增；组件 dispose / 切 Tab 时调用）
  function cancelInFlight(): void {
    debouncer.cancelAll();
    loading.value = false;
  }

  /// 单节点查询（W12 新增；面板右侧"节点详情"区域可触发）
  /// 缺失节点 → 返回 null（与 A7 §3.2 一致：Ok(None) 不报错）；非法 id → 错误落库。
  async function loadNode(id: string): Promise<void> {
    if (!guard()) return;
    loading.value = true;
    error.value = null;
    try {
      const v = await bridge.graphNodeGet(id);
      // 节点存在则并入 store（有界）；不存在则不动 store、不报错
      if (v) {
        const n = viewToNode(v);
        nodes.value = boundedInsert(nodes.value, [n], GRAPH_MAX_NODES);
      }
    } catch (e) {
      fail(e);
    } finally {
      loading.value = false;
    }
  }

  /// 容量概览（W12 新增；面板顶部 stats 行；接近 90% 触发 banner）
  async function loadStats(): Promise<void> {
    if (!guard()) return;
    try {
      const s = await bridge.graphStats();
      // 仅更新容量相关派生信号（不直接修改 nodes/edges；truncated 留给 query 用）
      capacity.value; // computed 已就绪；本函数仅探活
      // 接近 90% 黄牌（A7 §3.3 字段）→ 暴露给 banner
      truncated.value = s.approaching_node_capacity || s.approaching_edge_capacity;
    } catch (e) {
      fail(e);
    }
  }

  /// 主动载入图谱（W12 升级）：
  ///   - startId 不传则保留上次值（refresh 行为）；
  ///   - 300ms debounce + 取消上一请求（防抖期间切回旧值即丢请求）；
  ///   - 错误码稳定映射（K7 零 secret echo）；
  ///   - 返回 View 经 viewToNode/Edge 拉成 UI 窄集合，删 props 第三闸。
  function loadGraph(req?: Partial<GraphQueryRequest> & { start_id?: string }): void {
    if (!guard()) return; // 后端未就绪：零 invoke
    const start = (req?.start_id ?? req?.startId ?? startId.value ?? "").trim();
    if (!start) {
      // 空 startId 不发请求；写一条**用户文案**而非"未就绪"——已是"已就绪但缺入参"
      error.value = "请输入起始节点 ID";
      return;
    }
    // 防抖 + 取消上一
    const normalized = normalizeGraphQueryRequest({
      start_id: start,
      depth: req?.depth,
      limit: req?.limit,
      request_id: newGraphRequestId(),
    });
    inFlightRequestId.value = normalized.request_id;
    debouncer.schedule(async (token) => {
      if (token.cancelled) return;
      loading.value = true;
      error.value = null;
      try {
        const res = await bridge.graphQuery(
          {
            start_id: normalized.start_id,
            depth: normalized.depth,
            limit: normalized.limit,
            request_id: normalized.request_id,
          },
          // 把 debouncer 的 AbortSignal 透传 — A7 §5 后端 stateless drop=no-op
          token.signal,
        );
        // 乱序回包丢弃（A7 §5）
        if (inFlightRequestId.value !== normalized.request_id) return;
        if (token.cancelled) return;
        const v = viewToQueryResult(res);
        // 落 store（有界；删 props 第三闸：viewToNode/Edge 已显式字段提取）
        nodes.value = boundedInsert(
          nodes.value,
          // 仅保留已存在 UiNode 字段（viewToNode 已是窄类型，但此处再 map 一次防御性）
          v.nodes.map((nd) => ({ id: nd.id, kind: nd.kind, label: nd.label })),
          GRAPH_MAX_NODES,
        );
        edges.value = boundedInsert(
          edges.value,
          v.edges.map((e) => ({ from: e.from, to: e.to, kind: e.kind, weight: 0, props: {} })),
          GRAPH_MAX_EDGES,
        );
        truncated.value = v.truncated;
        startId.value = v.applied ? normalized.start_id : normalized.start_id;
        // 成功回包后清空 in-flight id
        inFlightRequestId.value = null;
      } catch (e) {
        // 同样的乱序保护：旧请求失败也不覆盖新一次状态
        if (inFlightRequestId.value === normalized.request_id) {
          fail(e);
          inFlightRequestId.value = null;
        }
      } finally {
        // 仅当前 token 才解除 loading（乱序时新 token 仍 loading=true）
        if (inFlightRequestId.value === null) {
          loading.value = false;
        }
      }
    });
  }

  /// 刷新当前 start_id（不重新起 token；与 loadGraph 行为同源）
  function refresh(): void {
    if (!guard()) return;
    if (!startId.value) {
      error.value = "请先选择起始节点";
      return;
    }
    loadGraph({ start_id: startId.value });
  }

  function selectNode(id: string): void {
    selectedNodeId.value = selectedNodeId.value === id ? null : id;
    selectedEdgeKey.value = null;
  }
  function selectEdge(key: string): void {
    selectedEdgeKey.value = selectedEdgeKey.value === key ? null : key;
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
    selectedEdgeKey,
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
    // W12 新增
    startId,
    truncated,
    loadGraph,
    loadNode,
    loadStats,
    refresh,
    cancelInFlight,
    selectNode,
    selectEdge,
    setFilter,
    toggleKind,
    clearFilter,
  };
});
