# 02 · 架构演化（Phase 0 → 6B）

> 语义治理不是一次性设计，而是随项目演进而分阶段收敛。每个阶段只解决一类语义问题，且都留稳定 tag。

| 阶段 | 解决的核心语义问题 | 关键产物 |
|-|-|-|
| **Phase 0** | 治理基础设施（规则/策略骨架）| 治理原则、策略文档 |
| **Phase 1** | Browser ↔ Grid 生命周期语义（hide vs destroy 不混淆）| `exitGrid` 否决、`closeGridAll` 语义化 |
| **Phase 1.5** | Semantic Registry 建立（State/Intent/Owner/SideEffect 四类真源）| `states.yaml` / `intents.yaml` / `owners.yaml` / `side-effects.yaml` |
| **Phase 1.6** | Gate 集成（Checker 接入 `pre-merge.sh`）| 提交前自动门禁 |
| **Phase 1.7** | Git 完整性与恢复（改坏可回退）| `check-git-repo-integrity.sh` / `git-recover.sh` / RECOVERY-PROCEDURE |
| **Phase 2** | Workspace / FilePanel 语义治理 | 文件操作单入口 |
| **Phase 3** | Bookmark 语义治理 | 收藏夹独立源，不与主页快捷方式合并 |
| **Phase 4** | Terminal 生命周期治理 | 终端 pane 经 store action，组件不直写 |
| **Phase 5** | Credential 安全治理 | 凭据仅存 keyring，密码不回前端 |
| **Phase 5.1** | Credential  hardening | origin 精确匹配、激活页签绑定 |
| **Closure Audit v1** | 五大语义模型（State/Intent/Owner/Writer/SideEffect）闭环审计 | 审计基线 |
| **Phase 6A** | Core Semantic Migration（Owner 唯一）| 删 `aiNavOpen` 双真源、`gridSession` owner 澄清、R8 |
| **Phase 6B** | Semantic Writer Enforcement（Writer 唯一 + Checker 可证明）| R9 函数作用域 writer 强制 |

## 演化主线

```text
无治理 ──▶ 生命周期语义(Phase1) ──▶ 真源登记(1.5) ──▶ 门禁(1.6) ──▶ 可恢复(1.7)
   │                                                  │
   └──────── 域治理(2/3/4/5/5.1) ──────────┘
                          │
                  闭环审计(Closure v1)
                          │
            唯一 Owner(6A) ──▶ 唯一 Writer + 可证明(6B) ──▶ Governance v1
```

## 关键纪律

- 每阶段**不扩大范围**：只解决本阶段语义问题，其余域显式登记为 Known Debt。
- 每阶段**留 tag**：可独立回放验证，治理基线不 push（仅本地）。
- Registry 是真源：改语义先改 YAML（走 SCR/ADR），不硬改脚本加 allow-list。
