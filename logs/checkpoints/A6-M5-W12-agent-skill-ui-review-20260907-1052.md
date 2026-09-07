# A6 M5-W12 Agent/Skill 工作区 UI 评审交付（UI REVIEW ONLY）

> LANE=**A6**　ROUTE=`AI:DEEP` / R:high
> 执行者：Lane **A6**（CodeBuddy 会话，Hy4 / 腾讯混元）　`IMPLEMENTER_MODEL=CodeBuddy Hy4`
> 时间：2026-09-07（本地 10:52 UTC ≈ CST 18:52）　基线 HEAD：`269269a`（`master`，与 `origin/master` 同步）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W12 Graph Live-Query Readonly Dispatch → A6 `START UI REVIEW ONLY`
> 性质：**UI 评审型（UI REVIEW ONLY）**——仅 review Agent/Skill 工作区面板，确认执行控件禁用且状态确定；**零代码改动、零后端改动**。

---

## 0. 本卡边界（A6 自约束，W12 Hard Stops）

| 项 | 值 |
|---|---|
| LANE | A6（M5-W12，REVIEW ONLY；Merge Order 未定，归 A0 集成） |
| Task | Review workspace UI for Agent/Skill panels after W12 graph changes；keep execution controls disabled and deterministic |
| Allowed Scope（写） | `src/components/workspace/**`、`scripts/check-agent-skill-ui-logic.mjs`、`logs/checkpoints/**`、`logs/assist/**`（仅本卡文档） |
| 写禁 | 不得改 `src-tauri/`、不得改 `src/components/graph/**`、不得新增后端命令、不得触碰 A8 图 UI 作用域 |
| 硬停止（W12） | 仍是 dry-run only；无真实 file/db/script/plugin/agent/skill 执行；无 TCP/HTTP listener/daemon；UI 状态不回显原始参数/query/token/cookie/Authorization；依赖新增须可选项 + feature-gated；仅 A0 push |
| 禁止 | 向 `src/components/workspace/**` 之外写入产品代码；push |

---

## 1. 启动门禁（实测）

```text
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git fetch origin ; git pull --ff-only        → 已快进至 269269a（含 W11 集成，含本 A6 W11 面板 loading 态）
git status --short --branch                  → ## master...origin/master（W11 集成后干净）
git rev-parse --short HEAD                   → 269269a
```

> 注：本会话上一段的 `git pull --ff-only` 实际已完成快进（HEAD 由 `5226aad`→`269269a`）。本 A6 W11 的面板 `loading-state` 改动与 checkpoint 已被 A0 集成进 `269269a`，工作树干净。本次 W12 评审基于 `269269a`。

---

## 2. 评审发现（逐条取证）

### 2.1 执行控件审计 —— 结论：禁用且无执行入口 ✅

对 `src/components/workspace/` 下 Agent/Skill 相关四个组件全量 grep 交互控件：

| 控件 | 文件:行 | 性质 | 是否执行入口 |
|---|---|---|---|
| `<button :disabled="!store.backendReady" @click="store.loadAgents()">刷新</button>` | `AgentManagerPanel.vue:36` | 只读刷新，未就绪则禁用 | 否 |
| `<button :disabled="!store.backendReady" @click="store.loadSkills()">刷新</button>` | `SkillManagerPanel.vue:38` | 只读刷新，未就绪则禁用 | 否 |
| `@click="store.selectAgent(a.id)"` / `selectSkill(s.id)` | `AgentManagerPanel.vue:53` / `SkillManagerPanel.vue:55` | 卡片选中（非执行） | 否 |
| `@click.self="emit('close')"`、`close` 按钮、`我已知晓` 按钮 | `PermissionPreviewModal.vue:18/22/39` | 关闭/知悉，无确认安装/运行 | 否 |
| `AgentChatPanel.vue` | 全文件 | **零 `<button>` / `@click`** | 否 |

**结论**：工作区 Agent/Skill 面板**不存在任何**安装/运行/发送/调用执行入口；唯一写向 store 的只有 `loadAgents`/`loadSkills`（只读读刷新），且在 `backendReady=false` 时按钮禁用。完全满足 W12「keep execution controls disabled」。

### 2.2 确定性状态 —— 结论：四态互斥且确定 ✅

`panelState()`（`src/utils/agentSkillUi.ts:289`）返回优先级：`error > loading > empty(只读壳文案) > ready`，四态互斥。

本卡在 W11 已补的 `loading` 渲染仍在位：
- `AgentManagerPanel.vue:45` `<div v-if="state.state === 'loading'" class="loading-state">…` ；`:88` `.loading-state` 样式
- `SkillManagerPanel.vue:47` 同构；`:95` `.loading-state` 样式

模板分支：`loading`（新）→ `v-else-if empty` → `v-else list`。`error` 由顶部 `banner-error` 承载，`empty` 文案在 `backendReady=false` 时含「只读壳」。`error/loading/empty/ready` 视觉可区分、确定性无歧义。

### 2.3 UI 逻辑测试 —— 结论：PASS ✅

```text
node scripts/check-agent-skill-ui-logic.mjs
  → check-agent-skill-ui-logic: 110 assertions passed, 0 failed
```

（与 W11 基线一致，未因 W12 任何改动回归。）

### 2.4 W12 作用域隔离 —— 结论：图改动未触及 workspace ✅

`src/components/workspace/` 近期提交：
- `269269a feat(M5): integrate W11 MCP stdio dry-run hardening`（含本 A6 W11）
- `8dea830 feat(A6): M5-W10 Agent/Skill UI lockdown (UI small only, no execution buttons)`
- `f99d2eb feat(A6): M5-6 Agent/Skill UI pure logic + panel shell (W5)`

**W12 图相关文件位置（属 A8/A7，不在 A6 作用域）**：
- `src/components/graph/`（EdgeDetail.vue / GraphFilter.vue / GraphPanel.vue / GraphViewer.vue / NodeDetail.vue）—— A8 图 UI 消费
- `src/stores/useGraphStore.ts`、`src/utils/graphUi.ts` —— A8 图 UI 逻辑

`ls src/components/workspace/ | grep -i graph` → **NONE**。W12 图改动与 A6 工作区面板**零交集、零回归**。

### 2.5 构建取证 —— 结论：无体积回归 ✅

```text
npm run build  →  ✓ built in 2.80s
  dist/assets/index-BIrezpj4.js  164.82 kB │ gzip: 59.04 kB   （主 chunk，< 22% 阈值）
```

（A6 本卡零代码改动；构建通过系既有状态校验。）

---

## 3. W12 Hard Stops 合规自审

| 硬停止 | 状态 | 证据 |
|---|---|---|
| dry-run only，无真实执行 | ✅ | 2.1 审计：无任何执行入口；store 仅只读 `loadAgents/loadSkills` 与选中 |
| 无 TCP/HTTP listener/daemon | ✅ | 纯 review，零进程/网络代码 |
| 无 secret 回显（UI 状态） | ✅ | Agent/Skill 面板不渲染任何参数/query/token；脱敏逻辑（W10）未改动且测试全绿 |
| 依赖新增须可选项 + feature-gated | ✅（无依赖改动） | 未改 `package.json` / `Cargo.toml` |
| 默认构建行为不变 | ✅ | 2.5：主 chunk 164.82 kB，无回归 |
| 仅 A0 push | ✅ | 本卡未 push，仅产出评审 checkpoint |

---

## 4. 观察项（非阻塞，留待 W13+）

- **error 态 body 渲染**：当 `panelState` 返回 `error` 且 `count===0` 时，模板 `state.state==='error'` 落入 `v-else` 渲染空 `list`，而错误由顶部 `banner-error` 承载。功能上已确定（banner 已显错误），但若追求严格对称，未来可在 `v-else-if loading` 与 `v-else-if empty` 之间增 `error` body 块。属**可读性打磨**，非 W12 阻塞，建议 W13 由 A6 视需要补。

---

## 5. 工作树中其它 Lane 在途产物（非本卡范围，未纳入）

实测 `git status --short` 显示 9 条其它 Lane 的 W12 交付物（A1/A2/A3/A5/A9 的 checkpoint/patch/assist 及少量脚本改动），**均与 A6 作用域无关**，本卡不 stage、不打包、不改动：
- `A logs/assist/A9-M5-W12-plugin-delta-nochange-20260907-2045.md`、对应 checkpoint（A9，staged）
- `M scripts/check-mcp-policy.py`（疑似 A3 MCP 评审脚本）
- `?? logs/assist/A2-M5-W12-20260907-2030.md`、`?? logs/assist/A3-M5-W12-mcp-graph-exposure-review-20260907-1051.md`、`?? logs/assist/A5-M5-W12-20260907-2045.md`
- `?? logs/checkpoints/A1-M5-W12-reconciliation-20260907-2030.md`、`?? logs/checkpoints/Lane-A3-M5-W12-mcp-graph-exposure-20260907-1051.patch`、`?? logs/checkpoints/Lane-A9-M5-W12-plugin-delta-nochange-20260907-2045.patch`

这些由各自 Lane 自行交付、A0 集成；A6 仅确认其作用域未越界。

---

## 6. Lane Output Template（A6）

```text
LANE=A6
STATUS=PASS（no-change review，UI logic PASS）
BASE=269269a
HEAD=logs/checkpoints/A6-M5-W12-agent-skill-ui-review-20260907-1052.md
FILES=（无代码改动；仅本 checkpoint）
VERIFY=node scripts/check-agent-skill-ui-logic.mjs → 110 passed/0 failed; npm run build → EXIT=0 (index 164.82 kB); grep 执行控件审计见 §2.1
CHECKPOINT=logs/checkpoints/A6-M5-W12-agent-skill-ui-review-20260907-1052.md
MERGE_NOTES=纯评审卡，零代码改动，不会与 A7/A8 图后端/图 UI 产生冲突；作用域 src/components/workspace/** 与 A8 的 src/components/graph/** 零交集；A10 安全复核可据此确认 Agent/Skill 面板无执行路径、无 secret 回显、确定性状态完整。
NEXT=W13（若 A6 继续）：可按 §4 观察项补 error-body 对称性打磨；否则转为图 UI（A8）或插件（A9）相关支持位，待 A0 推进。
```

---

## 7. 明确未做（不冒领）

- 未改任何产品代码（`src/components/workspace/**` 零写入；`src-tauri/` 零触碰）。
- 未生成代码补丁（本卡零源码改动，故不产出 `Lane-A6-M5-W12-*.patch`；交付物为评审 checkpoint）。
- 未触碰 A8 的 `src/components/graph/**`、`src/stores/useGraphStore.ts`、`src/utils/graphUi.ts`。
- 未改 `scripts/check-agent-skill-ui-logic.mjs`（W12 仅需其保持 PASS，本卡零改动即满足）。
- 未提交、未 push（指挥板：仅 A0 可向 `master` 提交与推送）。
