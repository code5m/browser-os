# UI COMPONENT AI GUIDE — AI 可维护性指南（§18）

> 目标：普通 AI Agent 修改 `Browser Omnibox` 时，主要只需读 `browser/ui` + shared/ui public contract，
> 不需要理解 Terminal / Workspace / Git / Database / App.vue 的全部历史。

---

## 1. 如何判断一个组件属于哪一层

按序回答（命中即停）：

1. **有 Browser / Terminal / Git / Database / Agent / Skill / Plugin / 文件域 业务语义？**
   → `CAPABILITY_UI`。放 `src/capabilities/<id>/ui/`。
2. **是产品工作台本身的通用容器/框架？**（导航、Dock、Toolbar、StatusBar、Home、Settings、命令面板、window chrome、贡献宿主）
   → `WORKBENCH`。放 `src/components/layout|home|system/`（未来 `src/framework/workbench/`）。
3. **无业务语义、只是交互模式？**（Button / Dialog / Tabs / EmptyState / Tree / SplitPane）
   → `SHARED_PRIMITIVE` 或 `SHARED_PATTERN`。放 `src/shared/ui/`。
4. **纯数值？**（色值 / 间距 / 圆角 / z-index）→ `TOKEN`。

**反例（禁止）**：
- ❌ 把 `Omnibox` 塞进 `shared/ui`（它有 URL/目录识别业务语义）
- ❌ 把 `Button` 注册成 Capability
- ❌ 把 Git 确认弹窗"泛化"成通用 ConfirmDialog

---

## 2. 什么时候**禁止**抽象

出现以下任一，**停止抽象**：

- 需要 `type` / `mode` / `variant` / `businessType` / `capability` / `feature` / `specialCase` 这类 prop
- props 超过 ~8 个，或出现大量业务 `switch`
- 组件开始需要理解 webview 生命周期 / PTY / Git 仓库 / DB 连接
- 只有 1 个消费者（**RULE OF TWO**：至少 2 个真实消费者才抽象）
- 只是"看起来像"，但 `SAME_LIFECYCLE` 不同
  > 例：浏览器页签（关 = 销毁 webview）vs 模块页签（关 = 切视图）→ **DO_NOT_MERGE**

---

## 3. 新增 shared component 的流程

1. 确认 ≥2 个真实消费者（引用 file:line）
2. 确认无业务 TS 类型依赖（如需 `ScriptParam`/`SkillDef` → 先做**视图模型投影**）
3. 组件**不得** import：`capabilities/**`、`src/stores/**` 业务 store、`bridge`、`@tauri-apps/*`
4. 组件**只能**拥有临时 UI state（如 open 动画态），**不得**拥有 `bookmarkPanelOpen` / `browserGridOpen` / `terminalSession`
5. 在 `ui-components.yaml` 登记（`layer` + `status: CANDIDATE`）
6. 跑 `node scripts/check-ui-boundaries.mjs`（UI-01/02/08 会自动生效）
7. 视觉等价验证（见 §5）

---

## 4. 如何查 consumers

```bash
# 查谁渲染了某组件
grep -rn "ComponentName" src --include=*.vue --include=*.ts

# 查机器可读 catalog 里的登记（含 layer / owner / consumers）
grep -n "id: <component-id>" docs/architecture/ui-system/ui-components.yaml
```

`ui-components.yaml` 每条含 `dependencies` 与 `consumers`，是**权威 consumer map**。

---

## 5. 如何做视觉回归

**诚实区分**：`STRUCTURAL_UI_PASS` ≠ `HUMAN_VISUAL_PASS`

| 手段 | 能证明 | 不能证明 |
|---|---|---|
| 首屏 DOM 文本探针 | 结构完整、无白屏、入口在位 | 像素一致 |
| computed style 快照 | 关键尺寸/色值未变 | 整体观感 |
| catalog 一致性 | 组件未丢失 | 视觉 |
| **人工 GUI 验收** | 真实视觉 + 交互 | — |

**迁移后必做**：
1. 重跑首屏探针，与 `UI-PRESERVATION-BASELINE.md §3` 逐字比对
2. 对照 `§2 必须保留的交互契约` 勾选
3. **人工目视确认**（默认判 REGRESSION 的情形见 baseline §4）

---

## 6. 如何避免业务 state 泄入 shared/ui

| 允许 | 禁止 |
|---|---|
| `const open = ref(false)`（临时动画态） | `bookmarkPanelOpen` |
| `const hover = ref(false)` | `browserGridOpen` |
| 受控 props + emit | 直接读 `useBrowserStore()` |
| 纯展示 computed | `bridge.*` / `invoke()` |

**判定口诀**：如果删掉这个组件会导致某个**业务功能**失效 → 它就不是 shared。

---

## 7. 门禁速查

```bash
node scripts/check-ui-boundaries.mjs              # 全量 10 条
node scripts/check-ui-boundaries.mjs --self-test  # 自检（含 positive/negative fixture）
```

| Gate | 含义 | 当前 |
|---|---|---|
| UI-01/02/08 | shared/ui 纯净性 | VACUOUS（shared/ui 未创建） |
| UI-03/03b | Workbench 不得直渲业务（22 处基线） | PASS |
| UI-04/04b | 能力间不得互相 import 内部 | PASS（1 处未声明依赖 WARN） |
| UI-05 | 禁止新增 position:fixed 浮层（12 处基线） | PASS |
| UI-06 | 禁止 toast-pop 等反模式 | PASS |
| UI-07 | App.vue 不得新增 native 命令（17 个基线） | PASS |
| UI-09 | 贡献宿主不得存业务真值（24 视图基线） | PASS |
| UI-10 | catalog 与真实文件一致（65/65） | PASS |

**baseline 更新规则**：消除一处债务 → 必须同步更新 `ui-boundary-baseline.json` 并在 commit 说明原因。
门禁对「基线与现实不符」同样 FAIL —— **不允许静默洗绿**。

---

## 8. 修改某能力 UI 时的最小阅读集

| 你要改 | 主要读 | 不需要读 |
|---|---|---|
| Browser Omnibox | `capabilities/browser/` + `contribution/types.ts` | Terminal / Git / DB |
| Terminal 面板 | `capabilities/terminal/ui/` + `terminal/resource/guard.ts` | Browser 宫格 |
| 文件面板 | `capabilities/workspace/ui/FilePanel.vue` | PluginManager |
| 一个 Button | `shared/ui/` + consumer map（§4） | 任何能力 |

**前提**：先查 `ui-components.yaml` 的 `owner` 与 `layer`，再进入对应目录。
