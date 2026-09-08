# Lane A3 — M5-W18-R3 Checkpoint

时间：2026-09-08 13:06 CST；模式 `RESEARCH_AND_PROTOTYPE`；NEXT `M5-W18-R3-UX`；`W19=CLOSED`。
状态：**DELIVERED（本 lane 提交，未 push，仅 A0 可 push）**。

## 1. 启动门禁（逐条实跑）

| 检查 | 结果 |
|---|---|
| 读取 `WORKSPACE_IDENTITY.md` / `PARALLEL_COMMAND_BOARD.md` / `M5-W18-R3-UX-TASKS-20260908.md` | 完成；当前派发入口 = R3 卡（board L8-16、§Current Dispatch Entry） |
| 推导本 lane WORKDIR | `/home/ainfinit/.codex/worktrees/m5-w18-a3/mvp-browser-os-v3`（符合 `m5-w18-aN` 模式，N=3） |
| 分支 | `codex/m5-w18-a3`（与 lane id 严格一致） |
| rebase | `git fetch origin && git rebase origin/master` → **Successfully rebased**；原 3 个 R/R2/R2B 提交因已被 A0 集成入 master 而 skipped（提示属预期），现 HEAD 基于 `200f0f1` |
| 工作树是否含他 lane 改动 | 否；rebase 后 `git status --short` 干净 |
| 是否 push | **否** |

补充：`WORKSPACE_IDENTITY.md` 的 `.workspace-identity` 文件在派生的 lane 工作树里仍是 `BACKV3_MAIN` 文案（工作树由主仓复制而来）。本次按身份文件 §Historical W18 lane exception 判定：路径/分支/lane id 三者严格匹配 R3 卡，故合法；该文案差异已记录，交 A0 决定是否在各 lane 工作树内修正。

## 2. 卡片交付核对（A3 - IDEA progressive-disclosure behavior）

| 卡片要求 | 落地位置 |
|---|---|
| 工具窗口边 | 规格 §2（left/right/bottom，无浮动窗口），模型 `EDGES` |
| 全部收起 / 恢复 | CR-4 / CR-5 + `collapseAll()` / `restorePrevious()` |
| 最大化（展开活动） | CR-6 + `expandActive()` |
| 固定 / 自动隐藏 | CR-7 / CR-8 + `togglePin()` / `toggleAutoHide()` |
| 焦点返回 | CR-10 + §5 焦点模型 + `focusTo()` |
| 分隔条限制 | §3 + `clampSize()` / `resizeEdge()`（含触限反馈） |
| 键盘焦点 | §5（含三套 keymap 提案） |
| 持久化 | §7（含与 A4 的边界、迁移映射） |
| 树：全部折叠 / 展开一级 / 有界展开全部 | §6 + `treeCollapseAll()` / `treeExpandOneLevel()` / `treeBoundedExpandAll()` |
| 确定性状态转移与冲突规则 | §4 表 CR-1…CR-12（全部为纯函数） |
| 不复制 IDEA 资产或产品代码 | §11；零产品文件改动（见 §5 变更清单） |

附带满足的全局验收：顶栏 ≤2 行 ≤80px（§2）；收起态四尺寸均 ≥85% 高 / ≥92% 宽（§0 表）；右键菜单按对象作用域组织并带禁用原因（§8，所有权归 A6）。

## 3. 验证结果

```
$ node logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs
A3-R3 tool-window state machine self-test: PASS (58 assertions)
collapsed-mode budget -> 1920x1080: content 100% w / 93.7% h | 1440x900: 100% w / 92.4% h
                        | 1366x768: 100% w / 91% h | 1024x720: 100% w / 90.4% h

$ node --check <原型内联脚本>        # 473 行
JS_SYNTAX_OK

$ git status --short --branch        # 提交后干净
## codex/m5-w18-a3...origin/master [ahead 3]   (2 提交：研究产物 + 本 checkpoint)
```

未跑 `npm run build` / `pre-merge.sh`：本波零产品代码改动，且规格明确"本轮仅文档/合成研究资产，不跑 11 次全量构建"（`WORKBENCH_BLUEPRINT-20260908.md` §7）。

## 4. 本 lane 提交

| SHA | 说明 |
|---|---|
| `96bc45a` | `research(M5-W18-R3,A3): tool-window progressive disclosure spec, state machine and prototype` |
| （本文件提交后追加） | `research(M5-W18-R3,A3): lane checkpoint` |

产物：

- `logs/research/M5-W18/A3-R3-toolwindow-disclosure-20260908.md`（规格，单一真源）
- `logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs`（纯函数模型 + 58 断言自测）
- `logs/research/M5-W18/A3-R3-toolwindow-prototype.html`（自包含合成原型）
- `logs/research/M5-W18/A3-checkpoint-R3-20260908.md`（本文件）
- `logs/checkpoints/Lane-A3-M5-W18-R3-toolwindow-20260908-1306.patch`（给 A0 的补丁）

## 5. 变更清单（仅 lane 自有文件，零产品代码）

```
logs/research/M5-W18/A3-R3-toolwindow-disclosure-20260908.md   (new)
logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs        (new)
logs/research/M5-W18/A3-R3-toolwindow-prototype.html           (new)
logs/research/M5-W18/A3-checkpoint-R3-20260908.md              (new)
logs/checkpoints/Lane-A3-M5-W18-R3-toolwindow-20260908-1306.patch (new)
```

未触碰：`src/**`、`src-tauri/**`、`scripts/**`、`src-tauri/permissions/**`、`capabilities/**`、他 lane 的 `logs/**`、`PARALLEL_COMMAND_BOARD.md`、`WORKSPACE_IDENTITY.md`、三份主文档。

## 6. 依赖与挂账（交 A0 / 相关 lane）

| 编号 | 内容 | 对象 |
|---|---|---|
| D-A3-1 | 默认边尺寸（左 260 / 底 240）需与密度审计对齐 | A2 |
| D-A3-2 | 持久化键名、`schema_version`、原子写落点由布局/文档状态模型统一；A3 只声明字段集与不落盘红线 | A4 |
| D-A3-3 | `compactMode`（浏览器沉浸）与专注模式是否合并 | A1 |
| D-A3-4 | 右键动作的命令身份/禁用原因/安全等级（A3 原型仅示范形态） | A6 |
| D-A3-5 | Git log 落点（中央编辑器 vs 底部工具窗口）默认取值；A3 只规定换边可逆 | A1 / A7 |
| D-A3-6 | 快捷键 `Ctrl+Alt+=` 双重语义（展开活动 vs 树展开一级）是否接受 | A0 / A6 |
| D-A3-7 | 全部收起是否跳过固定窗口（A3 取"一起收起 + 一键恢复"） | A10 / A11 |
| D-A3-8 | 浮动/脱离窗口：A3 建议本轮 REJECT（子 webview 位置方案脆弱） | A0 |
| D-A3-9 | `scripts/check-toolwindow-logic.mjs`（15 条建议断言）何时落地 | A0（W19 开片后） |
| D-A3-10 | lane 工作树内 `.workspace-identity` 文案仍为主仓文案 | A0 |

## 7. 声明

- 未改产品代码、依赖、ACL、capability、原生运行时、用户数据。
- 未 push；未改他 lane 文件；未改共享主文档。
- 原型与模型为合成数据、自包含、无网络、无原生调用；未引用 Rebased/IDEA/JetBrains 任何源码或资产（许可证与 fork 边界归 A7/A10）。
- W19 仍 `CLOSED`；本 lane 不自动开启任何编码片。
