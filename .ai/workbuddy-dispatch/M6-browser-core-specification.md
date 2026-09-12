# M6 — Browser Core Abstraction · Specification

> ## STATUS: DEFERRED (2026-09-13 Owner 裁决 GO_WITH_REDUCED_SCOPE)
>
> **本文件保留为未来设计参考，本轮不实施。**
> - 不实施：`BrowserRuntime` / `BrowserScene` / `syncScene` / `MockRuntime`（M6-A ~ M6-H 整条完整 Runtime 路线）
> - 降级原因：`logs/research/M6/M6-0-spike-result-20260913.md`
> - 本轮实际执行：M6 Reduced Scope（S1~S5，局部重构，零 Rust 改动、不改产品语义）
> - 下文 §2 APPROVED_TARGET / §4 各层职责 / §5 实施顺序 A–H **均为设计参考，不构成派发依据**

> Version: 2026-09-12
> Owner / Chief Architect Decision（见 `M6-A0-governance-decision-record.md`）
> Status: SPECIFICATION — 本轮只做规划与治理收口，不启动大规模源码迁移
> 命名：M6。本文件取代 `PLANS.md` 中旧的 Phase 2 / Phase 3 / Phase 5 编号。

---

## 0. 命名裁决（避免撞号）

已确认：`PLANS.md` 存在旧 `Phase 0~7` 体系，与后续 Phase 03/04/05 是两套编号，且 `PLANS.md` 实为空壳模板（仅标题，无内容）。

裁决：整条 Browser Core 统一命名为 **M6**。

| 编号 | 含义 | 状态 |
|---|---|---|
| PLANS.md 旧 Phase 0~7 | 空壳模板 | 作废（保留文件，标注 DEPRECATED，不再作派发依据） |
| M6 | Browser Core Abstraction | 当前阶段 |
| M7 | Workspace Retrieval Foundation | NEXT_CAPABILITY，排队在 M6 之后，不取消 |

总体路线：稳定浏览器（M1-0 基线） → M6 Browser Core 抽象 → M7 Workspace Retrieval → Agent / RAG / Skill 能力。

批次编号：M6-0（可行性 Spike）+ M6-A ~ M6-H。

---

## 1. CURRENT evidence（2026-09-12 实测，只读）

### 1.1 目标符号源码零命中

| 符号 | Rust (*.rs) | 前端 (*.ts / *.vue) |
|---|---|---|
| BrowserRuntime / MockRuntime | 0 | 0 |
| BrowserScene / syncScene | 0 | 0 |
| WebViewSafeShell / BrowserViewportAnchor | 0 | 0 |

实测门禁：

```json
{"check":"browser-runtime","status":"PASS","symbolsFound":0,
 "summary":{"filesScanned":102,"targetStatus":"not-yet-implemented"}}
```

注意：该 PASS 是哨兵，不是架构达标。脚本 L5-8 自述 "approved as TARGET but did NOT implement in source ... absence of these symbols is PASS"。语义修正方案见 dispatch plan。

### 1.2 现有锚点（迁移基点，不得推倒）

| 资产 | 位置 | 实测规模 |
|---|---|---|
| 统一 invoke 层，components 零直接 invoke | src/bridge.ts | 约 747 行 |
| 唯一 webview 定位调度器 | src/composables/useBrowserHost.ts | 207 行 |
| Rust 命令面 | src-tauri/src/bridge.rs | 约 7170 行 / 约 140 commands |
| Pinia stores | src/stores/ | 19 |
| Native 定位唯一修正路径 | tauri-browser-tabs/.../linux.rs ensure_size_allocated | 锁定 |

### 1.3 复杂度基线（本里程碑要降低的对象）

useBrowserHost.ts 为纯命令式 + 手写防御，调用面：

```text
bridge.tabPosition(id, {x,y,width,height})
bridge.gridPosition(i, rect)
bridge.gridSetZoom(i, zoom)
bridge.hideWebview(id)
bridge.hideAllWebviews()
bridge.debugLog(...)
```

手写同步防御状态共 8 类：

| # | 防御 | 位置 |
|---|---|---|
| 1 | lastKey + lastAt：tabPosition 50ms 去重 | L53-57 |
| 2 | lastGridSession + lastGridSent Map：宫格 rect 去重 | L12-13, L126-130 |
| 3 | lastHiddenTab：同会话同页签只移出一次 | L14, L171-174 |
| 4 | 双 requestAnimationFrame 合并 | L25-26 |
| 5 | rect 为 0 时 rAF 递归重试 | L39-45 |
| 6 | layoutGridNow(retry) 重试 10 次 x 100ms | L119-123 |
| 7 | layout.webviewsSuspended 全局闸门 | L28, L110 |
| 8 | mainView 变化时清空全部缓存 | L21 |

这 8 类防御就是 BrowserScene 声明式契约要消灭的东西。它们是真实痛点的沉淀，是判断本笔架构债真伪的关键证据。

---

## 2. APPROVED_TARGET

目标链（Owner 裁决确认）：

```text
Vue Component / Store
   -> BrowserScene              (期望浏览器现在是什么状态)
   -> BrowserRuntime interface  (能力契约，不含 WebView 实现细节)
        |- RealBrowserRuntime
        |- MockRuntime
   -> syncScene                 (把 Scene 落成真实 native 操作)
   -> Tauri / GTK WebKit WebView
```

分层澄清（Owner 补充裁决）：

| 层 | 是什么 | 不是什么 |
|---|---|---|
| GTK/WebKit WebView | 真正干活的浏览器引擎 | - |
| RealBrowserRuntime | 我们的软件如何控制这个引擎 | 不是另一个 WebView，不替代 GTK |
| BrowserScene | 期望状态的数据描述 | 不是 UI 布局，不描述样式 |
| MockRuntime | 不启动 GTK 时模拟该控制契约 | 不是本次重构的主要理由 |
| Vue Shell | 界面与交互壳层（已存在） | 不是要新建的 Vue 应用 |

Vue Shell 任务的准确含义 = 现有 Shell 的职责收敛与渐进迁移，不是再造一个新 Vue 应用。

---

## 3. NON-GOALS（明确不做）

1. 不实现 Workspace Retrieval（归 M7）。
2. 不实施 R3B Browser-First Workbench UI 改版（状态 DEFERRED）。
3. 不改 WebView 模型：1 tab = 1 native WebView 保持不变；禁止本阶段改为 1-4 Slot Pool。
4. 不整体重写 Vue Shell；禁止推倒 src/bridge.ts。
5. 不改变产品语义：Tab 生命周期、关闭策略、布局、UI 一律不动。
6. 不新增 src/domain/、src/application/ 目录，除非 Spike 证明必要。
7. 不为 Mock 而造 Real：若 RealRuntime 只是把 bridge.tabPosition() 改名成 runtime.tabPosition()，判定不值得做，立即停止并回报。
8. 不扩张运行时权限面（ACL / capability），不新增 Tauri 命令，除非 Spike 证明必需。
9. 不做 semantic / vector / AI 任何能力。

---

## 4. 各层职责

### 4.1 BrowserRuntime responsibility

职责：接收 BrowserScene，把它变成真实浏览器行为。

- 决定哪些 WebView show / hide / move / resize / focus
- 决定操作顺序
- 负责去重（替代 1.3 防御 1/2/3）
- 负责失败重试（替代防御 5/6）
- 负责 suspend 闸门（替代防御 7）

不负责：界面长什么样、用户在哪个 workspace、哪个 Vue panel 打开、用户点了什么。

### 4.2 MockRuntime contract

- 与 RealBrowserRuntime 实现同一 interface
- 记录收到的 scene 序列，供断言
- 不启动 Tauri / GTK / WebView，可在 Node 下运行
- 附带收益（非主要理由）：解开 G5 前端 0 单测

### 4.3 BrowserScene responsibility

职责：描述期望浏览器现在是什么状态。

必须包含（草案）：activeTabId、viewport rect、visibleTabs、mode（normal / grid / suspended）。

硬性约束（Owner 裁决第 3 条）：BrowserScene 不得把「1 tab = 1 WebView」焊死进 contract。它描述期望场景；当前 RealRuntime 可用 1:1 实现；未来若证明 Slot Pool 有必要，应只替换 Runtime/native 实现，而不是推翻 Vue Shell / BrowserScene contract。

禁止：Scene 里出现 webviewHandles、perTabWebviewId 等强绑定实现细节的字段。

### 4.4 WebViewSafeShell responsibility

- Vue 浮层 / 侧栏 / 面板变化时，为原生 WebView 提供统一安全边界
- 保证 WebView 遮挡与可见性由一处控制（当前靠 PROJECT-RULES.md 规则 + check-native-webview-overlay.mjs 静态检查，无运行时组件）
- 为 R3B 未来落地补上前置能力（R3B-09 悬置的根因）

### 4.5 BrowserViewportAnchor responsibility

- WebView 尺寸/位置的统一锚点
- 取代当前 getBoundingClientRect 与各组件自算 rect 的散落做法
- 必须遵守 PROJECT-RULES.md 规则 3（纯 CSS 像素，绝不乘 devicePixelRatio）

### 4.6 syncScene boundary

- 唯一的 scene 到 native 同步入口
- 收敛当前散落的 tabPosition / gridPosition / gridSetZoom / hideWebview / hideAllWebviews
- 幂等：同一 scene 重复下发不产生额外 native 调用
- 不得借机重新定义产品语义

---

## 5. 实施顺序（incremental migration sequence）

Owner 已裁决 A-H 顺序，且不得因为最终目标存在而一次性实施 A-H。

在此基础上前置一个 GO/NO-GO Spike，因为 Owner 明确提出：应先证明当前 useBrowserHost.ts 那些 position/show/hide/focus/retry/dedupe 的复杂度，能否通过 BrowserScene 到 syncScene 真正减少。若能明显减少，BrowserRuntime 值得做；若最后只是改名，那这个任务不值得做。

| 批次 | 内容 | 前置 | 出口 |
|---|---|---|---|
| M6-0 | 可行性 Spike（GO/NO-GO） | - | 见 7.1 |
| M6-A | BrowserRuntime interface + Real adapter + MockRuntime | M6-0 GO | interface 落地，行为零变化 |
| M6-B | WebViewSafeShell + BrowserViewportAnchor | A | 统一边界 |
| M6-C | BrowserScene contract | A | Scene 数据结构冻结 |
| M6-D | syncScene 统一原生同步 | C, B | 命令式散调收敛 |
| M6-E | 渐进迁移旧 browser commands / callers | D | adapter / strangler 收口 |
| M6-F | 清理确认无 caller 的旧调用 | E | 死代码移除 |
| M6-G | deterministic tests + MockRuntime tests | A-F | 门禁接线 |
| M6-H | packaged desktop / real native WebView acceptance | G | 用户真机验收 |

M6-0 未通过则整条线停止，回报 Owner 重新裁决，禁止直接进入 M6-A。

---

## 6. 行为兼容不变量（compatibility invariants）

M6 属 architecture migration，不是 product redesign。以下必须逐条保持：

| # | 不变量 | 依据 |
|---|---|---|
| I-1 | 当前 UI 基本行为不变 | Owner 6 |
| I-2 | Tab 生命周期不变 | Owner 6 |
| I-3 | 1 tab = 1 native WebView 实现不变 | Owner 3 |
| I-4 | Phase 04 普通关闭：no prompt / no persistent save / direct close | PROJECT-RULES [CURRENT] |
| I-5 | recentlyClosed 仅内存栈（仅 url,title）+ Ctrl+Shift+T | PROJECT-RULES [APPROVED] |
| I-6 | manual Session Save 独立，不被关闭流程触发 | PROJECT-RULES [DEPRECATED] auto_save_on_close |
| I-7 | app-exit policy 独立 | Owner 6 |
| I-8 | 已有 WebView focus/show/hide/position 行为不变，除非有明确 bug evidence | Owner 6 |
| I-9 | gtk_fixed_move + size_allocate 定位路径不变 | PROJECT-RULES 规则 1 锁定 |
| I-10 | 隐藏只移位置 x=-30000，绝不 webview.hide() 或缩到 1x1 | PROJECT-RULES 规则 3.5 锁定 |
| I-11 | 前端坐标纯 CSS 像素 | PROJECT-RULES 规则 3 锁定 |
| I-12 | HTML 浮层不得覆盖原生 WebView | PROJECT-RULES 规则 3.8 |

任何 batch 若需违反 I-1 至 I-12，必须停止并回报 Owner，不得自行裁决。

---

## 7. 确定性验收（deterministic acceptance）

### 7.1 M6-0 GO/NO-GO 判据

在不改动运行时行为的前提下，产出可评审方案，必须量化回答：

| 指标 | 当前基线 | GO 门槛 |
|---|---|---|
| useBrowserHost.ts 行数 | 207 | 显著下降（阈值由 Spike 实测给出） |
| 手写防御状态类数 | 8 | 显著减少 |
| 组件/store 直接操作 WebView 同步的调用点 | 6 类 bridge 调用 | 收敛为 1 个 syncScene |

NO-GO 判据：若方案最终等价于把 bridge.tabPosition() 改名为 runtime.tabPosition()，即未消除任何一类 1.3 防御，判定不值得做，停止并回报。

### 7.2 真正验收标准（Owner 明确）

不是「终于出现 BrowserRuntime 这个 interface 了」，
而是：Vue 层是否真的不再管理 GTK WebView 的底层同步细节。

### 7.3 回归门禁（每批必须全绿）

```bash
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
bash scripts/pre-merge.sh
node scripts/check-browser-runtime.mjs
node scripts/check-core-boundary.py
node scripts/check-native-webview-overlay.mjs
node scripts/check-window-drag.mjs
git diff --check
```

新增：scripts/check-browser-core-migration.mjs（M6-G 落地，见 dispatch plan）。

---

## 8. 回滚策略（rollback strategy）

- 采用 adapter / strangler 增量迁移，每批独立可回退
- 每批一个 commit，message 带 `M6-X` 前缀
- 任一批失败则 git revert 该批 commit，不影响前序批次
- M6-0 Spike 仅产出文档，零产品代码，天然无回滚成本
- 禁止跨批次大改动；禁止单 commit 混合多个批次

---

## 9. 原生桌面验收（native desktop acceptance）

- M6-H 必须真实 Tauri 客户端验收，不接受 Vite preview
- 依据 PROJECT-RULES.md 规则 3.7：验收前关闭旧进程，核对 PID / proc/PID/exe / 启动时间
- 依据规则 3.7：未真实桌面验收，不得写 GUI_PASS
- 必验项：多页签切换、宫格、面板展开收起时 WebView 跟随、窗口缩放、Ctrl+Shift+T、普通关闭无弹框

---

## 10. Phase 04 decision reconciliation

docs/AI/00-Architecture.md §2.3 中两条 PENDING 已被 Phase 04 解决，文档滞后：

| 原 PENDING | 现状态 |
|---|---|
| 关闭 tab 是否自动保存并直接关闭 | CLOSED / REJECTED — no prompt + 不持久化 + 直接关闭 |
| 是否增加 recentlyClosed + Ctrl+Shift+T | APPROVED / 已实现（acf4add） |

另两条由本次 Owner 裁决：

| 原 PENDING | 裁决 |
|---|---|
| Native WebView 模型 | APPROVED_CURRENT：1 tab = 1 WebView；slot pool = FUTURE / SEPARATE_DECISION |
| Vue Shell 迁移方式 | APPROVED：incremental migration；full rewrite = REJECTED |

---

## 11. R3B DEFERRED 与 LOCK 治理

见 `M6-A0-governance-decision-record.md`。要点：

- R3B = DEFERRED：不废弃、不阻塞 M6、不要求为 M6 接受 R3B UI
- 禁止通过「假装 ACCEPT R3B」解除 Product Code LOCK
- 该 LOCK 原文作用域为 W18/W19/R3B 产品改版路线，应被收窄，以免错误阻塞 M6
- 收窄动作属 A0 权限，本波不擅自执行，已生成 A0 decision record

---

## 12. 与 Workspace Retrieval 的依赖关系

| 方向 | 是否依赖 | 说明 |
|---|---|---|
| M6 Browser Core → M7 Retrieval | 否 | M6 不依赖检索能力 |
| M7 Retrieval → M6 Browser Core | 部分 | Retrieval 的消费方（UI 面板 / Agent）若需定位到浏览器场景，可复用 BrowserScene；但不阻塞 M7 起步 |

M7 排在 M6 之后，主因是**架构顺序**（先有稳定的浏览器能力边界，再在其上叠能力），不是技术硬依赖。若 Owner 后续裁决并行，需重新评估。

依赖关系（外部）：

| 项 | 关系 |
|---|---|
| R3B Workbench UI | 依赖 M6 的 WebViewSafeShell / BrowserViewportAnchor（R3B-09 悬置根因）；M6 不依赖 R3B |
| M5-W18 / W19 | 已 CLOSED；R3B 原型保留为未来 UI 参考 |
| G5 前端 0 单测 | M6-A 的 MockRuntime 为其解除前提之一 |
