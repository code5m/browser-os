# Phase 04 Dispatch Plan

**Status: CLOSED (2026-09-12)** — 实现已提交并通过验收；见末尾 `CHIEF_ARCHITECT_ACCEPTANCE`。

## Chief Architect Decision

Phase 03 CLOSED（2026-09-12）。Pre-Phase-04 Guardrail Hardening（npm 依赖结构化检测）已合并，整文件 SHA256 脆性消除。Phase 04（产品路线图 Phase 4：自动保存关闭页签）可启动。

## Phase 04 Goal

按 `phase-04-specification.md` 实现"自动保存关闭页签 + 最近关闭列表 + Ctrl+Shift+T"，**全前端、零原生改动、零权限扩张**。

## Shared Contract

所有 Phase 04 Agent 必须遵守 `phase-04-specification.md`；实现前必读：

- `AGENTS.md`, `PROJECT-RULES.md`, `.ai/context/README.md`, `.ai/registry.md`
- `docs/AI/00-Architecture.md`, `01-Detailed-Design.md`, `05-CODEBASE-MAP.md`
- `phase-04-specification.md`, `phase-04-dispatch-plan.md`
- 相关源：`useSessionStore.ts` / `useBrowserStore.ts` / `useSettingsStore.ts` / `SessionPanel.vue` / `SessionCloseDialog.vue` / `App.vue` / `types.ts` / `bridge.ts`

若某 Agent 无法满足规格，必须停下并返回 `Conflict Report`，不得静默越界。

## Dispatch Order

### Batch A — 设计与契约（Design & Contract）

- **Owner**：Detailed Design Agent（`02`）产出 + Architecture Agent（`01`）评审
- **产出**：`docs/AI` 下 Phase 04 详细设计章节（复用既有 `SessionPolicy` / 关闭协议描述，补充 `auto_save_on_close` 与 `recentlyClosed` 数据契约）；确认不变量与验收矩阵。
- **允许**：`docs/AI/**`（仅补充 Phase 04 设计段）
- **禁止**：`src/**`、`src-tauri/**`、`package.json`、`PROJECT-RULES.md`
- **验收**：设计文档与 specification 一致；无新增依赖 / 命令。

### Batch B — 前端 store + 策略（Frontend Store & Policy）

- **Owner**：Frontend Store Agent（内联定义；受 `01` / `02` 评审约束）
- **实现**：
  - `useSessionStore`：新增 `auto_save_on_close` 前端态（默认 `false`）；`requestClose(tabId)` 在 `auto_save_on_close=true` 时 `saveTab(tabId, preview)` + `closeTabNow(tabId)`（不弹窗）；保留 `resolveClose` 供 legacy 模式。
  - `useBrowserStore`：`recentlyClosed` 内存栈（封顶 20）+ `recordClose(url, title)`（在 `closeTabNow` 起始调用，捕获**所有**关闭）+ `restoreRecent()`（pop + `tabNew(url)`）。
  - `types.ts`：`SessionPolicy` 注释增补 `auto_save_on_close`（前端态）；新增 `RecentlyClosedEntry = { url: string; title: string }`。
  - `bridge.ts`：`setSessionPolicy` 透传（若采用纯前端态则可不改）。
- **允许**：`useSessionStore.ts` / `useBrowserStore.ts` / `types.ts` / `bridge.ts`
- **禁止**：`src-tauri/**`、其它 store、`package.json`
- **验收**：`check-architecture` / `check-browser-runtime` / `check-task-boundary` PASS；`git diff --check`。

### Batch C — UI 与快捷键（UI & Shortcut）

- **Owner**：Frontend UI Agent
- **实现**：
  - `SessionPanel.vue`：新增「关闭时自动保存」开关（绑定 `auto_save_on_close`）+ 「最近关闭」区块（列 `recentlyClosed`，点按调 `restoreRecent` 对应项）。
  - `useSettingsStore.ts`：keymap 增加 `recentlyClosed: "Ctrl+Shift+T"`。
  - `App.vue`：在既有快捷键分发中注册 Ctrl+Shift+T → `browser.restoreRecent()`（镜像 Ctrl+W `closeTab` 的接线方式）。
- **允许**：`SessionPanel.vue` / `useSettingsStore.ts` / `App.vue`
- **禁止**：`src-tauri/**`、`SessionCloseDialog` 语义改动（保持 legacy 行为）、`package.json`
- **验收**：`check-ui` / `check-native` PASS（无新 `fixed` 浮层、无危险原生 API）；`git diff --check`。

### Batch D — 交叉校验与验收（Cross-check & Acceptance）

- **Owner**：Review Agent（`12`）+ Chief Architect 自审
- **运行**：5 个 checker + `npm run check` + `npm run doctor` + `pre-merge --self-test` ALL_PASS + `git diff --check` + `npm run build`。
- **人工验收**：按 specification「人工验收项」逐条。
- **Review Agent 输出**：`PHASE_04_REVIEW_RESULT`。

## Review Dispatch

Review Agent 须验证：

- 每个改动的 checker 仍 PASS；`--json` 稳定；exit code 符合契约。
- 关闭协议"不可静默丢"精神被保留（`auto_save_on_close` 走显式 `sessionSave`）。
- `WebView` 生命周期路径未被改动（`closeTabNow` 原样）。
- 无 Agent 修改 forbidden 文件；无新 npm 依赖；无新运行时权限。
- `recentlyClosed` 不持久化敏感内容。

返回：

```text
PHASE_04_REVIEW_RESULT
Status: CODE_PASS | BLOCKED
Findings:
- ...
Open Questions:
- ...
Residual Risk:
- ...
Recommended Owner Actions:
- ...
```

## Chief Architect Acceptance Criteria

Phase 04 可关闭当：

- 所有 checker PASS；`npm run check` / `doctor` PASS；`pre-merge --self-test` ALL_PASS；`git diff --check` 通过。
- Review Agent 无 BLOCKER。
- 人工验收项全过；`closeTabNow` 路径原样（WebView 生命周期未被改动）。
- 体积增量 ≤ 25.2%；无新 npm 依赖；无新运行时权限。
- `src-tauri/**` 零改动。

最终结论：

```text
CHIEF_ARCHITECT_ACCEPTANCE
Phase: 04
Status: ACCEPTED
Result: CLOSED
```

## Autonomous Execution

遵循 `.ai/workbuddy-dispatch/autonomous-execution-policy.md`（No Ask Unless Blocked）：`CODE_PASS` 后自动 commit（**实现提交与治理文档提交分离**）、自动进入下一 Batch；仅命中 Stop Condition 时阻塞并请求 Owner 决策。

- 自动执行（不询问）：进入下一 Batch、提交已验证文件、`git diff --check` / `--self-test` / `--json` 验证、P3 / WARN 非阻塞项按注释处理、commit message 措辞、生成下一阶段派发卡。
- 必须阻塞：P1 / BLOCKER、Conflict Report、需改 forbidden 区、改变已批准架构 / 产品决策、破坏性 / 不可逆操作、两种合法方案均影响产品方向。

## Phase 04 Result

```text
CHIEF_ARCHITECT_ACCEPTANCE
Phase: 04
Status: ACCEPTED
Result: CLOSED
```

- 实现提交：`acf4add`（6 文件，+127 / -6）
- 规则更新提交：`4e87eb1`（PROJECT-RULES.md PENDING→APPROVED）
- 验收（Batch D）：`npm run build` ✓；volume gate ✓（基线 `build-metrics-052b18a.json`，增量 ≤ 25.2%）；
  Phase 03 五件套（architecture / ui / native / runtime / task-boundary）+ `npm run check` + `npm run doctor` ✓；
  `check-session-logic.mjs`（M1-9 会话关闭协议）✓；`git diff --check` ✓；`src-tauri/**` 零改动；
  无新 npm 依赖；无新运行时权限；`closeTabNow` 的 WebView 生命周期原样复用。
- 已知既有问题（**非 Phase 04 引入，不构成 Stop Condition**）：`pre-merge.sh` 在 `check-command-domain-policy.py`
  （M2-6.a）报 `CMD_NPM_DEP_ADDED:package.json`——检查器内嵌的 `package.json` 指纹基线（`ad521…`）与已提交
  `package.json` 指纹（`0d885…`）不一致，属**基线漂移**；`git diff HEAD -- package.json` 为空，确认与 Phase 04
  改动无关（6 个改动文件均未碰 `package.json` / 该检查器）。建议由 Phase 03 hardening 负责人按 6bc6491 / 554be5a
  同范式 rebaseline 该指纹，不属本阶段范围。

## Phase 04 裁决反转（Task B，2026-09-12）

Owner 最终产品裁决推翻原 `auto_save_on_close` 决策：

- 普通 Tab 关闭 = **不弹确认框 + 不持久化 + 直接关闭**（仅写入 `recentlyClosed` 内存栈 → `closeTabNow`）。
- `auto_save_on_close` 已撤销并移除（state / `requestClose` 自动保存分支 / `SessionPanel` 开关 / 相关死代码）。
- 原关闭协议（`SessionCloseDialog` 三选一）从普通 Tab 关闭路径移除，组件已删除；相关 checker 同步改为新裁决门禁。
- `recentlyClosed` 内存栈（≤20、仅 {url,title}、不落盘、Ctrl+Shift+T 恢复）与手动「保存会话」能力保留，且解耦于关闭。
- 应用退出保存策略（`auto_save_on_exit`）保持不变（独立生命周期，本次不触碰）。

配套提交：实现提交（src）+ 门禁/规则提交（scripts + PROJECT-RULES.md）。确定性门禁
`check-session-logic.mjs` / `check-session-persistence-policy.py` / `check-native-webview-overlay.mjs`
已更新为：普通关闭若重新触发 prompt 或 `sessionSave`，必然 FAIL。
