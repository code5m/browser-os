# HANDOFF — UI COMPONENT SYSTEM

> 每个稳定点的：HEAD / tag / 已完成阶段 / 指标 / 已知债 / 下一个确切任务。

---

## 当前稳定点

| 项 | 值 |
|---|---|
| HEAD | `ab1449a`（UI-5 完成）→ UI-6 closeout 提交后更新 |
| 最新 tag | `ui-system-phase5-token-foundation-pass` |
| 受保护 tag（未移动） | `universal-hotplug-capability-platform-v1-resource-fix-code-pass` = `3f57a92` |
| 分支 | `feature/capability-platform-v1` |
| 推送 | 无 |

---

## 阶段状态

| 阶段 | 状态 | HEAD | tag |
|---|---|---|---|
| UI-0 Inventory | **PASS** | `8e65184` | `ui-system-phase0-inventory-pass` |
| UI-1 Governance | **PASS** | `ed139a3` | `ui-system-phase1-governance-pass` |
| UI-2 Pilot Extraction | **PASS** | `2e9f55e` | `ui-system-phase2-pilot-pass` |
| UI-3 Shared Foundation | **PASS** | `31c6cb5` | `ui-system-phase3-shared-foundation-pass` |
| UI-4 Workbench Decoupling | **PASS** | `965f0d4` | `ui-system-phase4-workbench-decoupling-pass` |
| UI-5 Token Foundation | **PASS** | `ab1449a` | `ui-system-phase5-token-foundation-pass` |
| UI-6 Closeout | **PASS** | （本次提交） | `ui-component-system-v1-code-pass` |

---

## 指标

| 指标 | 值 |
|---|---|
| TOTAL_UI_COMPONENTS | 68 |
| SHARED_UI | 3（EmptyState / ContextMenu / ContextMenuItem） |
| WORKBENCH_UI | 17 |
| CAPABILITY_UI | 46 |
| UNKNOWN | **0** |
| VACUOUS UI rules | **0**（UI-1 时为 3） |
| Shell→业务渲染耦合 | 11（基线化，未新增） |
| Shell 持业务 store 耦合 | 22（基线化，未新增） |
| Shell 硬编码 Dock 页签 | **4 → 0** |
| TRUE_DUPLICATES | 3 → 1 已消除（EmptyState）；ModalShell 因视觉冲突未抽 |
| Design token 层 | v1（6 个语义 token，取值逐字等同原字面量） |

---

## 新增/强化的确定性门禁

| 脚本 | 作用 |
|---|---|
| `scripts/check-ui-boundaries.mjs` | UI-01..UI-10（shared 纯净性 / Shell 边界 / fixed 浮层 / 反模式 / native 命令 / 宿主不存业务真值 / catalog 一致） |
| `scripts/verify-ui-pilot.mjs` | 真实 Vue SSR 渲染，9 个用例与 before DOM 逐字节比对 + shared 纯净性 |
| `scripts/verify-dock-contribution.mjs` | D1 UX 一致 / D2 absent 无死页签 / D3 Shell 无硬编码与业务 switch |
| `scripts/verify-design-tokens.mjs` | T1 取值等同原字面量 / T2 语义命名 / T3 用 var / T4 已引入 / T5 死规则 / T6 真冲突未统一 |

全部已接入 `npm run check`。

---

## 已知债（诚实清单）

| # | 债务 | 为什么现在不修 |
|---|---|---|
| D-1 | 11 处 Shell 直渲业务面板（Git/Database/Task/Agent/Skill/Vault/Graph/Plugin/ToolBox/AINav/GridArchive） | 每一项都需人工视觉评估；已全部显式 baseline，新增即 FAIL |
| D-2 | 22 处 Shell 持业务 store | 同上；其中 App.vue 持 `useGitStore`、硬编码 `"成果工作区"` 目录名最该优先 |
| D-3 | `.app-name` 13px↔11px 泄漏（LEGACY_DRIFT 已确认） | 修正会改变列表视图字号/颜色 → 需人工视觉验收 |
| D-4 | modal 遮罩两套值（.35/z100 vs .4/z999，另有 scoped .35/z50） | INTENTIONAL/UNKNOWN；这也是 ModalShell 未提炼的原因 |
| D-5 | 菜单项 / 状态栏项无 contribution 类型 | Rule of Two 未满足（0 个真实跨能力消费者），不造万能 API |
| D-6 | `workspace/ui/FileEditor.vue` 未声明 browser 依赖 | 唯一真实边界瑕疵，显式 WARN |
| D-7 | `UnifiedTabBar.vue:127,137,145` 使用未 import 的 `bridge` | 真实 bug（右键菜单会抛 ReferenceError），但属业务修复、非 UI 系统范围，未在本轮改动 |
| D-8 | 4 个孤儿组件（TopBar / SidebarResizer / ResourcePanel / RunHistoryModal） | 孤儿 ≠ 删除；未动 |
| D-9 | 45+ 硬编码色值未 token 化 | 刻意小步；待后续批次 + 人工视觉 |
| D-10 | 14 个文档声明但运行时未注册的域 | 注册它们是 Capability Platform 范围，非 UI 系统 |

---

## 下一个确切任务

1. **Final Human Visual Acceptance**（唯一未完成的收口项）
   - framework profile：UI 保持、无白屏
   - full profile：Dock 页签 📂文件 💻终端 🌊资源 💾会话 顺序/图标/文案不变
   - 空态、右键菜单、主页、终端、宫格 HIDE/DESTROY 目视确认
   - 通过后才可创建 `ui-component-system-v1-pass`（**本次未创建**）
2. 之后按 D-1/D-2 逐项迁移，每迁一项必须附：before 依赖 / after 依赖 / UX 等价证据 / 门禁 / absence 行为。

---

## 绝对约束（后续不得违反）

- UI PRESERVATION 最高：组件化不是 redesign。
- 不得为 `shared/ui` 数量搬文件；不得造 God Component / prop explosion。
- 不得用 baseline 洗绿：基线与现实不符同样 FAIL。
- 不得把 VACUOUS 伪装成 PASS。
- 不得让 capability 资源泄漏回归（framework grid-child = 0）。
