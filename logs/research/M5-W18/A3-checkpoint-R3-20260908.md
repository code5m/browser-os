# Lane A3 — M5-W18-R3B Checkpoint

时间：2026-09-08（R3B 修订）；模式 `RESEARCH_AND_PROTOTYPE`；NEXT `M5-W18-R3B`；`W19=CLOSED`。
状态：**DELIVERED（本 lane 提交，未 push，仅 A0 可 push）**。
依据：`logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`（REVISE_TARGETED）+ `M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A3。

## 1. 启动门禁（逐条实跑）

| 检查 | 结果 |
|---|---|
| 读取 `WORKSPACE_IDENTITY.md` / `PARALLEL_COMMAND_BOARD.md` / `M5-W18-R3B-CORRECTION-TASKS-20260908.md` | 完成；当前派发入口 = R3B 卡 §A3 |
| 推导本 lane WORKDIR | `/home/ainfinit/.codex/worktrees/m5-w18-a3/mvp-browser-os-v3`（符合 `m5-w18-aN` 模式，N=3） |
| 分支 | `codex/m5-w18-a3`（与 lane id 严格一致） |
| rebase | `git fetch origin && git rebase origin/master` → **Successfully rebased**（本 lane 旧 5 个 R3 提交已被 A0 集成入 master，rebase 时 skipped，属预期）；现 HEAD 基于最新 `origin/master` |
| 工作树是否含他 lane 改动 | 否；rebase 后 `git status --short` 干净 |
| 是否 push | **否** |

## 2. 卡片交付核对（A3 — Collapse/restore and tree semantics）

| R3B 卡要求 | 落地位置 |
|---|---|
| 折叠/恢复对齐 A0：Collapse All 隐藏所有工具窗口（含固定） | 规格 §4 CR-4/CR-7、§9 Q2 闭合；`.mjs` 段 7.5 断言「collapse all closes every tool window, including pinned」 |
| 恢复由唯一快照回放可见性/尺寸/固定态 | `restorePrevious()` 回放 `pinned`；段 7.5 断言 `restore restores pin state` |
| 固定仅影响「替换」与「窄窗自动隐藏」，不影响收起 | CR-7（段 7.5 追加 `pin blocks ordinary replacement` 非收起断言） |
| 树折叠/展开语义与工具窗口折叠分离 | 规格 §6（树行为独立冻结）+ 模型 `treeCollapseAll/ExpandOneLevel/BoundedExpandAll` + 段 7.11–7.14 |
| 补 900×600（产品最小） | 规格 §0 六尺寸表；`.mjs` 自测循环 + 原型尺寸下拉（800×600 已弃） |
| 补键盘焦点断言（R3B-04） | `.mjs` 段 7.15：开/关/收起焦点回文档、固定不抢焦点、树选中为焦点锚、三套 keymap 全可达 |
| 移除原型 donor 品牌（R3B-02） | 原型 HTML 页签/文档标题「Rebased」字样移除；行为参考仅留报告叙述 |
| 几何 SSOT（R3B-05） | 顶栏 60（2×30）/状态 24/活动条 28，inner viewport 不含 OS 标题栏；`collapsedShares` 改 A0 公式 |

## 3. 验证结果

```
$ node logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs
A3-R3 tool-window state machine self-test: PASS (73 assertions)
collapsed-mode budget -> 1920x1080: 98.5% w / 92.2% h | 1440x900: 98.1% w / 90.7% h
  | 1366x768: 98% w / 89.1% h | 1200x800: 97.7% w / 89.5% h
  | 1024x720: 97.3% w / 88.3% h | 900x600: 96.9% w / 86% h   (全部 >=92% 宽 / >=85% 高 PASS)

$ node --check <原型内联脚本>        # 语法 OK（vm.compileFunction 校验通过）
JS_SYNTAX_OK

$ git status --short --branch        # 提交后干净
## codex/m5-w18-a3...origin/master [ahead 1]
```

未跑 `npm run build` / `pre-merge.sh`：本波零产品代码改动，且蓝图 §7 明确"本轮仅文档/合成研究资产，不跑 11 次全量构建"。

## 4. 本 lane 提交

| SHA | 说明 |
|---|---|
| `b8470f3`（R3B） | `research(M5-W18-R3B,A3): align collapse/pin geometry to A0 ruling, add 900x600 + keyboard-focus, drop donor branding` |
| `96bc45a` 等 5 个 R3 提交 | 已被 A0 集成入 `origin/master`（rebase 时 skipped） |

集成基线：最新 `origin/master`（rebase 后）。HEAD = `b8470f3`。

产物：

- `logs/research/M5-W18/A3-R3-toolwindow-disclosure-20260908.md`（规格，单一真源，含 §12 R3B 修订账）
- `logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs`（纯函数模型 + 73 断言自测）
- `logs/research/M5-W18/A3-R3-toolwindow-prototype.html`（自包含合成原型，无 donor 品牌）
- `logs/research/M5-W18/A3-checkpoint-R3-20260908.md`（本文件）

## 5. 变更清单（仅 lane 自有文件，零产品代码）

```
logs/research/M5-W18/A3-R3-toolwindow-disclosure-20260908.md   (modified)
logs/research/M5-W18/A3-R3-toolwindow-state-machine.mjs        (modified)
logs/research/M5-W18/A3-R3-toolwindow-prototype.html           (modified)
logs/research/M5-W18/A3-checkpoint-R3-20260908.md              (modified)
```

未触碰：`src/**`、`src-tauri/**`、`scripts/**`、`capabilities/**`、他 lane 的 `logs/**`、`PARALLEL_COMMAND_BOARD.md`、`WORKSPACE_IDENTITY.md`、三份主文档。

## 6. 依赖与挂账（交 A0 / 相关 lane）

| 编号 | 内容 | 对象 |
|---|---|---|
| D-A3-1 | 默认边尺寸（左 260 / 底 240）需与密度审计对齐 | A2（R3B §A2 几何） |
| D-A3-2 | 持久化键名、`schema_version`、原子写落点；A3 只声明字段集与不落盘红线 | A4（R3B §A4，须对齐 CR-7 折叠语义） |
| D-A3-3 | `compactMode`（浏览器沉浸）与专注模式是否合并 | A1 |
| D-A3-4 | 右键动作命令身份/禁用原因/安全等级（A3 原型仅示范形态） | A6 |
| D-A3-5 | Git log 落点默认取值；A3 只规定换边可逆 | A1 / A7 |
| D-A3-6 | 快捷键 `Ctrl+Alt+=` 双重语义（展开活动 vs 树展开一级） | A0 / A6（注意 R3B-06 已冻结 Ctrl+K / Ctrl+Shift+P 归属搜索/命令面板，与本款不冲突） |
| D-A3-7 | ~~全部收起是否跳过固定窗口~~ **已由 A0 R3B 裁决闭合**（含固定一并收起） | — |
| D-A3-8 | 浮动/脱离窗口：A3 建议本轮 REJECT | A0 |
| D-A3-9 | `scripts/check-toolwindow-logic.mjs`（17 条建议断言，含 TW_COLLAPSE_HIDES_PINNED / TW_KEYBOARD_FOCUS）何时落地 | A0（W19 开片后） |
| D-A3-10 | lane 工作树内 `.workspace-identity` 文案仍为主仓文案 | A0 |

## 7. 声明

- 未改产品代码、依赖、ACL、capability、原生运行时、用户数据。
- 未 push；未改他 lane 文件；未改共享主文档。
- 原型与模型为合成数据、自包含、无网络、无原生调用；原型 UI 已无 donor 品牌（R3B-02），行为参考仅保留在报告叙述中。
- W19 仍 `CLOSED`；本 lane 不自动开启任何编码片。
