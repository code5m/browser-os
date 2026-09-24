# Documentation Maturity Matrix（文档成熟度矩阵）

> Phase 1 交付物（手册 §27 / §37 / §49）。
> 真值快照 HEAD：`8ef19131d0f7fb17dd7edd705af2dead3ada0fcb`。
>
> **判定口径**：D0=无 README；D1=README 存在；D2=责任+边界文档化；D3=Domain+契约+资源+测试文档化；
> D4=机器链接+漂移检查；D5=独立审核就绪。
> 本仓**无机器字段记录 D**，下表为人工派生（依据：`src/capabilities/**/README.md` 搜索结果 = 0）。

---

## 汇总

| 等级 | 数量 | 说明 |
|---|---|---|
| D0（NO MODULE DOC） | 23 | 全部能力 + settings，目录内均无 README.md |
| D1–D5 | 0 | 尚未开始（Phase 1 Pilot 将补 3 个样例） |

> 注意：仓库级/架构级文档（`docs/architecture/**`、`capability-registry/*.yaml`、
> `semantic-registry/*.yaml`）已较丰富（D3+ 级架构文档），但**模块级 README 前门为 0**。
> 这恰是手册 §21「每个 Capability 必须有 README」未达标之处。

---

## 逐模块（节选关键项；完整见 DOMAIN-INVENTORY §1）

| 模块 | D | 证据 |
|---|---|---|
| browser / grid / workspace / terminal / bookmark / credential / database / git / agent / skill / plugin / graph / vault / resource_collection / task / clipboard / apps / tools / session / script / workbench / settings / home | D0 | `search_file README.md` under `src/capabilities/` → 0 命中 |
| src/capability（框架） | D0 | 无框架 README 前门 |
| src/shared / stores / composables / components | D0 | 无共享基础设施 README |

---

## 下一步（不在 Phase 1 物理迁移范围）

- Phase 1 Pilot：为 1 简单(skill) + 1 中等(git 或 database) + 1 复杂(workspace) 能力补 D1→D3 README。
- 推广阶段（Phase D3 全量 rollout）：其余能力 D0→D1+，并由 `check-module-documentation.mjs` 固化门禁。
- 禁止：D 等级高报；README 复制 registry 事实成第二真源（§23）。
