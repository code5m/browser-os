# Lane A6 · M5-W17 客户端导航可达性（Desktop Client Completeness）交付

- 时间：2026-09-07 16:58 CST
- 基线与分支：`master` @ `052b18a`（W15 release readiness，PUSHED），工作目录 `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
- 派发表：`PARALLEL_COMMAND_BOARD.md` §M5-W17 Desktop Client Completeness and Home Recovery Dispatch
- Lane 指派原文：`A6 | START CODE | src/components/layout/ActivityBar.vue, src/stores/useLayoutStore.ts, scripts/check-client-navigation-logic.mjs | Make existing modules discoverable and keyboard-accessible with stable active state/narrow-window behavior. Do not edit home components, MainArea.vue, or backend files.`

```text
LANE: A6
STATUS: PASS_WITH_DEBT（功能与门禁全绿；构建指标超当前预算，见 METRICS，需 A0 裁决）
SCOPE: src/stores/useLayoutStore.ts（改）, src/components/layout/ActivityBar.vue（改）, scripts/check-client-navigation-logic.mjs（新增）
DELIVERED: 活动条导航真源上收 + 无孤儿模块 + 窄窗口裁剪不丢入口 + 键盘漫游/Esc + 全按钮可访问名称 + 最近项展示脱敏
VERIFY: node scripts/check-client-navigation-logic.mjs → CLIENT_NAV_RESULT=PASS (60/60)
        npm run build → ✓ built（含其它 Lane 并发改动）
        干净 worktree（HEAD+仅 A6 补丁）npx vite build → 770,725B（HEAD 基线 767,059B）
        node scripts/check-ui-a11y-logic.mjs → UI_A11Y_RESULT=PASS
        node scripts/check-session-logic.mjs → SESSION_LOGIC_RESULT=ALL_PASS（34 断言）
        git diff --check → 干净
METRICS: A6 独占增量 +3,666B（+0.598pp）；HEAD+A6 = 770,725B = 25.742%，超 25.2% 上限 3,320B
PATCH: /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3/logs/checkpoints/Lane-A6-M5-W17-client-navigation-20260907-1658.patch
RISKS: ① 超指标预算（需 A0 抬限或裁我）；② 共享工作树被 A3/A5/A7 并发写入，主树构建数字已污染不可直接用；③ 键盘漫游只覆盖活动条一级按钮，扩展行内仍为 Tab 顺序；④ 未做真机/窄窗口人工验收（属 A8）
NO_PUSH: confirmed
```

## 1. 做了什么

### 1.1 `src/stores/useLayoutStore.ts`（导航真源 + 纯函数契约）

新增模块级导出（零后端依赖，可在 Node 下直接加载测试）：

| 导出 | 作用 |
|---|---|
| `TOP_NAV_ITEMS` | 一级入口（主页/浏览/终端/剪贴板/知识库），顺序即窄窗口裁剪优先级 |
| `NAV_MENU_SECTIONS` | ☰ 菜单分节（工作区/工具/同步），**新增剪贴板、知识库、终端**三项 |
| `MODULE_META` | 原 `MOD_META` 字面量上提为模块级导出（store 内 `const MOD_META = MODULE_META`，行为不变） |
| `navDensityForWidth(w)` / `NAV_DENSITY_FULL_PX=1180` / `NAV_DENSITY_COMPACT_PX=900` | 窗口密度 `full / compact / icon`，非法输入（NaN/0）一律落 `icon`，不抛异常 |
| `navTopViewsForWidth(w)` | 窄窗口裁剪：full 全显 → compact 前 3（主页/浏览/终端）→ icon 前 2（主页/浏览） |
| `isNavActive(mainView, view)` | 激活态唯一判定（严格相等），消除模板里多处各写一份 `===` 的漂移 |
| `nextNavIndex(cur, delta, len)` | ←/→ 环绕漫游索引；`len<=0` 返回 0，越界索引归一 |

Store 内新增：`windowWidth`（钳位 320–4096）、`navSection`（`''|grid|more|omni`，互斥）、派生 `navDensity`/`navTopViews`，动作 `setWindowWidth / toggleNavSection / closeNavSection`。
`setView()` 内统一 `navSection = ''`：视图一切换就收起扩展行，杜绝"换了视图还挂着上一视图菜单"的状态分裂。

### 1.2 `src/components/layout/ActivityBar.vue`

- 消费 store 真源：删除本地重复的 `topItems`/`menuSections` 字面量，一级入口按 `layout.navTopViews` 过滤；扩展行状态改用 `layout.navSection`。
- 键盘可达：`<nav aria-label="主导航" @keydown="onNavKeydown">`，`←/→` 在 `[data-nav-item]` 间环绕、`Home/End` 直达首尾；`Esc` 收起扩展行并把焦点还给触发按钮（`data-nav-toggle`）。
- 可访问名称：为所有此前只靠图标/emoji 的按钮补 `aria-label`（宫格、后退/前进/刷新、AI 导航、采集、系统设置、宫格模式、网址/资源、各格网址、三处 ✕ 收起、历史/常用开关）；触发按钮补 `aria-expanded` + `aria-controls`，扩展行补 `id="nav-{grid,more,omni}-row"`。
- 窄窗口：根节点 `:class="'nav-' + layout.navDensity"`；`compact` 先收 `.lab` 文字、`icon` 再收次要按钮（前往键/地址栏字号）；`mounted` 上报 `window.innerWidth`，`resize` 监听在 `onBeforeUnmount` 清理（与既有资源轮询定时器一起）。
- 隐私：最近网址/目录展示走既有 `redactSecrets`（`safeLabel`），点击仍用原始值；避免把带 `token/access_token` 的查询串原样贴在活动条上。

### 1.3 `scripts/check-client-navigation-logic.mjs`（新增，60 断言）

加载**真实** `useLayoutStore.ts`（真实 pinia 实例，不 mock store 逻辑）+ 对**真实** `ActivityBar.vue` 做源码级断言，覆盖 7 组：

1. 导航真源完整性（含无孤儿模块：从源码正则解析 `MainView` 联合类型，逐个校验有入口）
2. 窄窗口裁剪不丢功能（10 个宽度 × 每个被裁入口必须能在 ☰ 菜单找到）
3. 窗口密度边界（1180/1179/900/899/0/NaN）
4. 激活态判定（含 `editor` 覆盖层不误激活）
5. 键盘漫游索引（环绕、单元素、非法长度）
6. Store 行为（钳位、扩展行互斥、切视图自动收起、关闭末个页签回落主页）
7. ActivityBar 源码约束（可访问名称、Esc、无 raw `invoke(`、无新依赖、无敏感字面量、未触碰 home 组件/`MainArea`、store 不依赖 bridge）

> 该脚本自身抓到一个真实缺陷：icon 密度下"终端"被裁掉但 ☰ 菜单里没有入口 —— 已在 `NAV_MENU_SECTIONS` 工具节补 `{ view: "term" }` 修正，现全绿。

## 2. 验证（命令 → 结果）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
node scripts/check-client-navigation-logic.mjs
# CLIENT_NAV_RESULT=PASS (60/60)          exit 0

node scripts/check-ui-a11y-logic.mjs      # UI_A11Y_RESULT=PASS
node scripts/check-session-logic.mjs      # SESSION_LOGIC_RESULT=ALL_PASS（34 断言）
npm run build                             # ✓ built in 3.24s（含其它 Lane 并发改动）
git diff --check                          # 无输出（干净）
```

干净 worktree 对照（隔离 A6 独占增量，排除其它 Lane 污染）：

```bash
git worktree add --detach /tmp/w17-a6-wt HEAD     # 052b18a
ln -s <repo>/node_modules /tmp/w17-a6-wt/node_modules
cd /tmp/w17-a6-wt && npx vite build && du -sb dist    # 767,059  ← 与 board 记录的 052b18a=767,059 完全一致
cp <A6 两文件> ... && npx vite build && du -sb dist   # 770,725  → A6 独占增量 +3,666B
git worktree remove --force /tmp/w17-a6-wt            # 已清理，git worktree list 仅剩既有 2 个 codex worktree
```

## 3. 构建指标（METRICS，需 A0 裁决）

| 项 | 字节 | 相对基线 612,943B |
|---|---|---|
| 基线 `4f0e8ab` | 612,943 | — |
| 上限 25.2% 允许总量 | 767,405 | 25.200% |
| HEAD `052b18a` 实测 | 767,059 | 25.144%（余量仅 346B） |
| HEAD + A6 补丁 | 770,725 | **25.742%（超上限 3,320B）** |
| **A6 独占增量** | **+3,666** | **+0.598pp** |

- `cargo_warnings` 未增加（本 Lane 零 Rust 改动）；`chunk_over_500kb=false`。
- 共享工作树实测 `du -sb dist = 790,114B（28.9%）` —— **该数字已被 A3/A5/A7 并发写入的 `src/App.vue`、`HomePanel.vue`、`MainArea.vue`、`StatusBar.vue`、`useHomeStore.ts`、`homeUi.ts`、新增 home 组件污染，不可作为任何单 Lane 的结论**，仅说明"W17 五条代码 Lane 合起来必然远超 25.2%"。
- 按 W17 共享验收第 5 条：**不抬限**，在此如实上报精确增量，请 A0 裁决：
  - 选项 A（推荐）：为 W17 整波上调一次性上限（A6 独占需 ≥ 25.75%，另需预留 A3/A5/A7/A9 增量）。
  - 选项 B：裁我的补丁。可砍目录：`.nav-compact/.nav-icon` 样式 + Esc 焦点归还 + 键盘漫游（约 -1.5KB，仍约超 1.8KB），但会直接放弃共享验收第 3/4 条（键盘可达、窄窗口），不建议。
  - 选项 C：只保留"无孤儿模块 + 菜单补三项 + 可访问名称/aria"这类低成本项，放弃密度状态与 store 上收（约 -2KB，仍无法满足 346B 余量）。
- 结论：以 346B 的余量，W17 任何一条代码 Lane 都不可能达标；这不是 A6 单方可解的问题，请 A0 在整波层面处置。

## 4. 边界遵守（对照 W17 硬停）

- 未新增 Tauri 命令 / bridge 能力 / ACL 条目 / 文件权限 / 网络权限；`ActivityBar.vue` 无 `invoke(`、无 `@tauri-apps` 导入（已被脚本断言）。
- 未触碰 home 组件、`MainArea.vue`、`StatusBar.vue`、`App.vue`、任何 Rust/后端文件（脚本断言 `components/home`、`MainArea` 未出现）。
- 未引入新依赖：ActivityBar 新增 import 仅 `../../utils/redact`（既有）与 `../../stores/useLayoutStore`（既有）。
- 未启用任何被锁运行时权限（plugin invoke / 动态加载 / 网络 / daemon / 模型调用 / Agent-Skill 执行 / MCP / graph 写 / 后台 worker）。
- 未改 `pre-merge.sh`（A9 权限范围），接线建议见 §5。

## 5. 集成须知（A0 / A9）

1. 补丁为 3 个文件的完整 diff（`git apply` 即可；新脚本已 `git add -N` 故包含在 diff 内）。
2. `pre-merge.sh` 接线（A9 权限，建议照现有样式插入 UI 逻辑测试区）：
   ```bash
   (cd "$ROOT" && node "$SCRIPT_DIR/check-client-navigation-logic.mjs") >/dev/null 2>&1 \
     || pm_fail "check-client-navigation-logic.mjs（W17 客户端导航可达性门禁）"
   ```
   并在 `run_self_test` 的文件存在性检查里加一行 `[ -f "$SCRIPT_DIR/check-client-navigation-logic.mjs" ]`。
3. 与 A5/A3 的接缝：A6 只依赖 `useLayoutStore`，**不读** `useHomeStore`/`homeUi`，两者互不阻塞；主页入口由 A5 在 `src/components/home/` 内自行组织，与本补丁无文件重叠。
4. 与 A7 的接缝：A7 改 `MainArea.vue`/`StatusBar.vue`/`App.vue`，与本补丁零重叠；但 A7 若在 `App.vue` 里也接 resize，请复用 `layout.setWindowWidth` 而非再造一份宽度状态。
5. 与 A4/A9/A10 的接缝：本补丁新增的展示面只有活动条已有区域，脱敏复用既有 `redactSecrets`；`localStorage` 用法未改（仍只有 `browser-os-recent-dirs`）。

## 6. 遗留与风险（诚实）

- 键盘漫游只作用于活动条一级按钮；扩展行（宫格设置/☰ 菜单/最近与常用）内部仍为 DOM 顺序 Tab —— 若需要行内漫游，属下一波增补。
- 窄窗口阈值 1180/900 为经验值，未做真机多分辨率实测；真机/窄窗口人工验收属 A8。
- `layout.navSection` 与 `browser.aiNavOpen` 仍各自为政（AI 导航面板是独立状态，未纳入本次统一），无功能影响。
- A6 W16 遗留的 modal 焦点 backlog（AsyncState.vue / asyncView.ts 缺失、`modalA11y.ts` 被裁）**本次未处理**，W17 指派未含该项，仍挂账。
- 本 Lane 不 push，工作树改动与补丁一并交 A0 集成。
