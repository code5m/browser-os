# FINAL_AI_CHANGE_SURFACE_REPORT（PHASE F-A）

> 目的：回答「普通 AI/开发者修改一个局部能力时，是否需要理解整个仓库？」
> 方法：**真实仓库取证**（非理论），逐例列出最少必读文件与验证命令。
> 结论口径：PASS = 边界可导航 + owner 可定位 + 验证命令明确 + **无明显全仓认知依赖**。

---

## CASE A — Skill UI 变更（不改 Agent）

**任务**：修改 `SkillManagerPanel` 的 UI 行为。

**最少必读**
| 文件 | 作用 |
|---|---|
| `src/capabilities/skill/ui/SkillManagerPanel.vue` | 面板本体 |
| `src/capabilities/skill/state/useSkillStore.ts` | skill 域 owner |
| `src/capabilities/skill/public.ts` | 能力公开契约 |
| `src/shared/ui/*`（EmptyState/ContextMenu） | 共享 UI |
| `src/utils/agentSkillUi.ts` | **共享纯逻辑**（非 agent internal store） |

**是否必须读 Agent internal store？NO**
- 取证：`grep -rn "agent" src/capabilities/skill` 仅命中
  `from "../../../utils/agentSkillUi"`（纯函数层），
  **无** `capabilities/agent/**` 的 internal import。
- 旁证：`check-capability-boundaries`（CB-01 fail=0）。

**验证命令**：`npm run check`；`node scripts/check-capability-registry.mjs`
**结论：PASS**

---

## CASE B — Browser native close/destroy lifecycle

**任务**：修改 Browser 关闭/销毁生命周期。

**最少必读**
| 文件 | 作用 |
|---|---|
| `src/capabilities/browser/public.ts` | public 契约（纯再导出） |
| `src/capabilities/browser/state/useBrowserStore.ts` | lifecycle owner（`closeBrowser()` @564） |
| `docs/architecture/native-boundary/native-commands.yaml` | `open_browser`/`close_browser` → owner=browser, resource=WEBVIEW |
| `src-tauri/src/bridge.rs` | native 实现 |
| `scripts/check-native-capability-boundaries.mjs` | NATIVE-03/04 |
| `scripts/runtime-resource-absence.mjs` | RRA-01/02/07 |

**是否需要理解 Terminal / Workspace internals？NO**
- browser 包自含 `state/ui/public/manifest/index/resource`；`closeBrowser` 只调 `bridge.closeBrowser()`。
- Shell 侧仅 `App.vue:226` 调用 `browser.closeBrowser()`（public 动作，**非** `bridge.` 直连）。

**验证命令**：`node scripts/check-native-capability-boundaries.mjs`、`node scripts/runtime-resource-absence.mjs`
**结论：PASS**

---

## CASE C — 新增一个简单 Capability

**注册点（仅）**
- `src/capability/index.ts` → `ALL_CAPABILITIES`（:38）
- `src/capability/platform/catalog.ts` → `CATALOG_SOURCES`（:27）

**是否需要修改？**
| 目标 | 结论 | 证据 |
|---|---|---|
| `App.vue` 业务 switch | **NO** | App.vue 无能力面板 switch；面板经贡献渲染 |
| `MainArea` 业务 switch | **NO** | `MainArea` 用通用 `viewOf(view)` 遍历 `WORKBENCH_MAIN` 贡献 |
| Dock 硬编码页签列表 | **NO** | `dockTabs` 来自 `contributionRegistry.getDockTabContributions`（UI-4） |
| ActivityBar 能力专属 switch | **PARTIAL（已知债务）** | `ActivityBar.vue:334-335` 存在硬编码 view 键（`'files'/'term'`）的快捷按钮，导航清单 `menuSections` 亦内置 → 登记 **Debt-H-2（NON_BLOCKING / FUTURE）** |

**判断**：新增能力的功能面（state/ui/public/manifest/contribution）**完全在自身包内**；
仅「把入口放进导航 UI」需碰 ActivityBar —— 这是**已知且已登记的债务**，不构成全仓认知依赖。
**验证命令**：`npm run check`；`node scripts/check-capability-composition.mjs`
**结论：PASS（附 Debt-H-2 说明）**

---

## CASE D — 修改 shared EmptyState

**消费者可枚举（无需全文扫描）**
- 定义：`src/shared/ui/EmptyState.vue`、导出于 `src/shared/ui/index.ts`
- 消费者（14 处，可机器列出）：database / clipboard / task / workspace(Artifact, Repo, ScriptRunHistory, CommandSnippetPanel, ScriptPanel) 等能力面板 + shared 自身。
- 取证命令：`grep -rln "EmptyState" src`

**边界保证**：`shared/ui` 不得吸收业务语义 —— 由 `check-ui-boundaries`（vacuous=0）与
`docs/architecture/ui-system/SHARED-UI-CONTRACT.md` 约束。
**验证命令**：`node scripts/check-ui-boundaries.mjs`；`npm run check`
**结论：PASS**

---

## CASE E — Database connection lifecycle 变更

**修改面**
| 层 | 文件/边界 |
|---|---|
| capability | `src/capabilities/database/{manifest,public,index}.ts`、`ui/` |
| credential 边界 | 经 credential public/`credential` 能力（不直取 keyring） |
| native/resource | `native-commands.yaml`（`db_connect/db_query/db_disconnect/db_cancel` → owner=database, resource=DB_CONNECTION） |
| checker | `check-native-capability-boundaries`（NATIVE-04/05）、`check-sensitive-side-effects`（R7）、`check-capability-resource-boundary` |

**取证**：`grep -rn credential src/capabilities/database` 仅命中契约层引用，未直连 Rust keyring。
**验证命令**：`node scripts/check-sensitive-side-effects.mjs`、`node scripts/check-capability-resource-boundary.mjs`
**结论：PASS**

---

## CASE F — Native 命令归属可机器定位（抽样 5）

| 命令 | owner | resource | callers |
|---|---|---|---|
| `open_browser` | browser | WEBVIEW | `src/capabilities/browser/**` |
| `term_spawn` | terminal | PTY | `src/capabilities/terminal/**` |
| `git_status` | git | GIT_PROCESS | `src/capabilities/git/**` |
| `db_connect` | database | DB_CONNECTION | `src/capabilities/database/**` |
| `fill_browser_credential` | credential | KEYRING | `src/capabilities/browser/**`（secure flow） |

- 真源：`docs/architecture/native-boundary/native-commands.yaml`（**148** 条，机器可读）。
- 定位方式：查 YAML 即得 owner/resource/callers/permission；门禁 `check-native-capability-boundaries.mjs` 消费同一文件（NATIVE-01/06/07），**不是纯文档**。
**结论：PASS**

---

## 度量（§3）

| 指标 | 值 |
|---|---|
| FILES_NEEDED_PER_CASE | 3–6（均在单一能力包内 + 1 个共享层） |
| CROSS_CAPABILITY_INTERNAL_FILES_NEEDED | **0** |
| GLOBAL_FILES_NEEDED | 0–2（注册点 `ALL_CAPABILITIES` / `CATALOG_SOURCES`，非业务逻辑） |
| SHELL_FILES_NEEDED | 0（除导航入口清单这一 FUTURE 债务） |
| NATIVE_FILES_NEEDED | 1（`native-commands.yaml` 定位）+ 1（Rust 实现） |

## 结论

**AI_MAINTAINABILITY = PASS**

六个案例均证明：边界可导航、owner 可定位、验证命令明确、无全仓认知依赖。
唯一真实残留耦合（ActivityBar 导航入口清单）已作为 **Debt-H-2（NON_BLOCKING / FUTURE）** 显式登记，不谎报为零。
