# NAVIGATION-GOVERNANCE（STAGE H-E）

> 审计对象：`mainView` 枚举、`useLayoutStore` 的 activate*、`ActivityBar`、`MainArea` 视图渲染、
> `Home` / `Settings` 导航、能力贡献（`WORKBENCH_MAIN` / dock）。
> 原则：能力专属导航原则上应 contribution-driven；**Workbench 固有导航允许固定**；
> **不为「全部 contribution」破坏成熟 UX**。**UNKNOWN = 0**。

## 1. 分类定义

| 分类 | 含义 |
|---|---|
| `WORKBENCH_FIXED_NAV` | Workbench/Shell 自身固有导航（主页、设置、框架兜底），允许固定 |
| `CAPABILITY_CONTRIBUTED_NAV` | 由能力贡献驱动（view/槽/元数据），Shell 零能力专属知识 |
| `LEGACY_HARDCODED_CAPABILITY_NAV` | Shell 中硬编码的**能力专属**视图入口（应贡献化，但变更风险需评估） |
| `INVALID` | 无法归类的硬编码（必须为 0） |

## 2. 现状裁决

### 2.1 CAPABILITY_CONTRIBUTED_NAV（已达成的部分）

- 主区面板：全部经 `WORKBENCH_MAIN` 贡献，由 `MainArea` 的通用 `viewOf(view)` 渲染
  （`files/arts/repo/scripts/commands/audit/editor` ← workspace；
  `clip` ← clipboard、`apps` ← apps、`tools` ← tools、`db` ← database、
  `tasks` ← task、`plugin` ← plugin、`skills` ← skill、`agents` ← agent、
  `graph` ← graph、`vault` ← vault、`home` ← home、`settings` ← settings）。
- Dock 页签：`BROWSER_DOCK` 贡献驱动（UI-4），capability absent → 页签自动消失（无死页签）。
- 浏览器侧栏：`BROWSER_SIDEBAR` 贡献驱动。

### 2.2 JUSTIFIED_FIXED_NAVIGATION（允许固定，附理由）

| 项 | 位置 | 理由 |
|---|---|---|
| `home` 视图作为默认启动视图 | `useLayoutStore`（默认 `mainView`） | Home 是 Workbench 默认表面；即使 home 能力缺席，也需一个确定的默认视图（否则主区空白） |
| `settings` 入口 | Shell 设置入口 / `view='settings'` 贡献 | settings 是**框架 SERVICE**（STAGE I-C），其入口属 Workbench 固有；面板本体已贡献化 |
| `activateHome/activateBrowser/...` 语义助手 | `useLayoutStore` | 这些是**视图切换语义封装**，供快捷键/全局动作复用；切换目标本身不改变能力归属 |
| MainArea 未知视图兜底（第 199 行白名单） | `MainArea.vue` | 防止「未知 view → 主区空白/死区」的**框架兜底**；非导航菜单 |

### 2.3 LEGACY_CAPABILITY_NAVIGATION（诚实登记为债务）

| 项 | 证据 | 影响 | 处置 |
|---|---|---|---|
| `useLayoutStore` 硬编码 `activateBrowser()` / `activateTerm()` / `activateFiles()` / `activateEditor()` / `activateWorkspace()` / `clipOpen → setView("clip")` | `src/stores/useLayoutStore.ts`（Shell 持有能力专属 view 名） | Shell 持有少量能力名字符串；**不改变真源**（视图渲染仍走贡献），但违反「Shell 零能力专属知识」的理想态 | **NON_BLOCKING / FUTURE**：改为由贡献声明 `activationIntent`；本轮不改（改动面涉及快捷键、全局动作、dock，风险高于收益） |
| `ActivityBar` 的 `menuSections` 视图清单 | `src/components/layout/ActivityBar.vue` | 同上：导航项来自 Shell 内置清单而非贡献 | **NON_BLOCKING / FUTURE**：随「导航贡献化」专项处理（与 UI-03 债务同族） |
| `MainView` 枚举硬编码 | `src/stores/useLayoutStore.ts` 类型 | 新增能力需扩展枚举（门禁会提示），非真源冲突 | **NON_BLOCKING**：登记债务 |

## 3. 结论

```text
WORKBENCH_FIXED_NAV              = 4（home 默认视图 / settings 入口 / activate* 语义助手 / MainArea 兜底）
CAPABILITY_CONTRIBUTED_NAV       = 主区 13+ 视图 + dock 页签 + browser 侧栏（Shell 零能力专属知识）
LEGACY_HARDCODED_CAPABILITY_NAV  = 3（useLayoutStore activate* / ActivityBar menuSections / MainView 枚举）
INVALID                          = 0
UNKNOWN                          = 0
```

**NAVIGATION_GOVERNANCE = PASS（with FUTURE debt）**：
渲染链路已完全贡献驱动（这是「absent 不残留死视图」的关键），
剩余硬编码仅存在于**导航入口清单**层，属既有债务（与 UI-03 同族），**不阻断 RC**，但明确记录、不静默。
