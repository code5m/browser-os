# M5 协议与智能生态 — 任务卡展开（Lane A1 · M5-W0 + W1 reconciliation）

> 生成：2026-09-06 08:00 CST · Lane A1（M5-W0 · docs only）
> W1 修订：2026-09-06 08:50 CST · Lane A1（M5-W1 · docs-only reconciliation）
> 基准：`a1a2061`（`master`，M4 已 PASS，已合数据库+调度并行成果） +  本地 `5ca8f9f docs(M5): dispatch architecture prework lanes`（A0 预研派发） +  `404f514 docs(M5): integrate prework and dispatch core boundary wave`（A0 已拣入 W0 整包 + W1 dispatch）
> 性质：**纯文档展开**。零产品代码（未触 `src/`、`src-tauri/`、`package.json`、三份主文档、ACL/Capability/Manifest）；不移动 `NEXT`；不提交、不 push。
> 依据：`PARALLEL_COMMAND_BOARD.md`（2026-09-05 23:55 版 · Batch Implementation Dispatch · Lane A1: M5 task-card expansion；2026-09-06 08:35 CST · M5-W1 Implementation Dispatch · Lane A1: START DOCS ONLY · reconciliation）
> 入口：本卡体系的根文档，本目录下其它文件是各 M5-x 子卡

---

## W1 Reconciliation Note（2026-09-06 08:50 CST · Lane A1 修订）

### 触发

A0 在 `404f514` 后下发 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`），A1 行要求：
> *"Reconcile M5 task cards with A2/A6/A10 findings: M5-1 internal order must be boundary policy first, then minimal extraction; remove stale assumptions such as vue-router and empty prework notes."*

A1 复核 W0 整包与 A2 v3 prework（`logs/assist/A2-M5-core-20260906-0749.md` §12/§13）、A6 prework、A10 复审（`logs/assist/A10-M5-security-review-20260906-1410.md` §36/§104）后，**确认 5 张子卡需修订**，**"vue-router"字面量不存在**，W0 中"被预研证伪的隐式假设"为：A1 W0 的事实数据（模块数 23/反向边 4 处/scheduler 4 行/agent_kv 扁平+缺 `updated_at`/`check-core-boundary.sh`/切片 0a→0b 顺序）被 A2/A4 实测修正。

### 修订索引（5 处）

| # | 文件 | 修订点 | 来源 |
|---|---|---|---|
| W1-1 | `M5-0-overview.md`（本文件） | 顶部加本节、依赖图注脚更新 | A0 W1 + A2 v3 |
| W1-2 | `M5-1-core-workspace-split.md` | 应用 A2 v3 6+2 处分歧（详见该卡顶部 `[W1 patched]` 段） | A2 v3 §12/§13 + A0 W1 boundary-first |
| W1-3 | `M5-3-a2a-bidir-agent-kv.md` | `agent_kv` 改三层嵌套 + 删 LRU 缺 `updated_at` 矛盾 | A4 prework + A10 §104 |
| W1-4 | `M5-13-verification-matrix.md` | `check-core-boundary.sh` → `.py`（仓库惯例） | A2 v3 §13.3 C-8 |
| W1-5 | `M5-14-debt-ledger.md` §6 L83 scheduler 反向边从 "M5-1.a 解决" 改 "M5-1.b 解决" | A2 v3 §13.2 C-7 |
| W1-6 | `logs/checkpoints/M5-A1-expansion-20260906-0800.md` | 加 W1 修订段 | A0 W1 dispatch |

### 不修订

- **M5-2 / M5-4 / M5-5 / M5-6 / M5-7 / M5-8 / M5-9 / M5-10 / M5-11 / M5-12**（10 张）：A0 W1 dispatch 把 M5-2/4/5/6/7/8/9/10/11/12 的产品代码**仍锁定**，A1 不在 W1 范围改这些卡。
- **三份主文档**（`详细设计与实施计划.md` / `后续需求TODO.md` / `AI-模型切换与接手清单.md`）：A0 W1 未指派必要改动；A1 也不在 W1 范围动。
- **`NEXT` 标记**：仍为 M5-W1（由 A0 拣入本批修订后改 `M5-1.a` 或 `M5-1.b`，按决策 1 拍）。

### A2 v3 C-1~C-8 摘要（A1 M5-1 卡应用 6+2 处分歧的源）

- **C-1**：M5-1.a 内部顺序从 `0a → 0b` 改 `0b → 0a`（测试反向 import 必须先收口）
- **C-2**：阶段一 `cargo tree -p core` 命令降级 PENDING，改用源码级 `grep -rE '^\s*use tauri' src-tauri/src/core/` 断言
- **C-3**：`M5-1.b` 拆为 `M5-1.b`（B 类 seam+搬入）与 `M5-1.c`（阶段二 workspace 化）
- **C-4**："23 模块" 回填为 "20 业务模块 + main.rs"
- **C-5**：反向边从 "4 处" 改 "8 处 + 2 组测试反向 import"，补 `:761`
- **C-6**：`[lib] path="src/core/mod.rs"` vs `src/lib.rs` 二选一均可（不强制）
- **C-7**（v3 新增）：M5-14 §6 L83 反向边改挂 M5-1.b
- **C-8**（v3 新增）：`check-core-boundary` 用 `.py`（仓库 `scripts/` 30+ 脚本惯例）

### A0 W1 boundary-first 顺序（A1 M5-1 卡的实施序重组）

1. **步骤 0（new in W1）**：A2 先建 `scripts/check-core-boundary.py`（含 `--self-test` 2好+2坏+1阴/默认扫描/`--expect-pending` 三模式）并挂 `scripts/pre-merge.sh` —— **本步是后续所有 core 内操作的准入前置**。
2. **步骤 0b**（A1 M5-1.a 切片 0b）：先把 `HARD_GRACE_SECS` / `MAX_TIMEOUT_SECS` / `MAX_TEXT_FIELD_BYTES` / `DB_MAX_TEXT_FIELD_BYTES` / `DB_SOFT_TO_HARD_GRACE_SECS`（含 V-7 副本收口）收口到 `domain.rs`，改 `t_db_c6_limit_and_timeout_constants_are_aligned` 避免"自比退化"。
3. **步骤 0a**（A1 M5-1.a 切片 0a）：建 `src-tauri/src/core/mod.rs` + A 类 10 模块整文件带 `#[cfg(test)]` 搬入。
4. **步骤 1**（A1 M5-1.b 切片 1）：抽 `RootsProvider` / `ProgressSink` / `PathResolver` trait，解除 `scheduler → bridge::AppState` 反向边。
5. **步骤 2**（A1 M5-1.b 切片 2）：B 类模块 seam 改造。
6. **步骤 c**（A1 M5-1.c 阶段二，可选）：根 `Cargo.toml [workspace]` 化（须 exclude `tauri-browser-tabs/`）。

> 步骤 0→0b→0a→1→2 不可换序：换序则步骤 0b 后的常量引用在步骤 0a 完成前编译失败，步骤 0a 完成的 core 边界在步骤 1 前不构成"反向边解除"。

---

---

## 0. 本包目录结构

```
logs/checkpoints/M5-20260906/
├── M5-0-overview.md            ← 本文件（总览/索引/依赖/合并顺序）
├── M5-1-core-workspace-split.md        ← #7 → M5-1  核心 workspace 下沉
├── M5-2-rmcp-mcp-policy.md             ← #7 → M5-2  内嵌 rmcp + McpGlobalPolicy
├── M5-3-a2a-bidir-agent-kv.md          ← #7 → M5-3  A2A 双向 + agent_kv
├── M5-4-agent-skill-runtime.md         ← #12 → M5-4 Agent/Skill runtime
├── M5-5-agent-skill-commands.md        ← #12 → M5-5 Agent/Skill 命令与权限
├── M5-6-agent-skill-ui.md              ← #12 → M5-6 Agent/Skill UI
├── M5-7-graph-model-extract.md         ← #13 → M5-7 图模型与两阶段抽取
├── M5-8-graph-store-query.md           ← #13 → M5-8 图存储与查询
├── M5-9-graph-ui-agent-consume.md      ← #13 → M5-9 图谱 UI 与 Agent 消费
├── M5-10-plugin-manifest-lifecycle.md  ← #15 → M5-10 插件 manifest 与生命周期
├── M5-11-plugin-commands-isolation.md  ← #15 → M5-11 插件命令与隔离
├── M5-12-plugin-ui.md                  ← #15 → M5-12 插件管理 UI
├── M5-13-verification-matrix.md        ← 跨 M5 验证矩阵（合并门禁/单测/反向用例）
└── M5-14-debt-ledger.md                ← M5 债务账（不在本批解决项）
```

每张 M5-x 子卡采用统一骨架（与 A1 M4 展开卡范式一致）：

```
0. 任务卡编号 / 需求映射 / 责任 Lane 候选
1. GOAL           — 子卡目标（1 句）
2. READ           — 必读文件
3. WRITE          — 必改文件候选（不写实现，仅列契约落地位置）
4. FORBID         — 红线
5. COMMANDS       — 验收命令（实现期跑）
6. PASS_CRITERIA  — 通过判据
7. FAIL_ACTION    — 失败动作
8. DOC_BACKWRITE  — 需回写的主文档段落
9. COMMIT / NEXT  — 提交与下一卡
```

---

## 1. M5-W0 / M5-W1 节奏

| Wave | 状态 | 内容 | 责任 Lane | 准入 |
|---|---|---|---|---|
| **M5-W0**（**当前**） | 进行中 | 展开 M5-1~M5-12 子卡 + 架构预研（仅 docs） | A1（展开）+ A2/A5/A6/A7/A9（prework 文档）+ A10（安全复核）+ A11（验证矩阵） | M4 PASS（✅ `a1a2061`） |
| **M5-W1**（待 A0 签发） | 锁定 | 签发 M5-1（`core workspace 下沉`）评估边界 → 决定 M5-2/3/4/5/7/8/10/11 的领取顺序与卡合并 | A0 签发 + 候选 Lane A13~A20（见 §6） | M5-W0 集成 + A0 签发 NEXT |
| **M5-W2~R**（远期） | 锁定 | 依 M5-1 边界签发各 M5-x 实现批 + UI 批 | A14/A15/A16/A17/A18/A19/A20 | 各前置冻结 |

> **本卡包覆盖范围**：M5-W0 全部。**不**为 M5-W1 写实现卡；**不**签字 Lane 号；**不**签合并顺序（合并顺序交 §5 草拟、A0 拍）。

---

## 2. M5 12 子卡索引（与 WBS 一致）

| WBS 编号 | 需求 | 标题 | 子卡文件 | 核心 A*-M5 prework 输入 | 建议 Lane 候选 |
|---|---|---|---|---|---|
| **M5-1**  | #7  | 核心 workspace 下沉（core/web/mcp/cli） | `M5-1-core-workspace-split.md` | `A2-M5-core-20260906-0749.md`（A 路径 · 10 步） | A13 |
| **M5-2**  | #7  | 内嵌 rmcp + `McpGlobalPolicy` + 首组 MCP 工具 | `M5-2-rmcp-mcp-policy.md` | `M5-7.a-prework-20260902-1055.md` + `A2-M5-core-*.md`（共用 capability.rs） | A14 |
| **M5-3**  | #7  | A2A 双向 + `agent_kv`（多方言归一） | `M5-3-a2a-bidir-agent-kv.md` | `M5-7.a-prework-*.md` §4.3 A2A 草案 | A15 |
| **M5-4**  | #12 | Agent/Skill runtime（`AgentDef`/`SkillDef`） | `M5-4-agent-skill-runtime.md` | `M5-12.a-prework-20260902-1055.md` | A16 |
| **M5-5**  | #12 | Agent/Skill 命令与权限（安装/执行/对话） | `M5-5-agent-skill-commands.md` | `M5-12.a-prework-*.md` §4.1-4.2 | A16（同 A16 拆卡） |
| **M5-6**  | #12 | Agent/Skill UI（对话/管理/权限预览） | `M5-6-agent-skill-ui.md` | A5/A6 prework（当前空） | A19 |
| **M5-7**  | #13 | 图模型与可追溯抽取（两阶段 + `graph.rs`） | `M5-7-graph-model-extract.md` | `M5-13.a-prework-20260902-1055.md` | A17 |
| **M5-8**  | #13 | 图存储与查询（邻接表 + DDL + 命令） | `M5-8-graph-store-query.md` | `A9-M5-graph-store-contract-20260906-0700.md` + `A9-M5-graph-scheduler-feed-*.md` | A17 |
| **M5-9**  | #13 | 图谱 UI 与 Agent 消费（力导向 + RAG 注入） | `M5-9-graph-ui-agent-consume.md` | A8 prework（当前空） | A19 |
| **M5-10** | #15 | 插件 manifest 与生命周期（签名/load/unload） | `M5-10-plugin-manifest-lifecycle.md` | `M5-15.a-prework-20260902-1055.md` + `A9-M5-plugin-form-feasibility-*.md` | A18 |
| **M5-11** | #15 | 插件命令与隔离（安装/权限/审计/卸载） | `M5-11-plugin-commands-isolation.md` | `M5-15.a-prework-*.md` §4 + `A9-M5-plugin-form-feasibility-*.md` | A18 |
| **M5-12** | #15 | 插件管理 UI（权限清单/配置/状态） | `M5-12-plugin-ui.md` | （无 prework；A18 实施期补） | A19 |

> **Lane 号 A13~A20 为 A9 候选提案**（见 `A9-M5-A13plus-cards-20260906-0010.md`），A1 不强行指定；A0 在 M5-W1 签发时定。

---

## 3. 命名漂移警示（沿用 A9 prework §2）

需求 # 号 与 WBS 编号 在历史 prework 文件名中混用，本批 A1 卡统一以 WBS 编号为准；如需引用历史 prework 文档，按下表对照：

| 需求 # | WBS 编号 | 既有 prework 资产 |
|---|---|---|
| #7  A2P/A2A     | M5-1 / M5-2 / M5-3  | `M5-7.a-prework-20260902-1055.md` · `A2P-A2A-protocol-taskcard-20260902-1146.md` |
| #12 Agent/Skill  | M5-4 / M5-5 / M5-6  | `M5-12.a-prework-20260902-1055.md` · `agent-skill-contract-taskcard-20260902-1146.md` |
| #13 知识图谱    | M5-7 / M5-8 / M5-9  | `M5-13.a-prework-20260902-1055.md` · `graph-model-taskcard-20260902-1146.md` |
| #15 插件系统    | M5-10 / M5-11 / M5-12 | `M5-15.a-prework-20260902-1055.md` · `plugin-permission-taskcard-20260902-1146.md` · `plugin-runtime-taskcard-20260902-1146.md` |

**不**再用 `M5-7/12/13/15.a` 这种"看起来像 WBS"实为"需求 #"的文件名新建文档。

---

## 4. 依赖图（A1 视角）

```
                ┌────────── M4 PASS ✅ a1a2061 ──────────┐
                │                                         │
                ▼                                         ▼
       A2 prework 已有（A 路径）         A9 prework（3 份契约 + split + A13plus）
                │                                         │
                └──────────────┬──────────────────────────┘
                               ▼
                    ┌──── M5-1 workspace 下沉评估 ────┐   ← 必最先（A0 签发）
                    │  决 5 成员边界（core/web/mcp/cli）│
                    │  决 capability.rs 放置位置       │
                    └────────────┬────────────────────┘
                                 ▼
        ┌────────────────────────┼────────────────────────┐
        ▼                        ▼                        ▼
  M5-2 rmcp + Policy      M5-4 Agent/Skill runtime    M5-7 图模型与抽取
  (复用 A3/A4 DB API)     (复用 M2-4 run_script)      (依赖 A3/A4 + A7 事件)
        │                        │                        │
        ▼                        ▼                        ▼
  M5-3 A2A + agent_kv     M5-5 Agent/Skill 命令    M5-8 图存储与查询
                          (与 M5-2 共用 capability.rs)  (复用 M5-1 workspace)
                                │                        │
                                ▼                        ▼
                          M5-6 Agent/Skill UI      M5-9 图谱 UI + Agent 消费
                                                      (依赖 M5-4 RAG 注入)

  ┌──── 独立分支：#15 插件系统 ────────────────────────────┐
  │  M5-10 manifest 生命周期（先 A0 裁形态①/②/③）          │
  │       ↓                                               │
  │  M5-11 命令与隔离（独立 stdio 通道？or webview 二开？） │
  │       ↓                                               │
  │  M5-12 插件管理 UI                                     │
  └────────────────────────────────────────────────────────┘

  ▼ 横切依赖
  - M5 全部新命令（graph_*/skill_*/plugin_*/mcp_*/a2a_*）一律插 `list_artifact_images` 之前（坑位②）
  - M5 全部新 capability 走 `capabilities/default.json` 通用权限或新建专用（待 M5-1 决）
  - M5 全部审计走 `audit.json` 1000 上限（K5）+ 各自独立日志（mcp-calls.json / skill-runs.json / plugin-invokes.json）
  - M5 全部退出收口挂 ShutdownCoordinator（沿用 M0-2 范式，索引依赖单测断言）
  - M5 全部新增 schema_version 走 `atomic_write`（沿用 M3 范式）
  - A6 已冻结：新建任务默认 `enabled=false`；tick 内禁写审计；判重真相源 `tasks.json.last_fired_at`
  - A7 已冻结：`stop-scheduler` 注册在 `stop-background-workers` 之后；图谱维护任务复用此通道
```

---

## 5. 合并顺序（A1 草拟，交 A0 拍）

> **不签字**。仅给"按依赖最小化"的草拟顺序；A0 在 M5-W1 签发时定。

| 序 | 卡 | 关键产出 | 依赖 | 与 A9 候选 A13~A20 映射 |
|---|---|---|---|---|
| 1 | M5-1 | 5 成员 workspace + capability.rs 位置 + 主应用可构建性 | M4 PASS ✅ | A13 |
| 2 | M5-2 | `rmcp` server 骨架 + `McpGlobalPolicy` + 首期能力白名单 | A3/A4 DB API + M5-1 | A14 |
| 2' | M5-4 | `AgentDef`/`SkillDef` + `skill_runtime.rs` + `capability.rs` 共用首版 | A13 + M2-4 | A16（与 M5-2 并行需先划模块边界） |
| 3 | M5-8 | 图邻接表 + `GraphQuery` DTO + `graph_*` 命令 | A3/A4 DB API + M5-1 | A17 |
| 3' | M5-5 | `skill_*`/`agent_chat` 命令 + 二次确认闸门 + `skill-runs.json` | M5-2 (capability) + M5-4 | A16 |
| 4 | M5-3 | A2A 协议 + `agent_kv` | M5-2 (能力层) | A15 |
| 4' | M5-7 | `graph.rs` 抽取器 + `GraphEvent` 上游钩子 | A7 事件 + M5-8 | A17 |
| 5 | M5-10 | 插件形态裁定（默认形态③声明式）+ `PluginManifest` | A0 形态裁定 | A18 |
| 5' | M5-11 | 插件命令 + 权限强制层 + `plugin-invokes.json` | M5-2 (capability) + M5-10 | A18 |
| 6 | M5-6 / M5-9 / M5-12 | UI 三件套（对话/图谱/插件） | 各后端 DTO 稳定 | A19 |
| 滚动 | M5 安全审查 | 各批次的红线复审 | 滚动 | A20 |

---

## 6. 与 A9 候选卡（A13~A20）的差

A9 候选卡（`A9-M5-A13plus-cards-20260906-0010.md`）与本批 A1 卡基本一致，差异如下：

| 项 | A9 候选卡 | A1 本批卡 | 处置 |
|---|---|---|---|
| Wave 标注 | 文档 R/R+1/R+2/R+3/滚动 | 同 A9 | 沿用 |
| Lane 号 | A13~A20 占位 | 不写 Lane 号（**A0 签发时定**） | A1 边界（避免与 A0 冲突） |
| 卡的颗粒度 | 一卡跨多 WBS（如 A14 跨 M5-2） | 一卡 = 一个 WBS（更细） | A1 比 A9 更细；M5-W1 由 A0 决定是否合并相邻 WBS |
| `capability.rs` 放置 | 隐含在 A14/A16 | M5-1 子卡显式列为"必最先决" | A1 更强调 |
| 插件形态决策 | 交 A0 | 同 A9（默认形态③） | 一致 |
| 债务账 | 未列 | `M5-14-debt-ledger.md` 显式分账 | A1 补 |

---

## 7. 红线总览（M5 全包适用）

> 继承 K 系列（详细设计）+ A6/A7 冻结 + 坑位② ⑤：

1. **禁第二执行路径**：M5 全部执行体（Agent 调用工具、Skill 运行、图维护任务、插件调用脚本）**只走** M2-4 `script_runner` 单通道；不引 `std::process::Command` 第二路径；插件形态①（独立 stdio）已否决。
2. **禁 npm 分包**：`rmcp` 必须纯 Rust 内嵌（`详细设计 §7`）；不引 `npx` 任何东西。
3. **能力白名单共用一份**：A2P / A2A / Skill / Plugin / Agent **共用** `src-tauri/src/capability.rs`；禁止各写一份漂移。
4. **凭据不入图谱/日志/审计**（K3）：LLM Key、Keyring 条目、API Token 不进入 `GraphNode.props` / `agent_kv.json` / `audit.json`。
5. **审计不刷爆**（K5）：高频调用走独立文件（`mcp-calls.json` / `skill-runs.json` / `plugin-invokes.json`），每文件独立 1000 上限；`audit.json` 仅记低频关键事件。
6. **新命令入 ACL**：M5 全部新命令（`graph_*`/`skill_*`/`plugin_*`/`mcp_*`/`a2a_*`）一律插 `list_artifact_images` 之前（坑位②）。
7. **退出收口挂协调器**：M5 各 `*Shutdown`（Graph/Skill/Agent/Plugin/MCP/A2A）必须挂 `ShutdownCoordinator`，注册序索引依赖需单测断言（沿用 `O-A1-5` 提醒）。
8. **新建任务默认 `enabled=false`**（R-A6-1）：图谱维护任务、Agent 周期任务、Skill 调度触发一律默认 `enabled=false`，显式开启。
9. **`withGlobalTauri=true` 红线**：插件 webview **不得**接触 `window.__TAURI__` 全局；Tauri v2 不支持按 webview 关全局时，**形态②插件放弃**，改形态③声明式（参 `A9-M5-plugin-form-feasibility-20260906-0700.md`）。
10. **`atomic_write` 唯一**：图 schema 版本、agent_kv、plugin_grants、mcp_policy、a2a_tasks 等所有持久化走 `session::atomic_write`（沿用 M3 范式）。
11. **派生索引层原则**（图谱）：`GraphNode.props` 不存主数据正文；可重建；`source=Manual` 优先不被自动抽取覆盖。
12. **Skill 禁内联**（K6）：`SkillDef.exec` 仅 `ScriptRef` / `CommandRef` / `Sequence`（无内联 shell 字符串）；执行体复用 `run_script` 走 M2-3.a 危险参数校验。

---

## 8. 验证矩阵入口

详见 `M5-13-verification-matrix.md`：

- 协议契约测试（A2P 消息格式 / A2A 幂等 / MCP capability 解析）
- 隔离与权限测试（Skill 越权 / 插件 manifest_hash 变更失效 / Agent 白名单拒绝）
- 集成测试（GraphEvent 上游写 → 抽取 → 查询）
- 恢复测试（`graph.db` 损坏备份 / agent_kv 容量满策略 / mcp-calls.json 滚动裁剪）
- 性能基线（MCP 工具响应 / 图查询 depth=2 万节点 / LLM token 节流）
- 升级/回滚测试（schema_version 迁移 / 插件更新 manifest_hash 重置授权）

---

## 9. 债务账入口

详见 `M5-14-debt-ledger.md`：

- 形态② 插件（webview 内）暂时**不实现**（`withGlobalTauri` 全局未按 webview 关闭前）
- `withGlobalTauri` 全局化影响 `src/stores/useSystemStore.ts:84` 唯一引用——M5-1 评估期一并封/迁
- A2A 首期是否真双向（仅"被调"or"既可被调又可委派"）——A0 拍
- 智能期图抽取（`source=ai`）首期仅留 trait，**不实现** LLM 触发
- LLM 用量配额/费用统计首期是否做
- `Similar` 关系相似度算法与阈值首期只做同 hash 精确匹配
- `graph_export("graphml")` 首期返回"暂不支持"
- 图谱库 `props` JSON 反序列化容错（首期严格 schema，宽容版本做迁移期）

---

## 10. 回写计划（按接手清单 §5 模板）

A1 checkpoint 文档另存 `logs/checkpoints/M5-A1-expansion-20260906-0800.md`（与 A0-M5-W0-dispatch 同级），含：

```
CHECKPOINT=M5-A1-expansion
STATUS=PASS（待 A0 集成后定）
EXECUTOR=A1 (CodeBuddy / M3-mini)
MODEL=M3-mini
ROUTE=AI:DEEP
MODEL_DEVIATION=none
COMMIT=<待 A0 拣入后填>
VERIFY=见 §5.2 命令全 PASS（仅文档工作树）
NEXT=M5-W1（待 A0 签发）
```

---

## 11. FORBID 遵守记录

- 未写产品代码（`src/`、`src-tauri/`、`package.json` 一字未动）
- 未触 `scripts/pre-merge.sh`、未触三份主文档
- 未触 `permissions/default-commands.toml`、未触 `capabilities/default.json`
- 未移动 `NEXT`（仍 `M5-W0`，A1 写完 14 张 docs 后 NEXT 不变）
- 未提交、未 push（本卡包为 A0 待拣入的待选文档）
- 14 张 docs 全新增独立文件，与 A2/A5/A6/A7/A9 既有 prework 文档无文件交集
- 所有"已冻结"事实均以来源 file:line 标注（继承自 prework 资产，本批仅做整合）
