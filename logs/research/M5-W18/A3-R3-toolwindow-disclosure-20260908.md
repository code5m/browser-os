# A3 · M5-W18-R3 — IDEA 渐进披露行为规格（工具窗口 + 树行为）

版本：2026-09-08（R3B 修订）；Lane **A3**；模式 `RESEARCH_AND_PROTOTYPE`；NEXT `M5-W18-R3B`；`W19=CLOSED`。
证据基线：canonical `master` @ `200f0f1`（A3 工作树 `codex/m5-w18-a3` 已 rebase 到该提交）。
本文件只做研究与原型，**未改任何产品代码、依赖、ACL、capability、原生运行时或用户数据**。
R3B 修订依据：`logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`（裁决 `REVISE_TARGETED`，R3B 卡 §A3）。本次对齐 A0 裁决：收敛折叠/固定语义、几何 SSOT（60px 顶栏 / 24px 状态 / 28px 活动条）、补 900×600 与键盘焦点断言、移除原型 donor 品牌。

## 0. 产物与自检

| 产物 | 作用 | 自检 |
|---|---|---|
| `A3-R3-toolwindow-disclosure-20260908.md` | 本规格（单一真源） | 与下方规则表逐条对齐 |
| `A3-R3-toolwindow-state-machine.mjs` | 确定性状态机 + 树的纯函数参考实现 | `node …mjs` → **PASS（73 条断言）** |
| `A3-R3-toolwindow-prototype.html` | 自包含合成原型（6 种尺寸 / 空·加载·错误·禁用 / 右键作用域菜单 / 收起态活动条） | 内联脚本 `node --check` 通过；无外链、无网络、无原生调用 |
| `A3-checkpoint-R3-20260908.md` | Lane 交付说明与依赖挂账 | — |

收起态内容占比（A0 SSOT 公式：宽=(视口−28)/视口，高=(视口−60−24)/视口；模型实算，见 `.mjs` 输出）：

| 视口 | 内容宽 | 内容高 | 门槛 | 结论 |
|---|---|---|---|---|
| 1920×1080 | 98.5% | 92.2% | ≥92% 宽 / ≥85% 高 | PASS |
| 1440×900 | 98.1% | 90.7% | 同上 | PASS |
| 1366×768 | 98.0% | 89.1% | 同上 | PASS |
| 1200×800 | 97.7% | 89.5% | 同上 | PASS |
| 1024×720 | 97.3% | 88.3% | 同上 | PASS |
| 900×600（产品最小） | 96.9% | 86.0% | 同上 | PASS |

## 1. 现有产品证据（只读，用于接地，不是"已实现"的证明）

| 事实 | 位置 |
|---|---|
| 外壳为「单 `mainView` + 模块页签 + 侧栏 + 状态栏」，无工具窗口概念 | `src/App.vue:226-254`、`src/components/layout/MainArea.vue:122-270` |
| 侧栏宽度 clamp `[180, 560]`，**无内容区地板、无按视口收窄** | `src/stores/useLayoutStore.ts:210-212` |
| 侧栏显隐由视图白名单决定（files/clip/arts/repo/apps/audit） | `src/components/layout/Sidebar.vue:14` |
| 分隔条只有鼠标拖拽，**无键盘路径、无触限反馈** | `src/components/layout/SidebarResizer.vue:9-21` |
| 浏览器视图右侧 Dock 是"另一个"侧栏（独立开关/页签），与左侧栏不是同一套契约 | `useLayoutStore.ts:143-145,306-317`、`MainArea.vue:152-167` |
| 树是惰性加载：`treeChildren` / `treeExpanded` / `treeLoading`，`ensureTreeChildren` 逐层取数 | `src/stores/useWorkspaceStore.ts:416-458` |
| 树**没有**"全部折叠 / 展开一级 / 有界展开全部"入口 | `FilePanel.vue:18-32`（仅刷新 + 新建） |
| 状态栏 26px | `src/components/layout/StatusBar.vue:136` |
| 全局快捷键用 `matchKey(ctrl/shift/alt+key)`，三套方案 vscode/idea/eclipse | `src/App.vue:143-161`、`src/stores/useSettingsStore.ts:8-42` |
| 顶栏扩展行（宫格/☰/最近常用）互斥，切换视图即收起 | `useLayoutStore.ts:152-165,186-192` |
| 子 webview 位置/尺寸/隐藏方案是历史踩坑区 | `WORKBENCH_BLUEPRINT-20260908.md` §8.3、`踩坑记录-子webview撑不满-根因与修复.md` |

## 2. 模型：边、工具窗口、状态

- **三条边**：`left` / `right` / `bottom`。**不引入浮动/脱离窗口**（见 §9 Q1）。
- **每条边**：`{ visible, primary, size, autoHide }`，同一时刻只有一个 `primary`（CR-2）。
- **每个工具窗口**：`{ id, edge, fallbackEdge, open, pinned, autoHide, lastSize }`。
  **固定状态属于工具，不属于边**：换边、关闭再开都保留（与 `.mjs` 一致）。
- **顶栏**：常态 = 60px（A0 SSOT：两行各 30px，行 30 + 页签行 30）；收起态保留 60px 顶栏 + 24px 状态栏 + 28px 活动条，仅隐藏可缩放工具面板。
- **中央区**永远是文档（网页 / 文件 / SQL 控制台 / 笔记 / Git 视图），工具窗口只借空间，不替换它。

### 常量（`.mjs` 单一真源，R3B 已对齐 A0 几何 SSOT）

```
TITLEBAR 0（OS 标题栏在 inner viewport 之外，A0 口径）| TOP_ROW 30 | TOP_ROWS 60（A0：2×30=60）| STATUS 24（A0）
ACTIVITY_STRIP 28（收起态仍可见的紧凑工具条，A0 裁决）
ACCEPT: 收起态 高≥85% 宽≥92%；内容地板 520×320
收起态公式：宽=(视口−28)/视口；高=(视口−60−24)/视口
EDGE_LIMITS: left/right min200 max420 def260/280 share32%；bottom min140 max420 def240 share45%
NARROW: <1180 未固定侧工具自动隐藏；<900 固定侧工具与底部工具也自动隐藏
TREE: MAX_NODES 2000 | MAX_DEPTH 6 | CHUNK 200/帧 | CONFIRM_ABOVE 5000 | VIRTUALIZE_ABOVE 500 | PERSIST_MAX_PATHS 500
SNAPSHOT_MAX 3
```

## 3. 几何与预算

常态默认布局建议：**只开左侧 260px**：内容 1440−260=1180 宽（81.9%）、762 高（87.8%）。
不建议默认"左+底"同时常驻（1180×522，高仅 60.1%，属于卡片明令禁止的 permanent multi-panel squeeze）。

分隔条规则（CR-9）：`size = clamp(请求值, [min,max], 视口 share 上限, 内容地板)`；
触限时**必须返回 `clamped` 效果**给 UI 说明原因，禁止"拖不动但无反馈"的静默无效。
窄窗下若仍无法满足 520×320 地板 → 自动全部收起（模型内置，`.mjs` `applyNarrowPolicy`）。

## 4. 状态转移与冲突规则（确定性）

| 规则 | 内容 |
|---|---|
| **CR-1 替换除非固定** | 同侧已有 primary A 时开 B：A 未固定 → A 关闭、B 接管（不额外占空间）；A 已固定 → B 落到 `fallbackEdge`；备用边也被固定占用 → **拒绝并给出可执行提示**（不静默换边、不静默丢弃） |
| **CR-2 一edge一主** | 同边其余工具以边缘条按钮（stripe）形式存在，点击即按 CR-1 接管 |
| **CR-3 打开即恢复尺寸** | 打开时用 `lastSize` 经 clamp 后的值；无记录用默认值 |
| **CR-4 全部收起** | 存 1 份快照（最多 3 份）→ 三条边全部 `visible=false` → 焦点回文档。**含固定窗口一并收起**（A0 R3B 裁决：Collapse All hides every tool window, including pinned）；快照完整记录 `pinned` 状态，恢复时原样回放 |
| **CR-5 恢复上次** | LIFO；栈空 → 回到文档化默认布局（左侧文件 260） |
| **CR-6 展开活动** | 焦点在工具窗口 → 该边取到 share 上限、其余边收起（先快照，可逆）；焦点在文档 → 进入专注模式 |
| **CR-7 固定** | 仅影响「同侧替换」（CR-1）与「窄窗自动隐藏顺序」，**绝不影响**全部收起——固定窗口在 Collapse All 中同样被隐藏，靠唯一快照一键恢复（与 A0 裁决一致；R3B-03 A3/A4 分歧据此消解，A4 须对齐本款） |
| **CR-8 自动隐藏** | 只做浮层覆盖，**不占布局、不触发内容重排**（关键：避免子 webview 重排踩坑）；退出条件 Esc / 点击外部 / 焦点移出，三者任一 |
| **CR-9 调整尺寸** | 见 §3；触限显式反馈 |
| **CR-10 焦点归还** | 关闭工具窗口 → 回焦点栈中最近的合法锚点；栈空 → 该边 stripe 按钮；再无 → 文档区（绝不丢到 body） |
| **CR-11 常态最多两条边** | 开第三条边时，最久未交互（LRU）的那条自动隐藏，固定者豁免；保证不出现"两侧+底部同时挤压" |
| **CR-12 专注模式** | 隐藏全部工具 chrome（顶栏/页签/侧边/底部/状态栏），**只留一个返回动作**：右上角胶囊 + Esc；退出时恢复进入前的快照 |

转移函数是纯函数：`(state, action) -> { state, effects }`，输入不被修改（`.mjs` 有断言覆盖）。

## 5. 焦点与键盘

- **焦点锚点**：`{ surfaceId, selector, scrollTop, selection }`，焦点栈上限 32。
- **绝不抢焦点**的场景：后台刷新、toast、异步加载完成、布局落盘。（现有 `navSection` 的 Esc 归还逻辑踩过"focus 事件又把行重开"的坑——`ActivityBar` omni 行——本规格要求 Esc 归还前先抑制触发控件的 focus 副作用。）
- **树内键盘**：`↑/↓` 移动、`←/→` 折叠/展开、`Enter` 打开、`Home/End`、首字母跳转；`role=tree/treeitem` + `aria-expanded/level/setsize/posinset`，焦点环必须可见。
- **提出的快捷键补充**（沿用现有三方案结构，`matchKey` 已支持 ctrl/shift/alt 组合）：

| 动作 | vscode | idea | eclipse |
|---|---|---|---|
| 全部收起/恢复 | `Ctrl+Shift+B` | `Ctrl+Shift+F12` | `Ctrl+Shift+F12` |
| 展开活动 | `Ctrl+Alt+=` | `Ctrl+Alt+M` | `Ctrl+M` |
| 专注模式 | `Ctrl+Alt+F` | `Ctrl+Alt+F` | `Ctrl+Alt+F` |
| 固定/取消固定 | `Ctrl+Alt+P` | `Ctrl+Alt+P` | `Ctrl+Alt+P` |
| 自动隐藏 | `Ctrl+Alt+H` | `Ctrl+Alt+H` | `Ctrl+Alt+H` |
| 聚焦左/底/右工具 | `Alt+1` / `Alt+2` / `Alt+3` | 同 | 同 |
| 树：全部折叠 / 展开一级 / 有界展开全部 | `Ctrl+Alt+-` / `Ctrl+Alt+=` / `Ctrl+Alt+0` | 同 | 同 |

> 冲突待 A0/A6 裁决：`Ctrl+Alt+=` 同时用于"展开活动"与"树展开一级"，二者靠焦点域区分（树内 vs 全局），若不接受则树侧改 `Ctrl+Alt+Shift+=`。

## 6. 树行为冻结（collapse all / expand one level / bounded expand all）

基于现有惰性实现（`ensureTreeChildren`）定义，**纯函数、可取消、有预算**：

1. **全部折叠**：清空 `expanded`；**保留选中项**；**不偷偷展开祖先链**（否则用户点"全部折叠"却看到展开态，属不可解释行为）。
2. **展开一级**：以选中节点为锚，只展开其**直接子层**；选中叶子是**显式 no-op**（`SELECTED_LEAF`），不报错、不跳转；无选中时展开所有根一层。
3. **有界展开全部**：BFS，预算 `MAX_NODES=2000` / `MAX_DEPTH=6`，每帧应用 `CHUNK=200`（`requestAnimationFrame` 分块），可取消（token）；触发上限时返回 `truncatedReason ∈ {NODE_BUDGET, DEPTH_BUDGET}` 并在状态区显示"已展开至上限 N 项（剩余未展开）· 继续"。
4. **未加载层级不盲走**：`loaded=false` 的层以 `pendingLoads` 返回，由调用方逐层取数后再继续；单层取数失败只标记该节点（保留 caret + 错误徽标），不中止整体操作。
5. **确认阈值**：预估行数 ≥ `CONFIRM_ABOVE(5000)` 先确认"展开全部（约 N 项）？"。
6. **虚拟化阈值**：可见行 > 500 需窗口化渲染（W19 项，本轮只标阈值）。
7. **持久化上限**：展开路径最多落 500 条（FIFO 淘汰），**不落**文件正文、SQL、凭据、带敏感值的 URL。

## 7. 持久化边界（与 A4 的分工）

A3 只主张**工具窗口几何/可见性**部分，文档身份、布局 DTO 与迁移所有权归 A4：

| 落盘 | 不落盘 |
|---|---|
| `schema_version`、三条边的 `{size, primary, autoHide}`、`pinned` 集合、stripe 顺序、最近 3 份布局快照 | 文档内容/dirty、SQL 文本、查询结果、凭据、带敏感值的 URL、完整本地路径（沿用现有的脱敏展示约定）、审计原文 |

- 崩溃安全：复用现有原子写原语（Rust `session::atomic_write`：tmp + rename），旧版本读不懂则回退默认布局并**显式提示**，不静默清空。
- 全局 / 每工作区：边尺寸与固定集合为**全局**；`primary` 与展开集合为**每工作区**。

### 现状字段迁移映射（供 A1/A4 套用，A3 不写产品代码）

| 现状 | R3 落点 | 备注 |
|---|---|---|
| `sidebarOpen` / `sidebarWidth` / `leftTab` | `left.{visible,size,primary}` | 白名单式显隐（Sidebar.vue:14）改为边可见性 |
| `browserDockOpen` / `browserDockTab` | `right.{visible,primary}` | 右侧从"浏览器专属 Dock"升为通用工具边；浏览器视图内仍必须可达 |
| `compactMode`（隐藏地址栏+页签） | 保留为「浏览器沉浸」，**不等于**专注模式 | 二者是否合并交 A1（D-A3-3） |
| `navSection` | 保持瞬时，不持久化 | 现状即如此，不引入新持久化面 |
| `modTabs` / `activeModTab` | 文档身份（A4 领域） | A3 不拥有 |

## 8. 失败与状态覆盖（原型已覆盖）

空态 / 加载态（骨架屏）/ 错误态（可重试、稳定错误码）/ 禁用态（右键项带禁用原因，如"Git 暂存此文件"在非仓库路径禁用）/ 上下文菜单（按指针下对象作用域组织，不只是收容菜单）。
每个右键动作在 A6 的命令注册表里都要有：命令身份、禁用原因、快捷键、安全等级——本页只示范形态，所有权归 A6。

## 9. 待裁决项（交 A0 / 相关 lane）

- **Q1 浮动/脱离窗口**：A3 建议 **REJECT（本轮不做）**——Linux 子 webview 位置方案脆弱，浮动窗口会重开已失败路径；如需，走独立实验片。
- **Q2 全部收起是否跳过固定窗口**：**已由 A0 R3B 裁决（REVISE_TARGETED）闭合**——Collapse All 隐藏所有工具窗口（含固定），固定仅影响「替换」与「窄窗自动隐藏顺序」，不豁免收起；恢复由唯一快照回放可见性/尺寸/固定态。A3 状态机已于 `.mjs` 断言覆盖（73 断言段 7.5）。R3B-03 中 A3 与 A4 的分歧据此消解，A4 须对齐本款。
- **Q3 默认左 260 / 底 240**：需与 **A2 密度审计**对齐后定稿（依赖 A2）。
- **Q4 Git log 的落点**：Rebased 让 Git log 既可作为中央编辑器内容、也可作底部工具窗口。A3 只规定**同一工具换边必须可逆**（swap + 快照 + `lastSize` 分边记忆），默认落点交 A1/A7。
- **Q5 持久化键名与 `schema_version`**：归 **A4**；A3 只声明字段集与不落盘红线。
- **Q6 快捷键冲突**：见 §5 注，交 A0/A6。
- **Q7 多显示器 / DPI**：本轮不研究；`WORKBENCH_BLUEPRINT` §7 要求 DPI 1/1.5/2 实测，归 A8/A11。

## 10. 建议的 W19 纯逻辑测试（本轮**不落地**，等 A0 开片）

将来若实现，建议新增 `scripts/check-toolwindow-logic.mjs`（源码级纯断言，风格对齐 `check-client-navigation-logic.mjs`）：
`TW_EDGE_SINGLE_PRIMARY`、`TW_REPLACE_UNLESS_PINNED`、`TW_COLLAPSE_ALL_SHARE`（六尺寸 ≥85%/≥92%，含 900×600）、
`TW_RESTORE_LIFO`、`TW_EXPAND_ACTIVE_REVERSIBLE`、`TW_AUTOHIDE_NO_RELAYOUT`、`TW_SPLITTER_CLAMP_FEEDBACK`、
`TW_NARROW_POLICY`、`TW_FOCUS_RETURN_ORDER`、`TW_FOCUS_MODE_SINGLE_EXIT`、`TW_TREE_COLLAPSE_ALL_PURE`、
`TW_TREE_EXPAND_ONE_LEVEL`、`TW_TREE_BOUNDED_BUDGET`、`TW_TREE_NO_BLIND_WALK`、`TW_PERSIST_NO_SECRETS`、
`TW_COLLAPSE_HIDES_PINNED`、`TW_KEYBOARD_FOCUS`。

## 11. 合规与防回退

- 零产品源码改动；原型与模型为合成数据、自包含、无网络、无原生调用。
- 未复制 IDEA / JetBrains 任何资源与代码；行为规则均为本 lane 依据公开交互习惯 + 本仓库现状自行定义。
- Rebased 相关（Git 落点、许可证、`NOTICE.txt`）由 **A7/A10** 出证据；A3 不引用其源码。
- 不触碰子 webview 已失败路径（`queue_resize` / `set_size_request` / `webview.hide()`）；自动隐藏明确设计为**不触发重排**。
- R3B-02：原型 UI 已不含任何 donor 品牌（原页签/文档标题中的「Rebased」字样已移除）；行为参考仅保留在本文档叙述中，不进入产品-facing 原型文本。

## 12. R3B 修订账（A0 裁决对齐）

依据 `logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`（REVISE_TARGETED）+ `M5-W18-R3B-CORRECTION-TASKS-20260908.md` §A3。

| 项 | R3 旧状态 | R3B 修订 | 证据 |
|---|---|---|---|
| 几何 SSOT | 顶栏 80/状态 26/收起 40（与 A2/A0 不一致） | 顶栏 60（2×30）、状态 24、活动条 28，inner viewport 不含 OS 标题栏 | `CHROME`/`ACTIVITY_STRIP_W` 常量；`collapsedShares` 改 A0 公式 |
| 折叠/固定语义（R3B-03） | A3 已隐藏固定；但与 A4 表述分歧 | 显式对齐 A0：Collapse All 隐藏所有（含固定），固定仅影响替换/自动隐藏 | CR-4/CR-7、Q2 闭合；`.mjs` 段 7.5 断言 |
| 最小窗口（R3B-08） | 仅 4 尺寸，含 800×600 | 6 尺寸，含产品最小 900×600；800×600 已弃 | `collapsedShares` 自测循环 + 原型尺寸下拉 |
| 键盘焦点（R3B-04） | 仅可达性 | 新增焦点转移确定性断言（开/关/收起/固定不抢焦点/树选中锚） | `.mjs` 段 7.15 |
| donor 品牌（R3B-02） | 原型含「Rebased」 | 移除 2 处 | 原型 HTML 页签/文档标题 |

待 A0/A 相关 lane：Q3（默认尺寸，待 A2 密度审计）、Q4（Git 落点，待 A1/A7）、Q5（持久化键名，归 A4）、Q6（快捷键，待 A0/A6）、Q7（DPI，归 A8/A11）。本轮不解决、不越权。

`CONSUMED_PEERS`：A0 审计 `logs/checkpoints/A0-M5-W18-R3-acceptance-audit-20260908.md`；A2 几何约定（R3B §A2）、A4 持久化契约（R3B §A4，须对齐 CR-7）。
