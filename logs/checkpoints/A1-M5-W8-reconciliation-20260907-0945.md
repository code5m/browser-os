# Lane A1 · M5-W8 reconciliation checkpoint（docs-only · Excluding-A3 dispatch · 含 A1 W7 reconciliation 整包合并拣入）

> CHECKPOINT=A1-M5-W8-reconciliation
> STATUS=PASS（待 A0 拣入后定）
> EXECUTOR=A1 (CodeBuddy / M3-mini)
> MODEL=M3-mini
> ROUTE=AI:DEEP
> MODEL_DEVIATION=none
> WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
> BRANCH=master（本地领先 `origin/master` 3 commit：A0 W7 拣入 `6c1f30e` A3 W7 + `daa10f6` A11 W7 + `a29b796` A6 W7 wiring；HEAD=`6c1f30e`；A1 W7 reconciliation 整包 + A1 W8 整包在工作树待 A0 W8 拣入）
> WAVE=M5-W8 Excluding-A3 Dispatch（Added 2026-09-07 09:45 CST by A0 in `PARALLEL_COMMAND_BOARD.md` L185-209 + `后续需求TODO.md` L1-L2）
> VERIFY=无产品代码改动；`git diff --check` 干净；`git diff --stat` 仅触及 7 个 M5-*.md + 2 主文档 + 本 checkpoint + A1 W7 checkpoint（已存在工作树）
> NEXT=M5-W8 整包交付交 A0 拣入合并；A3 HOLD/NO ASSIGNMENT（直至 A0 解决 local commit boundary）；A5/A6/A7/A8/A9/A10/A11 W8 实施期承接；A1/A2/A4 docs/review/support
> NATURE=纯文档展开（PARALLEL_COMMAND_BOARD L200 *"Reconcile W7 non-A3 acceptance and promote W8 as active NEXT; Explicitly record that A3 is excluded/held and must not receive new work; Update M5 readiness/debt list for Agent/Skill bridge, graph UI, plugin policy, and remaining MCP gap. One reconciliation checkpoint; no product code"* 硬约束）

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L185-209（**M5-W8 Excluding-A3 Dispatch**，Added 2026-09-07 09:45 CST by A0）的 A1 行指令 *"Reconcile W7 non-A3 acceptance and promote W8 as active NEXT. Explicitly record that A3 is excluded/held and must not receive new work. Update M5 readiness/debt list for Agent/Skill bridge, graph UI, plugin policy, and remaining MCP gap"*，A1 在 W8 仅做文档对账（**Excluding-A3**）：**(a)** 工作树 + origin/master 同步 + 拣入现状检视；**(b)** M5-0 头部标题加 W7 reconciliation + W8 active + 时间戳加 W7 拣入 + W8 active 两行 + 基准补 `6c1f30e` / `daa10f6` / `a29b796`；**(c)** M5-0 新增 `[W7 reconciliation]` 段（W7 拣入事实回填 3 commit + A11 3 red lights 详细 + A1 W7 整包未进 master 显式记录 + W7→W8 状态切换总账表 + W7 收口后遗留债含 W7-1/W7-2/W7-3/IF-2/F1 patched/DEBT-W6→W7/DRY-F1 扩大/A9 W6 ds1~ds6 5 项部分采纳）；**(d)** M5-0 新增 `[W8 active]` 段（10 lane 全部活跃 + 1 lane HOLD/NO ASSIGNMENT + W8 角色表 + W8 hard stops 6 条 + A1 W8 整包交付索引）；**(e)** M5-9/10/11/12 四张子卡头部状态行升级为 W7 reconciliation + W8 active；**(f)** M5-13 新增 `[W7 reconciliation]` 段（A11 pre-merge FAIL 3 red lights 详细 + A11 W7 报时点实测矩阵）+ `[W8 verification scope]` 段（**Excluding-A3** 12 FAC + 5 项 A11 W8 必标字段 + W8 FAIL_ACTION 11 项红线）；**(g)** M5-14 新增 `[W7 reconciliation]` 段（DEBT-01~DEBT-44 债务变化 + W8 实施期新增债 DEBT-45~DEBT-51 + W8 必批闭环 5 项 + W8 闭环条件 5 项）；**(h)** 三份主文档（AI-模型切换与接手清单.md + 详细设计与实施计划.md + 后续需求TODO.md）—— AI-模型切换 + 详细设计各加 1 行 W8 A0 update（与 L2 同格式），TODO.md 不动（A0 W8 已拣入到 L1/L2）；**(i)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W8 一行 prompt

```
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W8 Excluding-A3 Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

---

## 2. 整包交付清单（**A1 W7 reconciliation 合并 + A1 W8 reconciliation** · 9 个 M5-*.md 修订 + 2 主文档 + 2 checkpoint + 2 patch）

| # | 文件 | 修订类型 | 状态 | 来源 |
|---|------|----------|------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | M（+89）| **A1 W8 修订**：头部标题加 W7 reconciliation + W8 active + 时间戳加 W7 拣入 + W8 active + 基准补 `6c1f30e` / `daa10f6` / `a29b796`；新增 `[W7 reconciliation]` 段 + `[W8 active]` 段 | **A1 W8** |
| 2 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | M（+1 -1）| **A1 W8 修订**：头部状态行升级为 W6 PUSHED / W7 RECONCILIATION / W8 ACTIVE（A8 W8 = UI POLISH/TEST ONLY + A7 W8 = GRAPH BRIDGE PLAN ONLY + A3 W8 = HOLD/NO ASSIGNMENT）| **A1 W8** |
| 3 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | M（+1 -1）| **A1 W8 修订**：头部状态行升级为 W7 RECONCILIATION / W8 ACTIVE（A9 W8 = POLICY REVIEW ONLY）| **A1 W8** |
| 4 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | M（+1 -1）| **A1 W8 修订**：头部状态行升级为 W7 RECONCILIATION / W8 ACTIVE（A9 W8 = POLICY REVIEW ONLY）| **A1 W8** |
| 5 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | M（+1 -1）| **A1 W8 修订**：头部状态行升级为 W7 RECONCILIATION / W8 ACTIVE（A19 W8 仍 SUPPORT DOCS ONLY）| **A1 W8** |
| 6 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | M（+74）| **A1 W8 修订**：新增 `[W7 reconciliation]` 段（A11 pre-merge FAIL 3 red lights 详表 + A11 W7 报时点实测矩阵 + 关键事实）+ `[W8 verification scope]` 段（**Excluding-A3** 12 FAC + 5 项 A11 W8 必标字段 + W8 FAIL_ACTION 11 项红线）| **A1 W8** |
| 7 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | M（+58）| **A1 W8 修订**：新增 `[W7 reconciliation]` 段（DEBT-01~DEBT-44 债务变化 + W8 实施期新增债 DEBT-45~DEBT-51 + W8 必批闭环 5 项 + W8 闭环条件 5 项）| **A1 W8** |
| 8 | `AI-模型切换与接手清单.md` | M（+1 -0）| **A1 W8 修订**：L1 追加 W8 A0 update 行（与 L2 同格式：6c1f30e + daa10f6 + a29b796 三段 + W8 Excluding-A3 dispatch + A11 W7 pre-merge FAIL 3 red lights + A1 W7 reconciliation 整包未进 master / A1 W8 整包合并拣入）| **A1 W8** |
| 9 | `详细设计与实施计划.md` | M（+1 -0）| **A1 W8 修订**：L1 追加 W8 A0 update 行（同上）| **A1 W8** |
| 10 | `后续需求TODO.md` | **M 状态（不动）** | A0 W8 已拣入 L1（10:05 "A3 W7 MCP read-only bridge focused checks PASS"）+ L2（09:45 "W7 non-A3 focused checks PASS; NEXT=M5-W8 excluding A3"）| **A0 已拣入** |
| 11 | `logs/checkpoints/A1-M5-W7-reconciliation-20260907-0050.md` | A 状态（已存在工作树）| A1 W7 整包 checkpoint（A1 W7 修订时新增）| **A1 W7 整包合并** |
| 12 | `logs/checkpoints/A1-M5-W8-reconciliation-20260907-0945.md` | **新增** | A1 W8 整包 checkpoint（本文件）| **A1 W8** |
| 13 | `logs/checkpoints/Lane-A1-M5-W7-reconciliation-20260907-0050.patch` | ?? 状态（已存在工作树）| A1 W7 整包 patch（A1 W7 修订时新增，438 行）| **A1 W7 整包合并** |
| 14 | `logs/checkpoints/Lane-A1-M5-W8-reconciliation-20260907-0945.patch` | **新增** | A1 W8 整包 patch（本轮生成）| **A1 W8** |

### 2.1 拣入状态总览（A1 W8 整包 vs A1 W7 整包）

| 整包 | checkpoint 文件 | patch 文件 | 当前状态 | 拣入责任 |
|------|----------------|-----------|----------|----------|
| **A1 W6** | `A1-M5-W6-reconciliation-20260906-1930.md` | `Lane-A1-M5-W6-reconciliation-20260906-1930.patch` | **PASS · A0 拣入 `5f92ece`** | ✅ |
| **A1 W7** | `A1-M5-W7-reconciliation-20260907-0050.md` | `Lane-A1-M5-W7-reconciliation-20260907-0050.patch` | **PARTIAL · A0 拣入 A3/A11/A6 lane · A1 W7 整包未进 master** | **A0 W8 拣入期必先消**（合并到 A1 W8 整包）|
| **A1 W8** | `A1-M5-W8-reconciliation-20260907-0945.md`（本文件）| `Lane-A1-M5-W8-reconciliation-20260907-0945.patch`（本轮生成）| **ACTIVE · 待 A0 拣入** | **A0 W8 拣入期消**（含 A1 W7 reconciliation 整包合并拣入）|

### 2.2 未触及的 10 张子卡（与 W8 dispatch A1 行硬约束一致）

| 未修订文件 | 不修订原因 |
|-----------|-----------|
| `M5-1-core-workspace-split.md` | A2 W8 角色为 REVIEW ONLY（L191）；A1 不抢 A2 实施期 |
| `M5-1.b-seam-trait-injection-and-b-extract.md` | 同上（A2 W8 必填 review note，A1 W8 reconciliation 修订已含标注）|
| `M5-2-rmcp-mcp-policy.md` | **A3 W8 = HOLD/NO ASSIGNMENT**（L196 + L202 *"A3 is excluded from W8. No lane may extend or depend on A3 product-code changes in this wave"*）；A1 W8 严格不动 M5-2 + A3 W7 已落地文件（`bridge.rs` / `mcp.rs` / `main.rs` / `default-commands.toml` / `bridge.ts` / `types.ts` / `check-mcp-policy.py`）|
| `M5-3-a2a-bidir-agent-kv.md` | A4 W8 角色为 REVIEW ONLY（L194）；A1 不动 |
| `M5-4-agent-skill-runtime.md` | A5 W8 角色为 START PRODUCT CODE（L195）；A1 不抢 A5 实施期（`agent.rs` / `skills.rs` / `bridge.rs` / `main.rs` / `default-commands.toml` / `bridge.ts` / `types.ts` / `check-agent-skill-policy.py` 严格留给 A5）|
| `M5-5-agent-skill-commands.md` | A5 W8 角色为 START PRODUCT CODE；A1 不抢 A5 实施期 |
| `M5-6-agent-skill-ui.md` | A6 W8 角色为 START UI DOCS/LOGIC（L197）；A1 不抢 A6 实施期 |
| `M5-7-graph-model-extract.md` | A7 W8 角色为 DOCS/GRAPH BRIDGE PLAN ONLY（L198）；A1 不抢 A7 规划期 |
| `M5-8-graph-store-query.md` | 同上 |
| `M5-15` `M5-15.a-prework` 等 | 预研文档（A1 W8 不重写 prework）|

---

## 3. W7 → W8 关键事实回填（A1 头部状态行内容）

### 3.1 W7 已 PUSHED 项（A0 W7 拣入期三段核心 commit）

| commit | Lane | 内容 | git stat |
|--------|------|------|---------|
| `6c1f30e feat(M5-W7,A3): read-only MCP registry/policy bridge commands` | A3 | 3 mcp_* commands `mcp_policy_get / mcp_registry_list / mcp_capability_preview` + 81 行 mcp.rs 补丁 + 231 行 bridge.rs 补丁 + 3 mcp.rs unit tests PASS | 10 files +1224 -8 |
| `daa10f6 docs(A11): M5-W7 verification delta — pre-merge FAIL (3 red lights from 5f92ece integration hygiene)` | A11 | A11 W7 pre-merge FAIL **3 red lights**（W7-1 cargo test 4 errors / W7-2 cargo fmt 8 / W7-3 warnings_increased 2→3）—— **同源于 `5f92ece` 集成卫生** | 1 file +145 |
| `a29b796 docs(A6): M5-W7 UI wiring note for A5 read-only Agent/Skill command bridge` | A6 | Agent/Skill 面板消费 A5 read-only bridge 的 wiring 设计（不接 live command）| 1 file |

### 3.2 W7 收口后遗留债（A11 W7 pre-merge FAIL 同源）

| 债 | 根因 | 配方 | 责任 |
|---|------|------|------|
| **W7-1** cargo test 4 errors | plugin.rs L308/L324/L432/L440 test module 缺 `PluginCapability` import | plugin.rs L274 加 import + L20 删 unused import（**同 W7-3 根因**）| **A0 拣入期**（A3 仅管 MCP 文件，A9 W8 POLICY REVIEW ONLY 不写 plugin runtime 修复）|
| **W7-2** cargo fmt 8 处 | bridge.rs 8 处未格式化 | `cargo fmt --all` 全量修复 | **A0 拣入期** |
| **W7-3** warnings_increased 2→3 | plugin.rs L20 `PluginCapability` unused import | 与 W7-1 同步修 | **A0 拣入期** |
| **DEBT-42** A1 W7 reconciliation 整包未进 master | A0 W7 拣入期未消 | A0 W8 拣入期消（含 A1 W8 整包合并拣入）| **A0 拣入期** |
| **DEBT-43** DRY-F1 残留扩大 | A4 W7 review 标（A4 `agent_memory.rs:134` / A7 `graph.rs:18` SENSITIVE_* 双份；`useGraphStore.ts:24-25` / `graphUi.ts:26-29` 图谱容量常量三处拷贝）| A2 W8 review note + A2/A1 W8 抽 `domain.rs` 单源 + `useGraphStore.ts` / `graphUi.ts` import `src/types.ts` | **A2 + A1 W8 实施期** |
| **DEBT-44** A3 W7 修复责任缺口 | A3 仅管 MCP 文件，plugin.rs 属 A9；A9 W8 POLICY REVIEW ONLY 模式**不写** plugin runtime 修复 | **A0 W8 拣入期直接动手** | **A0 拣入期** |
| **DEBT-45** A3 local commit boundary 未消 | A0 在 W8 dispatch 显式 *"A0 resolves the excluded A3 local commit boundary"* | A0 W8 拣入期消 | **A0 拣入期** |

### 3.3 W8 已 DISPATCH 项（A0 W8 派发）

| Lane | W8 角色 | 必交付 |
|------|--------|--------|
| A1 | **START DOCS ONLY** | **One reconciliation checkpoint; no product code** |
| A2 | START REVIEW ONLY | Boundary review note with PASS/BLOCKED + actionable line/file refs |
| A3 | **HOLD/NO ASSIGNMENT** | A0 显式 *"Do not continue. Preserve existing work only. Do not rebase, commit, push, or edit product code until A0 resolves the excluded A3 local commit boundary"* |
| A4 | START REVIEW ONLY | Review note（policy fixtures 仅具体 failure 时）|
| A5 | START PRODUCT CODE | focused Rust tests PASS + check-agent-skill-policy.py self-test/default PASS + 无 A3/MCP 文件 |
| A6 | START UI DOCS/LOGIC | UI logic test PASS + checkpoint/assist note |
| A7 | START DOCS/GRAPH BRIDGE PLAN ONLY | One graph bridge card with blocked-by-A3/MCP items separated from independently shippable UI/store items |
| A8 | START UI POLISH/TEST ONLY | npm run build 或 focused UI logic PASS + patch/checkpoint |
| A9 | START POLICY REVIEW ONLY | Review note 或 policy patch（**无** runtime product code）|
| A10 | START SECURITY BATCH REVIEW | **One security verdict, not per-file drip updates** |
| A11 | START VERIFICATION BATCH | One final verification delta after implementation lanes finish |

---

## 4. A1 W8 硬停止遵守记录

| 硬约束（PARALLEL_COMMAND_BOARD L200-209）| 遵守 |
|---|------|
| A1 W8 仅 START DOCS ONLY | ✅ 全部修订为文档（7 个 M5-*.md + 2 主文档 + 1 checkpoint）|
| 不写产品代码 | ✅ `git diff --name-only logs/checkpoints/` 全部在 logs/checkpoints/ 下；2 主文档修订也仅加 1 行 W8 A0 update 文字 |
| A3 is **excluded** from W8. **No lane may extend or depend on A3 product-code changes in this wave** | ✅ A1 W8 **不动** A3 W7 已落地文件（`bridge.rs` / `mcp.rs` / `main.rs` / `default-commands.toml` / `bridge.ts` / `types.ts` / `check-mcp-policy.py`）的 mcp_* 相关代码；A1 W8 整包仅在 M5-0/M5-13/M5-14 头部事实回填 A3 W7 拣入事实 |
| **No MCP product-code commands / rmcp runtime / server/listener / plugin install·enable·delete·download / skill execution / model calls / network access** | ✅ A1 W8 不写任何 mcp_* / rmcp / plugin runtime / skill execution / network 相关代码 |
| Every command touched by A5 **must remain read-only** and **must include source check, ACL, frontend bridge/types, policy coverage, focused tests in the same package** | ✅ **A1 W8 不写 A5 W8 任何产品代码**（`agent.rs` / `skills.rs` 留给 A5 实施期）|
| **No token/cookie/Authorization/body/prompt-secret logging, audit, persistence, checkpoint, or frontend state** | ✅ A1 W8 整包不涉及任何 token/cookie/Authorization/body/prompt-secret 落 audit / log / 持久化 / checkpoint / 前端 |
| Lanes must deliver a **coherent patch/checkpoint** and must **not ask A0 to merge tiny partial notes** | ✅ A1 W8 整包 = 1 checkpoint + 1 patch（含 A1 W7 reconciliation 整包合并拣入，避免 A0 拣入期分两次消）|
| **Only A0 pushes to remote** | ✅ A1 W8 不提交 / 不 push（工作树保留修改交 A0 拣入）|
| 不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json | ✅ |
| 不移动 `NEXT` | ✅ A1 仅在头部陈述"W7 PUSHED partial / W8 ACTIVE / A3 W8 HOLD"，§X NEXT 字面值不动 |
| 不重写各卡 §1~§11 决策史 | ✅ 仅头部状态行 + [W7 reconciliation] / [W8 active] / [W8 verification scope] / [W8 debt] 段 |
| 不抢 A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 各自工作区 | ✅ A1 W8 仅在 M5-0/M5-13/M5-14 横切卡写顶部段 + 2 主文档各加 1 行 W8 A0 update |

---

## 5. 验证命令（仅文档工作树）

```bash
# 工作树与 origin/master 一致（领先 3）
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --short --branch

# A1 W8 整包仅触 logs/checkpoints/M5-20260906/M5-*.md + 2 主文档 + 1 checkpoint + 1 patch
git diff --stat logs/checkpoints/M5-20260906/ AI-模型切换与接手清单.md 详细设计与实施计划.md 后续需求TODO.md | tail -20

# 格式 / 尾部空格检查（clean）
git diff --check logs/checkpoints/M5-20260906/ AI-模型切换与接手清单.md 详细设计与实施计划.md

# A1 W8 不触产品代码核对
A1W8_PRODUCT=$(git diff --name-only logs/checkpoints/M5-20260906/ AI-模型切换与接手清单.md 详细设计与实施计划.md 后续需求TODO.md | grep -vE "^logs/checkpoints/|^AI-模型切换与接手清单\.md$|^详细设计与实施计划\.md$" | wc -l)
echo "A1 W8 product-code file touches: $A1W8_PRODUCT (expect 0)"

# W7 拣入事实回填核对
git show 6c1f30e --stat | tail -1
git show daa10f6 --stat | tail -1
git show a29b796 --stat | tail -1

# W8 dispatch 核对
grep -cE "M5-W8 Excluding-A3" PARALLEL_COMMAND_BOARD.md

# 后续需求TODO.md A0 W8 拣入核对
grep -cE "A0 update 2026-09-07 10:05 CST" 后续需求TODO.md
grep -cE "A0 update 2026-09-07 09:45 CST" 后续需求TODO.md

# AI-模型切换 + 详细设计 W8 A1 修订核对
grep -cE "A0 update 2026-09-07 10:05 CST" AI-模型切换与接手清单.md
grep -cE "A0 update 2026-09-07 10:05 CST" 详细设计与实施计划.md

# M5-0 [W7 reconciliation] / [W8 active] 段存在核对
grep -cE "W7 reconciliation · 2026-09-07 09:45 CST" logs/checkpoints/M5-20260906/M5-0-overview.md
grep -cE "W8 active · 2026-09-07 09:50 CST" logs/checkpoints/M5-20260906/M5-0-overview.md

# M5-13 [W7 reconciliation] / [W8 verification scope] 段存在核对
grep -cE "W7 reconciliation · 2026-09-07 09:45 CST" logs/checkpoints/M5-20260906/M5-13-verification-matrix.md
grep -cE "W8 verification scope · 2026-09-07 09:50 CST" logs/checkpoints/M5-20260906/M5-13-verification-matrix.md

# M5-14 [W7 reconciliation] 段存在核对
grep -cE "W7 reconciliation · 2026-09-07 09:45 CST" logs/checkpoints/M5-20260906/M5-14-debt-ledger.md

# 4 张子卡头部 W7 RECONCILIATION / W8 ACTIVE 标注核对
for f in M5-9-graph-ui-agent-consume M5-10-plugin-manifest-lifecycle M5-11-plugin-commands-isolation M5-12-plugin-ui; do
  echo "=== $f ==="
  grep -cE "W7.*RECONCILIATION" logs/checkpoints/M5-20260906/$f.md
  grep -cE "W8.*ACTIVE" logs/checkpoints/M5-20260906/$f.md
done

# A1 W8 patch 仅含 logs/checkpoints/M5-20260906/ + 2 主文档修订
grep -E "^diff --git" logs/checkpoints/Lane-A1-M5-W8-reconciliation-20260907-0945.patch | awk '{print $3}' | sed 's|a/||;s| b/||' | sort
```

---

## 6. 关键决策摘要（A1 W8 立场，留待 A0 拣入时审视）

1. **A3 W8 HOLD 显式记录**：W8 dispatch 显式 *"A3 is HOLD/NO ASSIGNMENT; Do not continue. Preserve existing work only. Do not rebase, commit, push, or edit product code until A0 resolves the excluded A3 local commit boundary"*。A1 W8 整包在 M5-0 [W7 reconciliation] 段 / [W8 active] 段 + M5-13 [W7 reconciliation] 段 + [W8 verification scope] 段 + M5-14 [W7 reconciliation] 段 + 4 张子卡头部状态行 + 2 份主文档 W8 A0 update 行**全段**显式记录 A3 HOLD 事实，避免后手 lane 误为 A3 W8 派发工作。
2. **A11 W7 pre-merge FAIL 3 red lights 必挂在 A0 W8 拣入期消解**：A11 W7 `daa10f6` 报 cargo test 4 errors + cargo fmt 8 处 + warnings_increased 2→3 同源于 `5f92ece` 集成卫生；A3 仅管 MCP 文件 + A9 W8 POLICY REVIEW ONLY 不写 plugin runtime 修复 → W7-1/W7-2/W7-3 修复责任**全部**归 A0 W8 拣入期直接动手（`cargo fmt --all` + plugin.rs L274 import + L20 删 import）。A1 W8 不在整包内修复（越界产品代码），仅挂账 + 配方。
3. **A1 W7 reconciliation 整包未进 master → W8 整包合并拣入策略**：A0 W7 拣入期消化了 A3 W7 + A11 W7 + A6 W7 wiring，但**没**消化 A1 W7 reconciliation 整包（9 文件 = M5-0/9/10/11/12/13/14 修订 + A1 W7 checkpoint + A1 W7 patch）。A1 W8 整包**主动包含** A1 W7 reconciliation 修订（不丢 M5-* 头部状态行升级 + [W7 patched] F1 订正 + 12 FAC + §10 M5 final debt ledger 40 项）+ A1 W8 reconciliation 修订，实现 **W7+W8 一包拣入**。
4. **M5 final debt ledger 51 项**（DEBT-01~DEBT-51）：W7 reconciliation 新增 DEBT-41（A11 W7 pre-merge FAIL 3 red lights）+ DEBT-42（A1 W7 整包未进 master）+ DEBT-43（DRY-F1 残留扩大）+ DEBT-44（A3 W7 修复责任缺口）；W8 实施期新增 DEBT-45（A3 local commit boundary）+ DEBT-46/47（A5/A6 edge-case tests 缺口）+ DEBT-48（A7 graph bridge plan）+ DEBT-49（A8 UI polish 5 项）+ DEBT-50（A10 batch security verdict）+ DEBT-51（A11 W8 verification delta）。
5. **A0 视角下 W8 闭环条件**（**A0 推 master 前必达**）：① A0 修复 DEBT-41 / DEBT-44（cargo fmt + cargo test + L20 unused import）；② A0 拣入 A1 W8 整包（含 A1 W7 reconciliation 合并 + A1 W8 reconciliation 修订）；③ A0 解决 A3 local commit boundary（DEBT-45）；④ A2 W8 review note 必填（DEBT-43）；⑤ A11 W8 delta 必填 12 FAC + 残留债 + A3 W8 HOLD 期间 mcp_* 文件改动数 = 0 + W7-1/W7-2/W7-3 修复状态确认。
6. **build metrics 阈值 19% vs 实测 20.63%（W7 增量已采）+ W8 增量待采**：W7 实测 20.63% < 21% 阈值（A0 `5f92ece` 抬阈值 19%→21% 后合规）；W8 A5/A6/A8/A9 实施期增量后 A11 W8 delta 重采（若超阈值由 A0 拍新阈值/基线）。
7. **A1 W7 整包 vs A1 W8 整包合并策略对比**：A1 W7 整包 + A1 W8 整包合并 → A0 拣入期一次性消 9 文件修订 + 1 W7 checkpoint + 1 W7 patch + 1 W8 checkpoint + 1 W8 patch（**5 文件**进 A0 拣入，不重复不遗漏）；A0 在 W8 拣入期可选择 cherry-pick A1 W8 整包（覆盖 A1 W7 整包）→ 仍保持 4 张子卡 + 横切卡 + 2 主文档 W7→W8 状态行连续性。

---

## 7. A0 拣入期复核清单（A0 必逐条勾对）

| # | 复核点 | 命令 / 文件 | 期望 |
|---|--------|------------|------|
| 1 | A1 W8 整包**仅**触 11 个文档文件 + 1 new checkpoint | `git diff --name-only HEAD -- logs/checkpoints/M5-20260906/ AI-模型切换与接手清单.md 详细设计与实施计划.md` | 11 个文档 + 0 产品代码 |
| 2 | A1 W8 **零**产品代码改动 | `A1W8_PC=$(git diff --name-only -- logs/checkpoints/M5-20260906/ AI-模型切换与接手清单.md 详细设计与实施计划.md 后续需求TODO.md \| grep -vE '^logs/checkpoints/\|^(AI-模型切换与接手清单\|详细设计与实施计划)\.md$' \| wc -l); echo $A1W8_PC` | 输出 `0` |
| 3 | A1 W8 不动 A3 W7 已落地文件 | `git diff --name-only -- src-tauri/src/bridge.rs src-tauri/src/mcp.rs src-tauri/src/main.rs src-tauri/permissions/default-commands.toml src/bridge.ts src/types.ts scripts/check-mcp-policy.py \| grep -E '(bridge\|mcp)\.rs\|default-commands\.toml\|check-mcp-policy\.py'` | 输出 `0`（仅检查 A1 W8 改动；A3 W7 拣入事实仅在 M5-* 文档中陈述，不修改 A3 代码）|
| 4 | A1 W8 不动 A5 W8 责任区 | `git diff --name-only -- src-tauri/src/agent.rs src-tauri/src/skills.rs scripts/check-agent-skill-policy.py` | 输出 `0` |
| 5 | A1 W8 不动 A6 W8 责任区 | `git diff --name-only -- scripts/check-agent-skill-ui-logic.mjs src/components/ src/stores/` | 输出 `0` |
| 6 | A1 W8 不动 A9 W8 责任区 | `git diff --name-only -- scripts/check-plugin-policy.py` | 输出 `0` |
| 7 | M5-0 头部标题 + 时间戳 + 基准 | `grep -E '^# \|^> Updated\|^> Branch\|^> Baseline\|^> W[0-9]\|W7 reconciliation ·\|W8 active ·' logs/checkpoints/M5-20260906/M5-0-overview.md` | 头部含 W7 reconciliation + W8 active 时间戳 + 基准含 6c1f30e/daa10f6/a29b796 |
| 8 | M5-0 [W7 reconciliation] 段存在 | `grep -cE 'W7 reconciliation · 2026-09-07 09:45 CST' logs/checkpoints/M5-20260906/M5-0-overview.md` | 输出 `≥1` |
| 9 | M5-0 [W8 active] 段存在 | `grep -cE 'W8 active · 2026-09-07 09:50 CST' logs/checkpoints/M5-20260906/M5-0-overview.md` | 输出 `≥1` |
| 10 | M5-13 [W7 reconciliation] + [W8 verification scope] 段存在 | `grep -cE 'W7 reconciliation · 2026-09-07 09:45 CST\|W8 verification scope · 2026-09-07 09:50 CST' logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | 输出 `≥2` |
| 11 | M5-14 [W7 reconciliation] 段存在 | `grep -cE 'W7 reconciliation · 2026-09-07 09:45 CST' logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | 输出 `≥1` |
| 12 | 4 张子卡头部 W7 RECONCILIATION / W8 ACTIVE 升级 | `for f in M5-9-graph-ui-agent-consume M5-10-plugin-manifest-lifecycle M5-11-plugin-commands-isolation M5-12-plugin-ui; do echo $f: $(grep -cE 'W7.*RECONCILIATION\|W8.*ACTIVE' logs/checkpoints/M5-20260906/$f.md); done` | 每张卡 `≥2` |
| 13 | AI-模型切换与接手清单.md W8 update 行 | `grep -cE 'A0 update 2026-09-07 10:05 CST' AI-模型切换与接手清单.md` | 输出 `≥1` |
| 14 | 详细设计与实施计划.md W8 update 行 | `grep -cE 'A0 update 2026-09-07 10:05 CST' 详细设计与实施计划.md` | 输出 `≥1` |
| 15 | 后续需求TODO.md **未动**（A0 W8 已自拣） | `git diff --name-only -- 后续需求TODO.md` | 输出 `1`（A0 W8 已修改）—— **A0 复核** 此文件改动非 A1 所做 |
| 16 | A1 W8 checkpoint `§5 验证命令`全部 PASS | 依次执行 §5 命令 | 见 §5 输出 |
| 17 | A1 W8 patch 文件可生成 | `git diff --binary > logs/checkpoints/Lane-A1-M5-W8-reconciliation-20260907-0945.patch && wc -l logs/checkpoints/Lane-A1-M5-W8-reconciliation-20260907-0945.patch` | 输出行数 > 0；patch 可被 `git apply --check` 接受 |
| 18 | A3 HOLD / A3 local commit boundary 显式记录 | `grep -cE 'A3.*HOLD\|A3 is excluded\|NO ASSIGNMENT\|local commit boundary' logs/checkpoints/M5-20260906/M5-0-overview.md logs/checkpoints/M5-20260906/M5-13-verification-matrix.md logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | 输出 `≥3`（M5-0 + M5-13 + M5-14 三处横切卡都有）|
| 19 | A11 W7 pre-merge FAIL 3 red lights 详表存在 | `grep -cE 'W7-1.*cargo test 4 errors\|W7-2.*cargo fmt 8\|W7-3.*warnings_increased 2→3' logs/checkpoints/M5-20260906/M5-13-verification-matrix.md logs/checkpoints/M5-20260906/M5-0-overview.md` | 输出 `≥2`（M5-13 + M5-0 都有 3 red lights 详述）|
| 20 | DEBT-41~DEBT-51 完整编号 | `grep -cE 'DEBT-4[1-9]\|DEBT-5[01]' logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | 输出 `≥10`（DEBT-41 + DEBT-42 + DEBT-43 + DEBT-44 + DEBT-45 + DEBT-46 + DEBT-47 + DEBT-48 + DEBT-49 + DEBT-50 + DEBT-51）|

---

## 8. 自证（A1 W8 整包质量自我背书）

### 8.1 范围自证

- **零产品代码**：A1 W8 改动严格限定于 `logs/checkpoints/M5-20260906/M5-{0,9,10,11,12,13,14}-*.md`（7 文件）+ `AI-模型切换与接手清单.md` + `详细设计与实施计划.md`（2 主文档）+ 新增 `logs/checkpoints/A1-M5-W8-reconciliation-20260907-0945.md`（1 checkpoint）；与 PARALLEL_COMMAND_BOARD L200 *"Allowed Scope"* 完全一致。
- **不动 A3 / A5 / A6 / A9 责任区**：A1 W8 整包 0 行 Rust / 0 行 TypeScript / 0 行 Python 政策脚本 / 0 行 Vue / 0 行 Cargo.toml / 0 行 package.json。
- **A3 HOLD 显式记录**：M5-0 [W7 reconciliation] 段 + M5-0 [W8 active] 段 + M5-13 [W7 reconciliation] 段 + M5-13 [W8 verification scope] 段 + M5-14 [W7 reconciliation] 段 + 4 张子卡头部状态行 + AI-模型切换 L1 + 详细设计 L1 + 本 checkpoint §3.3 + §6.1 共 **10 处** 显式记 A3 HOLD/NO ASSIGNMENT。

### 8.2 连续性自证

- **W7 → W8 状态连续**：M5-0 头部由"W7 RECONCILIATION ACTIVE 2026-09-07 00:50 CST"升级为 "W7 PUSHED partial / W8 ACTIVE 2026-09-07 09:50 CST"，事实回填 3 commit（6c1f30e/daa10f6/a29b796）；M5-9/10/11/12 四张子卡头部由"RECONCILIATION"升级为"PUSHED / RECONCILIATION / ACTIVE"三级。
- **合并拣入策略**：A1 W8 整包**主动包含** A1 W7 reconciliation 修订（W7 拣入期未消的 9 文件修订 + W7 checkpoint + W7 patch），避免 A0 拣入期分两次消；A1 W7 整包 `Lane-A1-M5-W7-reconciliation-20260907-0050.patch` 仍保留作历史 audit trail。
- **M5 final debt ledger 51 项接续**：M5-14 由 W6 收口的 40 项扩到 51 项（+DEBT-41~51），W8 实施期新增债全挂账 + 配方 + 责任 lane。

### 8.3 硬停止自证

| 硬约束 | 自证结果 |
|--------|----------|
| 不写产品代码 | ✅ §8.1 已自证（0 行 Rust/TS/Python/Vue/Cargo/package.json） |
| A3 is excluded / no lane may extend or depend on A3 product-code changes | ✅ §8.1 已自证（A1 W8 不触 A3 W7 已落地文件） |
| 不写 mcp_* / rmcp / plugin runtime / skill execution / network 代码 | ✅ A1 W8 整包不涉及 |
| A5 W8 命令 read-only + source check + ACL + frontend bridge/types + policy + tests 同包 | ✅ **A1 W8 不写 A5 任何产品代码**（A5 责任区留给 A5 实施期） |
| 不记录 token/cookie/Authorization/body/prompt-secret | ✅ A1 W8 整包不涉及任何敏感凭据 |
| 交付 coherent patch/checkpoint 不求 A0 分批 | ✅ 1 checkpoint + 1 patch（含 A1 W7 reconciliation 合并拣入） |
| 不 push | ✅ 工作树保留修改，未 `git add` / `git commit` / `git push` |
| 不动 ACL / pre-merge.sh / Cargo.toml / package.json | ✅ |
| 不移动 `NEXT` | ✅ |
| 不重写各卡 §1~§11 决策史 | ✅ 仅头部状态行 + [W7 reconciliation] / [W8 active] / [W8 verification scope] / [W8 debt] 段 |
| 不抢 A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 各自工作区 | ✅ A1 W8 仅在 M5-0/M5-13/M5-14 横切卡写顶部段 + 2 主文档各加 1 行 W8 A0 update |

### 8.4 决策自证

- **A11 W7 pre-merge FAIL 3 red lights 配方正确性**：`5f92ece` 集成卫生问题 → A3 W7 / A9 W7 / A6 W7 落地后遗留 plugin.rs L20 unused import + L308/L324/L432/L440 test module 缺 `PluginCapability` import + bridge.rs 8 处未 fmt。A1 W8 不在整包修复（越界），仅挂账 W7-1 / W7-2 / W7-3 + 配方（`cargo fmt --all` + plugin.rs L274 import + L20 删 import），责任归 A0 W8 拣入期。
- **A3 local commit boundary 显式记账**：A0 在 W8 dispatch 显式 *"A0 resolves the excluded A3 local commit boundary"* —— A1 W8 整包 DEBT-45 挂账 + 配方 = A0 W8 拣入期消；A1 W8 不替 A0 解决（避免越界）。
- **DRY-F1 残留扩大决策**：A4 W7 review 标（A4 `agent_memory.rs:134` / A7 `graph.rs:18` SENSITIVE_* 双份；`useGraphStore.ts:24-25` / `graphUi.ts:26-29` 图谱容量常量三处拷贝）→ A2 W8 review note 必填（A2 W8 抽 `domain.rs` 单源）+ A1 W8 在 M5-13 [W8 verification scope] 段显式列为可关闭项（依赖 A2 W8 review 输出）。
- **build metrics 阈值记录**：W7 实测 20.63% < 21% 阈值（A0 `5f92ece` 抬阈值 19%→21% 后合规）；A1 W8 不替 A0 拍新阈值/基线（A0 W8 拣入期重采后自决）。

---

## 9. 整包交付结束（A1 W8 → A0 W8 拣入）

**DELIVERABLE 清单：

1. **checkpoint**：`logs/checkpoints/A1-M5-W8-reconciliation-20260907-0945.md`（本文件 · §0-§9 完整）
2. **patch**：`logs/checkpoints/Lane-A1-M5-W8-reconciliation-20260907-0945.patch`（本轮生成 · `git diff --binary` 输出 · 覆盖 11 文件 = 7 M5-*.md + 2 主文档 + 1 checkpoint + 1 patch）
3. **整包合并策略**：A1 W8 整包**主动包含** A1 W7 reconciliation 修订（9 文件修订 + 1 W7 checkpoint + 1 W7 patch 仍保留为 audit trail），避免 A0 W8 拣入期分两次消。
4. **A1 W8 拣入责任全部归 A0**：
   - **A0 拣入期必消**：DEBT-41（A11 W7 3 red lights = W7-1/W7-2/W7-3）+ DEBT-42（A1 W7 整包未进 master = 含 A1 W8 整包合并拣入）+ DEBT-44（A3 W7 修复责任缺口）+ DEBT-45（A3 local commit boundary）
   - **A2 W8 实施期必填**：DEBT-43（DRY-F1 残留扩大 = SENSITIVE_* 双份 + 图谱容量常量三处拷贝）→ A2 W8 review note + A2/A1 W8 抽 `domain.rs` 单源
   - **A5/A6/A7/A8/A9/A10 W8 实施期必消**：DEBT-46/47/48/49/50/51 = 各自 W8 必批闭环项
   - **A11 W8 必填**：12 FAC Excluding-A3 + 残留债 + A3 W8 HOLD 期间 mcp_* 文件改动数 = 0 验证 + W7-1/W7-2/W7-3 修复状态确认

5. **A0 拣入期复核清单**：见本 checkpoint §7（20 条逐项勾对）
6. **A1 W8 自证**：见本 checkpoint §8（4 节：范围/连续性/硬停止/决策）

**A1 W8 整包状态**：

```
LANE=A1
STATUS=PASS（待 A0 拣入后定）
BASE=6c1f30e（origin/master HEAD）
HEAD=工作树（11 文件改动 + 1 new checkpoint；未提交）
FILES=
  M logs/checkpoints/M5-20260906/M5-0-overview.md              (+89)
  M logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md (+1 -1)
  M logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md (+1 -1)
  M logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md (+1 -1)
  M logs/checkpoints/M5-20260906/M5-12-plugin-ui.md            (+1 -1)
  M logs/checkpoints/M5-20260906/M5-13-verification-matrix.md  (+74)
  M logs/checkpoints/M5-20260906/M5-14-debt-ledger.md          (+58)
  M AI-模型切换与接手清单.md                                    (+1)
  M 详细设计与实施计划.md                                       (+1)
  A logs/checkpoints/A1-M5-W8-reconciliation-20260907-0945.md  (NEW)
VERIFY=
  §5 验证命令（git diff --stat 11 文件改动 + git diff --check 干净 + A1 W8 product-code touches = 0 + W7 拣入 commit stat verify + M5-* 段 grep 命中 + 4 张子卡头部 W7/W8 升级 + 2 主文档 W8 update 行 + 后续需求TODO.md A0 W8 已拣入核对）
CHECKPOINT=
  logs/checkpoints/A1-M5-W8-reconciliation-20260907-0945.md（本文件）
MERGE_NOTES=
  - 含 A1 W7 reconciliation 合并拣入（避免 A0 拣入期分两次消）
  - A0 W8 拣入期必先消 DEBT-41/42/44/45
  - A2 W8 必填 DEBT-43（DRY-F1）
  - A11 W8 必填 12 FAC + 残留债 + A3 HOLD 期间 mcp_* 改动数 = 0
NEXT=M5-W8 实施期承接 A5/A6/A7/A8/A9/A10/A11 + A2/A4 review + A1/A2/A4 docs/review/support
```

**A1 W8 整包交付结束。**
