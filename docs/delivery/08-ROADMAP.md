# 08 · 后续路线（Roadmap）

> 本文件描述 Semantic Governance v1 完成后的可选方向。**本验收阶段不启动任何新治理域、不开始新业务开发**，
> 等待下一阶段指令。

## 已具备的能力（v1 交付）

- 四支柱闭环：Registry（State/Intent/Owner/Writer/SideEffect）+ Checker（R1–R9）+ Gate（pre-merge）+ Recovery。
- 13 个语义治理阶段全部 CLOSED，tag `semantic-governance-v1` 标记基线。

## 可选方向（需独立任务单 + SCR/ADR）

| 方向 | 说明 | 前提 |
|-|-|-|
| **Product Evolution** | 在已治理的语义骨架上做新功能开发 | 新需求进入，复用现有 canonical intent / owner |
| **M4 其余 14 域治理** | Terminal/Bookmark/Plugin/MCP/Clipboard/Script 等纳入 Registry | 走 SCR，登记 canonical_writer 后再扩 R9 范围 |
| **Checker 盲区消除** | Debt-6A-3（R8 解构/动态声明）/ Debt-6B-1（R9 parenless 箭头）| 独立任务单，不降低现有断言 |
| **Runtime 深化** | 更多 store 行为契约进 closure-logic 运行时校验 | 不扩大治理域前提下增量加 |
| **文档/工具** | Registry 可视化、Checker 报告聚合到 CI 看板 | 非阻断，可并行 |

## 不建议的方向（本阶段边界）

- ❌ 为"门禁变绿"放宽 R1–R9 断言或删失败夹具。
- ❌ 把 Registry 当万能方案、强行把所有变量登记（违反"不把所有变量强制 Registry"）。
- ❌ 在 v1 验收内清理 Known Debt（债务应显式存在、独立任务单关闭）。

## 交接状态

- **Semantic Governance v1: CLOSED**
- **Latest Tag: `semantic-governance-v1`**
- **Next: Product Evolution / New Feature Development（待指令）**
