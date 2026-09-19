# KNOWN_DEBT_V1（已知债务总表）

> 日期：2026-09-19
> 纪律：债务**显式存在、不静默消失**；关闭需独立任务单驱动并同步更新本表、相关 Checker 与文档。
> 真源 ledger：`docs/architecture/semantic-governance/Known-Debt.md` + `docs/architecture/HANDOFF_CURRENT_STATE.md` 的 Known Debt 段。
> 本阶段（Semantic Governance v1 Final Acceptance）**不清理债务**，仅汇总登记。

---

## 一、已关闭（CLOSED）

| ID | 债务 | 关闭阶段 | 关闭证据 |
|-|-|-|-|
| Debt-5-1 | 凭据明文进 Pinia/localStorage/日志 | Phase 5.1 | `check-credential-owner.mjs` RULE-011B 0 命中；凭据仅存 keyring |
| Debt-5-2 | 凭据 facade 未收敛（list/fill/import 散落）| Phase 5.1 | 经 `useCredentialStore` 单一 facade；origin 精确匹配 |
| Debt-5-3 | 激活页签未绑 `activeTabId` 填充 | Phase 5.1 | `fill` 传 `{credential_id, activeTabId}`；单测覆盖 |
| Debt-6A-aiNavOpen | `aiNavOpen` 双真源（useBrowserStore + useLayoutStore）| Phase 6A | R8 守护唯一 owner；useLayoutStore 死重复已删 |
| Debt-6A-2 | `gridSession` 函数级单写者未机器化 | Phase 6B | **R9** `SEMANTIC_STATE_WRITER_VIOLATION` 函数作用域强制（buildGrid/forceGridRelayout 唯一合法 writer）|

---

## 二、保留（RETAINED / KNOWN DEBT）

### 语义治理范围内

| ID | 债务 | 状态 | 是否阻塞 v1 | 处置纪律 |
|-|-|-|-|-|
| Debt-001 | Grid UDS socket cleanup（退出残留 socket 累积）| KNOWN DEBT | 否 | 触碰用户数据目录，须独立任务单 + 备份，本阶段不修 |
| Debt-002 | `toggleGridToolbar`（死代码 + layout↔browser 循环依赖风险）| KNOWN DEBT | 否 | 修复需架构级（意图下沉/运行时注入），不动 import |
| Debt-003 | `closeGridCell` orphan API（无 UI 入口）| KNOWN DEBT | 否 | `check-view-intent.mjs` 报 INFO WARN（不阻断），预期 |
| Debt-004 | Terminal checker（`check-terminal-policy.py` / `-ui-logic.mjs` 在 self-test 失败）| KNOWN DEBT | 否 | Phase 外（终端非 Browser/Grid 语义范围）；不得为变绿放宽断言 |
| Debt-6A-1 | M4 其余 14 域 `observed_not_governed`（Terminal/Bookmark/Plugin/MCP/Clipboard/Script…）| KNOWN DEBT | 否 | 显式登记，不静默消失；后续走 SCR 才治理 |
| Debt-6A-3 | R8 声明形态仅识别 `const X = ref/reactive`（解构/动态声明盲区）| KNOWN DEBT | 否 | 与 R2 同源；登记于 Known-Debt |
| Debt-6B-1 | R9 brace 配对对"无参 parenless 箭头"(`const f = x => {}`) 不识别 enclosing | KNOWN DEBT | 否 | 仅影响极少数写法；真实 writer 均为 `function NAME()`，未触发误报 |

### 产品级（M4 并发债，语义治理范围外，如实登记不跨阶段处理）

| ID | 债务 | 来源 | 备注 |
|-|-|-|-|
| D23 | 终端 GUI 实点未实测 | M4 Terminal | Lane A3 交付债，与语义治理无关 |
| D24 | 终端吞吐基线未重采 | M4 Terminal | — |
| D25 | `on_channel_dead` 未 wait | M4 Terminal | — |
| D26 | 终端历史未按字符封顶 | M4 Terminal | — |
| W17-D1~D6 | M5-W17 引入的回归（含 D6=B10-b MainArea 兜底 v-else 恒渲染）| M5 | 交 A0/A11 裁决，非语义治理范围 |
| F-A4-* / F-A11-* | 跨 lane 架构发现 | M5 | 交 A0 采纳前裁决 |

---

## 三、处置原则（重申）

1. **不静默消失**：债务只能显式登记、显式关闭；禁止删本表/改文档来"消化"债务。
2. **不在本阶段修**：上述保留债务均不在 Semantic Governance v1 范围；修复须独立任务单。
3. **禁止降低门禁"修复"**：不得放宽断言、加 ignore、删失败测试（见 RECOVERY-PROCEDURE 场景 1）。
4. **修复后同步**：关闭某条时同步本表、相关 Checker 与 `docs/operations/recovery/` 引用文档。
