# 06 — Maturity Matrix（成熟度 × 热插拔，二者不得混淆）

> **C4 ≠ HP2**。C4 表示资源可被运行时控制；HP2 表示能运行时注册/注销。两者正交。

## 现有 4 个积木

| 能力 | Maturity | Hot-Plug | 说明 |
|---|---|---|---|
| bookmark | C3 | **HP2** | 无重资源，运行时 register/unregister 已验证 |
| workspace | C3 | HP0 | 需先验证主视图贡献整体摘除后 Shell 不残留死视图 |
| browser | C3 | HP0 | WebView 生命周期未治理（停用/释放策略待实现） |
| terminal | C3 | HP0 | PTY/会话释放未验证 |

## 尚未成为积木的能力（候选）

| 能力 | Maturity | 说明 |
|---|---|---|
| git | C1 | owner 已物理分离，**未契约化** |
| database | C1 | 同上 |
| task / graph / plugin / skill / agent | C0 | 未被 catalog 收录 |
| clipboard / apps / tools / home | C0 | 由 Shell 直连，未契约化 |

## 为什么没有 C4/C5

- **C4** 要求资源可被运行时控制（停用/挂起/恢复）且有验证。Browser/Terminal 未验证 → 保持 HP0/C3。
- **C5** 要求**真实释放证据**（例如终端销毁后 PTY 与子进程确实消失）。
  今晚无运行中的应用实例 → 进程级测量为 **UNKNOWN** → **不允许声称 C5**。

> 原则：`destroy() 函数存在 ≠ C5`（§29）。
