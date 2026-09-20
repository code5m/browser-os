# Semantic Change Request — SCR-20260920-terminal-owner-extraction

> Phase 8E / Train D：把 Terminal 域从 `useSystemStore`（Terminal + Clipboard + Apps 混居）抽取为**独立 owner**，
> 使 Terminal 可物理隔离为 capability 并达到 C3（OPTIONAL / 可组合）。
> 审计依据：`docs/architecture/capability-modularization/phase8e/TRAIN-D-TERMINAL-AUDIT.md`。

---

## New Semantic

名称: `useTerminalStore`（Owner，新符号）—— 取代 `useSystemStore` 作为 terminal 域 owner。

类型:

- [ ] State（**不新增任何 state**，仅迁移既有 11 个受治理状态的 owner）
- [ ] Intent（**不新增任何 intent**，仅迁移既有 intent 的 owner）
- [x] Owner
- [ ] Side Effect

分类:

- [x] CURRENT_FACT（代码中已存在并可验证，本次仅改变**归属与物理位置**）
- [ ] ACCEPTED_ADR
- [ ] TARGET_CONTRACT
- [ ] PROPOSED_CHANGE
- [ ] KNOWN_DEBT

## Why Existing Semantic Cannot Represent

已查询条目：`states.yaml` / `intents.yaml` / `owners.yaml` 的 `terminal` 段（Phase 4 冻结）。

无法表示的原因：现有 registry 用**单个 owner 符号** `useSystemStore` 表达 terminal 域，
但该符号同时是 Clipboard 与 Apps 的 owner（`dependencies.yaml` 的 `known_coupling: Debt-7A-2`）。
物理上把整个 `useSystemStore.ts` 搬进 `src/capabilities/terminal/` 会让 Clipboard/Apps 被
静默并入 Terminal 能力 —— 即「Terminal absent 会连带带走剪贴板」，与组合性目标直接冲突。
因此必须在语义层先给 Terminal 一个**专属 owner 符号**，才能做物理迁移。

## Existing Alternatives

| 既有条目 | 所在文件 | 为何不能用 |
|---|---|---|
| `useSystemStore`（现 terminal owner） | `src/stores/useSystemStore.ts` | 与 Clipboard/Apps 同文件共享 owner，无法表达「Terminal 可独立缺失」 |
| Capability Runtime (`runtime.ts`) | `src/capability/runtime.ts` | Runtime 只持编排元数据（id/state/enabled），RT-13 静态断言禁止其持业务 state；**不得**作为 owner |
| 新建第二份 termPanes | — | 违反 INV-4-1（面板注册表唯一真源）/ `COMPONENT_WRITES_TERMINAL`，R8 会判 `SEMANTIC_STATE_MULTI_OWNER` |

## State / Intent / Owner Impact

影响的状态（**11 个，语义不变，仅 owner 符号与实现路径变更**）:
`terminalOpen` / `termPanes` / `termGrid` / `termGridCount` / `activeTermId` / `autoConfirmCli` /
`termProbeOn` / `m0Cfg` / `m0StartTs` / `droppedChunks` / `droppedBytes`

影响的 Intent（**7 个，语义不变**）:
`addTermPane` / `killTerm` / `termWrite` / `setActiveTerm` / `restartTerm` / `bindTermWriter` / `replayTermHistory`

Owner 是否变化: **是**

```
terminal:
  owner: useSystemStore            →  useTerminalStore
  implementation: src/stores/useSystemStore.ts
                →  src/capabilities/terminal/state/useTerminalStore.ts
```

`useSystemStore` 保留为 **Clipboard + Apps** 的 owner（其 terminal 状态声明全部移除）。

## Second Source of Truth Risk

- [x] 有风险（说明如何避免）

说明：迁移期间同一状态若在两个文件同时声明，即产生第二真源。避免手段：

1. 物理迁移使用 `git mv` 语义（一次提交内「新文件 + 旧文件删除」），不做双写期；
2. `owner_implementations.useTerminalStore.paths` 首项指向新路径，旧 `src/stores/useSystemStore.ts`
   **不列入** Terminal 的候选（避免 RI-DUPLICATE）；
3. 新增 `scripts/check-terminal-owners.mjs`：`useSystemStore.ts` 不得再声明任何 terminal state
   （TERM-01/02 机器强制）；
4. `termPanes` 仍是唯一面板注册表真源（INV-4-1 不变）。

## Checker Impact

需要新增/修改的规则或条目:

- [x] `states.yaml`（11 个 terminal state 的 `owner` + `canonical_writer` 符号；`owner_implementations` 增 `useTerminalStore`；scope 文字）
- [x] `intents.yaml`（terminal 段 7 个 intent 的 `owner` 符号 + 段注释）
- [x] `owners.yaml`（`terminal.owner` + `authorized_callers`；`COMPONENT_WRITES_TERMINAL` 描述中的 owner 名）
- [ ] `side-effects.yaml`（`termProcess` 语义不变，仅 owner 引用名随动；不改 `requires_declaration`）
- [x] `capabilities.yaml`（terminal：`semanticOwner=useTerminalStore`、`status=COMPATIBILITY_WRAPPED`、`activatable=true`、`entrypoint=src/capabilities/terminal`）
- [x] `dependencies.yaml`（`known_coupling Debt-7A-2` 由「Terminal+Clipboard 无法拆分」更新为「已解耦；Clipboard/Apps 仍同处 useSystemStore」）
- [x] `check-semantic-registry.mjs`（R2 夹具中的 terminal owner 文件路径同步）
- [x] `check-capability-composition.mjs`（新增 Terminal C5-TERM-* / C6-TERMINAL-ABSENT）
- [x] 新增 `check-terminal-owners.mjs`（TERM-01/02/03 机器强制）
- [x] `check-terminal-policy.py`（store/pane/resize 路径同步 + `TERM_HISTORY_CLEAR_MISSING` 检测器重基线化）
- [x] `check-terminal-ui-logic.mjs`（对接 per-pane 真实 API；修既有 PRE_EXISTING CHECKER DEBT）

预期 checker 结果：`--self-test` ALL_PASS；真实仓库 `fail=0`（warn/info 数如实上报，不新增 allow-list）。

## ADR Required

- [ ] 需要
- [x] 不需要（**无语义裁决**：本次零新增/零删除 state 与 intent、零语义变更，只把「谁是 owner」从混居
      store 换成专属 owner，并把实现路径搬到能力包内。判据与不变量全部沿用 Phase 4/6A/6B 已冻结结论：
      INV-4-1（termPanes 唯一真源）、INV-4-2（activeTermId ∈ termPanes.id）、`componentWritesTermPanes` REJECTED、
      M3.c 历史隐私红线。）

ADR 编号/文件: 无（沿用 Phase 4 `Phase4-design.md` + Phase 6B writer enforcement 冻结结论）

## Reviewer Decision

- [x] APPROVED
- [ ] REJECTED
- [ ] NEEDS REVISION

裁决理由：Terminal 的 11 个 state 与 7 个 intent 已在 Phase 4 冻结为 CURRENT_FACT 且语义不改；
本次仅解决「owner 混居导致 Terminal 无法单独缺失」的结构性问题，属**归属与物理位置变更**，
有完整 owner 唯一性护栏（新 checker）与 absent 资源不产生证明（composition C6-TERMINAL-ABSENT 动态测试）。

是否确认为"当前事实"而非"提案":

- [x] 是（代码中已存在并可验证；迁移后由 `check-terminal-owners.mjs` 与 R2/R8 持续验证）

Reviewer: autonomous-train-executor
日期: 2026-09-20

## 落地检查（合并前）

- [ ] `node scripts/check-semantic-registry.mjs --self-test` PASS
- [ ] `node scripts/check-semantic-registry.mjs` 无新增 FAIL
- [ ] Registry YAML 已更新
- [ ] 本 SCR 已归档于 `docs/architecture/semantic-changes/`
