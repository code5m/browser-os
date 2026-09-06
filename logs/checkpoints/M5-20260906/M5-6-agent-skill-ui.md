# M5-6 Agent/Skill UI（对话 / 管理 / 权限预览）

> 子卡 ID：**M5-6** · 需求 #12 · `[S3|LEVERAGE:1|COMPLEX|AI:NORMAL|R:high]`
> 责任 Lane 候选：**A19**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L568（`M5-6 Agent/Skill UI`）
> 主预研：暂无 prework 文档（A5/A6 prework 仍空，由 A19 实施期补）
> 配套：`M5-4-agent-skill-runtime.md`（后端）· `M5-5-agent-skill-commands.md`（15 条命令）
>
> **W3** BLOCKED（待 A16 = A6 W5 实施期承接）· **W4** ACTIVE（**A6 UI pure logic/panel shell**；详见本卡顶部 `[W5 next-card acceptance criteria]` 段）

---

## [W5 next-card acceptance criteria · 2026-09-06 18:35 CST] A6 M5-6 W5 实施期 acceptance criteria（UI pure logic + panel shell · 不调 live runtime）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L156（**A6 M5-W5** *"Implement M5-6 Agent/Skill UI pure logic and panel shell: validation display, permission preview, capability list, empty/error states. Prefer helper module + headless logic test. Do not execute skills, install plugins, or call live runtime."*）+ L134-170 硬约束 + A6 W4 UI contract（`562efb9` 拣入 *"convert A5 domain -> UI data contract + panel state plan"*）锁定的 SkillExec / AclLevel / AgentDef / StreamChunk 经 Tauri event 镜像 + no-router 锚点 useLayoutStore.ts MainView+MOD_META。
> **消费依赖（已落地，A6 W5 可直接接入）**：
> - **A5 W4 AgentDef/SkillDef 已落**（`1610939` 拣入 `src-tauri/src/agent.rs` 104 行 + `src-tauri/src/skills.rs` 147 行）—— A6 W5 UI 可在 `src/types.ts` 加对应 TS 镜像（无 drift）。
> - **A5 W4 permission preview API 已落**（`1610939` `agent.rs` / `skills.rs` 内的 `preview_*` 纯函数）—— A6 W5 UI 可直接消费。
> - **A6 W4 UI data contract 已锁**（`562efb9`）—— SkillExec / AclLevel / AgentDef / StreamChunk 四个 TS 类型在 `src/types.ts` 经 Tauri event 镜像；no-router 锚点 useLayoutStore.ts MainView+MOD_META。
> - **A4 W4 agent_kv 已落**（`1610939` `agent_memory.rs`）—— A6 W5 UI 可消费 memory 列表/详情。
> **A1 W5 角色**：A1 W5 **不**改 §1~§11 决策史；仅在头部加本 `[W5 next-card acceptance criteria]` 段，**明确 A6 W5 实施期 4 项 AC + 5 项 hard stops**，供 A6 / A10 / A11 / A0 验收。

### W5 A6 M5-6 实施期 acceptance criteria（4 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **UI 纯逻辑 helper module 冻结** | `src/utils/agentSkillUi.ts`（或类似 helper）冻结纯函数：① `validateAgentDefInput(def) -> ValidationResult` ② `validateSkillDefInput(def) -> ValidationResult` ③ `formatPermissionPreview(preview: PermissionPreview) -> PreviewItem[]` ④ `renderCapabilityBadge(cap: CapabilityKind) -> BadgeDescriptor` ⑤ `emptyStateFor(kind) -> EmptyStateDescriptor` ⑥ `errorStateFor(err) -> ErrorStateDescriptor` —— 全部纯函数（无 Tauri invoke、无网络、无 fs）| `node scripts/check-agent-skill-ui-logic.mjs` PASS + A11 抽查 |
| AC-2 **校验展示 + 错误高亮** | 校验失败时面板显示字段级错误（field + message + severity：error/warn），禁止把后端 DTO 整个堆到 UI 错误提示；不展示原始后端错误堆栈（脱敏后仅留 message + 1 行 hint）| UI 逻辑单测 + 视觉走查（manual checklist） |
| AC-3 **permission preview 桥接 + capability 列表** | 面板在 install/run/chat 前调用 A5 W4 提供的 `preview_*` API（消费 `MCP_CAPABILITY_V1` + AclLevel + touches_fs + returns_url）；capability 列表用 `formatCapabilityBadge` 统一渲染；二次确认弹窗的文案复用 `formatPermissionPreview` 输出（不允许 UI 自由发挥文案）| UI 逻辑单测 + 与 A5 W4 `preview_*` 函数签名 0 drift |
| AC-4 **empty/error 状态** | 每种面板（Chat / Manage / Permission）至少 2 个 empty state（list-empty / filtered-empty）+ 2 个 error state（load-fail / action-fail）；空态/错态**不**暴露任何后端原始字段；图标 + 文案 + 复试图标三件套 | `node scripts/check-agent-skill-ui-logic.mjs` 含 empty/error 覆盖 + A11 抽查 |

### W5 A6 M5-6 实施期 hard stops（5 项）

| HS | 约束 | 来源 |
|----|------|------|
| W5-HS1 | **不调 live runtime / 不装插件 / 不执行 skill** —— UI 仅展示/校验/preview，**不**调 `skill_run` / `agent_chat` / `skill_install` 等真实 Tauri 命令 | PARALLEL_COMMAND_BOARD L156 |
| W5-HS2 | **无 execution runtime / installer / network/model calls / 新后端 commands** | PARALLEL_COMMAND_BOARD L166 |
| W5-HS3 | **无新 npm 依赖**（A6 须复用现有 Vue 3 + Pinia + D3.js + 既有 `useLayoutStore`） | A6 W5 dispatch L156 + W4-HS2 |
| W5-HS4 | **capability 真源单点**（必须复用 `MCP_CAPABILITY_V1` + A5 W4 `preview_*` 纯函数，禁止在 `agentSkillUi.ts` 写 capability 白名单副本） | A2 W2 + A3 W3 + R-B3 + A5 W4 AC |
| W5-HS5 | **所有 lane 必须从 `origin/master` pull，不 push** | PARALLEL_COMMAND_BOARD L170 |

### W5 验证清单（供 A11 收口）

- `npm run build` PASS（`dist/assets/index-*.js` 大小不破 IF-2 阈值；W5 仍受 A0 大小门禁约束）
- `node scripts/check-agent-skill-ui-logic.mjs` PASS
- `python3 scripts/check-agent-skill-ui-policy.py --self-test` PASS（如新增 UI policy 脚本）
- `python3 scripts/check-agent-skill-ui-policy.py` PASS
- `python3 scripts/check-agent-skill-ui-policy.py --expect-pending` PASS（如有 PENDING）
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- A11 比对 `src/types.ts` 与 A5 W4 AgentDef/SkillDef 0 drift
- **A10 复审 PASS**（no second execution path / no installer / no network / no model provider / no download / capability 真源单点 / 无 prompt-secret 展示）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W5-*.md`

### W5 A1 不修订范围（本卡）

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W5 **不动**（决策史保持 W0 原文；W5 AC 在本顶部段单列）。
- **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W5 不动（policy 脚本由 A6 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。

---

## 0. 编号与锚定

- 批次任务号 `M5-6`；需求号 #12；WBS L568 一致。
- 依赖：M5-5 ✅（15 条命令稳定）+ `src/components/browser/AINavPanel.vue` 既有面板 ✅
- 前端栈：Vue 3 + Pinia + 既有 AI store（`src/stores/useAIStore.ts`）

---

## 1. GOAL

在前端实现 Agent/Skill 三个核心 UI：① 对话面板（流式渲染，扩展既有 AINavPanel）② 管理面板（列表/详情/编辑/删除）③ 权限预览面板（安装/运行前的二次确认弹窗）。**所有写操作必走 M5-5 闸门**（前端不绕过 `bridge.ts`）。

---

## 2. READ

1. `src/components/browser/AINavPanel.vue`（既有 AI 面板，**全读**——延用其 store/keyboard 快捷键）
2. `src/stores/useAIStore.ts`（既有 AI 状态管理，**全读**）
3. `src/bridge.ts`（15 条新命令的 TS 包装，**全读**）
4. `src/types.ts`（AgentDef/SkillDef/RunRecord TS 镜像）
5. `M5-4-agent-skill-runtime.md` §4.3（流式契约）
6. `M5-5-agent-skill-commands.md` §4.1（15 条命令）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src/components/agent/ChatPanel.vue` | **新增** | 对话面板（流式渲染、取消按钮、历史回看） |
| `src/components/agent/SkillManager.vue` | **新增** | Skill 管理面板（列表/详情/安装/删除/编辑） |
| `src/components/agent/AgentManager.vue` | **新增** | Agent 管理面板（列表/详情/安装/删除/编辑） |
| `src/components/agent/PermissionPreviewModal.vue` | **新增** | 通用权限预览弹窗（安装/运行前必显） |
| `src/components/agent/RunHistoryModal.vue` | **新增** | 运行历史查看 |
| `src/stores/useAgentStore.ts` | **新增** | Agent/Skill 状态管理（流式累积、列表缓存、闸门确认态） |
| `src/composables/useAgentStream.ts` | **新增** | 流式订阅（包事件解包 + 重连 + 取消） |
| `src/router/agent.ts` | **新增** | Agent/Skill 路由（与既有 `pages/` 同款） |
| `src/locales/zh-CN.json` `src/locales/en-US.json` | 扩展 | 14 个 i18n key |
| `src/styles/agent.scss` | **新增** | 样式（与既有 dark/light 双主题适配） |

---

## 4. 关键契约

### 4.1 三个面板布局

- **对话面板**（`ChatPanel.vue`）：
  - 顶部：Agent 选择器（多 Agent 切换）
  - 中部：流式消息列表（按 `event` 自动滚动 + 取消按钮）
  - 底部：输入框 + 附件（复用既有 `useFileUpload` composable）
  - 侧栏：会话列表（按 `session_id` 分组）

- **Skill/Agent 管理**（`SkillManager.vue` / `AgentManager.vue`）：
  - 表格：id / version / display_name / acl / 安装状态 / 操作
  - 操作按钮：详情、编辑、删除、（重新）安装
  - 详情：含 `metadata` / `capabilities` / `tests` / `runs` 最近 10 条
  - 必触发 `PermissionPreviewModal`

- **权限预览**（`PermissionPreviewModal.vue`）：
  - 必显项（**禁**折叠）：Skill/Agent 名称 + 版本 + ACL + capabilities（逐项） + 目标操作 + 风险提示
  - 二次确认（keyring 二次认证）按钮用于 `Dangerous`
  - 取消 = 拒绝（无"延后"）

### 4.2 流式渲染契约

- 监听 `agent://<agent_id>/stream` 事件
- 按 `session_id` 路由到 `useAgentStore.stream[sessionId]`
- 自动滚动：仅当用户位于底部；用户向上滚动时禁用
- 取消：发 `agent_chat_cancel` 命令

### 4.3 闸门确认态管理

- `useAgentStore` 维护 `pendingConfirms: Map<id, {action, payload, expires_at}>`
- 前端到 Tauri 命令的"二次确认"流程：
  1. UI 调用 `skill_install` → Tauri 返回 `CONFIRM_REQUIRED` + `request_id`
  2. UI 自动打开 `PermissionPreviewModal`，展示详情
  3. 用户点击确认 → UI 调用 `confirm_<action>` 命令（带 `request_id`）
  4. Tauri 二次校验 + 执行
- **禁**前端直接绕开闸门执行

---

## 5. FORBID

- **不**让任何 UI 路径直接走 Tauri `invoke`（必须经 `bridge.ts`）
- **不**让闸门弹窗被折叠/隐藏/默认确认
- **不**让流式渲染不节流（与 M5-4 §4.3 一致）
- **不**让 Agent 切到无对应 capability 的状态
- **不**让 CSS 与 dark/light 主题冲突（双主题适配是硬要求）
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 文件结构
ls src/components/agent src/composables/useAgentStream.ts src/stores/useAgentStore.ts 2>&1

# B. 反向用例
# N1: ChatPanel 发 1 MB 文本 → 前端截断到 64 KB
# N2: 用户向上滚动时收 chunk → 不自动跳到底
# N3: Dangerous Skill 装时 PermissionPreviewModal 不显 → 阻断
# N4: 不通过 bridge.ts 直接 invoke → 阻断（CI 断言）
# N5: 切换 dark/light 主题 → 无样式崩
# N6: i18n key 缺失 → 阻断

# C. e2e 冒烟
pnpm test:unit   # 含 ChatPanel / Manager / PermissionPreview
pnpm test:e2e    # 含闸门弹窗 e2e

# D. 编译与基线
pnpm build
pnpm tsc --noEmit
pnpm lint
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | 5 个新组件落地 | 命令 A |
| 2 | 流式渲染节流 + 取消按钮 | 单测 N2 |
| 3 | 闸门弹窗必显 | 单测 N3 |
| 4 | UI 仅经 `bridge.ts` 调 Tauri（无裸 `invoke`） | CI 断言 N4 |
| 5 | dark/light 主题适配 | 单测 N5 |
| 6 | i18n 双语齐 | 单测 N6 |
| 7 | `pnpm test:unit` + `pnpm test:e2e` 全绿 | 命令 C |
| 8 | `pnpm build` 无错 | 命令 D |
| 9 | `pre-merge.sh` ALL_PASS | 复用既有 |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 闸门被绕过 | 阻断（红线） |
| 裸 `invoke` 出现 | 阻断（必须经 bridge.ts） |
| 流式不节流 | 阻断（M3-4.b 静默事件先例） |
| dark/light 主题崩 | 阻断（视觉红线） |
| i18n 缺 key | 阻断 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L568 `[ ]` → `[x]`
2. `后续需求TODO.md` §12 状态 `DONE`
3. `AI-模型切换与接手清单.md` NEXT 移至下一卡
4. `logs/checkpoints/M5-6.a-2026MMDD-HHMM.md`

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A19 实施填
- **NEXT**：M5-7（M5-13 验证矩阵留待后期），与 Graph 同 A19/A17 拆卡

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src/`、`package.json`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
