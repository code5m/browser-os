# A8 M5-W12 Graph UI Consumer Checkpoint

> Lane: A8（Graph UI 唯一持有者）
> 派工条目：PARALLEL_COMMAND_BOARD.md W12 §A8 START PRODUCT CODE UI NARROW
> 任务：消费 W12 read-only graph commands (graph_query / graph_node_get / graph_stats)
> 集成次序：先 A7 落地命令 → A0 集成 A8 本 patch → 命令面解锁（GRAPH_COMMANDS_AVAILABLE=true 已就位）
> 工作树：6 files / +820 / -17（本 lane 自身）；未触碰 src-tauri/* 与 check-mcp-policy.py
> 验证：113/113 + build OK + git diff --check clean + A8 自身门禁全绿
> 未 push：留待 A0 集成（与 W11 A8 patch 风格一致）

## 交付（6 文件 +820/-17）

| 文件 | 增量 | 内容 |
| --- | --- | --- |
| src/types.ts | +139 | 6 个 W12 View DTO + 13 稳定错误码联合 + GraphErrorView |
| src/bridge.ts | +57 | 升级 graphQuery 接 AbortSignal；新增 graphNodeGet/graphStats + 命令禁用错误 |
| src/utils/graphUi.ts | +228 | 6 个 W12 纯函数 + 3 个新类型 + 2 个新常量 + 1 个 AbortableToken 接口 |
| src/stores/useGraphStore.ts | +169/-X | loadGraph 重写 + loadNode/loadStats/refresh/cancelInFlight 4 个新方法 |
| src/components/graph/GraphPanel.vue | +62 | start_id 输入区 + 截断 banner + 刷新按钮 + onBeforeUnmount 取消 |
| scripts/check-graph-ui-logic.mjs | +182 | W12 70+ 增量断言（13 码 / 删 props / 容量裁剪 / 防抖 / 兜底） |

## 关键契约边界（与 A7 W12 实施卡 §3.4 / §5 / §7.5 镜像）

- 字段命名：snake_case 直对齐 domain.rs（W12 §3.1 表 1:1）
- 错误码：13 码 1:1（无 GRAPH_TRUNCATED — 实际是 result 字段非错误码；与 A7 graph.rs L80-92 一致）
- K7 三闸：① 后端 `GraphNodeView::from` ② serde ③ 前端 `viewToNode/Edge` 显式提取
- Cancellation：300ms debounce + 跨环境 AbortController polyfill + 取消上一
- request_id 关联：A7 §5 乱序回包丢弃

## 自检

```
node scripts/check-graph-ui-logic.mjs   → 113/113 PASS
npm run build                          → exit 0 (GraphPanel 15.07 kB)
git diff --check                       → clean
```

## Open to A0

详见 `logs/assist/A8-M5-W12-graph-ui-consumer-20260907-2251.md` §8。

集成时无需修改本 patch 的任何内容；`GRAPH_COMMANDS_AVAILABLE` 已是 `true`
（部署段已翻），直接应用即可。

## Patch

`logs/checkpoints/Lane-A8-M5-W12-graph-ui-consumer-20260907-2251.patch`（47.8 KB）
