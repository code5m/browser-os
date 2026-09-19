# Lifecycle Model（生命周期模型）

> Phase 7A　基线：`semantic-governance-v1`

---

## 1. 生命周期状态机

```text
                    register
                       │
                       ▼
                  ┌─────────┐
                  │ DEFINED │  manifest 已注册，尚未实例化
                  └────┬────┘
                       │ resolve (依赖满足)
                       ▼
                  ┌─────────┐
        ┌────────►│  READY  │  可激活
        │         └────┬────┘
        │              │ activate
        │              ▼
        │         ┌─────────┐
        │         │ ACTIVE  │  能力正在提供功能
        │         └────┬────┘
        │              │
        │      suspend │ │ background
        │              ▼ ▼
        │      ┌────────────────┐
        │      │ SUSPENDED /    │
        │      │ BACKGROUND     │
        │      └───────┬────────┘
        │              │ resume
        │              ▼
        │         (回到 ACTIVE)
        │              │
        │              │ destroy
        │              ▼
        └────────┌──────────┐
                 │ DESTROYED│
                 └──────────┘
        disable（仅安全能力）：ACTIVE/SUSPENDED → READY
```

---

## 2. 状态定义

| 状态 | 含义 | 谁可进入 |
|---|---|---|
| `DEFINED` | manifest 已注册，未实例化 | `register()` |
| `READY` | 依赖已解析，可激活 | `resolve()` 成功 |
| `ACTIVE` | 正在提供功能 | `activate()` |
| `BACKGROUND` | 仍在运行但无前台视图（如调度任务） | 声明支持的能力 |
| `SUSPENDED` | 冻结状态与视图，保留状态真源 | **仅声明 `supportsSuspend: true`** |
| `DESTROYED` | 释放资源，状态按 persistence 策略处理 | `destroy()` |
| `DISABLED` | 显式停用（不等同销毁） | 仅安全能力允许 |

---

## 3. 诚实边界：今晚实际支持什么

> **不得把目标态写成已实现。** 下表是能力矩阵真实状态。

| 能力 | register | resolve | activate | suspend | destroy | 今晚真实状态 |
|---|---|---|---|---|---|---|
| Bookmark | ✅ | ✅ | ✅ | ⬜ 声明不支持 | ⬜ | **COMPATIBILITY_WRAPPED**（试点） |
| Browser | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | **NOT_INTEGRATED**（仅登记） |
| Grid | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | **NOT_INTEGRATED**（仅登记） |
| Terminal | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | **NOT_INTEGRATED**（仅登记） |
| 其余 CAPABILITY | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ | **NOT_INTEGRATED**（仅登记） |

**今晚唯一真实做到 lifecycle 操作的是 Bookmark 的 register/resolve/activate**，且这是通过兼容适配器包装既有 `useBookmarkStore` 实现的，**不是物理卸载/重载**。

图例：
- `CURRENTLY_COMPOSABLE`：已能真正独立启停 → **今晚：无**
- `COMPATIBILITY_WRAPPED`：经适配器接入 Runtime，但底层仍是既有系统 → **今晚：Bookmark**
- `TARGET_COMPOSABLE`：目标态，尚未实现 → Browser / Grid / Terminal / 其它

---

## 4. 生命周期与 Owner 的边界（硬约束）

```text
Runtime 只做编排（orchestration），绝不成为业务状态 Owner。
```

| 禁止 Runtime 做的事 | 真正 Owner |
|---|---|
| 管理 Browser tabs / gridSession | `useBrowserStore` |
| 管理 Terminal 进程 / termPanes | `useSystemStore` |
| 管理 Credential / keyring | Rust `KeyringStore` |
| 直写任何 Semantic Registry 已登记状态 | 该状态的 canonical owner/writer |

Runtime 允许：
- 调用能力**已声明的** lifecycle 钩子（`onActivate` / `onSuspend`）
- 读取 manifest 做依赖解析与资源分类
- 记录能力状态（**Runtime 自己的状态**，与业务状态分离，且不进入 Semantic Registry 作为业务真源）

> 这条边界由 7C 实现 + `check-capability-registry.mjs` 强制。
