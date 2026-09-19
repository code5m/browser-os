# 07 · 已知债务（Known Debt）

> 本文件为 `KNOWN_DEBT_V1.md` 的管理者视图摘要。完整条目与处置纪律见该文件。
> 原则：**债务显式存在、不静默消失；关闭须独立任务单驱动**。

## 已关闭（CLOSED）

| ID | 债务 | 关闭阶段 |
|-|-|-|
| Debt-5-1 / 5-2 / 5-3 | 凭据明文/ facade 未收敛/ 激活页签未绑定 | Phase 5.1 |
| Debt-6A-aiNavOpen | `aiNavOpen` 双真源 | Phase 6A |
| Debt-6A-2 | `gridSession` 函数级单写者未机器化 | Phase 6B（R9）|

## 保留（RETAINED）

### 语义治理范围内

| ID | 债务 | 是否阻塞 v1 |
|-|-|-|
| Debt-001 | Grid UDS socket cleanup（退出残留累积）| 否 |
| Debt-002 | `toggleGridToolbar`（死代码 + 循环依赖风险）| 否 |
| Debt-003 | `closeGridCell` orphan API | 否 |
| Debt-004 | Terminal checker 既有失败（Phase 外）| 否 |
| Debt-6A-1 | M4 其余 14 域 `observed_not_governed` | 否 |
| Debt-6A-3 | R8 声明形态盲区（解构/动态声明）| 否 |
| Debt-6B-1 | R9 parenless 箭头函数识别盲区 | 否 |

### 产品级（M4/M5 并发债，语义治理范围外）

| ID | 债务 |
|-|-|
| D23–D26 | 终端 GUI 实点 / 吞吐基线 / on_channel_dead / 历史字符封顶 |
| W17-D1~D6 | M5-W17 引入回归（含 MainArea 兜底 v-else 恒渲染）|
| F-A4-* / F-A11-* | 跨 lane 架构发现，交 A0 裁决 |

## 管理层须知

- 上述保留债务**均不阻塞 Semantic Governance v1 验收**（已验证核心治理域闭环）。
- 它们会在后续独立任务单中修复，修复时同步更新 Registry / Checker / 文档。
- 本阶段**不清理**这些债务，仅如实汇总登记。
