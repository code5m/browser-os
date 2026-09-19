# Phase Acceptance Matrix（语义治理各阶段验收矩阵）

> 日期：2026-09-19
> 范围：Single Semantics Governance 全部已完成阶段（Phase 0 → Phase 6B）。
> 每个阶段均有独立 closeout 文档与稳定 tag，可独立回放验证。

| Phase | Scope | Tag | Status |
|-|-|-|-|
| Phase 0 | Single Semantics Governance 基础设施 | `semantic-phase0-infra-pass` / `semantic-phase0-policy-pass` | ✅ CLOSED |
| Phase 1 | Browser / Grid Semantic Lifecycle | `semantic-phase1-browser-grid-pass` | ✅ CLOSED |
| Phase 1.5 | Semantic Registry（states/intents/owners/side-effects）| — | ✅ CLOSED |
| Phase 1.6 | Semantic Gate Integration（pre-merge 接入）| — | ✅ CLOSED |
| Phase 1.7 | Git Integrity & Recovery | `semantic-phase1.7-git-integrity-pass` | ✅ CLOSED |
| Phase 2 | Workspace / FilePanel Semantic Governance | `semantic-phase2-workspace-pass` | ✅ CLOSED |
| Phase 3 | Bookmark Semantic Governance | `semantic-phase3-bookmark-pass` | ✅ CLOSED |
| Phase 4 | Terminal Lifecycle Governance | `semantic-phase4-terminal-pass` | ✅ CLOSED |
| Phase 5 | Credential Security Governance | `semantic-phase5-credential-pass` | ✅ CLOSED |
| Phase 5.1 | Credential Security Hardening | `semantic-phase5.1-credential-hardening-pass` | ✅ CLOSED |
| Semantic Closure Audit v1 | 闭环审计（State/Intent/Owner/Writer/SideEffect）| — | ✅ CLOSED |
| Phase 6A | Core Semantic Migration（唯一 owner）| `semantic-phase6a-core-closure-pass` | ✅ CLOSED |
| Phase 6B | Semantic Writer Enforcement（唯一 writer + Checker 可证明）| `semantic-phase6b-writer-enforcement-pass` | ✅ CLOSED |
| **Semantic Governance v1** | **Registry + Checker + Gate + Recovery 总验收** | **`semantic-governance-v1`** | ✅ CLOSED |

## 说明

- 上表 Tag 列即各阶段收口时的 annotated tag（均存在、未移动、未删除）。
- Phase 1.5 / 1.6 / 1.7 / Semantic Closure Audit v1 无独立语义 tag，但其 closeout 文档与 gate 接线已并入后续 tag 基线。
- 全部 tag 均为本地（`git tag`，未 push），符合"治理基线不得 push"的约定。
- 真实验证：`node scripts/check-semantic-registry.mjs --self-test` → `ALL_PASS`；
  `node scripts/check-semantic-registry.mjs` → `fail=0`；`check-semantic-closure-logic.mjs` → `27/27`。
