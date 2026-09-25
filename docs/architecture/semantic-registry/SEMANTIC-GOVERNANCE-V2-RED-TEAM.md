# Semantic Governance V2 — Freeze Red Team

> 对应规格 Phase B：Final 3 完成后独立红队（21 攻击）+ 冻结。
> 机器真源：`docs/architecture/semantic-registry/{states,owners,intents,side-effects}.yaml` + `scripts/check-semantic-registry.mjs`。
> 所有 BLOCAKER 必须为 0 才允许冻结。

## 自动化门禁证据（来自真实扫描，HEAD=eab29a9）

- `node scripts/check-semantic-registry.mjs --self-test` → `SELF_TEST_RESULT=ALL_PASS`（positive/negative/FP/locator/R6 全过）
- `--json` 真实扫描 → `status=PASS`，`fails=0`，`files_scanned=196`，`warns=6`（既有 R5，非阻断），`infos=63`（observed_not_governed 良性）
- 专项：`R2 fails=[]`、`RI fails=[]`、`R6 fails=[]`、`R8 fails=[]`、`R9 fails=[]`、`R4 fails=[]`

## 21 项攻击清单与裁决

| # | 攻击 | 机制 / 证据 | 裁决 |
|---|---|---|---|
| 1 | duplicate state | R1/R6 真实扫描 fail=0；states.yaml 无重复键（YAML 解析唯一） | PASS |
| 2 | duplicate owner | R8 真实扫描 fail=0；每 state 单一 owner（busy/loading/error/backendReady/recents 为显式共享 observed，非双 owner 真相源） | PASS |
| 3 | illegal writer | R9 真实扫描 fail=0；forbidden_writers 在 owners.yaml 显式声明 | PASS |
| 4 | fake intent | R4 真实扫描 fail=0；intents.yaml 仅登记代码真实存在的意图 | PASS |
| 5 | fake side effect | R5 warn（受控，设计内）；side-effects.yaml 仅登记 call_sites 真实命中 | PASS |
| 6 | store-name-as-domain | 各 store 名=语义 owner（useGraphStore=graph 域），无「以 store 名冒充 domain」 | PASS |
| 7 | directory-owner-as-semantic-owner | capability 目录 owner = manifest.semanticOwner = states.yaml owner（三者一致），无「目录即 owner」幻觉 | PASS |
| 8 | derived stored | R6 真实扫描 fail=0；派生量 forbidden_writers=everyone，无 `.value=` 写穿 | PASS |
| 9 | stale locator | RI 真实扫描 fail=0；owner_implementations 解析全部命中磁盘 | PASS |
| 10 | public contract bypass | R3/R9 fail=0；owners.yaml forbidden_callers 约束组件直写；bridge 调用经 owner intent | PASS |
| 11 | native owner bypass | graph.rs/plugin 原生命令由对应 owner 经 bridge 调用；无跨 owner 原生写 | PASS |
| 12 | resource mismatch | graph.rs 拥有原生有界存储；plugin 资源由原生 install side effect 管理；前端不持有资源真源 | PASS |
| 13 | README second truth | 各 capability README 描述与 registry 一致；无第二真源陈述 | PASS |
| 14 | registry second truth | YAML 为机器单源；coverage-matrix.md 为投影，不重复真相 | PASS |
| 15 | checker weakening | 本批次仅新增锚点 + 注册，未删减任何断言；R6 增强为「仅 declaredStored 判违规」，更强非更弱 | PASS |
| 16 | blanket allowlist | checker 无 blanket allowlist；observed_not_governed 为显式命名列表（非通配） | PASS |
| 17 | framework service forced into capability | settings/session/resource/workbench 保持 NOT_APPLICABLE；未升格为 capability | PASS |
| 18 | workspace God Store regression | 代码实证 useWorkspaceStore 仅 audit+recents+编排；子域（file/artifact/repo/script/snippet）各自 owner（Phase 8C-0 已分解）；SCR-Final-3 显式防回归 | PASS |
| 19 | plugin five-state collapse | 代码实证 plugin store 不持五态；detail.state 为后端五态投影（ACTIONS_FOR 仅门控按钮）；红线禁止持久化/回显签名原文等 | PASS |
| 20 | graph projection promoted to source truth | 代码实证 graph 前端 nodes/edges 为 graph.rs 投影缓存；states.yaml 标 `derived_from: graph.rs`；前端不重建图/不引依赖 | PASS |
| 21 | Browser/Grid semantic regression | Phase E 冻结语义未改动；本批次零业务代码；mainView/gridOpen/desiredGridVisibility/isBrowserVisible 字节级未变 | PASS |

## 诚实债务（非 BLOCKER，记录在案）

- `busy`（useBookmarkStore 键）被 plugin 复用、`recents`（useHomeStore 键）被 workspace 复用 → 跨域同名碰撞，已显式标注，建议 Phase C 改名（workspaceRecents / pluginBusy）以彻底消除语义歧义。
- `loading`/`error`/`backendReady` 为跨域共享 observed 态，全局 observed 集合覆盖，非第二真相源。
- 6 条 R5 warn 为 browser/grid 冻结域既有认知标记，非本次治理面。

## 冻结裁定

BLOCKER = 0 → **允许冻结**。Tag：`semantic-governance-v2-full-coverage-pass`（annotated，仅本地，不 push）。

## 最终语义计数（冻结态）

- FULLY_GOVERNED = 26（capability 16 + framework 10）
- OWNER_PENDING_SCR = 0
- UNGOVERNED = 0
- UNKNOWN = 0
- NOT_APPLICABLE = 4（settings/session/resource/workbench）
- REGISTERED_STATES ≈ 204 / INTENTS = 106 / OWNERS = 25 / SIDE_EFFECTS = 42
- BROWSER_GRID_FROZEN_SEMANTICS = UNCHANGED
- BUSINESS_CODE_CHANGED = NO
