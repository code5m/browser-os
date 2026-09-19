# 07 — Checker Gap Analysis

> 检查现有 Checker 覆盖：R1 重复状态 / R2 未注册状态 / R3 Owner 越界 / R4 重复意图 /
> R5 副作用未知 / R7 凭据敏感输入 / S1+S2 keyring 契约。
> 分类：COVERED / MISSING / NOT_RELIABLY_CHECKABLE（不为覆盖率造脆弱规则）。

## 1. 覆盖矩阵

| Rule | 目标 | 范围内覆盖 | 范围外覆盖 | 分类 |
|---|---|---|---|---|
| R1 Duplicate State | 同概念多真源 | COVERED（Registry 级：states.yaml `duplicate_names` / derived 重复名拦截） | MISSING | NOT_RELIABLY_CHECKABLE（代码级跨 store 同名字段需 AST 扫描，脆弱） |
| R2 Unregistered State | 未注册状态 | COVERED（仅 `governed_files` 内） | MISSING（14 新增域静默） | MISSING（范围外） |
| R3 Owner Violation | 越界写/调 | COVERED（Browser/Grid/Credential 3 owner） | MISSING | MISSING（Terminal/Bookmark/Workspace 越界无 rule，Debt-4-2） |
| R4 Duplicate Intent | 重复入口 | COVERED（rejected_intents 红线拦截） | MISSING（范围外意图无登记） | COVERED（范围内） |
| R5 Side Effect Unknown | 未登记副作用 | COVERED（已登记项 awareness 机制） | MISSING（SE-11..20 未登记故不查） | MISSING（范围外） |
| R7 Credential Sensitive Input | 敏感输入泄露 | COVERED（sensitive:true 状态 frontend_ident 泄露汇） | n/a | COVERED |
| S1/S2 Keyring Contract | keyring 数据流 | COVERED（rejected intents + DTO 字段） | n/a | COVERED |

## 2. 具体盲区（MISSING）

- **G-CHECK-1**：`aiNavOpen` 双真源（01-S-11）**不会被 R1 拦截**——R1 只查 Registry 内 duplicate_names，
  不扫代码两个 store 同名字段。属 NOT_RELIABLY_CHECKABLE（AST 级同名检测易误报，不建议为覆盖率强造）。
- **G-CHECK-2**：范围外 14 域的状态/意图/副作用**完全无 checker**——R2 仅 `governed_files`、R3 仅 3 owner、
  R5 仅登记项。任何越界/重复/未登记在这些域零拦截。
- **G-CHECK-3**：`plugin_install` / `run_script` / `git push` / `db_query` / `clipboard_write` 等安全敏感副作用
  无机器约束（仅 Rust 侧 `security_policy.rs` 黑名单部分覆盖进程启动，但语义层未登记）。
- **G-CHECK-4**：Terminal/Bookmark/Workspace 的组件越界（COMPONENT_WRITES_TERMINAL 已在 R3 模式，但
  COMPONENT_WRITES_BOOKMARK / COMPONENT_WRITES_WORKSPACE 未模式化——Debt-4-2 已知）。

## 3. 不建议造的脆弱规则

- 不为"代码级跨 store 同名字段"造 R1 增强（误报率高：selected/fileContent/preview 等合法同名局部态会爆）。
- 不为"范围外域"强行扩展 R2/R3（会瞬间产生海量噪声且超出 registry_scope 授权）。
- 不把 `implementation-detail` 状态纳入任何 rule。

## 4. 建议新增 checker（若未来扩展治理，见 08）

| 候选 Checker | 触发条件 | 优先级 |
|---|---|---|
| R3-terminal/bookmark/workspace 模式化 | 补 Debt-4-2 越界模式 | P2 |
| side-effect 登记校验（SE-11..20） | 新副作用调用点须带 `side-effect:` 标记 | P1（安全敏感） |
| plugin_install 安全策略 checker | 插件安装须过 security_policy 白名单 | P1 |

> 现有 R1–R7 + S1/S2 **未削弱**，本审计不新增任何 checker（保持 READ-ONLY）。若实施迁移，按 08 单独
> `chore(checker)` 提交。
