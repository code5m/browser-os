# Overnight Release Train — HANDOFF / 恢复点

> 无人值守长任务的外部长期记忆。**新 Agent（或 context 切换后）先读本文件 + `phase8c0/` 文档 + semantic registry + capability registry，再继续**。
> 更新时间：2026-09-20（Overnight Release Train）。

## 运行基线

- PROJECT: `/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3`
- BRANCH: `feature/capability-platform-v1`
- OVERNIGHT_START_HEAD: `aef86207179d8712b71f74aeefbf4ecad1fe7c89`（= v8c0a-files-owner）
- OVERNIGHT_FSCK: OK（仅 dangling，无 corrupt）
- 硬约束：**不 push / 不 merge master / 不移旧 tag / 不 sudo / 不动用户数据 / 不覆盖已装**

## 进度

| Train | 范围 | 状态 | tag / HEAD |
|-------|------|------|-----------|
| A | Workspace 单体分解（8C-0A..0E） | **PASS** | capability-phase8c0-workspace-decomposition-pass |
| B | Workspace 物理 capability 隔离 + C3 | **PASS** | capability-phase8c-workspace-composable-pass (9074b57) |
| C | Browser/Grid 隔离 + C3 | **PASS** | capability-phase8d-browser-composable-code-pass (d6a2134) |
| D | Terminal 隔离 + C3 | **PASS**（Terminal = C3；CURRENTLY_COMPOSABLE 3→4） | capability-phase8e-terminal-composable-code-pass |
| E | Developer family（Database/Git）边界审计 + 能力化 | **PASS**（边界固化；Database/Git = C1，未强拆 C3） | capability-phase8e-developer-family-audit-pass |
| F | Resource Governor + Profiles | **PASS**（真实 profile minimal 零 PTY/零 WebView；Governor 薄层；测量 UNKNOWN 不估算） | capability-phase8e-resource-governor-profiles-pass |
| G | Final acceptance + red team | **PASS**（全门禁+自检绿；红队无 HARD STOP） | capability-modularization-v1-code-pass |

**CURRENTLY_COMPOSABLE = 4**（Bookmark C3 + Workspace C3 + Browser C3 + Terminal C3）。目标已达成（≥4）。Database/Git = C1（边界固化，未强拆 C3）。**Train A–G 全部 PASS，FINAL tag 已打。**

### Train D 摘要（PASS，Terminal = C3）

- **owner 迁移**（走 AUDIT → SCR → REGISTRY → CODE → CHECKER → TEST）：`terminal` 的 owner 符号由
  `useSystemStore`（Terminal+Clipboard+Apps 混居，Debt-7A-2）抽为 **`useTerminalStore`**
  （`src/capabilities/terminal/state/useTerminalStore.ts`）。11 个 state / 7 个 intent **零语义变更**。
  `useSystemStore` 收敛为 **Clipboard + Apps**（剩余共居 = Debt-8E-1，显式未静默）。
- **物理结构**：`src/capabilities/terminal/{manifest,public,index}.ts` + `state/useTerminalStore.ts` +
  `ui/{TerminalPane,TerminalView,TerminalDockPanel}.vue` + `ui/useTerminalResize.ts`。
  未建 `adapters/`（与 bookmark/workspace/browser 形态一致；Debt-8E-2）。
- **贡献**：`terminal.main.term` → **新通用槽 `workbench-main-resident`**（常驻主视图，显隐由能力自管，
  保住「切走不卸载 xterm」）；`terminal.dock.term` → `browser-dock`。
- **Shell 零 Terminal 知识**：MainArea 删掉 TerminalPane / 终端视图 / Dock 终端硬编码 / `ensureTerm` watch；
  StatusBar / App / UnifiedTabBar / ActivityBar 经 `capabilities/terminal/public` 只读消费；
  Workspace 的「在终端打开」走 shared 窄缝 `src/composables/terminalNav.ts`（不新增 workspace→terminal 依赖边）。
- **C3 ABSENT 真证明**：PTY 出生点唯一 = `ui/TerminalView.vue`（挂载即 `ensureTerm`）；
  absent → 槽为空 → 组件不挂载 → 零 `term_spawn_channel` invoke。新门禁 `check-terminal-owners.mjs`
  给出静态 8 条 + 真实 bootstrap 冒烟 3 条 + 真实 store 行为 9 条 = **20/20**。
- **门禁**：composition 22→**33/33**；新 terminal-owners **20/20**；terminal-policy 自检 29→**30 变异**且
  默认门禁由 **FAIL→PASS**（既有检测器失配修复）；terminal-ui-logic 由**崩溃→32 断言 PASS**；
  pilot PLT-04/05 重基线 8/8；semantic registry `fail=0 warn=6 info=72` 与迁移前逐位一致。
- **成熟度**：Terminal = **C3**（C4 无物理 suspend、C5 未做真机资源释放实测 → **不高报**）。
- 债务：Debt-7A-2 **CLOSED**；新增 Debt-8E-1/2/3/4（显式）。

### Train E 摘要（PASS，Developer family 边界审计 + 能力化；Database/Git = C1）

- **审计结论**：Database（`useDatabaseStore`）、Git（`useGitStore`）、Repo（`useRepoStore`）三 owner
  **物理分离**，无合并 `DeveloperStore`；Repo Context ≠ Git Operation；Credential 只经后端密钥库
  引用（前端零持久化）。三者此前已在 Semantic/Capability Registry 登记为独立 owner 的 CAPABILITY。
- **边界机器固化**：新增 `scripts/check-developer-owners.mjs`（**11/11 PASS**）：
  DEV-01 三 owner 分离 / DEV-02 各 owner 声明自身状态 / DEV-02b 无 `useDeveloperStore` /
  DEV-02c 无第二真源 / DEV-03 凭据只经引用 / DEV-04 Repo≠Git / DEV-05 跨能力不互相 import 内部。
  已接入 `npm run check` 与 `pre-merge.sh` Phase 03。
- **成熟度诚实裁决（不高报）**：Database / Git 当前是 **always-loaded Workspace 面板**
  （DatabasePanel/GitPanel 在 `src/components/workspace/`），connect 即建真实 DB 连接、操作即 spawn
  git 子进程（重资源），**非 absent-composable**，故记为 **C1**。未强行抽为可选能力包（避免触碰
  Workspace 表面 + 稀释边界清晰度的无语义迁移）。CURRENTLY_COMPOSABLE 仍为 **4**（未新增）。
  物理抽取为可选能力包达到 C3 = **Debt-8E-5（Database）/ Debt-8E-6（Git）**，交后续 train。
- 既有 `check-database-policy.py` / `check-database-ui-logic.mjs` 的失配（pre-merge 历史 FAIL）属 DB
  后端管道语义，独立于能力边界，本 Train 未强行修复（不归并以稀释 Terminal 债已修的清晰度），
  仅以 DEV 门禁在边界维度补位。

## 已成 capability 的形态（Bookmark / Workspace / Browser 同一模式，可直接复用）

```
src/capabilities/<id>/
  manifest.ts   CapabilityDefinition（semanticOwner=已冻结 owner；status=COMPATIBILITY_WRAPPED；activatable=true；dependsOn 只列 TS 运行时可解析的已注册能力）
  public.ts     纯再导出（外部/Shell 经此消费，不 import 内部）——注意 check-capability-boundaries 的 PUBLIC_FILES 已含 public.ts
  index.ts      defineAsyncComponent + contributionRegistry.registerContribution(...) + <id>Capability（onActivate=register...）
  state/        owner store（物理迁入；owner_implementations locator 首候选指向此处）
  ui/           能力 UI 组件（Shell 经 contribution 渲染，不 import）
```

- 通用 Contribution 契约：`src/capability/contribution/types.ts`（slots: browser-sidebar/address-bar-actions/activity-bar-trailing/**workbench-main**/**browser-host**/**browser-dock**；Contribution.view 用于按 layout.mainView 认领视图）。
- bootstrap：`src/capability/index.ts` 依次 register/resolve/activate bookmark → workspace → browser。
- Shell host：`src/components/layout/MainArea.vue` 用 `viewOf(view)`/`dockOf(view)`/`browserHostComp` 渲染。
- 门禁：`scripts/check-capability-composition.mjs`（Bookmark C1-C4 + Workspace C5-WS-*/C6-WORKSPACE-ABSENT + Browser C5-BR-*/C6-BROWSER-ABSENT，22/22）、`check-capability-boundaries.mjs`、`check-capability-registry.mjs`、`check-workspace-owners.mjs`、`check-semantic-registry.mjs`。

## Train C 摘要（PASS，Browser = C3）

- `useBrowserStore` → `state/`；`BrowserHost`（原生宿主）+ `ResourceWaterfall` + `SessionPanel` → `ui/`。
- 贡献：`browser-host`（原生宿主，absent → 不创建 webview）+ `browser-dock`（net/session）。
- Shell→`capabilities/browser/public`；MainArea 经贡献渲染。
- **双向耦合解法**：浏览器 ↔ workspace 互读（浏览器写 recents、workspace 读浏览器 url）。用 `src/composables/{browserNav,recentsNav}.ts` 窄缝（shared，门禁不判跨能力）打断直接依赖；方向依赖 = `bookmark→browser`、`workspace→browser`（required），`browser→bridge`。**无环**（CB-04 仅剩历史 optional 环 agent↔knowledge_graph）。
- Grid 保持 Browser heavy 子资源面（未强制独立 capability；冻结语义 ADR-P1A-1/2/3/10、ADR-SEM-P6A-1/3 未改）。
- 多 checker 的 `src/stores/useBrowserStore.ts` 路径已统一迁移（scripts/ 内 sed）。
- 验收：build PASS；runtime `bootstrap activated=true(3 能力) / vue mounted / 0 真实 error`；semantic PASS+ALL_PASS；boundaries PASS(0 fail)+self-test 12/12；registry PASS(0 fail)；composition 22/22(+self-test 3/3)；closure 27/27；npm run check PASS。

## Train D 实施前预读（已执行，保留作历史切片）

> 状态：**Train A–G 全部完成**。FINAL tag = `capability-modularization-v1-code-pass`。
> CURRENTLY_COMPOSABLE = 4；Database/Git = C1（边界固化）。Human GUI 验收仍由用户完成。

### Train F/G 摘要（PASS）

- **Train F（Resource Governor + 真实 Profile）**：`src/capability/profiles.ts` 定义单源
  `CAPABILITY_PROFILES`（minimal=bookmark+workspace / developer|full=+browser+terminal）；
  `bootstrapCapabilityRuntime(profile)` 只注册列出的能力 → minimal 端到端零 PTY/零 WebView（真实
  profile，非 UI hide）。`src/capability/resourceGovernor.ts` 为薄协调层（只持 runtime 引用，零业务
  state），activate/background/suspend/hibernate/destroy 全委托 rt 公共 API，不绕过 owner。Terminal
  生命周期诚实改为 `supported: [ACTIVE]`（suspendable:false），suspend 抛 SUSPEND_NOT_SUPPORTED →
  不谎报 C4/C5。新增 `check-composition-profiles.mjs`（11/11）。`measure-resources.mjs` 真实 /proc 测量，
  分类 MEASURED/DECLARED/UNKNOWN（headless 无实例 → UNKNOWN，不估算）。
- **Train G（终检 + 红队）**：全门禁 + 自检绿（semantic fail=0 warn=6；composition 33/33；
  terminal-owners 20/20；terminal-policy 30 变异；terminal-ui-logic 32；developer-owners 11/11；
  composition-profiles 11/11；pilot 8/8；npm run check/build PASS；git diff --check clean；fsck clean）。
  红队 §16 攻击项逐项无 HARD STOP。`check-capability-pilot` PLT-04 解析器修复（Train F 引入的回归）。
  已知 pre-merge RED（尾随空白 + grid-close checker）= M4/M5 历史债，非本系列引入，未掩盖。

前置：复读 Terminal 冻结语义（states.yaml `terminalOpen/termPanes/termGrid/termGridCount/activeTermId/autoConfirmCli/termProbeOn/m0Cfg/m0StartTs/droppedChunks/droppedBytes`；owners.yaml `terminal`；intents.yaml terminal 段）。

**已知耦合（必须显式处理，不得 blanket ignore）**：
- Debt-7A-2：`useSystemStore` 同时是 **Terminal 与 Clipboard** 的 owner（clipText/clipHistory/apps/appFilter/filteredApps 也在其中）。→ 移动整个 store 到 `capabilities/terminal/state/` 时，clipboard 状态**不属** Terminal 语义；建议：Terminal 只声明 terminal* 状态归属，clipboard 状态登记为「同文件内的异域状态」债务（或本阶段就把 clipboard 拆到 `useClipboardStore`——属 Train E/后续）。
- PTY 子进程由 `bridge.term_spawn_channel`（前端唯一入口）+ Rust `term_*` 命令驱动；`TerminalPane.vue` 挂载即 spawn。

建议切片（每步 green）：
1. `git mv src/stores/useSystemStore.ts src/capabilities/terminal/state/`（或先拆分 clipboard）。
2. `git mv` TerminalPane（+ 终端宫格相关）→ `capabilities/terminal/ui/`；MainArea 终端视图 + dock 终端改经贡献（workbench-main view=term / browser-dock view=term）。
3. manifest/public/index；贡献 `terminal-host` 或复用 workbench-main view=term。
4. Shell（App/ActivityBar/StatusBar/MainArea）→ `capabilities/terminal/public`。
5. registry：states locator（useSystemStore 路径）、capabilities.yaml(terminal COMPATIBILITY_WRAPPED/activatable)、dependencies.yaml、composition 增加 Terminal C5/C6（absent → 不创建 PTY 子进程）。
6. 验收：**absent → 不创建 PTY / destroy → child cleanup**（C3；若真实证明 release → C4/C5，禁止推测）。
7. tag `capability-phase8e-terminal-composable-code-pass`。

随后 Train E（Database/Git，credential 边界只经 reference）、Train F（Resource Governor + Minimal/Developer/Full profiles，真实测量）、Train G（终检 + 红队 + delivery docs）。

## 速查命令

```
node scripts/check-semantic-registry.mjs [--self-test]
node scripts/check-workspace-owners.mjs
node scripts/check-capability-boundaries.mjs [--self-test]
node scripts/check-capability-registry.mjs [--self-test]
node scripts/check-capability-composition.mjs [--self-test]
node scripts/check-semantic-closure-logic.mjs
npm run build   &&   npm run check
bash scripts/pre-merge.sh   # 全门禁（含 cargo，较重）
```

## 继承约束

- owner 变更：AUDIT → ADR/SCR → REGISTRY → IMPL → CHECKER → TEST，禁止静默迁移。
- 禁止 wrapper / 第二状态真源 / shared 垃圾桶（窄缝如 browserNav/recentsNav 属合法 shared 接口，非垃圾桶）/ facade 永久化；禁止降低门禁换 PASS。
