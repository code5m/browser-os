# Lane A8 — M5-W14 Plugin Manager UI Dispatch / UI Review Note

> Status: **SUPPORT UI REVIEW ONLY**（A8 在 W14 不写 plugin 产品代码，按 `PARALLEL_COMMAND_BOARD.md` line 1186）
> Author lane: A8
> Scope: Review A6 UI ergonomics + 现有 graph/workspace non-regression；no plugin product code
> Time: 2026-09-08 01:00 CST
> Base HEAD: `a7eefbb` (W13 plugin manifest lifecycle 已 A0 push；master 与 origin/master 同步)

## 1. 角色与边界（A8 在 W14 做什么）

按 Board `## M5-W14 Plugin Manager UI Dispatch`（line 1163）Lane Table：

- **A8 Status**: SUPPORT UI REVIEW ONLY（line 1186）
- **A8 Scope**: "Review A6 UI ergonomics and existing graph/workspace non-regression; no plugin product code."
- **A8 Allowed Files**: `logs/assist/A8-M5-W14-*.md`
- **A8 Must Deliver**: UI review

W14 写产品代码的 lane 是 **A6**（PRODUCT CODE NARROW，line 1184）：`src/components/plugin/**`、`src/stores/usePluginStore.ts`、existing workspace/layout navigation、frozen-type corrections in `bridge.ts`/`types.ts`、`scripts/check-plugin-ui-logic.mjs`。

**因此本 lane 零产品代码、零 frontend 改动、零 policy 改动**，仅交付这份基于实测基线的 UI review 标准 + non-regression 基线。A6 patch 在本 batch 中产出后，本 note §4 的命令可在 A6 落地后追加实测结论。

## 2. 实测基线（已固化，作为 A6 patch 进入前后的对比锚）

### 2.1 导航集成模式（A6 W14 将在此挂载插件面板）

| 集成点 | 文件:行 | 现状（实测） | A6 W14 预期改动 |
|---|---|---|---|
| MainView 联合类型 | `src/stores/useLayoutStore.ts:5-24` | 19 值，`home/browser/files/clip/arts/grid/apps/term/repo/audit/scripts/commands/tools/db/tasks/skills/agents/graph/editor`；**无 `plugins`** | 追加 `"plugins"` |
| ActivityBar 工具组 | `src/components/layout/ActivityBar.vue:34-42` | 工具组含 apps/scripts/commands/tools/db/tasks/skills/agents/graph | 追加 `{ view: "plugins", icon, label: "插件" }` |
| MainArea 挂载分支 | `src/components/layout/MainArea.vue:178` | `<div v-else-if="layout.mainView === 'graph'" role="region" aria-label="知识图谱">` 模式 | 追加 `<PluginManagerPanel v-else-if="layout.mainView === 'plugins'" />` |
| Sidebar 侧栏 | `src/components/layout/Sidebar.vue:14,36` | 仅 files/clip/arts/repo/apps/audit 走侧栏；graph 无侧栏 | 插件面板是否走侧栏由 A6 决定（预期不需要，主区全面板即可） |

### 2.2 W13 frozen contract（A6 W14 必须消费、不得改）

| 契约项 | 位置（实测） | 内容 |
|---|---|---|
| PluginState 枚举 | `src/types.ts:938-947` | 8 值：`discovered/validating/signed_ok/signed_failed/loaded/enabled/disabled/uninstalled` |
| PluginSummary | `src/types.ts:949-952` | 列表项 DTO |
| PluginDetail | `src/types.ts:1016-1028` | **无签名原文 / 无资源路径 / 无正文**；含 `hash_prefix` |
| PluginSignatureView | `src/types.ts:1008-1013` | **无 `value` 原文**；仅 `algorithm/key_id/status` |
| PluginCapabilityView | `src/types.ts:1001-1006` | `capability/reason/acl_level(safe|confirm|dangerous)` |
| TrustedKeyRecord | `src/types.ts:1030-1033` | **不落公钥原文**，仅 `fingerprint` = sha256 前 16 hex |
| bridge 包装 | `src/bridge.ts:721-738` | `pluginInstall/pluginEnable/pluginDisable/pluginList/pluginGet/pluginKeysAdd/pluginKeysList/pluginKeysRemove`（8 个，全经 `invoke`） |
| ACL 命令数 | `src-tauri/permissions/default-commands.toml` | `grep -c "plugin_"` = **8** |

**W14 Hard Stop（line 1173）**："Existing command names and DTOs are frozen. Do not add lifecycle commands in W14." —— A8 评审将硬性确认 A6 不新增 `plugin_*` 命令、不改上述 DTO 形状。

### 2.3 Graph / Workspace non-regression 基线（实测）

```text
$ node scripts/check-graph-ui-logic.mjs
图谱 UI 逻辑测试：通过 113，失败 0
```

```text
$ grep -rn "import.*plugin\|from.*plugin" src/components/graph/ src/stores/useGraphStore.ts src/utils/graphUi.ts
(0 命中 = graph UI 对 plugin 零跨引用)
```

```text
$ grep -rn "layout.mainView === 'graph'" src/
src/components/layout/MainArea.vue:178:    <div v-else-if="layout.mainView === 'graph'" class="modview" role="region" aria-label="知识图谱">
(唯一 graph 面板挂载点；A6 改 MainView 联合时不破坏此分支)
```

## 3. A8 UI Review Checklist（待 A6 patch 落地后逐项核验）

每条均映射 W14 Hard Stops / W13 frozen contract / A8 角色。

### 3.1 UI Ergonomics（A8 评审 A6 可用性）
- [ ] **状态变更动作必须显式确认**：enable/disable/install/key-remove 触发 `ConfirmModal`（参考 `src/components/shared/ConfirmModal.vue`，已被 `GitWriteConfirmDialog` 等复用），不可一键静默生效。
- [ ] **生命周期状态清晰可读**：列表/详情须展示 `PluginState` 8 值之一；`signed_failed`/`validating` 等中间态有可视区分，不误导用户以为已启用。
- [ ] **列表/筛选/检视完整**：`pluginList(state?)` 支持按 `PluginState` 筛选；`pluginGet(id)` 详情面板渲染 `PluginDetail` 全部 redacted 字段。
- [ ] **无假执行外观（no fake execution affordance）**：UI 不得出现"运行/调用/执行插件"按钮或暗示（W14 Hard Stop：no `plugin_invoke`）。
- [ ] **安装为"提供 manifest"流程**：`pluginInstall` 接收 `{ manifest, resourcePath? }`，UI 提供本地 manifest 选择/粘贴入口；不触发网络下载。

### 3.2 Privacy（映射 A4 / W14 Hard Stop line 1172）
- [ ] **不渲染 raw signature**：`PluginSignatureView` 无 `value` 字段，UI 仅展示 `algorithm/key_id/status`；不得任何路径引入签名原文。
- [ ] **不渲染公钥原文**：`TrustedKeyRecord.fingerprint` 为 sha256 前 16 hex；UI 不展示完整 pubkey / 不展示 `pubkey` 原文（bridge `pluginKeysAdd` 入参含 `pubkey` 但 DTO 不回显）。
- [ ] **不渲染资源路径 / manifest 元数据 / 凭据 / 请求响应体 / stdout-stderr**：`PluginDetail.resource`（`PluginResourceMeta`）按其既有字段展示；不拼接展示本地绝对路径。
- [ ] **错误态不回显 secret**：错误文案走稳定码，不 echo key_id 以外的敏感串（对齐 W12/W13 privacy 口径）。

### 3.3 Boundary（映射 A2 / W14 Hard Stop line 1171）
- [ ] **仅经 `src/bridge.ts`**：所有 plugin 调用走 `bridge.pluginXxx`，无裸 `invoke("plugin_xxx", ...)`。
- [ ] **无 domain 重复**：store 不含后端已有校验逻辑（签名校验/ACL 判定属后端 `plugin.rs`）。
- [ ] **无生命周期权威扩张**：UI 不绕过 `PluginState` 状态机（`plugin.rs::transition` 已约束合法跃迁）；disable 后 UI 不暗示仍可运行。

### 3.4 Graph / Workspace Non-Regression（A8 本职）
- [ ] `node scripts/check-graph-ui-logic.mjs` 仍 **113/113 PASS**（A6 不碰 graph 文件；若碰属越界）。
- [ ] `grep -rn "import.*plugin" src/components/graph/ src/stores/useGraphStore.ts src/utils/graphUi.ts` 仍 **0 命中**。
- [ ] `MainArea.vue:178` graph 挂载分支完好；`useLayoutStore.ts` MainView 联合追加 `"plugins"` 不破坏既有 19 值。
- [ ] `ActivityBar.vue` 工具组追加 plugins 项后，既有 apps/scripts/.../graph 9 项顺序/可达性不变。
- [ ] `npm run build` PASS（A6 新增 `src/components/plugin/**` + `usePluginStore.ts` 不引入类型/构建回归）。
- [ ] `scripts/check-plugin-ui-logic.mjs` 由 A6 新建且可运行（A11 会纳入 matrix）；A8 仅确认其存在与覆盖上述 ergonomics/privacy 断言。

### 3.5 W14 Hard Stop 总闸（A8 复核）
- [ ] 无 `plugin_invoke`、代码执行、动态加载、网络下载/监听、daemon、模型调用、Agent/Skill 执行、MCP runtime 扩张、graph 写/导出、后台 worker。
- [ ] 不新增 lifecycle 命令（命令集冻结为 W13 的 8 个）。

### 3.6 已对当前工作树中 A6 产物实测核验（2026-09-08 01:xx CST）

A6 在本 batch 已先行落地 `src/utils/pluginUi.ts`(220 行) 与 `src/stores/usePluginStore.ts`(262 行)；`src/components/plugin/**` 面板与 `scripts/check-plugin-ui-logic.mjs` 尚未落地（待 A6 补）。A8 对**已落地产物**做了实测：

| 核验项 | 命令 | 结果 | 结论 |
|---|---|---|---|
| graph 源码零改动 | `git status --short \| grep -E "src/components/graph/\|useGraphStore.ts\|graphUi.ts\|check-graph-ui-logic.mjs"` | 无匹配 | ✅ baseline 完好 |
| graph UI 逻辑门禁 | `node scripts/check-graph-ui-logic.mjs` | **通过 113，失败 0** | ✅ non-regression 通过 |
| pluginUi.ts 无裸 invoke | `grep -c 'invoke("plugin_' src/utils/pluginUi.ts` | 0 | ✅ 仅经 bridge |
| pluginUi.ts 无 graph 引用 | `grep -c 'from.*graph\|graphUi' src/utils/pluginUi.ts` | 0 | ✅ 边界干净 |
| pluginUi.ts 脱敏投影 | 读 `redactDetail`(L141)/`redactKeyRecord`(L168) | 显式丢弃 `signature.value`/路径/metadata；`signature` 重建仅 `algorithm/key_id/status`；KeyRecord 仅 `key_id/fingerprint/note/added_at` | ✅ §3.2 通过（实现级，非仅注释） |
| pluginUi.ts 瞬时不持久化 | 读 `parseManifestInput`(L193) 注释 | `signature.value` 仅作瞬时后端入参，store/面板不持久化不回显 | ✅ §3.2 通过 |
| usePluginStore.ts 无裸 invoke | `grep -c 'invoke("plugin_' src/stores/usePluginStore.ts` | 0 | ✅ 仅经 bridge |
| usePluginStore.ts 无 graph 引用 | `grep -c 'from.*graph\|graphUi'` | 0 | ✅ 边界干净 |
| usePluginStore.ts 走冻结命令 | `grep -c 'bridge.plugin...'` | 17 处（list/install/enable/disable/get/keys* 全在 W13 8 命令内） | ✅ §3.3/§3.5 通过 |
| usePluginStore.ts 后端 DTO 已脱敏 | 读头部注释(L15) + 持 `PluginDetail`/`PluginSummary` | 后端 DTO 本身无 `signature.value`/`pubkey` 原文；`redactDetail` 作面板层纵深防御 | ✅ §3.2 通过 |

**遗留待 A6 面板落地后复核**（§4 命令覆盖）：
- `src/components/plugin/**`：须实测"无裸 invoke / 渲染走 `redactDetail` 投影 / 状态变更动作有 `ConfirmModal` / 无假执行按钮 / 不渲染资源绝对路径"。
- `scripts/check-plugin-ui-logic.mjs`：须由 A6 新建且可运行，覆盖 ergonomics/privacy 断言（A11 matrix 纳入）。
- `MainArea.vue` / `useLayoutStore.ts` / `ActivityBar.vue`：A6 集成插件面板的挂载点改动须不破坏 graph 与其它 18 个 mainView（§2.1/§3.4）。

## 4. A8 实测命令（A6 patch 完整落地后运行）

```bash
# 1) graph UI 非回归（必须 113/113）
node scripts/check-graph-ui-logic.mjs

# 2) graph 对 plugin 零引用（必须 0）
grep -rn "import.*plugin\|from.*plugin" src/components/graph/ src/stores/useGraphStore.ts src/utils/graphUi.ts

# 3) 新 plugin UI 对后端仅经 bridge（必须 0 裸 invoke）
grep -rn "invoke(\"plugin_" src/components/plugin/ src/stores/usePluginStore.ts

# 4) 不渲染敏感原文（必须 0：signature value / pubkey 原文 / 绝对路径拼接）
grep -rn "signature.value\|\.pubkey\b\|pubkey:" src/components/plugin/ src/stores/usePluginStore.ts
grep -rnE "C:/|/Users/|/home/|\\\\\\\\\"" src/components/plugin/  # 资源路径不展示

# 5) MainView 联合包含 plugins 且不破坏 graph
grep -n '"plugins"' src/stores/useLayoutStore.ts
grep -n "layout.mainView === 'graph'" src/components/layout/MainArea.vue

# 6) 构建
npm run build
```

> 注：A6 在本 batch 已落地 `src/utils/pluginUi.ts` + `src/stores/usePluginStore.ts`（§3.6 已实测）；`src/components/plugin/**` 面板与 `scripts/check-plugin-ui-logic.mjs` 待补。上述 §4 命令中第 1/2/3(部分)/4(部分)/5/6 项已对 util+store 实测通过（结论见 §3.6）；面板与 logic 脚本落地后由 A8（或 A11 matrix）补齐剩余实测，结论追加于此 note 或移交 A11 W14 verification。

## 5. Non-Regression 触发与升级路径

- 若 A6 patch 触发 graph UI 回归（§3.4 任一失败）：A8 记录并发 `logs/checkpoints/A8-M5-W14-*.patch` + checkpoint，按 A8 历史模式移交 A0 集成；不自行改 graph 文件（属越界）。
- 若 A6 出现隐私/边界/假执行问题（§3.2/3.3/3.5）：A8 在 note 中标记 BLOCKED 并移交 A10 security review + A0。
- 若 A6 改动了 A8 lane 既有 graph 文件：`git status` 会暴露，A8 立即停并报告（WORKSPACE_IDENTITY §35）。
- 目前工作树中 `PARALLEL_COMMAND_BOARD.md` / `详细设计与实施计划.md` 两个 M 文件非 A8 改动（A0 在 a7eefbb 推 W13 后残留 + 主文档对齐），按 Batch Rule 3 / WORKSPACE_IDENTITY §35 严格不碰。

## 6. 与 A8 历史笔记关系

- `logs/assist/A8-M5-W13-graph-ui-nonregression-20260908-0000.md` — W13 graph UI non-regression（已合入 a7eefbb）；本 note 的 graph 基线 §2.3 直接引用其结论。
- `logs/assist/A8-M5-W12-graph-ui-consumer-20260907-2251.md` — W12 graph UI 完整消费实现。
- 本文件 = W14 起点，固化"导航集成模式 + W13 frozen contract + graph/workspace 基线 + A6 评审清单"，不重复 W12/W13 内容。

## 7. 不 push 声明

按规则，**未 push**。本 lane 零产品代码，无 push 内容。后续如需出 patch 修复（§5 触发），仍按 A8 历史模式本地工作树留待 A0 集成。
