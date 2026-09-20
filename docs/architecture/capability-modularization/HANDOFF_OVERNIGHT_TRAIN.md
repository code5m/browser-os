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

| Train | 范围 | 状态 | tag |
|-------|------|------|-----|
| A | Workspace 单体分解（8C-0A..0E） | **PASS** | capability-phase8c0-workspace-decomposition-pass |
| B | Workspace 物理 capability 隔离 + C3 | **PASS** | capability-phase8c-workspace-composable-pass |
| C | Browser/Grid 隔离 + C3 | 进行中/待做 | — |
| D | Terminal 隔离 + C3 | 待做 | — |
| E | Developer family（Database/Git） | 待做 | — |
| F | Resource Governor + Profiles | 待做 | — |
| G | Final acceptance + red team | 待做 | — |

**CURRENTLY_COMPOSABLE = 2**（Bookmark C3 + Workspace C3）。目标 ≥4。

## Train A 摘要（PASS）

`useWorkspaceStore`（God Store）→ `useFileStore` / `useArtifactStore` / `useRepoStore` / `useScriptStore` / `useSnippetStore` + Workspace Core（audit+recents+refresh）。UNKNOWN=0 / SECOND_TRUTHS=0。详见 `phase8c0/04-WORKSPACE-CORE-CLOSEOUT.md`。

## Train B 摘要（PASS，Workspace = C3）

- 物理隔离：`src/capabilities/workspace/{state/,ui/,manifest.ts,public.ts,index.ts}`。6 个 owner store 物理迁入 `state/`；9 个 workspace 面板迁入 `ui/`。
- 公共边界：`public.ts`（纯再导出，Shell 与外部消费经此）。
- 通用贡献：`index.ts` 经 `contributionRegistry.registerContribution` 注册 `workbench-main`（view=files/arts/repo/scripts/commands/audit/editor）+ `browser-dock`（view=files）。
- Contribution 契约扩展：新增 `view?` 字段 + `CONTRIBUTION_SLOTS.WORKBENCH_MAIN` / `BROWSER_DOCK`。
- Shell 解耦：`MainArea.vue` 用 `viewOf(view)`/`dockOf(view)` 按贡献渲染，不再 import Workspace 内部；App/ActivityBar/StatusBar/UnifiedTabBar 经 `capabilities/workspace/public`。
- C3 ABSENT：`check-capability-composition.mjs` 新增 C5-WS-* + C6-WORKSPACE-ABSENT（16/16 PASS）。
- 回归修复：`GitPanel.vue` 的 `ws.repos` → `useRepoStore().repos`（8C-0C 遗留）；删除死组件 `Sidebar.vue`。
- 门禁：build PASS；runtime `bootstrap activated=true / vue mounted / view=files 渲染`；semantic PASS(+self-test)；cap boundaries PASS(0 fail)+self-test 12/12；cap registry PASS(0 fail)+self-test 13/13；composition 16/16；npm run check PASS；closure 27/27。

## 关键恢复点（annotated tags，最新在上）

- `capability-phase8c-workspace-composable-pass` (9074b57) — Train B / Workspace C3
- `capability-phase8c0-workspace-decomposition-pass` (eb172d4) — Train A
- `capability-phase8c0d-script-snippet-owner-pass` (1388166)
- `capability-phase8c0c-repo-owner-pass` (a6ddf34)
- `capability-phase8c0b-artifact-owner-pass` (a6ca383)
- `v8c0a-files-owner` (aef8620)
- （历史）`capability-phase8b-bookmark-composable-pass`

## 下一步（Train C: Browser/Grid → C3）

先**复读 Phase 1 冻结语义**（`docs/architecture/semantic-registry/{states,owners,intents,side-effects}.yaml` 中 Browser/Grid 段；ADR-P1A-1/2/3/10、ADR-SEM-P6A-1/3）。冻结项：`mainView` / `gridOpen` / `desiredGridVisibility` / `isBrowserVisible` / `gridSession` / `aiNavOpen` / HIDE-vs-DESTROY / `closeGrid` 语义——不得重新设计。

建议路径（保持每步 green）：
1. 新建 `src/capabilities/browser/`（manifest/public/index/ui/state/adapters）。Browser=CAPABILITY，Grid=其 heavy 子资源面（**先证伪再拆**）。
2. `src/stores/useBrowserStore.ts` → `state/`（更新 owner_implementations locator + governed_files）。
3. Browser UI（BrowserHost/ResourceWaterfall/SessionPanel/TopBar 的浏览器部分/GridArchiveBar 等）→ `ui/`；Shell 经 contribution（新槽如 `browser-surface` / 复用 workbench-main view=browser/grid）渲染。
4. 原生 webview 调用集中到 `adapters/`（bridge），满足 CB-07 精神；Runtime 不得持 webview 业务态。
5. C3 ABSENT：Browser 未注册 → Shell 启动、不创建 webview；Workspace/Bookmark 仍成立。
6. 成熟度：最低 Browser=C3；若真实 runtime release 证明 → C4/C5（禁止推测）。tag `capability-phase8d-browser-composable-code-pass`。

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
- 禁止 wrapper / 第二状态真源 / shared 垃圾桶 / facade 永久化；禁止降低门禁换 PASS。
