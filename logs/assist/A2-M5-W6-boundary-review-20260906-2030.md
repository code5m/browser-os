# A2 · M5-W6 边界审查笔记：A8 Graph UI / A9 Plugin Manifest 的 core boundary 与 seam 方向（仅文档，无产品代码）

> 生成：2026-09-06 20:30 CST · Lane A2（M5-W6 · SUPPORT/REVIEW ONLY）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W6 Parallel Dispatch（L136-172，base `77b1e3e` docs(M5): dispatch W6 graph UI and plugin lanes）→ A2：**SUPPORT/REVIEW ONLY**；任务="Review A8/A9 for core boundary and seam direction; no product code."；交付=`logs/assist/A2-M5-W6-*.md` 边界笔记。
> 配套：A1 `M5-9-graph-ui-agent-consume.md` / `M5-10-plugin-manifest-lifecycle.md` / `M5-11-plugin-commands-isolation.md`；A7 W5 `domain.rs` L2042-2116 + `graph.rs`；`core/seam.rs`。

```text
LANE=A2
STATUS=REVIEW_DONE（docs only，无产品代码；未 push）
BASE=77b1e3e（W6 dispatch）
HEAD=logs/assist/A2-M5-W6-boundary-review-20260906-2030.md
FILES=logs/assist/A2-M5-W6-boundary-review-20260906-2030.md（仅本文档）
VERIFY=见 §Verify
CHECKPOINT=本文件
MERGE_NOTES=见 §F（6 项关键发现 + 红线清单 + gate 建议）
NEXT=见 §NEXT
```

## 0. 审查性质声明（重要）

A8/A9 的 W6 **产品代码尚未进入 canonical 树**（HEAD `77b1e3e` 仅是 W6 dispatch 文档提交；只有 A0 可 push 集成，A8/A9 在各自 worktree 产出后由 A0 拣入）。故本笔记是**实施前（pre-implementation）边界审查**，基于：
- W6 dispatch 任务定义与硬约束（board §M5-W6，L136-172）
- A1 权威卡 M5-9 / M5-10 / M5-11
- A8/A9 既有 W5 预研笔记（已读）
- A7 已集成的真实 DTO（`domain.rs` L2042-2116 实测）+ `graph.rs`
- `core/seam.rs`（ProgressSink/PathResolver/RootsProvider）+ `check-core-boundary.py`

目的：为 A8/A9 划清 **core boundary 与 seam 方向红线**，并显式标记 **W6 dispatch 对 M5 权威卡的收窄冲突**（否则 A8/A9 会过度 scope 并触发 W6 Hard Stop）。当 A8/A9 代码经 A0 集成后，A0/A10 可凭本笔记 §6 红线清单做二次复核。

> **交叉核对（已做）**：A1 已在 M5-9 / M5-10 卡顶部新增 `[W6 next-card acceptance criteria]` 段（4 AC + 5 HS）。其 W6 收窄方向与本文 §F1/§F2 一致，并**已显式解决**原 §3 WRITE 的范围冲突（§F1/§F2 改为"已由 A1 W6 AC 解决"）。但 A1 W6 AC-1 要求 `summarizeNode` 返回 `source/created_at/...` 等字段，与 A7 已交付 DTO（§F3）**自相矛盾**，仍需 A0/A1 回解。本文 §F1–§F4 已据此校准。

## 1. W6 角色与硬约束（来自 board §M5-W6）

- **只有 A8 与 A9 可在 W6 写产品代码**；A1-A7、A10、A11 均为 docs/review/support。A2 仅审查。
- A8 Hard Stops：不得加后端命令、不得 live agent 消费、不得 model 调用、不得 graph rebuild worker；新命令须同源检查+ACL+前端 bridge/types+策略+测试同包，**W6 偏好无命令**。
- A9 Hard Stops：不得 install/delete/download/execute/enable 真实插件；**仅纯 manifest/lifecycle 策略**；签名仅纯校验（除非完全本地）。
- 全局：所有 store/map/list 必须 **bounded**；不得 log/persist token/cookie/Authorization/body/prompt-secret；各 lane 先 `git pull` 自 `origin/master` 且**不得 push**。

## 2. 实测证据锚点（本树）

| 项 | 结果 | 对审查的含义 |
|---|---|---|
| core 边界 gate | 默认 `all invariants hold（ACTIVE=7，core 文件=3）`；`--self-test` `PASS(ACTIVE=7, 坏样本=9)` | 现状干净；A8/A9 产物若误入 `core/` 会被此 gate 拦（见 §R-C） |
| `graph_*` Tauri 命令 | `bridge.rs`/`bridge.ts`/`src/types.ts` grep 均 **0 命中** | A7 W5 未暴露命令 → **A8 W6 不得调用 live `graph_query`**；TS 类型需 A8 自建并镜像 DTO |
| `plugin_*` 命令 | `bridge.rs` grep **0 命中**；ACL 末条 `list_artifact_images` 在 L118 未变 | A9 W6 **不得新增 `plugin_*` 命令/ACL**（W6 硬约束），M5-10 卡 §3 的 10 条命令属后续波次 |
| A7 图 DTO | `domain.rs` L2042-2116：`GraphNode{id,kind,label,props}` / `GraphEdge{from,to,kind,weight,props}`；`GraphNodeKind∈{File,Dir,Tab,Script,Skill,Agent,Tag,Topic}`（**无 Plugin**）；常量 `GRAPH_MAX_DEPTH=4/QUERY_LIMIT=1000/NODES=5000/EDGES=20000/LABEL_MAX_BYTES=256/PROPS_MAX_BYTES=64KiB/NODE_ID_HEX_LEN=64` | A8 的 TS 类型必须字段级镜像此 DTO（单一真源） |
| `capability.rs` | **不存在**；能力真源碎片化：`domain.rs:1540 MCP_CAPABILITY_V1` + `security_policy.rs:2067/2070 SKILL/AGENT_CAPABILITY_V1 = &[]`（空） | A9 **不得新建 `capability.rs`**；`PLUGIN_CAPABILITY_V1` 须落既有碎片源之一；W6 仅做结构校验（见 §F4） |
| `scripts/check-graph-policy.py` | 存在（`scripts/`，非 `src-tauri/scripts/`） | A8/A9 策略脚本先例与同族（A5 `check-agent-skill-policy.py` / A6 `check-agent-skill-ui-logic.mjs`） |

## 3. A8（M5-9 Graph UI）core boundary & seam 方向

**边界定位**：前端（`src/components/**`、`src/stores/**`、`src/types.ts`、`src/bridge.ts`）。前端不属 Rust `core/`，故 core 边界 gate 不直接适用；但"core boundary 精神"在此转化为：**A8 不得把后端职责前移**——即不得为图查询新增 Tauri 命令、不得 live 调 `graph_query`、不得实跑 agent 消费。

**seam 方向（前端层）**：
- 类型单一真源 = `domain.rs` 的 `GraphNode`/`GraphEdge`。A8 在 `src/types.ts` 新增的 TS 镜像必须**逐字段对齐**（id/kind/label/props / from/to/kind/weight/props），`GraphNodeKind`/`GraphEdgeKind` 用字面量联合类型镜像（snake_case）。**禁止 A8 自创字段或重命名字段**。
- `src/bridge.ts` 若新增 `graph_*` 包装，W6 内必须是 **inert stub**（无 live `invoke`），且签名对齐 DTO，待 A7 后续暴露命令后由 A0 统一接线。

**红线（A8 必须遵守）**：
1. 无 `graph_*` 命令（不在 bridge.rs/main.rs/ACL 出现）。
2. 无 live agent 消费（**不建 `useGraphRag.ts`**；RAG 注入延后）。
3. **无新 npm 依赖**：W6 禁用 D3.js 等；力导向布局延后，仅做 list/search/filter + 节点详情摘要 + 容量/错误/空态 + helper 模块 + headless 逻辑测试（薄面板壳）。
4. `useGraphStore.ts` **bounded**：缓存节点/边上限对齐 `GRAPH_QUERY_LIMIT=1000`（可见≤500）；不得无界增长。
5. `NodeDetail`/`EdgeDetail` **不渲染 `props` 正文**（K7）；不 log/persist 图内容/secret。
6. `npm run build` PASS、`scripts/check-graph-ui-logic.mjs` PASS（A8 须新增该脚本，镜像 A6 `check-agent-skill-ui-logic.mjs`）。

## 4. A9（M5-10/11 Plugin Manifest/Lifecycle）core boundary & seam 方向

**边界定位**：后端（`src-tauri/src/domain.rs` 加类型、可选 `plugin.rs`/`plugin_manifest.rs`/`plugin_lifecycle.rs`、`security_policy.rs`、`scripts/check-plugin-policy.py`）——均在 **bin-side**（`src-tauri/src/`），不属 `core/`。

**seam 方向（后端层）**：
- W6 纯策略 = 内存中 manifest 结构校验 + 生命周期状态机（无 FS 读写、无签名加密执行）。**本波不需要 seam**（不碰路径/roots/keyring）。
- **设计建议（为后续波次预留依赖反转）**：`validate_plugin_manifest(&PluginManifest)`、`PluginLifecycle::transition(...)` 等纯函数应**只吃 `&PluginManifest`/内存状态**，不内联 `app.path()`/`std::fs`/keyring。待 M5-11/12 接入 `plugins_dir()` 扫描与签名校验时，再注入 `RootsProvider`/`PathResolver`/`KeyringStore`（与 A7 `graph_store` 复用 `database.rs` 单连接 + `PathResolver` 的同款模式），避免二次重构。
- 保持 `plugin*.rs` **Tauri 自由**（无 `use tauri`/`AppHandle`/`crate::bridge`），既符合 W6 硬约束，也使其未来可候选进 `core/`（与 A7 图模型同列）。

**红线（A9 必须遵守）**：
1. **无 `plugin_*` 命令**、无 ACL 改动（W6 硬约束；M5-10 卡 §3 的 10 条命令属后续波次）。
2. **无 Ed25519 加密执行**：W6 仅纯结构校验（manifest schema / `min_app_version` 可解析 / `hash` 字段形态 / `capabilities` 引用形态），签名验证延后。
3. **无 FS 变更**：不写 `plugins_dir()`、不写 `trusted-pubkeys.json`、不 install/uninstall/enable/disable 真实插件。
4. 能力引用校验 **W6 仅做结构校验**（capability 非空、reason 存在），**不硬编码成员枚举**（见 §F4）。
5. manifest 元数据 **bounded**：display_name/description 等字符串设字节上限；manifest 内不得含 token/password/secret 字段。
6. `scripts/check-plugin-policy.py` 须含 self-test/default/pending + pre-merge 挂载（镜像 `check-agent-skill-policy.py`/`check-graph-policy.py`），且 **FAIL 若 bridge.rs 出现 `plugin_*` 命令**（兜底 W6 红线 1）。

## 5. 发现（Findings）

### §F1 — A8 W6 范围：M5-9 权威卡 §3 WRITE 与 W6 收窄的冲突（**已由 A1 W6 AC 解决**）
M5-9 卡 §3 WRITE 列了 `GraphViewer.vue`(D3.js 力导向)、`useGraphRag.ts`(RAG 注入)、`src/router/graph.ts`、i18n 8 key、`graph.scss`；但 W6 dispatch 把 A8 收窄为"**pure logic and panel shell; no live agent consumption; no new dependency**"。
**现状（已核对）**：A1 在 M5-9 卡顶部已加 `[W6 next-card acceptance criteria]` 段（4 AC + 5 HS），显式收窄：AC-1 要求 `src/utils/graphUi.ts` 纯函数 helper（filter/search/summarize/empty/error/truncate）；W6-HS1 不加命令、W6-HS2 不调 live agent（RAG 冻结到 W7+）、W6-HS3 不调 model/rebuild worker、W6-HS4 **无新 npm 依赖（d3 延后到 W7+）**、W6-HS5 K7 严守。→ **本冲突已由 A1 W6 AC 解决**；A8 须以卡顶部 `[W6 next-card acceptance criteria]` 段为准，**不要**按原始 §3 WRITE 全量实现（否则触发 W6 Hard Stop）。A2 原"范围冲突"裁定方向与 A1 一致，此处留作归档。

### §F2 — A9 W6 范围：M5-10 权威卡 §3 WRITE 与 W6 收窄的冲突（**已由 A1 W6 AC 解决**）
M5-10 卡 §3 WRITE 列了 10 条 Tauri 命令（改 bridge.rs/ACL）、`plugin_signature.rs`(Ed25519)、`plugins/` 目录写、`keys/trusted-pubkeys.json` 写、`capability.rs` 扩展。W6 dispatch 显式收窄为"**pure manifest/lifecycle policy slice; no install/delete/download/execute/enable; no signature enforcement**"。
**现状（已核对）**：A1 在 M5-10 卡顶部已加 `[W6 next-card acceptance criteria]` 段（4 AC + 5 HS）：AC-1 `PluginManifest` DTOs+`validate_plugin_manifest` 纯函数；AC-2 `PluginState`/`PluginLifecycleEvent`/`transition` 状态机；AC-3 `PermissionManifestRule(&CapabilityRegistry)`；AC-4 `check-plugin-policy.py`(6 ACTIVE 码)+pre-merge；W6-HS1 不 install/delete/execute、W6-HS2 不网络/不签名强制、W6-HS3 **不注册 10 条 plugin_* 命令**、W6-HS4 不破 capability 单点、W6-HS5 不写 secret。**本冲突已由 A1 W6 AC 解决**；A9 须以卡顶部 `[W6 next-card acceptance criteria]` 段为准，**不要**按原始 §3 WRITE 加命令/ACL/签名/FS 写。A2 原"范围冲突"裁定方向与 A1 一致。

### §F3 — GraphNode/GraphEdge DTO 缺 source/时间戳，但 A1 W6 AC-1 仍要求 summarizeNode 返回它们（**A1 W6 AC 与 A7 已交付 DTO 自相矛盾**）
A7 真实 DTO（`domain.rs` L2082-2100）**只有** `id/kind/label/props`，**无 `source`/`source_ref`/`created_at`/`updated_at`/`extractor_version`**（A9 W5 §3.1 已记录此漂移）。
**矛盾点（新发现）**：A1 W6 在 M5-9 卡顶部 `[W6 next-card acceptance criteria]` 的 **AC-1** 却要求 `summarizeNode(node) -> NodeSummary` 返回 **`source / source_ref / created_at / updated_at / extractor_version` + 邻居计数**（白名单 8 字段见 W6-HS5）。即 **A1 自己的 W6 AC 与 A7 已 ship 的 DTO 不一致**——A8 若严格按 AC-1 构造 `NodeSummary` 会引用不存在的字段。
→ **裁定**：
- A8 的 `src/types.ts` TS 镜像 + `summarizeNode` 必须以 **A7 实际 DTO**（`id/kind/label/props`）为唯一真源；`source`/时间戳在 A7 未补字段前**不得出现在 `NodeSummary`**（或仅以 `Option`/空占位返回，且 UI 不依赖）。
- 这是 A0/A1 需回解的**卡片缺陷**：要么 (a) A7（A17）在后续波次给 `GraphNode` 加 `source`/`extractor_version`/时间戳字段（遵循 M5-7 §4.3/4.5 可追溯 GOAL），要么 (b) A1 修正 W6 AC-1 的 `NodeSummary` 白名单为实际 DTO 字段。在 A0 裁决前，A8 按"实际 DTO"实现、不前端编造字段。
- `props` 正文仍严守 K7 不渲染（A1 W6-HS5 同源）。
- A9 W5 §3.1 已预判此漂移，本裁定为其落地确认。

### §F4 — capability.rs 不存在 + 能力真源碎片化；PLUGIN_CAPABILITY_V1 必须落 `security_policy.rs`（**不得新建 capability.rs**）
实测：`src-tauri/src/capability.rs` **不存在**；能力真源碎片化：`domain.rs:1540 MCP_CAPABILITY_V1` 与 `security_policy.rs:2067/2070 SKILL_CAPABILITY_V1=AGENT_CAPABILITY_V1 = &[]`（当前空）。A9 W5 §4 已标记此碎片化 open。
**A1 W6 AC 校准**：A1 W6 AC-3 要求 `PermissionManifestRule(Vec<PluginCapability>, &CapabilityRegistry) -> PermissionVerdict`，且拒绝原因含 `CapabilityNotInWhitelist`——即 **W6 必须做成员枚举校验**（非仅结构校验）；W6-HS4 要求"PLUGIN_CAPABILITY_V* 必须走同一文件，禁止在 plugin.rs 内嵌 capability 字面量"。但卡/A1 文中"capability.rs"作为文件名**不存在**——真实单点文件是 `security_policy.rs`（SKILL/AGENT 之家）。
→ **裁定**：
- A9 **不得新建 `capability.rs`**（M5-10 卡 §3 与 A1 W6 AC 中"capability.rs"措辞均基于不存在的文件，属 stale 引用）。
- A9 须在 **`security_policy.rs`** 现有 `SKILL_CAPABILITY_V1`/`AGENT_CAPABILITY_V1` 旁**追加 `PLUGIN_CAPABILITY_V1`**（可初始为 `&[]` 或首期空，待 A0 W4 §4 收口填充），并定义 `CapabilityRegistry`（聚合同文件三_slice）供 `PermissionManifestRule` 取 `&CapabilityRegistry` 参数做成员枚举校验。
- **W6 做成员枚举校验**（capability 必须在 `PLUGIN_CAPABILITY_V1` 内 + reason 非空 + 同 plugin 去重 + 上限 `MAX_PLUGIN_CAPABILITIES=5`），不止结构校验。
- 跨 MCP/Skill/Agent/Plugin 的统一 capability 真源（第四个 `*_CAPABILITY_V1`）收口仍归 A0 在 W4 §4 裁决，A9 仅就地把 Plugin 加进既有 `security_policy.rs`，不另开文件、不内嵌字面量。

### §F5 — 无 graph_*/plugin_* 命令；A8/A9 不得新增或调用
实测 `bridge.rs`/`bridge.ts`/`src/types.ts` 三者对 `graph_`/`plugin_` 均 0 命中，ACL 末条仍是 `list_artifact_images`(L118)。
→ **裁定**：A8 W6 不调用 live `graph_query`（命令不存在）；A9 W6 不新增 `plugin_*` 命令/ACL。两者均不得在 W6 引入任何新 Tauri 命令（W6 硬约束"prefer no command"，且命令须同包原子落地，超出 W6 单 lane 范围）。

### §F6 — Plugin 不在 GraphNodeKind；图 UI 暂不含插件节点
A7 DTO `GraphNodeKind` 无 `Plugin`（A9 W5 §3.1 已确认）。
→ **裁定**：A8 W6 的图 UI 无需（也无法）渲染插件节点；若产品要求"插件→提供 Skill/Agent"进力导图，须由 A7（A17）在 `GraphNodeKind` 加 `Plugin`（及 `Provides` 边）后，A8 后续波次再消费。W6 不阻塞、不预置。

## 6. 红线复核清单（供 A0/A10 在 A8/A9 代码集成后二次复核）

**A8（前端）**
- [ ] 无 `graph_*` 命令（grep bridge.rs/main.rs/ACL = 0）
- [ ] `src/types.ts` 的 `GraphNode`/`GraphEdge` 字段与 `domain.rs` L2082-2100 逐字段一致（无自造字段）
- [ ] `useGraphStore.ts` 有 bounded 上限（≤ `GRAPH_QUERY_LIMIT`=1000，可见≤500）
- [ ] `NodeDetail`/`EdgeDetail` 不渲染 `props` 正文（K7）
- [ ] 无新 `package.json` 依赖（无 D3.js）；`npm run build` PASS
- [ ] `scripts/check-graph-ui-logic.mjs` 存在且 PASS
- [ ] 无 live agent 消费（`useGraphRag` 不存在或被禁用）

**A9（后端策略）**
- [ ] 无 `plugin_*` 命令（grep bridge.rs = 0）；ACL 末条仍 `list_artifact_images`
- [ ] `plugin*.rs` 无 `use tauri`/`AppHandle`/`crate::bridge`；无 `std::fs` 写 `plugins_dir`/`trusted-pubkeys.json`
- [ ] 无 Ed25519/crypto 执行（W6 仅 schema 校验；签名强制留 W7+）
- [ ] `PLUGIN_CAPABILITY_V1` **加进 `security_policy.rs` 既有 `SKILL`/`AGENT` 旁**（**不得新建 `capability.rs`**）；`PermissionManifestRule` 经 `&CapabilityRegistry` 做**成员枚举校验**（capability 须在白名单内 + reason 非空 + 同 plugin 去重 + 上限 `MAX_PLUGIN_CAPABILITIES=5`）
- [ ] `scripts/check-plugin-policy.py` self-test/default/pending PASS + pre-merge 挂载；且 FAIL 若 bridge.rs 含 `plugin_*` 命令
- [ ] `cargo test` 相关 target PASS；warning 不超基线

## 7. 策略脚本与 gate 建议

- **A8** 必须新增 `scripts/check-graph-ui-logic.mjs`（headless 逻辑测试，镜像 A6 `check-agent-skill-ui-logic.mjs`），断言：TS 类型字段与 DTO 一致、store bounded、K7 不渲染 props、无 `graph_` 命令调用。
- **A9** 必须新增 `scripts/check-plugin-policy.py`（镜像 `check-agent-skill-policy.py`/`check-graph-policy.py`）：ACTIVE 码建议含 `PLUGIN_NO_COMMAND`(bridge.rs 无 plugin_*)、`PLUGIN_MANIFEST_SCHEMA`(结构字段)、`PLUGIN_CAP_STRUCTURAL`(capability 非空+reason)、`PLUGIN_NO_SECOND_PATH`、`PLUGIN_NO_SIGNATURE_ENFORCE`(W6 不加密)、`PLUGIN_BOUNDED_META`；并接入 `scripts/pre-merge.sh`。
- **core 边界 gate 加固（沿用 A2 W5 建议，仍有效）**：`check-core-boundary.py` 的 `BIN_ONLY_MODULES` 当前仅 6 个，未覆盖 `database`/`domain`/`plugin*` 等全部 bin 模块。建议把 `main.rs` 声明的全部 bin 模块纳入，使 R-B2/R-B3 在 phase-1 被完整机器守门（若 A9 误把 `plugin*.rs` 放进 `core/`，gate 方能拦住）。属产品代码改动，由 A0 后续波次落地，W6 A2 不实现。

## 8. seam 方向结论（前瞻）

- **W6 不需要改动 `core/seam.rs`**：A8（纯前端逻辑+stub 后端）与 A9（纯内存策略）均不触碰 路径/roots/keyring/进度 的运行时注入点。
- **但应预留 seam 注入面**：A9 的纯校验/状态机函数应只吃 `&PluginManifest`/内存状态，未来接 `plugins_dir()` 扫描与签名校验时注入 `RootsProvider`/`PathResolver`/`KeyringStore`（同 A7 `graph_store` 模式），而非内联 `app.path()`。A8 前端的"类型单一真源"即 `domain.rs` DTO——这是前端层的 seam 等价物。
- 若 A0 后续要把 `plugin*.rs` 纯策略搬入 `core/`（与 A7 图模型同列），届时 seam 注入即生效；当前 W6 留 bin-side 即可。

## 9. Verify（本笔记证据）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
# 1) core 边界 gate（现状基线）
python3 scripts/check-core-boundary.py            # all invariants hold（ACTIVE=7，core 文件=3）
python3 scripts/check-core-boundary.py --self-test  # CORE_POLICY_SELF_TEST=PASS（ACTIVE=7，坏样本=9）
# 2) 无 graph_*/plugin_* 命令（W6 红线）
grep -rnE "graph_|plugin_" src-tauri/src/bridge.rs   # 0 命中
grep -c "GraphNode\|GraphEdge" src/types.ts          # 0（TS 类型待 A8 建）
# 3) ACL 末条未变
grep -n "list_artifact_images" src-tauri/permissions/default-commands.toml | tail -1  # 118
# 4) A7 图 DTO 真源
grep -nE "pub struct GraphNode|pub enum GraphNodeKind|GRAPH_QUERY_LIMIT|GRAPH_MAX_NODES" src-tauri/src/domain.rs  # 2082/2042/2112/2114
# 5) capability 真源碎片化（无 capability.rs）
ls src-tauri/src/capability.rs                      # No such file
grep -nE "CAPABILITY_V1" src-tauri/src/security_policy.rs  # 2067 SKILL / 2070 AGENT (均为 &[])
grep -nE "MCP_CAPABILITY_V1" src-tauri/src/domain.rs        # 1540
# 6) A8/A9 W6 产品码尚未进 canonical 树（本笔记为 pre-implementation 审查）
git status --short --branch                         # ## master...origin/master（干净）
```

## 10. NEXT

- 交 A0：① 采纳本笔记对 **W6 dispatch 收窄 M5-9/M5-10 卡范围冲突**的裁定（§F1/§F2），并在 dispatch 中明示"A8/A9 W6 仅策略/pure-logic，全量功能延后 A18/A19 后续波次"；② 排期 `check-core-boundary.py` 的 `BIN_ONLY_MODULES` 加固（§7）；③ 推动 W4 §4 能力真源收口（决定 `PLUGIN_CAPABILITY_V1` 落点，§F4）。
- A8（W6 产品码）：按 §3 红线交付薄面板壳 + headless 逻辑测试 + `check-graph-ui-logic.mjs`；TS 类型严格镜像 `domain.rs`；不碰命令/RAG/D3.js。
- A9（W6 产品码）：按 §4 红线交付 DTO + 纯校验 + 生命周期状态机 + `check-plugin-policy.py`；不碰命令/ACL/签名/FS。
- A0/A10 在 A8/A9 代码集成后，凭 §6 红线清单做二次复核。
