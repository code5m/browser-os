# 08 — Migration Plan（提议，不执行）

> ⚠️ 本文件**仅提议迁移方案**，本审计阶段**不修改任何业务代码**。
> 所有迁移须人工确认后，按 `SCR → Review → ADR → Registry → 代码` 流程分批实施。

## 原则

- 范围内（Phase 1–5.1）**已闭合，无强制迁移**。
- 迁移只针对本审计发现的 gap（05/06/07），且优先安全敏感项。
- 严禁"大爆炸"式把 14 个新增域一次性塞入 Registry（违反过度治理红线）。

## M1 — 安全敏感副作用登记（P1，最高优先）

| 项 | 登记内容 | 影响文件 | 风险 | 回滚 |
|---|---|---|---|---|
| M1-a | side-effects：`pluginInstall`（fs+process+动态加载） | side-effects.yaml + 新 checker | 中（插件系统是 Stage-I 锁定，须与 plugin runtime 红线一致） | 仅 registry/checker，可 revert |
| M1-b | side-effects：`clipboardWrite`（OS 剪贴板，凭据红线） | side-effects.yaml | 低 | revert |
| M1-c | side-effects：`dbConnect`/`dbQuery`（网络 DB）+ `gitPush`（网络） | side-effects.yaml | 低 | revert |
| M1-d | side-effects：`runScript`/`runCommand`（spawn OS 进程） | side-effects.yaml + 安全策略 checker | 中 | revert |

> 实施时 `requires_declaration` 默认 false（沿用 writeFile/termProcess 口径，避免对既有合法调用产生 R5 噪声），
> 待稳定后再按需收紧。

## M2 — 范围内小修（P2，低优先）

| 项 | 内容 | 影响文件 | 风险 |
|---|---|---|---|
| M2-a | 合并 `aiNavOpen` 双真源（G-01）→ 单一 owner | useBrowserStore / useLayoutStore | 低（需先确认 canonical） |
| M2-b | panel 开关布尔统一（G-02）：clipOpen/fileEditorOpen/browserDockOpen 逐个登记 owner 或建 panel 注册表 | useLayoutStore | 低 |
| M2-c | `gridSession` 权属澄清（G-03）：注明"前端 epoch 镜像，生命周期真源 Rust" | states.yaml 注释 | 无 |

## M3 — Owner Checker 扩展（P2）

| 项 | 内容 | 影响文件 | 风险 |
|---|---|---|---|
| M3-a | R3 模式化 Terminal/Bookmark/Workspace 越界（Debt-4-2）：补 `COMPONENT_WRITES_BOOKMARK`/`COMPONENT_WRITES_WORKSPACE` | check-semantic-registry.mjs | 中（须先复算既有调用，防误报） |

## M4 — 范围外域治理（P3，最大，需专项决策）

| 项 | 内容 | 决策 |
|---|---|---|
| M4-a | 是否将 git/agent/task/db/graph/session/vault/image/plugin/resource/settings/home/grid-archive/workbench 纳入 Semantic Registry | **待人工 SCR 决策**（建议分期：先安全敏感域 plugin/db/git/clipboard，再其余） |
| M4-b | 每个新增域配 owner + canonical_writer + checker | 随 M4-a |

## 迁移顺序建议

```text
M1 (安全副作用登记)  →  M2 (范围内小修)  →  M3 (R3 扩展)  →  M4 (范围外域, 分期)
```

## 停止条件

本审计**止于此处**。迁移未启动，等待人工确认 M1/M2/M3/M4 范围与优先级。
