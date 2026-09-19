# Shell Boundary（外壳边界）

> Phase 7A　基线：`semantic-governance-v1`
> 目的：划清 **Application Shell / Capability Runtime / Capability / Semantic Registry** 四者边界，
> 防止 Runtime 变成 God Runtime，防止 Capability 体系成为绕过语义治理的后门。

---

## 1. 四层职责

```text
┌─────────────────────────────────────────────────────┐
│ Stable Application Shell                            │
│  职责：窗口/布局/导航/生命周期宿主/关停顺序           │
│  不负责：任何具体业务                                │
└──────────────────────┬──────────────────────────────┘
                       │ 提供 CapabilityContext
┌──────────────────────▼──────────────────────────────┐
│ Capability Runtime                                  │
│  职责：register / resolve / enable / disable /      │
│        activate / suspend / inspect                │
│  不负责：业务状态（绝不成 Owner）                     │
└──────────────────────┬──────────────────────────────┘
                       │ 调用已声明 lifecycle 钩子
┌──────────────────────▼──────────────────────────────┐
│ Capability（经 Adapter 包装既有系统）                 │
│  职责：提供功能；业务状态仍归既有 Owner               │
└──────────────────────┬──────────────────────────────┘
                       │ 必须遵守
┌──────────────────────▼──────────────────────────────┐
│ Semantic Registry（冻结基线，不得绕过）              │
│  State / Intent / Owner / Writer / SideEffect        │
└─────────────────────────────────────────────────────┘
```

---

## 2. Shell 独占（Capability 不得染指）

| 职责 | Owner |
|---|---|
| 主窗口 / 视图导航 / 布局密度 | `useLayoutStore`（Semantic Registry 已登记） |
| 应用启动与关停顺序 | `session.rs` / `shutdown.rs` |
| Pinia 安装与 store 注册 | `main.ts` |
| 安全策略与程序黑名单 | `security_policy.rs` |
| Native 调用通道 | `bridge.ts` |

---

## 3. Runtime 禁止清单（7C 强制）

```text
禁止 Runtime 自己管理 Browser tabs / gridSession
禁止 Runtime 自己管理 Terminal 进程 / termPanes
禁止 Runtime 自己管理 Credential / keyring 材料
禁止 Runtime 直写 Semantic Registry 已登记的任何业务状态
禁止 Runtime 持有第二份业务状态真源
禁止 Runtime 引入 DI 容器 / 动态 require / 反射加载（今晚不做）
禁止 Runtime 绕过 bridge.ts 直连 native
```

Runtime **只允许**：
- 解析 manifest、检测依赖与环
- 调用能力**已声明**的 lifecycle 钩子
- 维护**自己的**注册表状态（与业务状态分离，且不作为业务真源写入 Semantic Registry）
- 输出 inspect 报告

---

## 4. 与 Semantic Registry 的分工（禁止重复维护同一事实）

| 问题 | 由谁回答 | 文件 |
|---|---|---|
| "这个概念/状态是什么意思？谁拥有、谁能写？" | **Semantic Registry** | `docs/architecture/semantic-registry/{states,intents,owners,side-effects}.yaml` |
| "这个能力提供什么、依赖什么、生命周期与资源策略是什么？" | **Capability Registry** | `docs/architecture/capability-registry/{capabilities,dependencies,resources}.yaml` |

**不重复规则**：
- Capability manifest **不得重复声明** Semantic Registry 已登记的 state/owner/writer 语义；manifest 中只允许出现**引用**（如 `semanticOwner: useBookmarkStore`）。
- 若某能力需要新增语义（新 State / Intent / Owner / Writer / Side Effect），**必须先走 SCR + Reviewer 裁决 + 更新 Semantic Registry**，Capability Registry 才能引用。
- 违反即由 `check-capability-registry.mjs` + `check-semantic-registry.mjs` 双门禁阻断。

> Skill / Notes / Script 三个能力**当前在 Semantic Registry 中无登记 Owner**（见 Inventory §3）。
> 它们若进入 Capability Registry，须先补 SCR 登记 Owner —— 这是 7B 的硬前置。

---

## 5. 兼容优先（Compatibility First）

```text
旧系统 ──► Capability Adapter ──► Capability Runtime
```

**禁止**（今晚）：
- 一次把全部 store 移动目录
- 一次拆 monorepo
- 一次拆十几个 npm package
- 删除旧系统

Adapter 只做"包装 + 声明"，**不改既有业务行为**。

---

## 6. 边界自检清单（每个 Phase 收口时逐条确认）

```text
[ ] Runtime 未持有任何业务状态真源
[ ] 未新增 Semantic Registry 未登记的 State/Intent/Owner/Writer
[ ] 未绕过 bridge.ts
[ ] 未改动 security_policy.rs 的边界
[ ] 未删除旧系统（仅包装）
[ ] 未把 TARGET 写成 CURRENT
[ ] 未编造实测数字
```
