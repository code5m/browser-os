# A6 M5-W11 Agent/Skill UI 小修整包交付（UI SMALL ONLY）

> LANE=**A6**　ROUTE=`AI:DEEP` / R:high
> 执行者：Lane **A6**（CodeBuddy 会话，Hy4 / 腾讯混元）　`IMPLEMENTER_MODEL=CodeBuddy Hy4`
> 时间：2026-09-07（本地 10:20 ≈ CST 18:20）　基线 HEAD：`5226aad`（`master`，本地领先 `origin/master`）
> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W11 MCP Stdio Dry-Run Hardening Dispatch → A6 `START UI SMALL ONLY`
> 性质：**UI 小修整（UI SMALL ONLY）**，零后端命令、零依赖、零策略脚本、零主文档改动。

---

## 0. 本卡边界（A6 自约束，W11 Hard Stops）

| 项 | 值 |
|---|---|
| LANE | A6（M5-W11，Merge Order 未定，归 A0 集成） |
| Allowed Scope（写） | `src/components/workspace/**`、`src/stores/**`、`scripts/check-agent-skill-ui-logic.mjs`、checkpoint/assist |
| 任务 | Agent/Skill UI 小修整：**禁用执行入口**、**确定的空/错误/加载态**、**不新增后端命令** |
| 硬停止（W11） | 仍是 dry-run only；无真实 file/db/script/plugin/agent/skill 执行；无 TCP/HTTP listener/daemon；stdout 响应/日志/审计/UI 状态不回显原始参数/query/token/cookie/Authorization；依赖新增须可选项 + feature-gated，默认构建不变；仅 A0 push |
| 禁止 | 新增后端命令；新增/修改依赖；触碰 `src-tauri/`；改 `pre-merge.sh`；改三份主文档；push |

---

## 1. 现状实测（本卡独立复取，非引用）

| # | 实测项 | 命令/来源 | 结果 |
|---|---|---|---|
| 1 | Agent/Skill UI 逻辑层测试 | `node scripts/check-agent-skill-ui-logic.mjs` | **110 assertions passed, 0 failed** |
| 2 | 前端构建 | `npm run build` | EXIT=0；主 `index` chunk = **164.82 kB**（gzip 59.04 kB），远低于 22% 阈值；Agent/Skill 面板均为独立懒加载 chunk（AgentManagerPanel 2.92 kB、SkillManagerPanel 2.03 kB），无体积回归 |
| 3 | 执行入口现状 | 读 `AgentManagerPanel.vue` / `SkillManagerPanel.vue` / `AgentChatPanel.vue` | **无任何执行按钮**：仅「刷新」按钮且 `:disabled="!store.backendReady"`；ChatPanel 无发送/输入控件；安装/运行路径在 `backendReady=false` 下惰性零 invoke（W10 已锁） |
| 4 | 三态确定性现状 | 读 `src/utils/agentSkillUi.ts::panelState` | `panelState` 已给出 `error > loading > empty(只读壳文案) > ready` 四态；但**两面板模板只渲染 `empty` 与 `list`**，`loading` 态被 `v-else` 吞成空列表，**视觉上与空态不可区分**（确定性缺口） |
| 5 | secret 不回显现状 | 读 `useAgentStore.ts` + `check-agent-skill-ui-logic.mjs` §W9/W10 | 原始输入文本不进 store；校验错误串 / 解析 def 经 `SECRET_REDACT_PATTERNS` 纵深脱敏；测试 W10-9/10/11、W9-12/13/20 全绿 |

**结论**：W11 要求的「禁用执行入口」「secret 不回显」在 W10 已满足；唯一缺口是**加载态无可见渲染**——这正是 W11「确定的加载态」要补的小修整。

---

## 2. 本次改动（仅 2 个面板文件，+6 / -2）

| 文件 | 改动 |
|---|---|
| `src/components/workspace/AgentManagerPanel.vue` | 在错误横幅之后、空态之前插入 `v-if="state.state === 'loading'" class="loading-state"`，把 `v-if empty` 改为 `v-else-if`；补 `.loading-state` 样式 |
| `src/components/workspace/SkillManagerPanel.vue` | 同上对称改动 |

模板语义变为：

```html
<div v-if="state.state === 'loading'" class="loading-state">{{ state.message }}</div>
<div v-else-if="state.state === 'empty'" class="empty-state">{{ state.message }}</div>
<div v-else class="list"> … </div>
```

`state.message` 在 loading 时为 `panelState` 返回的 `"加载中…"`，与 `error` / `empty(只读壳文案)` / `ready` 互斥且确定，彻底消除「加载中显示成空列表」的歧义。

**未触碰**：`AgentChatPanel.vue`（已有 `（等待响应…）` 确定性占位）、`useAgentStore.ts`（零改动，执行路径仍锁）、`scripts/check-agent-skill-ui-logic.mjs`（零改动，仍 110 断言全绿）、`src/utils/agentSkillUi.ts`（零改动）。

---

## 3. 验证证据（本卡实测）

```text
node scripts/check-agent-skill-ui-logic.mjs
  → check-agent-skill-ui-logic: 110 assertions passed, 0 failed

npm run build
  → ✓ built in 3.10s
  → dist/assets/index-BIrezpj4.js  164.82 kB │ gzip: 59.04 kB   （主 chunk，< 22% 阈值）
  → dist/assets/AgentManagerPanel-CVBXvhE5.js  2.92 kB
  → dist/assets/SkillManagerPanel-BvvnlZIQ.js  2.03 kB

read_lints src/components/workspace/AgentManagerPanel.vue
read_lints src/components/workspace/SkillManagerPanel.vue
  → 0 diagnostics

git diff --check -- src/components/workspace/AgentManagerPanel.vue src/components/workspace/SkillManagerPanel.vue
  → 0（无尾随空白 / 合冲突标记）
```

> 共享工作树说明：本工作树仍含 M4 / M5 其它 Lane 的在途产物（A3 database、A4 security_policy、A7 scheduler、A11 验证等），
> 但本卡 `git diff` 经 `git diff -- <本卡两文件>` 隔离，仅含 A6 的 6 行改动，不混入他 Lane 代码。

---

## 4. W11 Hard Stops 合规自审

| 硬停止 | 状态 | 证据 |
|---|---|---|
| dry-run only，无真实执行 | ✅ | 无执行按钮；store 执行动作在 `backendReady=false` 下惰性零 invoke（W10-2/3/4 测试覆盖） |
| 无 TCP/HTTP listener/daemon | ✅ | 纯模板改动，无新增进程/网络代码 |
| 无 secret 回显（stdout/日志/UI 状态） | ✅ | 仅加 `loading-state` 文本节点，不渲染任何参数/query/token；脱敏逻辑未改动且测试全绿 |
| 依赖新增须可选项 + feature-gated | ✅（无依赖改动） | 未改 `package.json` / `Cargo.toml` |
| 默认构建行为不变 | ✅ | `npm run build` 主 chunk 164.82 kB，无回归 |
| 仅 A0 push | ✅ | 本卡未 push，交付补丁 + checkpoint 交 A0 拣入 |

---

## 5. Lane Output Template（A6）

```text
LANE=A6
STATUS=PASS
BASE=5226aad
HEAD=logs/checkpoints/Lane-A6-M5-W11-20260907-1020.patch
FILES=src/components/workspace/AgentManagerPanel.vue, src/components/workspace/SkillManagerPanel.vue
VERIFY=node scripts/check-agent-skill-ui-logic.mjs → 110 passed/0 failed; npm run build → EXIT=0 (index 164.82 kB); read_lints → 0
CHECKPOINT=logs/checkpoints/A6-M5-W11-agent-skill-ui-polish-20260907-1020.md
MERGE_NOTES=变更仅 2 个面板模板（+6/-2），与其它 Lane 文件零交集；无命令/类型/ACL/依赖改动；W11 A6 仅 UI SMALL ONLY，无需 A5/A3 协调；A10 可直接取本补丁做 secret/执行路径复核（预期结论：无新增执行路径、无 secret 回显）。
NEXT=W12（图实时查询 / 插件运行时）由 A7/A9 出实施卡；A6 在 W12 仍属 UI 支持位，待 A0 推进。
```

---

## 6. 明确未做（不冒领）

- 未新增/修改任何后端命令、bridge 封装、types 镜像（W11 明确「no new backend command」）。
- 未改 `src/stores/useAgentStore.ts`（执行路径仍按 W10 口径锁死，本卡仅前端展示层补 loading 态）。
- 未改 `scripts/check-agent-skill-ui-logic.mjs`（W11 仅需其保持 PASS，本卡零改动即满足）。
- 未触碰 `src-tauri/`、`scripts/pre-merge.sh`、三份主文档、ACL。
- 未提交、未 push（指挥板：仅 A0 可向 `master` 提交与推送）。
