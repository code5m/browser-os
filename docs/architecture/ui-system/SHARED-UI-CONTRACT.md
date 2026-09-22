# SHARED UI CONTRACT — Phase UI-3（§12/§13）

> 目标不是组件数量，而是**形成真实规则**。
> 所有规则由 `scripts/check-ui-boundaries.mjs`（UI-01/02/08）与
> `scripts/verify-ui-pilot.mjs`（结构等价）机器校验，不靠自觉。

---

## 1. 目录与公开入口

```
src/shared/ui/
├── index.ts          ← 唯一公开入口（PUBLIC ENTRY）
├── EmptyState.vue
├── ContextMenu.vue
└── ContextMenuItem.vue
```

**规则**
- 外部**只能** `import { X } from "<相对路径>/shared/ui"`（即 index.ts）。
- **禁止**依赖 `shared/ui` 内部文件路径或未来子目录内部实现。
- 目录由**真实迁移需要**驱动，不预建空目录/空组件。

---

## 2. 依赖法（DEPENDENCY LAW，§13）

**允许**
- Vue（含 `withDefaults(defineProps<...>())`、`slot`、attrs 透传）
- 通用 UI 契约与类型
- design tokens / 全局样式类（如 `.empty`、`.ctx-menu`）
- 底层无业务工具函数

**禁止**
- `capabilities/**`（含 `public` 出口）
- 业务 store（`useBrowserStore` / `useGitStore` / `useTaskStore` …）
- 业务 composable
- `bridge` 业务 API、`@tauri-apps/*`、`invoke()`
- 任何具体域：Browser / Terminal / Workspace / Bookmark / Git / Database / Agent / Skill / Plugin

机器校验：UI-01（capability）、UI-02（store）、UI-08（native）。当前均为 **REAL PASS**（非 VACUOUS）。

---

## 3. 命名规则

| 类型 | 命名 | 例 |
|---|---|---|
| 通用 primitive | PascalCase，无域前缀 | `EmptyState`、`ContextMenu` |
| 子部件 | `<Parent><Part>` | `ContextMenuItem` |
| 禁止 | 域前缀 / 业务名 | ❌ `BrowserButton` ❌ `GitDialog` |

出现域前缀 ⇒ 说明它其实属于 CAPABILITY_UI，应移出 shared。

---

## 4. 分层规则

```
TOKEN（全局样式类 / 未来的 CSS 变量）
   ↓
SHARED_PRIMITIVE（无业务语义，纯交互模式）
   ↓
SHARED_PATTERN（无业务语义，但承载一种复合布局/展示模式）
   ↓
WORKBENCH / CAPABILITY_UI（消费者）
```

- shared 层**不知道**任何 Capability。
- 消费者可以是 Workbench 也可以是 Capability —— 二者平等。

---

## 5. 组件 API 质量（§14）

**硬性拒绝**
- God Component：一个组件承担多种不相关形态
- Prop Explosion：出现 `mode` / `businessType` / `feature` / `capability` / `specialCase` / `isBrowser` / `isTerminal` / `isWorkspace` 等业务开关
- 组件内部出现业务 `switch`

**优先**
- composition（slot）优先于配置项
- 小而聚焦的 primitive
- attrs 透传（点击等行为留给消费者），而不是把行为封装进组件

**当前实例**

| 组件 | props | 说明 |
|---|---|---|
| `EmptyState` | `text` / `as` / `live` | 3 个，全部通用渲染参数 |
| `ContextMenu` | `x` / `y` | 2 个坐标；菜单内容走 slot |
| `ContextMenuItem` | `danger` | 1 个通用样式开关；点击走 attrs |

---

## 6. State 所有权规则

| 允许 | 禁止 |
|---|---|
| 受控 props | 业务 state |
| 临时 UI state（如 open 动画态） | `bookmarkPanelOpen` / `browserGridOpen` / `terminalSession` |
| 纯展示 computed | 读 store |

**判定口诀**：删掉这个组件若导致某个**业务功能**失效 ⇒ 它不是 shared。

---

## 7. 无障碍契约（Accessibility）

- 默认**不擅自添加** `role` / `aria-*`：
  迁移必须保持 before/after 结构等价，原有 `role="status" aria-live="polite"` 由 `live` prop **显式开启**，仅当原实现已有时才开。
- 需要新增 a11y 能力时，作为**独立变更**单独记录，不得混在重构里。
- 未来键盘导航等增强以 slot/attrs 透传方式实现，不在 shared 内硬编码业务键位。

---

## 8. 样式 / Token 消费规则

- 当前 shared 组件**不带 `<style>` 块**。
  - 原因：带 style 会产生 `data-v-xxx` 属性，破坏与旧实现的 DOM 等价性。
  - 样式继续由全局类（`.empty` / `.ctx-menu` / `.ctx-item`）提供 → computed style 与迁移前完全一致。
- 这是**刻意的过渡策略**：等 UI-5 建立 Design Token 层后，再统一评估是否改为组件内消费 token。

---

## 9. 测试 / 验证规则

每个 shared 组件必须：

1. **结构等价证明**：`scripts/verify-ui-pilot.mjs` 用真实 Vue SSR 渲染，
   输出 HTML 与**迁移前手写 DOM** 逐字节比对（含 prop 各变体）。
2. **纯净性证明**：无 `capabilities/`、无业务 store、无 `bridge`/`invoke`、无 `<style>` 块。
3. **消费者证明**：≥2 个真实 consumer（Rule of Two），并在 `ui-components.yaml` 的 `consumers` 列出。
4. **门禁**：`node scripts/check-ui-boundaries.mjs` 全绿；`npm run check` 全绿；`npm run build` PASS。

**禁止**：只写组件不写验证；用断言自证而不用真实渲染。

---

## 10. 加入 shared/ui 的流程

1. 证明 ≥2 真实 consumer（附 file:line）
2. 确认无业务 TS 类型依赖（若依赖 `ScriptParam` 等 ⇒ 先做视图模型投影）
3. 组件无 `<style>` 块（或明确记录为何需要）
4. 在 `ui-components.yaml` 登记（`layer` + `consumers` + `status`）
5. `index.ts` 导出
6. 迁移消费者（保留原 `v-if` / 事件 / 文案逐字节不变）
7. 扩展 `verify-ui-pilot.mjs` 的 before/after 用例
8. 跑全套门禁 + build
9. 有人工目视时补 `HUMAN_VISUAL`（本阶段统一 PENDING）

---

## 11. 当前状态

| 项 | 值 |
|---|---|
| shared/ui 组件 | 3（EmptyState / ContextMenu / ContextMenuItem） |
| 真实 consumer 文件 | 14 |
| 迁移实例数 | 13（EmptyState）+ 14（ContextMenu 系） |
| UI-01 / UI-02 / UI-08 | **REAL PASS**（VACUOUS = 0） |
| 结构等价证明 | 9/9 SSR 用例 PASS |

## 12. 明确不迁移（记录，避免重复讨论）

| 候选 | 判定 | 原因 |
|---|---|---|
| `ModalShell` | 暂缓 | `.modal-mask` 存在**冲突的 scoped 定义**（z-index 999 vs 50、bg .4 vs .35），统一将改变视觉 → 违反 VISUAL_EQUIVALENCE。冲突调查移交 UI-5（§24）。 |
| `FilterInput` | 不迁 | 仅 2 处且 margin 不同（`margin-bottom:8px` vs `margin:4px 0`）；单元素抽象收益 < 引入 variant prop 的成本。 |
| `PanelHeader` | 不迁 | 5+ 处视觉差异明显（`.ai-head` / `.ftree-head` / `.git-col-head` 结构不同），属 VISUALLY_SIMILAR_BUT_SEMANTICALLY_DIFFERENT。 |
| 三套 Tabs | **DO_NOT_MERGE** | 生命周期根本不同（浏览器页签关 = 销毁 webview）。 |
| `ConfirmModal` 泛化 | **DO_NOT_MERGE** | 实为 Git 专属，泛化会 prop explosion。 |
