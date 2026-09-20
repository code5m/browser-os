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
| D | Terminal 隔离 + C3 | **NOT_REACHED**（下一步） | — |
| E | Developer family（Database/Git） | NOT_REACHED | — |
| F | Resource Governor + Profiles | NOT_REACHED | — |
| G | Final acceptance + red team | NOT_REACHED | — |

**CURRENTLY_COMPOSABLE = 3**（Bookmark C3 + Workspace C3 + Browser C3）。目标 ≥4（差 Terminal）。

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

## 下一步（Train D: Terminal → C3，达成 CURRENTLY_COMPOSABLE=4）

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
