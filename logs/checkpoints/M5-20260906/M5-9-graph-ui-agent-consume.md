# M5-9 图谱 UI + Agent 消费（力导向 + RAG 注入）

> 子卡 ID：**M5-9** · 需求 #13 · `[S3|LEVERAGE:1|COMPLEX|AI:NORMAL|R:high]`
> 责任 Lane 候选：**A19**
> 父卡：`详细设计与实施计划.md` L572（`M5-9 图谱 UI 与 Agent 消费`）
> 主预研：暂无 prework 文档（A8 prework 仍空）
> 配套：`M5-8-graph-store-query.md`（DTO 稳定）

---

## 0. 编号与锚定

- 批次任务号 `M5-9`；需求号 #13；WBS L572 一致。
- 依赖：M5-8 ✅（9 条命令稳定）+ M5-6 ✅（既有 `useAgentStore` 已落地）
- 前端栈：Vue 3 + Pinia + D3.js（力导向布局）

---

## 1. GOAL

实现图谱可视化 UI（节点/边力导向布局、按 kind 着色、点击查看详情、拖拽、缩放、过滤）与 Agent RAG 注入（用户在 Agent 对话中显式"基于图谱"提问时，按 `graph_query` 取相关节点/边摘要注入 `system_prompt`）。

---

## 2. READ

1. `src/components/browser/AINavPanel.vue`（**全读**——`useAgentStore` 桥接）
2. `M5-6-agent-skill-ui.md`（既有 Agent UI 范式）
3. `M5-8-graph-store-query.md` §4.2（`GraphQuery` DTO + 上限）
4. `src/bridge.ts`（`graph_*` TS 包装，**全读**）
5. `src/types.ts`（`GraphNode` / `GraphEdge` TS 镜像）
6. `src/router/agent.ts`（路由）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/components/graph/GraphViewer.vue` | **新增** | D3.js 力导向布局 + 拖拽/缩放/过滤 |
| `src/components/graph/NodeDetail.vue` | **新增** | 节点详情（不显示 props 正文） |
| `src/components/graph/EdgeDetail.vue` | **新增** | 边详情 |
| `src/components/graph/GraphFilter.vue` | **新增** | 按 kind / source 过滤 |
| `src/composables/useGraphQuery.ts` | **新增** | 包装 `graph_query` + 缓存 + 取消 |
| `src/composables/useGraphRag.ts` | **新增** | RAG 注入（显式触发） |
| `src/stores/useGraphStore.ts` | **新增** | 图谱状态（选中节点、视图参数、缓存） |
| `src/router/graph.ts` | **新增** | 图谱路由 |
| `src/locales/zh-CN.json` `src/locales/en-US.json` | 扩展 | 8 个 i18n key |
| `src/styles/graph.scss` | **新增** | 样式（dark/light 双主题） |

---

## 4. 关键契约

### 4.1 `GraphViewer.vue` 交互

- **节点**：圆 + 标签 + 颜色按 kind；大小按 degree（边数）
- **边**：线 + 箭头 + 粗细按 weight
- **交互**：拖拽（d3-drag）+ 缩放（d3-zoom）+ 点击（高亮邻居 + 打开详情）
- **过滤**：`GraphFilter` 多选 kind + source
- **布局**：d3-force（forceLink + forceManyBody + forceCenter）
- **首期上限**：可见节点 ≤ 500（多则提示"过滤缩小"）

### 4.2 `NodeDetail.vue` / `EdgeDetail.vue`

- 显示：id / kind / label / source / source_ref / created_at / updated_at / extractor_version
- **不**显示 `props` 正文（K7）
- `source_ref` 路径点击 → 打开对应文件/tab（受 `capability.rs` 校验）

### 4.3 RAG 注入契约（`useGraphRag.ts`）

- **仅显式触发**：用户在 ChatPanel 输入框加 `/graph <query>` 前缀
- 流程：
  1. 解析 `query` → 调 `useGraphQuery.search`（先按 label 模糊匹配 FTS5 起点）
  2. 起点 + `max_depth=2` 调 `graph_query`
  3. 把结果序列化为"图谱上下文"（节点 label + kind + 关系描述，**不**含 props）
  4. 注入到 `agent_chat` 的 `system_prompt` 临时扩段（首期仅限本轮）
  5. UI 标注"已基于图谱"标记
- **禁**隐式注入（避免 token 浪费 + 上下文污染）

### 4.4 性能与可访问性

- 大图（> 500 节点）自动隐藏弱连接
- 键盘快捷键：`Esc` 关闭详情，`f` 打开过滤
- ARIA 标签：节点/边可读屏

---

## 5. FORBID

- **不**让 `NodeDetail` / `EdgeDetail` 显示 `props` 正文（K7）
- **不**让 RAG 注入隐式触发（仅 `/graph` 前缀）
- **不**让 UI 路径绕过 `bridge.ts`（M5-6 范式）
- **不**让图谱渲染不节流（拖拽时 ≥ 30fps 目标）
- **不**破 K7（props 不出 UI）
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 文件结构
ls src/components/graph src/composables/useGraph* 2>&1

# B. 反向用例
# N1: NodeDetail 显示 props → 阻断
# N2: RAG 隐式注入（无 /graph 前缀）→ 阻断
# N3: 拖拽不节流 → 阻断
# N4: dark/light 主题崩 → 阻断
# N5: i18n 缺 key → 阻断
# N6: 屏读器无法读 → 阻断

# C. e2e 冒烟
pnpm test:unit
pnpm test:e2e

# D. 编译与基线
pnpm build
pnpm tsc --noEmit
pnpm lint
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 5 个新组件 + 2 个 composable + 1 store 落地 | 命令 A |
| 2 | `props` 正文不显示 | 单测 N1 |
| 3 | RAG 仅 `/graph` 前缀触发 | 单测 N2 |
| 4 | 拖拽节流 ≥ 30fps | 单测 N3 |
| 5 | dark/light 主题适配 | 单测 N4 |
| 6 | i18n 双语齐 | 单测 N5 |
| 7 | ARIA 标签 | 单测 N6 |
| 8 | `pnpm test:unit` + `pnpm test:e2e` 全绿 | 命令 C |
| 9 | `pnpm build` 无错 | 命令 D |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| `props` 出现在 UI | 阻断（K7） |
| 隐式 RAG 注入 | 阻断（token 浪费 + 污染） |
| 裸 `invoke` | 阻断 |
| dark/light 主题崩 | 阻断 |
| i18n 缺 key | 阻断 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L572 `[ ]` → `[x]`
2. `后续需求TODO.md` §13 状态 `DONE`
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-10`
4. `logs/checkpoints/M5-9.a-2026MMDD-HHMM.md`

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A19 实施填
- **NEXT**：M5-10（插件 manifest 与生命周期）

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
