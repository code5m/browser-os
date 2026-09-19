# Phase 6A — Migration Report

> 范围：仅审计确认的三项真正 `MIGRATION_REQUIRED`（M2-a / M2-b / M2-c）。不涉及 M4 其余 14 域、Terminal/Plugin/MCP/Clipboard/Script/Agent。

## 变更清单（代码）

| # | 文件 | 变更 | 依据 |
|-|-|-|-|
| 1 | `src/stores/useBrowserStore.ts` | 新增 `toggleAiNav()` 唯一 toggle 入口并导出 | ADR-SEM-P6A-1 |
| 2 | `src/stores/useLayoutStore.ts` | 删除死重复 `aiNavOpen`（声明 `:146` + 导出 `:352`） | ADR-SEM-P6A-1 |
| 3 | `src/components/layout/ActivityBar.vue` | AI 导航开关改走 `browser.toggleAiNav()` | ADR-SEM-P6A-1 |

> M2-b（panel）与 M2-c（gridSession）为**权属澄清/登记**，无代码行为变更（`gridSession` 本已是单一真源、面板本就独立）。

## 变更清单（Registry）

| # | 文件 | 变更 |
|-|-|-|
| 4 | `docs/architecture/semantic-registry/states.yaml` | `aiNavOpen` 升为 GOVERNED（owner=useBrowserStore，canonical_writer=toggleAiNav/gotoAI，forbidden_writers=useLayoutStore）；移出 observed |
| 5 | `docs/architecture/semantic-registry/owners.yaml` | `browser_grid_lifecycle` 增 `aiNavOpen` |
| 6 | `docs/architecture/semantic-registry/states.yaml` | 面板开关 `sidebarOpen/clipOpen/fileEditorOpen/browserDockOpen/browserDockTab` 升为 GOVERNED（owner=useLayoutStore）；移出 observed |
| 7 | `docs/architecture/semantic-registry/states.yaml` | `gridSession` 升为 GOVERNED（owner=useBrowserStore，adr=ADR-SEM-P6A-3）；移出 observed |
| 8 | `docs/architecture/semantic-registry/owners.yaml` | `browser_grid_lifecycle` 增 `gridSession` |

## Before / After

### M2-a aiNavOpen

```
BEFORE: useBrowserStore.aiNavOpen (真源) + useLayoutStore.aiNavOpen (死重复)
AFTER : useBrowserStore.aiNavOpen 唯一真源（layout 副本已删）
```

### M2-b Panel

```
BEFORE: 5 个面板开关处于 observed_not_governed（已登记未治理）
AFTER : 5 个面板开关 GOVERNED，各自身份/owner 明确；bmPanelOpen 明确为派生（不入 Registry）
```

### M2-c gridSession

```
BEFORE: gridSession 处于 observed_not_governed（权属未明）
AFTER : gridSession GOVERNED（内存缓存失效纪元；writer=buildGrid/forceGridRelayout；只读消费=useBrowserHost）
```

## 未做（明确不做）

- 不合并任何面板（各异域）。
- 不删除 `observed_not_governed` 其余条目（M4 域留待后续）。
- 不触碰 gridSession 的运行时代码（本已单真源）。
- 不引入 vitest / 新测试框架（项目现无前端单测，改走既有 `.mjs` checker 约定）。
