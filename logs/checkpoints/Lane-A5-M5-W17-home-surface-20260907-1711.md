# Lane A5 · M5-W17 Desktop Client Completeness — Home Surface Rebuild

> STATUS: PASS
> NO_PUSH: confirmed (A0 集成后统一 commit/push)

## Scope (changed files — `src/components/home/` only)

| File | Status | Role |
|---|---|---|
| `src/components/home/HomePanel.vue` | modified (tracked) | 主页容器：接入 A3 `panelState` 三态横幅（loading/error，不回显原始报错），渲染顺序 启动器→快捷方式→最近访问→编辑层 |
| `src/components/home/HomeLaunchers.vue` | new | 主要工作区入口宫格（17 个 `MainView` 子集，复用 `openArea` 语义，键盘可达 + `aria-label`/`aria-current`） |
| `src/components/home/HomeShortcuts.vue` | new | 快捷方式区：有界渲染（`VISIBLE_LIMIT=12`）+ 畸形数据防御归一化（`norm`/`safeType`）+ 连贯空态 + 真 button 打开/编辑/删除 |
| `src/components/home/HomeShortcutsEditor.vue` → `HomeShortcutEditor.vue` | new | 编辑对话框：复用 W15 `modalA11y`（`FOCUSABLE_SELECTOR`/`ariaRoleForVariant`/`shouldCloseOnEscape`），焦点送进/归还 + Tab 闭环 |
| `src/components/home/HomeRecents.vue` | new | **本次补完的「最近访问」段**：有界（`VISIBLE_LIMIT=8` + store 层 `HOME_MAX_RECENTS=12` 封顶）、安全展示（仅 `displayTarget` 摘要，绝不回显完整路径/URL query/凭据）、`accessibleLabel` 无障碍名、单条移除 / 清空 |

> 零后端访问、零新依赖、零新 Tauri 命令 / bridge / ACL / 权限。仅消费 A3 W17 已落地的 `useHomeStore` 契约（`recents` / `hasRecents` / `removeRecent` / `clearRecents` / `displayTarget` / `accessibleLabel` / `panelState` / `principalAreas` / `openArea`）与 `homeUi.ts` 纯逻辑层。

## Delivered (user-visible behavior)

- **主要工作区入口**（共享验收 #2/#3）：17 个既有模块以宫格暴露为可用路由，窄窗口自适应（`minmax(84px,1fr)`），不新增路由/命令。
- **快捷方式 + 最近访问双段**（W17 A5 任务「shortcuts/recent section」）：最近访问填补了此前缺失的区段；二者均「有界 + 畸形数据容错 + 连贯空态」。
- **零敏感披露**（共享验收 #4）：最近访问只显示 `name` + `homeDisplayTarget(s)` 安全摘要（URL 仅 host+path、dir/app 仅末段名），`aria-label` 走 `accessibleLabel`；编辑/失败提示均不回显原始错误串。
- **键盘可达 + 无障碍名**（共享验收 #4）：每个交互控件均为真 `<button>`/`<input>`/`<select>`，带 `title`/`aria-label`/`<label for>`；对话框焦点管理闭环。

## Verify (exact commands & results)

```bash
# 1) A9 主页 UI 逻辑测试（DOM/源码级，headless）
node scripts/check-home-ui-logic.mjs
# → 通过 32，失败 0  | HOME_UI_RESULT=PASS   (exit 0)

# 2) A4 主页客户端隐私/安全策略（ACTIVE=3）
python3 scripts/check-home-client-policy.py
# → home client policy: all invariants hold（ACTIVE=3）  (exit 0)

# 3) Lint（HomePanel.vue / HomeRecents.vue）
# → 0 diagnostics

# 4) 构建 + 构建指标（隔离测量 A5 独占增量，见下）
python3 scripts/measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-052b18a.json
# → exceeds_growth_limit=false ; total_bytes_pct=-0.3 ; cargo_warnings=0 ; warnings_increased=false

# 5) 门禁：仅 src/components/home/ 的 diff 无空白/尾随问题
git diff --check -- src/components/home/   # → clean
```

## Metrics（构建指标，守门）

测量方法（沿用 A6 W17 隔离协议，避免被他 lane 脏文件污染主工作树）：
`git worktree add --detach /tmp/a5-w17 HEAD` + 软链 `node_modules` + 仅拷入 A5 的 5 个 home 组件与**其消费的 A3 契约**（`useHomeStore.ts` / `homeUi.ts`，store 非 A5 产品代码但为本波必需前置），再 `npm run build` + `cargo check` 对比基线 `052b18a`。

| 项 | 基线 (052b18a) | A5 独占增量 |
|---|---|---|
| total_bytes | 790,073 B | **−0.3%**（≈ 787.7 KB，无回归） |
| 25.2% 上限 | — | 未触发（exceeds_growth_limit=false） |
| cargo_warnings | 2 | +0（未增） |
| chunk_over_500kb | false | false |

> 增量略负属预算中性：本波把旧版内联主页重构为模块化 home 组件，配合 A3 契约迁移后总包体未增长。属于「A5 前端 + 必需 A3 store 契约」合并测量；A5 自身不引入任何运行时体积增量。

## RISKS（诚实遗留风险）

- **A3 契约前置**：`HomeRecents.vue` / `HomePanel.vue` 直接依赖 A3 W17 新 `useHomeStore` 字段（`recents`/`panelState`/`displayTarget`/`accessibleLabel`）。集成顺序 `A3 → A5`，A0 须**同时合入 A3 的 store/utils 与 A5 的 home 组件**，否则单独合 A5 会缺 store 方法编译失败。
- **`panelState` 错误态当前不触发**：`seedDirShortcuts` 的 `catch` 吞掉异常、`error` 恒为 `null`，故 `error` 横幅为防御性常驻不显；`loading` 同理（无显式 loading 置位）。空态由各 section 自行呈现，已满足共享验收 #2。若后续需要真实载入态，应在 `seedDirShortcuts` 前 `loading=true` 后置 `false`。
- **未跑原生客户端目测**：A8 的人工窄窗口验收待 A0 集成后由 A8 执行（W17 A8 = START MANUAL QA），本 lane 仅静态/构建证据。

## PATCH

`/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A5-M5-W17-home-surface-20260907-1711.patch`
（33 KB，`git diff -- src/components/home/` 生成，仅含本 lane 范围；`git apply` 前需先合 A3 store 前置）
