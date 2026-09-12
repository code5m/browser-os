# Phase 04 Specification — 自动保存关闭页签 (Auto-Save on Tab Close)

> 产品路线图 `PLANS.md` Phase 4 = 自动保存关闭页签。
> 本规格为权威裁定文档；`PROJECT-RULES.md` 的对应 `[PENDING]` 由本文件裁定（见末尾决策记录）。
> 治理约定见 `.ai/workbuddy-dispatch/autonomous-execution-policy.md`（No Ask Unless Blocked）。

## Chief Architect Decision（裁定 PROJECT-RULES `[PENDING]`）

`PROJECT-RULES.md` 两条 `[PENDING]` 在 Phase 04 一并裁定：

- **PENDING-1**「关闭页签是否改为自动保存并直接关闭」→ **是**。新增前端策略 `auto_save_on_close`：开启后关闭页签即自动保存会话并直接关闭，不再弹三选一确认框。
- **PENDING-2**「是否增加"最近关闭的页签"和 Ctrl+Shift+T」→ **是**。新增 `recentlyClosed` 内存栈（封顶 20）+ Ctrl+Shift+T 快速重开。

**向后兼容（零回归）**：现有 `close_prompt`（三选一确认）与 `auto_save_on_exit`（退出自动保存）保持不变。三种模式并存：

| `auto_save_on_close` | `close_prompt` | 关闭行为 |
|---|---|---|
| `true`  | 任意 | 自动 `sessionSave` + 直接关闭（Phase 04 新行为） |
| `false` | `true` | 维持现有三选一弹窗（legacy） |
| `false` | `false` | 直接关闭、不保存（等同现有"删除并关闭"） |

**默认 = 现状**（`close_prompt=true, auto_save_on_close=false`）→ 行为完全不变。

## 目标（Purpose）

关闭浏览器页签时不再静默丢弃：提供"自动保存并关闭"模式，并配套"最近关闭的页签"快速恢复（Ctrl+Shift+T）。在**零原生改动、零运行时权限扩张、不破坏现有 WebView 生命周期与关闭协议**的前提下落地。

## 范围（Scope）

**包含**：

- 前端策略状态 `auto_save_on_close`（前端内存态，不落后端、不持久化；升级为后端同步见非阻塞跟进 P4-1）
- 关闭页签时按策略自动 `sessionSave` 并 `closeTabNow`
- `recentlyClosed` 内存栈（封顶 20，存 `{url, title}`）+ 重开
- Ctrl+Shift+T 全局快捷键
- `SessionPanel` 新增「关闭时自动保存」开关与「最近关闭」区块

**明确划界（不包含）**：

- 不改动原生 WebView 关闭/隐藏路径（`closeTabNow` 的 `bridge.tabClose` + splice + `tabActivate(next)` + `schedulePosition` + `syncFreeze` 必须原样复用）
- 不新增 Rust 命令 / 不改 `src-tauri/**`
- 不扩张运行时权限面（无新 command → 无新 ACL / capability）
- 不新增 npm 依赖（Phase 03 硬化约束）
- 不引入 HTML `fixed` 浮层覆盖 WebView（规则 3.8）

## 允许修改文件（Allowed）

- `src/stores/useSessionStore.ts`
- `src/stores/useBrowserStore.ts`
- `src/stores/useSettingsStore.ts`
- `src/components/browser/SessionPanel.vue`
- `src/components/browser/SessionCloseDialog.vue`（保持，legacy 模式使用）
- `src/App.vue`（注册 Ctrl+Shift+T 分发）
- `src/types.ts`（`SessionPolicy` 增补前端字段说明 / 新增 `RecentlyClosedEntry`）
- `src/bridge.ts`（`setSessionPolicy` 若需透传新字段；若保持纯前端态则不改）

## 禁止修改文件（Forbidden）

- `src-tauri/**`（无后端改动）
- `tauri-browser-tabs/**`
- `AGENTS.md` / `PROJECT-RULES.md`（规则文件，由对应 Owner 更新）
- `package.json`（不新增依赖）
- `scripts/**`（不改动 checker；若需新 checker 另行立项）
- `docs/AI/**`（架构文档仅在设计变更时由 Design Agent 更新）
- `src-tauri/permissions/**`

## 不变量（Invariants）

- **WebView 生命周期零改动**：`closeTabNow` 的原生拆除路径必须原样复用；Phase 04 仅在其"之前/之后"挂前端逻辑，禁止修改该路径本身（参照 `useBrowserStore.closeTabNow`：`bridge.tabClose(id)` → `tabs.splice` → `tabActivate(next.id)` → `schedulePosition()` → `syncFreeze()`）。
- **无回归 v0.3.0 基线**：关闭/重开页签后网页仍撑满、可点击、前进后退正常。
- **关闭协议"不可静默丢"精神保留**：`auto_save_on_close` 关闭时显式 `sessionSave`（后端脱敏落盘），而非丢弃；`close_prompt` 模式行为不变。
- **隐私**：`recentlyClosed` 仅存 `{url, title}`，不落盘、不含页面正文/凭据；`sessionSave` 复用后端脱敏（见 `useSessionStore.saveTab` / `capturePreview`）。
- **不破坏现有 `auto_save_on_exit`**：退出自动保存不受影响。
- **UI 边界**：「最近关闭」区块置于既有布局面板（`SessionPanel` dock），禁止 `position: fixed` / `z-index` 覆盖浏览器 WebView 区（规则 3.8）。
- **体积闸门**：增量须满足 `TOTAL_BYTES_GROWTH_LIMIT_PCT=25.2%`。
- **无新 npm 依赖**。

## 验收命令（Acceptance Commands）

```bash
node scripts/check-architecture.mjs
node scripts/check-ui.mjs
node scripts/check-native.mjs
node scripts/check-browser-runtime.mjs
node scripts/check-task-boundary.mjs
npm run check
npm run doctor
bash scripts/pre-merge.sh --self-test
git diff --check -- <changed files>
npm run build
```

全部须 PASS / exit 0。

## 人工验收项（Manual Acceptance）

- 默认（`close_prompt=true, auto_save_on_close=false`）行为不变：关闭弹三选一。
- 开启「关闭时自动保存」后：关闭页签静默保存 + 关闭，无弹窗；在 `SessionPanel`「历史会话」出现该存档。
- 任意方式关闭页签后，「最近关闭」出现该条目；Ctrl+Shift+T 重开最近一条；点条目亦可重开。
- 重开后网页正常（撑满 / 可点击 / 前进后退）。
- 关闭协议弹窗（legacy）仍可用；退出自动保存仍可用。
- 真实桌面验收：Ctrl+Shift+T 多屏 / 中文输入法下坐标不失真（借现有快捷键实测方法）。

## 停止条件（Stop Conditions）

- P1 / BLOCKER（如 `check-native` 对关闭路径误报、WebView 生命周期被破坏）。
- Conflict Report（store 间循环依赖等）。
- 需改动 `src-tauri/**` 或新增依赖 / 命令（触发 Owner 决策）。
- 体积超限且无法瘦身。

## 回滚方式（Rollback）

- 纯前端改动：`git revert` 本次提交即可；`auto_save_on_close` 为前端态，回滚后恢复默认行为。
- 不涉原生 / 数据迁移，无不可逆操作。

## 决策记录（Resolves PROJECT-RULES `[PENDING]`）

- PENDING-1 关闭页签是否改为自动保存并直接关闭 → **改为自动保存并直接关闭（opt-in）**。
- PENDING-2 是否增加最近关闭的页签和 Ctrl+Shift+T → **增加**。
- 后续由 Project Rules Owner 将 `PROJECT-RULES.md` 对应 `[PENDING]` 迁移为 `[APPROVED]`，本规格为权威裁定。

## 非阻塞跟进（Non-blocking follow-ups）

- **P4-1**：`auto_save_on_close` 升级为后端同步（持久化偏好）。
- **P4-2**：`recentlyClosed` 与 saved-session 去重 / 关联（避免自动保存同时产生历史存档与最近关闭两条）。
- **P4-3**：最近关闭支持跨重启（如需）。
