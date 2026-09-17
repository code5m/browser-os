# 17E · CHECKER REVIEW — Round-2 Adversarial Enforceability Audit

> Reviewer: **Agent E** · Round-2 adversarial · READ-ONLY
> 对象：`11-CHECKER-DESIGN.md` (RULE-001..013)
> 裁定角度：**不看规则"讲得好不好听"，只看能否被机器稳定检出、误报/漏报率是否可接受、小模型是否能一行绕过、规则本身是否冻结了错误语义。**
> 未修改 `src/`、`src-tauri/`、`scripts/`、`tests/`；未执行写操作。

---

## 0. 总体裁定

| 指标 | 结果 |
|---|---|
| 规则总数 | 13 |
| **ready**（按设计即可落地 Phase 0） | **2** — RULE-005, RULE-009 |
| **partial**（语义成立但检测手段须重做） | **7** — RULE-001, 002, 003, 006, 008, 010, 011(A 仅) |
| **rejected**（不应落地 / 需先解决前置） | **4** — RULE-004, 007, 012, 013 |
| round-1 称"已满足，看守即可"的 4 条中真正可直接看守 | **2** |
| 实测偏差最大的一条 | RULE-004（实测 **7** 处 / round-1 承认 **1** 处，差 7 倍） |
| 被证伪的 round-1 论据 | **3** 条（RULE-012 正则零命中；RULE-011-B 实测 100% 误报；"三方检查器矛盾"实为两方） |
| round-1 **低估**的既有门禁损坏量 | `check-grid-close-logic.mjs` 当前实为 **2 处失败**（round-1 只报 G2，漏 G1 #4） |
| 可能**冻结错误语义**的规则 | RULE-007（禁止的正是官方 hide 机制）、RULE-013（被管对象可能无消费者）、RULE-002 |

---

## RULE-001 · gridOpen 仅由 buildGrid/closeGridAll 赋值

**Goal**: 写入点收敛到 2 个生命周期 owner，防"只翻标志位不关宫格"留孤儿进程。
**Actual violation pattern**: 当前写点实测仅 2 处：`useBrowserStore.ts:319`(=true, buildGrid)、`:486`(=false, closeGridAll)。**写点确实已收敛**。
**Can static checker detect it?**: **PARTIAL**
**Detection method**: regex（现状）+ 必须升级为 symbol scan / AST
**False-positive risk**: **LOW** · **False-negative risk**: **HIGH**（仅 regex 时）
**Known bypass**: ① `gridOpen.value = !gridOpen.value`（toggle 式，正则 `(true|false)` 不匹配）；② helper 转发 `setGridOpen(v)`；③ **`useBrowserStore().$patch({ gridOpen: true })`**——Pinia 一等公民 API，对一切 `gridOpen.value =` 正则完全不可见；④ reactive 解包。
**Small-model bypass example**: `await browser.$patch({ gridOpen: true })` —— 语义等效违规，门禁全绿。
**Recommended checker**: Architecture · **Worth enforcing**: **YES**（但须以"写点数量冻结 + 禁 $patch/helper 转发"落地）
**裁定：PARTIALLY_CONFIRMED**

```
SR-EVID-E001 Claim: gridOpen 写入点仅 2 处  Reviewer: E  Classification: FACT
File: src/stores/useBrowserStore.ts  Symbol: buildGrid / closeGridAll  Line: 319; 486
Observed: `gridOpen.value = true`(319) / `= false`(486)；全仓无第三写入点。
Why: 支持 round-1"当前 2 处已收敛"  Confidence: HIGH
```
```
SR-EVID-E002 Claim: 仅靠 `gridOpen\.value\s*=(true|false)` 可被 $patch 绕过  Reviewer: E  Classification: INFERENCE
File: src/stores/useBrowserStore.ts  Line: 14-20; 670
Observed: gridOpen 由 L670 显式返回 → 是 pinia state key → `$patch({gridOpen})` 合法可达，与 `.value =` 完全等效。
Why: 挑战 round-1"低误报 + 看守即可"——低成本正则守不住 API 形态  Confidence: HIGH
```

---

## RULE-002 · isBrowserVisible 公式不得含 gridOpen

**Goal**: 必须严格 = `layout.mainView === "browser"`，防回归 B9-4 空白。
**Actual violation pattern**: ① computed 体重现 gridOpen；② 谓词内联进 BrowserHost 模板（符号消失 → 提取器失配）。
**Can static checker detect it?**: **PARTIAL** · **Detection method**: regex（函数体提取）+ 建议 runtime test
**False-positive risk**: **MEDIUM**（round-1 称"低"，偏乐观）· **False-negative risk**: **LOW**
**Known bypass**: 谓词抽成 helper `const inBrowser = () => layout.mainView === "browser"` → 若只断言"不含 gridOpen"，helper 化后可合法塞回 gridOpen 而不再被扫到。
**Small-model bypass example**: `isBrowserVisible` 改为 `computed(() => inBrowser())`，helper 内写 `!gridOpen.value && mainView==="browser"` —— 语义完整回归 B9-4，"computed 体不含 gridOpen"照样 PASS。
**Recommended checker**: UI · **Worth enforcing**: **YES**
**裁定：CONFIRMED（G2 漂移属实）+ UNDERSTATED（round-1 少报一处既有门禁失败）**

```
SR-EVID-E003 Claim: G2(L112) 断言已废弃旧公式  Reviewer: E  Classification: FACT
File: scripts/check-grid-close-logic.mjs  Line: 107-126（关键 112）
Observed: L112 断言 `/!gridOpen\.value/.test(formula) && /layout\.mainView === "browser"/.test(formula)` → 第一条件 false → fail，exit 1
Why: 确认 round-1 核心漂移论断  Confidence: HIGH
```
```
SR-EVID-E004 Claim: 同一脚本还有 round-1 未报的第二处失败（G1 #4）  Reviewer: E  Classification: FACT
File: scripts/check-grid-close-logic.mjs vs useBrowserStore.ts  Line: 87-90; 483-511
Observed: L88 断言 `if (/schedulePosition\(/.test(closeGridAll))`；真实 closeGridAll 体内**没有** `schedulePosition(`（只有 L509 `relocate()`）→ 必 FAIL
Why: **低估** round-1"门禁健康度"表；修 G2 时必须同时处理 G1#4  Confidence: HIGH
```
```
SR-EVID-E005 Claim: 新公式成立但依赖一条**无任何规则看守**的不变式  Reviewer: E  Classification: INFERENCE
File: useBrowserStore.ts + useBrowserHost.ts  Line: 149-151; 619-663; 59-95（64/68）
Observed: 陈旧 `gridOpen=true + mainView="browser"` 下，宫格子窗由 watch→syncViewVisibility→hideAllWebviews 自愈；新公式优于旧公式。
Why: 支持冻结公式，但规则未锁住使其成立的下游不变式  Confidence: HIGH
```

---

## RULE-003 · mainView 外部裸赋值一律禁止

**Goal**: 视图切换只经 setView/openModule/closeModTab，保留 fileEditorOpen/navSection 清理。
**Actual violation pattern**: 实测 **10 处**（round-1 排除 buildGrid 后称 9 处，计数正确）：`FileEditor.vue:11`、`TopBar.vue:21`(模板内联)、`useBrowserStore.ts:161/207/323(白名单)/503/545`、`useSessionStore.ts:120`、`useWorkspaceStore.ts:912/924`。附带：`useWorkspaceStore:911/923` 同时裸写 `layout.fileEditorOpen = true`。
**Can static checker detect it?**: **PARTIAL** · **Detection method**: regex 现状 / 建议 AST 成员写判定
**False-positive risk**: **LOW** · **False-negative risk**: **HIGH**
**Known bypass**: ① `layout.mainView = targetView`（变量，非字符串字面量）；② `layout["mainView"]=`；③ 解构后 `mainView.value=`；④ `$patch`；⑤ **白名单不可达**——setView/openModule/closeModTab 从不写 `layout.mainView`，内部一律用局部 `mainView.value`（194/258/264/286/309/312），故 `layout\.mainView\s*=` 永远不会在白名单内命中；唯一真实白名单是 `buildGrid:323`。
**Small-model bypass example**: `layout.$patch({ mainView: "browser" })` 一行绕过全部清理。
**Recommended checker**: UI · **Worth enforcing**: **YES**（须换检测方式）
**裁定：PARTIALLY_CONFIRMED（事实正确 / 检测设计有误：白名单不可达）**

```
SR-EVID-E007 Claim: 10 处真写点  Reviewer: E  Classification: FACT
File: src/**  Line: FileEditor.vue:11; TopBar.vue:21; useBrowserStore.ts:161,207,323,503,545; useSessionStore.ts:120; useWorkspaceStore.ts:912,924
Confidence: HIGH
```
```
SR-EVID-E008 Claim: 三个白名单函数对该正则不可达  Reviewer: E  Classification: FACT
File: useLayoutStore.ts  Line: 193-199; 253-265; 300-314
Observed: 唯一写入点 L194 `mainView.value = v;`（store 内部形式，按通用约定本就不命中）
Why: 挑战 round-1 的"排除位于 setView/openModule/closeModTab 函数体内"——白名单形同装饰  Confidence: HIGH
```

---

## RULE-004 · 组件不得手拼宫格生命周期

**Goal**: `.vue` 不得自行编排 buildGrid/closeGridAll/closeGridOne。
**Actual violation pattern**: 实测 **7 处 / 5 个 .vue**（round-1 仅承认 `UnifiedTabBar:170`，**低估约 7 倍**）：`App.vue:199`、`HomeLaunchers.vue:47`、`UnifiedTabBar.vue:170`、`ActivityBar.vue:123/134/167/398`。其中 `ActivityBar:398` 与 `HomeLaunchers:47` 属**纯触发**（经 store action，owner 仍在 store），非"编排"——真实误报来源。
**Can static checker detect it?**: **PARTIAL**（regex A 可 / regex B 不可）
**False-positive risk**: **MEDIUM** · **False-negative risk**: **MEDIUM**
**Known bypass**: ① store 转发一层 `browser.rebuildGrid()`；② 搬进 composable（`.ts` 不在 Scope）；③ regex B 的 `[\s\S]{0,120}?` 窗口已被证明会漏（E013）。
**Small-model bypass example**: `const g = { n: 6 }; Object.assign(browser, { gridCount: g.n }); await browser.buildGrid();`
**Recommended checker**: UI（但**不是** Phase 0） · **Worth enforcing**: **YES（长期）/ NO（Phase 0 直接落地）**
**裁定：OVERSTATED（可行性与迁移成本被严重低估；目标 API 尚不存在 → 落地当天即被白名单吃光）**

```
SR-EVID-E010 Claim: 正则 A 当前命中 7 处  Reviewer: E  Classification: FACT  Confidence: HIGH
SR-EVID-E011 Claim: `exitGrid` **不存在**；`openModule('grid')` 存在但不覆盖全部语义  Reviewer: E  Classification: FACT
File: useBrowserStore.ts:666-716 / useLayoutStore.ts:253-265
Why: 挑战 round-1 "应改 exitGrid/openModule('grid')"——一半建议指向不存在的 API  Confidence: HIGH
SR-EVID-E012 Claim: `if (gridOpen) layoutGrid(); else buildGrid();` 重复 4 次  Reviewer: E  Classification: FACT
File: App.vue:199; ActivityBar.vue:167; UnifiedTabBar.vue:170; useWorkbenchStore.ts:22
Why: 真实架构债，但正确顺序是"先加 store action + 迁移"，而非"先落门禁"  Confidence: HIGH
SR-EVID-E013 Claim: regex B 的 120 字窗口既命中也漏  Reviewer: E  Classification: FACT
File: ActivityBar.vue:116-135 — L120-123 间距约 75 字符 → 命中；L129-134 间距约 200 字符 > 120 → 漏报
Confidence: HIGH
```

---

## RULE-005 · IPC 三源闭包 + typed-invoke 覆盖

**Goal**: 每个 FE invoke 必须命中 Rust 注册 ∩ ACL；修复正则漏 typed invoke。
**Can static checker detect it?**: **YES** · **Detection method**: regex（集合差）+ AST
**False-positive risk**: **LOW** · **False-negative risk**: **MEDIUM**
**Known bypass**: 命令名常量化 `const CMD="skill_list"; invoke(CMD)`；嵌套泛型；bridge.ts 之外的直接 invoke 不在 Scope。
**Small-model bypass example**: `const CMDS = { list: "skill_list" }; invoke(CMDS.list)`
**Recommended checker**: IPC · **Worth enforcing**: **YES** ← 本批设计最好的一条
**裁定：CONFIRMED（拟议正则实测有效）**

```
SR-EVID-E014 Claim: 拟议正则对真实 bridge.ts 逐行可用，且会新捕获 5 个占位  Reviewer: E  Classification: FACT
验证: ① untyped `invoke("open_browser",…)`(L122)✓ ② `invoke<Artifact>("collect_selection",p)`(L134)✓
③ 内联对象泛型 `invoke<{ request_id?: string }>("skill_install",…)`(L418) —— `<...>` 内含 ? 与 : 但不含 > → 匹配✓
④ `invoke<{ run_id: string }>("skill_run",…)`(L432)✓；全仓无嵌套泛型、无命令串换行
Confidence: HIGH
SR-EVID-E015 Claim: "必须同时传 n 与 urls" 是编译器已保证的不变式  Reviewer: E  Classification: FACT
Why: bridge 签名强类型，缺参即编译错（应由 vue-tsc 把关）；而"保留 L317 降级"是源码冻结型断言，值得保留  Confidence: HIGH
SR-EVID-E016 Claim: 修好正则后 5 命令是否"被抓到"取决于入不入 KNOWN——不能两者兼得  Reviewer: E  Classification: INFERENCE
Confidence: HIGH
```

---

## RULE-006 · 原生 webview 显隐/定位仅经授权适配

**Goal**: `bridge.tabPosition/gridPosition/hideWebview/hideAllWebviews/tabActivate` 只允许在 useBrowserStore.ts 与 useBrowserHost.ts。
**Can static checker detect it?**: **YES** · **Detection method**: import boundary / regex（文件白名单）
**False-positive risk**: **LOW**（当前 0 命中）· **False-negative risk**: **MEDIUM**
**Known bypass**: 新建"看起来合法"的中间模块（如 `src/composables/useWebviewScene.ts`）→ 文件白名单天然放行；`(bridge as any).hideWebview(...)`。
**Small-model bypass example**: 新建 `src/utils/webviewScene.ts` 导出 `hideScene = () => bridge.hideAllWebviews()`，TopBar 调用 → 语义等于组件直连，门禁全绿。
**Recommended checker**: Native · **Worth enforcing**: **YES**（廉价且当前满足）
**裁定：CONFIRMED**，但 round-1 **未发现本规则与 RULE-009 在 `tabActivate` 上白名单互斥**

```
SR-EVID-E017 Claim: 5 个被管方法调用点全落两文件  Reviewer: E  Classification: FACT
Line: store 170/179/204/500/642/649; host 141/155/172 — 全仓 14 命中，其余零  Confidence: HIGH
SR-EVID-E018 Claim: RULE-006 与 RULE-009 对 `bridge.tabActivate` 白名单**互斥**  Reviewer: E  Classification: FACT
Line: 文档 82-92（006 允许 store+host）vs 118-125（009 只允许 store），二者都匹配 `bridge\.tabActivate\s*\(`
Why: 一旦把 tabActivate 收敛进 useBrowserHost，将在 009 通过/006 失败或反之  Confidence: HIGH
```

---

## RULE-007 · 禁止 position-as-hide hack

**Goal**: 不得用 `tabPosition(id,{x:-30000,…})` 当 hide，统一走 hideWebview/hideAllWebviews。
**Actual violation pattern**: TS 侧仅 1 处 `useBrowserStore.ts:170`（黑闪修复）。Rust 侧另有 6 处 `-30000`（`bridge.rs:586/593/625/628/677/678`，均为 `hide_bounds`），不在 Scope 也不应牵连。
**Can static checker detect it?**: **YES** · **False-positive/negative**: **LOW/LOW**
**Known bypass**: `const OFF = -30000; bridge.tabPosition(id,{x:OFF,...})` → 立即失配。
**Recommended checker**: Native（建议降级为 advisory doc lint） · **Worth enforcing**: **NO**
**裁定：OVERSTATED（把"调用拼写之差"包装成"语义之差"；会推动对已知时序 hack 的危险重构）**

```
SR-EVID-E019 Claim: `-30000` 是本项目**官方、书面的**隐藏机制  Reviewer: E  Classification: FACT
File: PROJECT-RULES.md:71,76; docs/AI/00-Architecture.md:43,178; bridge.rs:576,586,589,593,625,628,677,678
Observed: 规则明文"隐藏非激活页签 = update_rect(id,(-30000,原y,原w,原h))"；-100000 才是被禁值。
  bridge.rs 注释强调"刻意不用 set_visible(false)（WebKitGTK 死锁）"
Why: **反驳** round-1 隐含前提——`bridge.hideWebview` 内部执行的正是 -30000；RULE-007 禁止的只是"从另一个函数名发动同一动作"，属**命名层规则**，收益近零  Confidence: HIGH
SR-EVID-E020 Claim: L170 的存在理由是**刻意绕过去重**，改为 hideWebview 有回归风险  Reviewer: E  Classification: INFERENCE
File: useBrowserStore.ts:166-172; useBrowserHost.ts:59-95, 151-156
Observed: 注释明确"新 webview 首帧落在初始 bounds（深色）会闪黑块…避免 1x1 触发 WebKit reflow deadlock"
Why: round-1 的"待重构为 hideWebview"是无收益且有回归风险的建议  Confidence: MEDIUM-HIGH
```

---

## RULE-008 · activateWeb 不得误用 isBrowserView()

**Goal**: `UnifiedTabBar.activateWeb` 必须精确判 browser 视图。
**Actual violation pattern**: 函数体内出现 `isBrowserView(`。**当前 0 命中——源码已合规**：L152 `if (layout.mainView !== "browser") layout.setView("browser")`；陷阱注释 L149-151 真实存在；反向断言成立（`isActiveWeb` L156-158 确用 `isBrowserView()`）。
**Can static checker detect it?**: **YES**（回归守卫）· **False-positive**: **LOW**（但规则措辞会制造误报）· **False-negative**: **MEDIUM**
**Known bypass**: 改名/改箭头函数 → `async function` 提取器返回 null → **fail-open**。
**Recommended checker**: UI（并入 check-client-navigation-logic.mjs）· **Worth enforcing**: **YES**（低成本守卫，价值有限）
**裁定：CONFIRMED（事实与位置正确）/ 价值 per-cost 偏低**

```
SR-EVID-E021 Claim: 源码已满足，当日 0 命中  Reviewer: E  Classification: FACT  Confidence: HIGH
SR-EVID-E022 Claim: 规则文字（须用 `layout.mainView === "browser"`）与真实代码（`!==`）不同构 → 照字面实现第一天就 FAIL  Reviewer: E  Classification: FACT  Confidence: HIGH
SR-EVID-E023 Claim: 拟议提取器无法处理同步函数 `isActiveWeb`（硬编码 `async function`）→ 反向断言 fail-open  Reviewer: E  Classification: FACT  Confidence: HIGH
```

---

## RULE-009 · 页签命令不得裸调（store 外）

**Goal**: `bridge.tabClose/tabNew/tabOpen/tabActivate/tabReload` 只允许在 useBrowserStore.ts，保护 recentlyClosed 恢复栈。
**Can static checker detect it?**: **YES** · **Detection method**: import boundary / 文件白名单
**False-positive**: **LOW**（0 命中）· **False-negative**: **MEDIUM**
**Known bypass**: 中间模块转发；`bridge["tabNew"](...)`；注意 `recordClose` 是 store 导出成员（L691），外部可合规调用同一路径——规则只挡 bridge 层。
**Recommended checker**: UI（建议与 RULE-006 合并为 `check-bridge-intent.mjs`，共用 allowlist）· **Worth enforcing**: **YES**
**裁定：CONFIRMED**

```
SR-EVID-E024 Claim: 所有 bridge.tab* 调用点均在 useBrowserStore.ts（14 命中）  Reviewer: E  Classification: FACT  Confidence: HIGH
SR-EVID-E025 Claim: BookmarkPanel.vue:51 注释不会被正则命中（后接中文逗号非 `(`）  Reviewer: E  Classification: FACT  Confidence: HIGH
```

---

## RULE-010 · 破坏型文件系统命令仅限 useWorkspaceStore

**Goal**: `deletePath/writeFile/renamePath/movePath/createFile` 只许在 useWorkspaceStore.ts。
**Actual violation pattern**: **当前 0 命中**（useWorkspaceStore.ts:275/317/335/351/389/684/739/929；UnifiedTabBar.vue:125 仅 revealPath，豁免正确）。
**Can static checker detect it?**: **YES** · **False-positive**: **LOW** · **False-negative**: **MEDIUM-HIGH**（清单不完整）
**Known bypass**: 未纳入清单的其它 FS 变更命令（见 E026）。
**Small-model bypass example**: `await bridge.createDir(path)` —— 与 createFile 同级，不在清单，门禁放行。
**Recommended checker**: Native（FS 边界）· **Worth enforcing**: **YES**（须先补全清单）
**裁定：PARTIALLY_CONFIRMED**

```
SR-EVID-E026 Claim: 清单遗漏真实存在的 `createDir`  Reviewer: E  Classification: FACT
File: src/bridge.ts L266(saveImage) / 497(writeFile) / 505(revealPath) / 508(movePath) / 515(createFile) / 518(createDir) / 520(deletePath) / 522(renamePath)
Observed: `createDir`(L518) 具同等副作用却不在五个正则项内；`saveImage`(L266) 亦是写入  Confidence: HIGH
```

---

## RULE-011 · 凭据命令必经 facade，且前端不持密码

**Goal**: 凭据命令只在新 `useCredentialStore`；前端不得持有明文密码。
**Actual violation pattern**: 直连点实测 **3 处**（round-1 只点 CredentialList，漏 BookmarkPanel）：`CredentialList.vue:44`(list)、`:79`(fill)、`BookmarkPanel.vue:109`(import)。
**Can static checker detect it?**: **PARTIAL**（A 可 / B 否）
**False-positive risk**: **HIGH**（B 实测 100%）· **False-negative risk**: **HIGH**（B）
**Known bypass**: 命名类防护通用缺口：`const pwd = ref("")`；`console.log(getSecret())`。真实红线**已由类型层保证**（E028），用名字复刻必然两头漏。
**Recommended checker**: A: Native(keyring)+UI；B: 应改为 **AST/类型断言**（断言 `BrowserCredentialItem` 不含 password/secret/token 字段）
**Worth enforcing**: **PARTIAL — A: YES-IF（先建 facade）/ B: NO**
**裁定：OVERSTATED（B 是伪规则；A 的 owner 假设可能错误）**

```
SR-EVID-E027 Claim: 拟议正则 B 在真实仓库上**唯一命中是 100% 误报**  Reviewer: E  Classification: FACT
File: src/components/browser/BookmarkPanel.vue L134
Observed: `<input ref="passwordInput" class="hidden-file" type="file" ... />` —— Vue 模板 ref（DOM 句柄名），与"持有密码"无关。TP=0，FP=1。
Confidence: HIGH
SR-EVID-E028 Claim: 真实红线已由**类型层**实现  Reviewer: E  Classification: FACT
File: src/types.ts:174-186; CredentialList.vue:8-15,20-23,68-89,131-132
Observed: 后端只返回 url/username/has_password/credential_id/origin；组件只持 items/loading/error/fillingId；填充只传 credential_id + tabId；UI 只显示 has_password ? "✓ 已保存" : "—"
Why: RULE-011-B 想守的不变式**已成立且由类型和后端边界保证**  Confidence: HIGH
SR-EVID-E029 Claim: facade 不存在；且把所有凭据命令塞进它可能是错的 owner 划分  Reviewer: E  Classification: FACT
Observed: 全仓无 useCredentialStore；`importBrowserCredentials` 的调用者是**书签面板的 CSV 导入流**
Why: 若照执行会把"书签导入"强并入"凭据 store"，制造新的所有权泄露  Confidence: MEDIUM-HIGH
```

---

## RULE-012 · layout.aiNavOpen 死重复真源须删除

**Goal**: 禁止访问 `layout.aiNavOpen`，唯一真源 `browser.aiNavOpen`。
**Actual violation pattern**: 真死物是 `useLayoutStore.ts:146`(声明) 与 `:338`(返回) —— **但拟议正则 `layout\.aiNavOpen` 对这两行都不匹配，全仓实测 0 次**。即：**按 round-1 自己的检测方案，这条规则今天会输出 PASS，从而给一个仍然存在的死重复签发合格证。**
**Can static checker detect it?**: **NO（按拟议方案）/ YES（换用反向计数 + 返回值禁用）**
**Detection method**: symbol scan（不可达导出扫描），而非 `layout\.` 前缀正则
**False-positive**: LOW（换方法后）· **False-negative**: **HIGH**（原方案必漏）
**Worth enforcing**: **YES**（必须换检测手段后）
**裁定：CONTRADICTED（检测方案）/ CONFIRMED（语义）**

```
SR-EVID-E030 Claim: 拟议正则对目标零命中，规则当日假 PASS  Reviewer: E  Classification: FACT
File: useLayoutStore.ts:146(定义), 338(返回); 全仓 `layout\.aiNavOpen` = 0 命中
Observed: 定义写作 `const aiNavOpen = ref(false);`（非 `layout.aiNavOpen`）；返回写作 `aiNavOpen,`
Why: **反驳** round-1 "正则 layout.aiNavOpen → 命中即报（含定义与返回）"  Confidence: HIGH
```

---

## RULE-013 · gridToolbarOpen 跨 store 写入收敛

**Goal**: 只允许 toggleGridToolbar 写；closeGridAll 跨 store 直写（:488）应收敛。
**Actual violation pattern**: 双 writer 属实（useLayoutStore:211 与 useBrowserStore:488）。但 E032：**该标志可能压根没有消费者**，且 round-1 正则会漏掉它打算保留的那个 writer。
**Can static checker detect it?**: **PARTIAL** · **False-positive**: LOW · **False-negative**: MEDIUM
**Known bypass**: `layout.gridToolbarOpen = !layout.gridToolbarOpen`（toggle 风格）→ `(true|false)` 形态完全绕过。
**Recommended checker**: UI · **Worth enforcing**: **NO（先澄清消费者）**
**裁定：PARTIALLY_CONFIRMED（双 writer 事实成立）/ 规则理据很可能建立在不存在的失效模式上**

```
SR-EVID-E031 Claim: round-1 正则匹配不到它打算保留的唯一合法 writer  Reviewer: E  Classification: FACT
File: useLayoutStore.ts:210-215（关键 211）
Observed: L211 写作 `gridToolbarOpen.value = !gridToolbarOpen.value;` —— `(true|false)` 形态失配；`layout\.gridToolbarOpen\s*=` 亦失配
Why: 规则会"只见违规、不见合法"  Confidence: HIGH
SR-EVID-E032 Claim: 该标志在全仓**没有任何消费者** → 宣称的失效模式不可观测  Reviewer: E  Classification: FACT
Observed: 仅 useLayoutStore.ts:144(定义)/211(写)/213,214(自身条件读)/336(返回)，useBrowserStore.ts:488(写)；无任何 .vue 绑定、无 computed/getter/watch 读取
Why: **挑战规则本身的语义** —— 与 RULE-012 属同一类"新兴死重复"；若照 RULE-013 收敛成单 writer，等于为一个死标志立法。本批中最需警惕的"冻结错误语义"候选  Confidence: MEDIUM-HIGH
```

---

## 交叉问题 · SessionCloseDialog "三方口径矛盾"是否 REAL？

**裁定：OVERSTATED（实为两方；被点名的第三方立场相反；唯一冲突方也仅在特定条件下才生效）**

```
SR-EVID-E033 Claim: check-session-persistence-policy.py 与 check-native-webview-overlay.mjs **立场一致**，不是矛盾方  Reviewer: E  Classification: FACT
File: scripts/check-session-persistence-policy.py L17-25; 224-227; 230-231; 564-575
Observed: L21-23 文档串明确"原关闭协议（SessionCloseDialog/...）**必须已撤销、不得残留**"；L230-231 是**违规检测器**；L573-575 是负向自测用例
Why: **反驳** 00-EXECUTIVE-SUMMARY:42 与 16-REVIEW-REPORT:203 把该脚本列为矛盾方（ADR-CHECKER-CONFLICT-001 的一半论据不成立）  Confidence: HIGH
SR-EVID-E034 Claim: 唯一真实冲突方是 check-ui.mjs，但今天是**潜在**冲突  Reviewer: E  Classification: FACT
File: scripts/check-ui.mjs L33-46(37), 49-54(53), 194-243, 409; src/components/browser/SessionCloseDialog.vue(不存在)
Observed: ① 全仓 SessionCloseDialog 文件数=0 → L223 existsSync 为 false → 结构检查空转；② 但 L37/L53 **allowlist 是活着的**——若小模型重建该组件，check-ui 会放行而 check-native 会 FAIL
Why: round-1"直接矛盾"今日跑不红，属**潜在矛盾**；最小修复是删 check-ui.mjs:37 与 :53 两条 allowlist + 删 L222-243 空转块  Confidence: HIGH
SR-EVID-E035 Claim: check-native.mjs 完全不涉及 FE 侧 RULE-006/007 语义  Reviewer: E  Classification: FACT
File: scripts/check-native.mjs L25-30; 42-78
Why: RULE-006/007 落到 check-native.mjs 名下不合适，需新建 FE 侧脚本  Confidence: HIGH
SR-EVID-E036 Claim: "源码级解析是唯一可行"的前提已被同仓兄弟脚本证伪  Reviewer: E  Classification: FACT
File: check-grid-close-logic.mjs L12-15 vs check-client-navigation-logic.mjs L19-60, 228-237
Observed: 后者用 nodeModule.registerHooks + 最小 window 桩，**直接 import 真实 useLayoutStore.ts** 并对活值断言
Why: **推翻** round-1 为 RULE-001/002/003/008 选"正则+函数体提取"的核心理由；RULE-002 应改为运行态取值断言  Confidence: HIGH
SR-EVID-E037 Claim: setView 的清理面被过度描述  Reviewer: E  Classification: FACT
File: useLayoutStore.ts L192-199 — 仅 `mainView.value=v` + fileEditorOpen + navSection；注释写明"仅做 UI 状态切换"
Why: 校准 RULE-003 价值：收益真实但**仅限两项 UI 清理**  Confidence: HIGH
```

---

## 汇总判定表

| RULE | Verdict | READY_FOR_PHASE_0 | 最短落地前提 |
|---|---|---|---|
| 001 gridOpen 写点 | PARTIALLY_CONFIRMED | **PARTIAL / YES-IF** | 加 $patch 禁用 + toggle 形态 + 写点计数冻结 |
| 002 isBrowserVisible 公式 | **CONFIRMED** + UNDERSTATED(G1#4) | **PARTIAL / YES-IF** | 同时修 G1#4；优先改运行态取值断言 |
| 003 mainView 裸赋值 | PARTIALLY_CONFIRMED | **PARTIAL / YES-IF** | 改 AST 成员写检测 + 删不可达白名单 + 禁 $patch |
| 004 组件手拼宫格 | **OVERSTATED** | **NO** | 先加 ensureGrid()/exitGrid() store action，迁移 7 站点；弃用 regex B |
| 005 IPC typed-invoke | **CONFIRMED** | **YES** | 只需确定 5 命令入 KNOWN 还是转为 FAIL |
| 006 原生显隐出口收敛 | **CONFIRMED** | **PARTIAL / YES-IF** | 与 RULE-009 共用一份 tabActivate allowlist |
| 007 禁止 position-as-hide | **OVERSTATED**（= 官方机制） | **NO** | 降级 advisory；若坚持须先做黑闪回归 QA |
| 008 activateWeb 谓词 | **CONFIRMED**（已合规） | **PARTIAL / YES-IF** | 改措辞为"不得依赖 isBrowserView"；用可处理同步函数的提取器 |
| 009 页签命令裸调 | **CONFIRMED** | **YES** | 与 RULE-006 合并同文件 allowlist |
| 010 破坏型 FS 命令 | PARTIALLY_CONFIRMED | **PARTIAL / YES-IF** | 补 createDir（及写入类）后再落地 |
| 011 凭据 facade / 无明文密码 | **OVERSTATED** | **NO**（B 明确否决） | A：先建 facade 且重新裁定 owner；B：改类型断言 |
| 012 layout.aiNavOpen | **CONTRADICTED**（检测）/ CONFIRMED（语义） | **NO** | 断言改为"计数==0 且返回块不含 aiNavOpen" |
| 013 gridToolbarOpen | PARTIALLY_CONFIRMED / 理据存疑 | **NO** | 先查清是否真有 UI 消费者（证据显示无） |

**计数**：`ready = 2`（005, 009）· `partial = 7`（001, 002, 003, 006, 008, 010, 011-A）· `rejected = 4`（004, 007, 012, 013；011-B 单独否决）

---

## 给 Round-1 的三条反向要求（Phase 0 前必须回应）

1. **不要用正则替代编译器或类型系统。** RULE-005 的"createGrid 须同时传 n 与 urls"是 TS 已保证的；RULE-011-B 的"前端不持明文密码"是 types.ts:177-186 已保证的。把这两类不变式正则化只会得到 FP 100%（E027 实测）且漏掉真正攻击面。
2. **先有红灯够少的 statutory surface。** RULE-004 落地当日产生 7 条红灯而迁移目标 `exitGrid` 不存在（E011）——几乎必然导致"把这 7 处加进白名单"这一最廉价的合规动作，**反而把隐性债固化成受祝福的模式**。必须先落 store action。
3. **澄清 RULE-013 之后再动手。** 现有证据（E032）显示 `layout.gridToolbarOpen` 与 RULE-012 的 `layout.aiNavOpen` 是同一类"新兴死重复"：为它规范 writer 数量，是在为一个没有消费者的标志立法。建议 Phase 0 先做"不可达导出扫描"，一次性回答这两个标志的存废。
