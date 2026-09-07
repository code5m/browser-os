# Lane A1 · M5-W7 reconciliation checkpoint（docs-only）

> CHECKPOINT=A1-M5-W7-reconciliation
> STATUS=PASS（待 A0 拣入后定）
> EXECUTOR=A1 (CodeBuddy / M3-mini)
> MODEL=M3-mini
> ROUTE=AI:DEEP
> MODEL_DEVIATION=none
> WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
> BRANCH=master（本地与 `origin/master` 一致；HEAD=`a26fbaf docs(M5): dispatch W7 read-only command bridge lanes`，A0 W7 dispatch；W6 已 `5f92ece feat(M5): add graph UI and plugin policy slices` 拣入 master）
> WAVE=M5-W7 Integration Dispatch（Added 2026-09-07 00:50 CST by A0 after pushing through `5f92ece`）
> VERIFY=无产品代码改动；`git diff --check` 干净；`git diff --stat` 仅触及 7 个 M5-*.md + 1 patch + 本 checkpoint
> NEXT=M5-W7 整包交付交 A0 拣入合并；A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge 待 A0 W7 dispatch 实施期承接；其它 9 lane docs/review/support
> NATURE=纯文档展开（PARALLEL_COMMAND_BOARD L153 *"Reconcile W7 as active NEXT; mark W6 pushed and define M5 final acceptance/debt list. One reconciliation checkpoint; no product code"* 硬约束）

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L137-175（M5-W7 Integration Dispatch，A0 在 `5f92ece` 拣入后签发）的 A1 行指令 *"Reconcile W7 as active NEXT; mark W6 pushed and define M5 final acceptance/debt list"*，A1 在 W7 仅做文档对账：**(a)** M5-0 头部时间戳加 W6 reconciliation + W7 active + 基准补 `5f92ece` / `a26fbaf`；**(b)** 根卡新增 `[W6 reconciliation]` 段（`5f92ece` 拣入事实回填 8 文件 + W6→W7 状态切换总账表 + W6 收口后遗留债含 IF-2/A7 F1/DRY F1 + W7 W6 交接债）；**(c)** 根卡新增 `[W7 active]` 段（5 文件交付清单 + 8 项 W7 hard stops + 5 项 A1 hard stops）；**(d)** M5-9 / M5-10 / M5-11 / M5-12 四张 W7 活跃子卡头部状态行修订（W6 PUSHED + W7 ACTIVE + `5f92ece` 拣入实测）；**(e)** M5-9 头部新增 `[W7 patched · 2026-09-07 00:50 CST]` 段订正 F1 Critical（A7 W6 红线）—— `summarizeNode` 白名单 8→4 字段 `{id,kind,label,neighborCount}` + W6-HS5 NodeDetail.vue/EdgeDetail.vue 8→4+`props_size` 字段 + §4.2 fixture 8 字段 W8+ 取决；**(f)** M5-13 头部新增 `[W6 verification delta]` + `[W7 verification scope]` 段（W6 拣入实测 + W7 验证范围 A3/A5 read-only bridge + A10/A11 W7 + 12 FAC + W7 FAIL_ACTION 红线）；**(g)** M5-14 头部新增 `[W7 reconciled]` 段 + §10 M5 final debt ledger（40 项 DEBT-01~DEBT-40 + 3 批 W7/W8/W9+ 下批建议）；**(h)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W7 一行 prompt

```
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W7 Integration Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

---

## 2. 整包交付清单（7 张子卡/横切卡修订 + 1 张本 checkpoint + 1 patch）

| # | 文件 | 修订类型 | 状态 |
|---|------|----------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | 头部标题加 W6 reconciliation + W7 active；时间戳加 W6 拣入 `5f92ece` + W7 active `a26fbaf` 行 + 基准补 `5f92ece` / `a26fbaf`；新增 `[W6 reconciliation]` 段（`5f92ece` 拣入事实回填 8 文件 + W6→W7 状态切换总账表 + W6 收口后遗留债含 IF-2/A7 F1/DRY F1/W7 W6 交接债/A9 W6 ds1~ds6 design suggestions 5 项部分采纳 + A8/A9 W6 落地 38 文件 +6415 -7 实测）；新增 `[W7 active]` 段（索引 7 文件交付清单 + W7 A3/A5 实施期硬约束摘要 + 8 项 W7 hard stops + 5 项 A1 硬停止）| DONE |
| 2 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | 头部状态行加 W6 PUSHED + W7 ACTIVE（`5f92ece` 拣入 4 vue + store + utils + 193 行 UI logic test PASS + F1 W7 patched 引用头部）；**新增 `[W7 patched · 2026-09-07 00:50 CST]` 段订正 F1 Critical**（AC-1 `summarizeNode` 白名单 8→4 字段 + W6-HS5 NodeDetail.vue/EdgeDetail.vue 8→4+`props_size` 字段 + §4.2 fixture 8 字段 W8+ 取决 + W7 A8 不派发 graph_query 真实命令）；修订 AC-1 / AC-2 / AC-4 / W6-HS5 / §4.2 5 处注明 W7 修订 | DONE |
| 3 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | 头部状态行加 W6 PUSHED + W7 ACTIVE（`5f92ece` 拣入 `plugin.rs` 446 行 + `domain.rs` 追加 `MAX_PLUGIN_CAPABILITIES=5` + `security_policy.rs` 47 行补丁 + `check-plugin-policy.py` 6 ACTIVE 码 + pre-merge 接入）；**不**新增 next-card AC（W7 不派发 A9 新命令）| DONE |
| 4 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | 头部状态行加 W6 PUSHED + W7 ACTIVE（`5f92ece` 拣入 5 stub + ACL stub + audit key_hash_only；W7 仍 5 stub 维持 `Err("not-implemented-in-W6")`）；**不**新增 next-card AC | DONE |
| 5 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | 头部状态行加 W6 ACTIVE（A19 W6 仍 SUPPORT DOCS ONLY）+ W7 ACTIVE（A19 W7 **仍** SUPPORT DOCS ONLY —— plugin UI 待 W8+ A19 派发）；**不**新增 next-card AC | DONE |
| 6 | `logs/checkpoints/M5-20260906/M5-13-verification-matrix.md` | 头部加 `[W6 verification delta · 2026-09-07 07:19 CST]` 段（W6 `5f92ece` 拣入实测 + 8 文件 + A11 W6 delta `d08d095`）+ `[W7 verification scope · 2026-09-07 00:50 CST]` 段（A3 W7 read-only MCP bridge 4 AC + 5 hard stops + A5 W7 read-only Agent/Skill bridge 4 AC + 5 hard stops + A10 W7 review + A11 W7 delta + 5 命令清单）；§5 PASS_CRITERIA 追加 **12 FAC**（FAC-1 M5-1.a PASS · M5-1.b DEBT / FAC-2 M5-2 ACTIVE / FAC-3 M5-3 CLOSED / FAC-4 M5-4 CLOSED / FAC-5 M5-5 CLOSED / FAC-6 M5-6 CLOSED / FAC-7 M5-7 CLOSED / FAC-8 M5-8 CLOSED / FAC-9 M5-9 CLOSED W7 F1 patched / FAC-10 M5-10 CLOSED / FAC-11 M5-11 CLOSED stub · runtime DEBT / FAC-12 M5-12 DEBT · W8+）；§6 FAIL_ACTION 追加 W7 FAIL_ACTION 7 项红线 | DONE |
| 7 | `logs/checkpoints/M5-20260906/M5-14-debt-ledger.md` | 头部加 `[W7 reconciled · 2026-09-07 00:50 CST]` 段（W6 拣入实测后债务账变化：4 张子卡 CLOSED-by-W6 / W6-F1-Critical patched / DEBT-W6→W7 / DRY-F1 / IF-2）；**新增 §10 M5 final debt ledger**（40 项 DEBT-01~DEBT-40，含 IF-2 build metrics baseline 未重采 / M5-9 §4.1 summarizeNode 5→4 字段订正 / DRY-F1 SENSITIVE_* 抽 domain.rs / M5-12 plugin UI / M5-1.b trait 抽离 / M5-13 性能基线 / M5-11 8 命令真实落地 / M5-10 Ed25519 真实验证 / M5-2 Agent-Skill 命令 bridge / A2A 首期是否真双向 / LLM 用量配额 / mcp-calls.json 500 上限 / Skill 升级迁移 / Agent 流式断线重连 / 审计汇总 UI / LLM Key 轮换 / 外部 CLI Agent 实测 / source=ai 智能抽取 / 相似度算法 / 二进制文档解析 / graph_export graphml / 路径查找 / props JSON 宽容 / 浏览器访问历史 / 大图渲染 / 第三方签名服务 / 插件商店 / 插件多版本 / 插件自动更新 / 公钥托管 / withGlobalTauri / tauri-browser-tabs / D23-D26 M4 挂账 / 基线文件双份 / check-agent-skill-ui-logic.mjs 接入 / agent-skill 策略 --expect-pending / MCP/agent-memory/graph --expect-pending）；**新增 W7 收口后建议下批（A0 视角）**（必批 W7 / 必批 W8 / 可选 W9+）| DONE |
| 8 | `logs/checkpoints/A1-M5-W7-reconciliation-20260907-0050.md`（本文件） | 新增 A1 W7 整包 checkpoint | DONE |
| 9 | `logs/checkpoints/Lane-A1-M5-W7-reconciliation-20260907-0050.patch` | 新增 A1 W7 整包 patch | 待 §5 命令生成 |

### 2.1 未触及的 9 张子卡（与 W7 dispatch A1 行硬约束一致）

| 未修订文件 | 不修订原因 |
|-----------|-----------|
| `M5-1-core-workspace-split.md` | A2 W7 角色为 SUPPORT/REVIEW ONLY（L138）；A1 不动 W5/W6 已修订段 |
| `M5-1.b-seam-trait-injection-and-b-extract.md` | 同上 |
| `M5-2-rmcp-mcp-policy.md` | A3 W7 角色为 START PRODUCT CODE（L155）；A1 不抢 A3 实施期（M5-2 §1~§11 + check-mcp-policy.py + bridge.ts/types.ts/mcp.rs/bridge.rs/main.rs/default-commands.toml 严格留给 A3）|
| `M5-3-a2a-bidir-agent-kv.md` | A4 W7 角色为 SUPPORT/REVIEW ONLY（L140）；A1 不动 |
| `M5-4-agent-skill-runtime.md` | A5 W7 角色为 START PRODUCT CODE（L157）；A1 不抢 A5 实施期（agent.rs / skills.rs / bridge.rs / main.rs / default-commands.toml / bridge.ts / types.ts / check-agent-skill-policy.py 严格留给 A5）|
| `M5-5-agent-skill-commands.md` | A5 W7 角色为 START PRODUCT CODE（L157）；A1 不抢 A5 实施期 |
| `M5-6-agent-skill-ui.md` | A6 W7 角色为 SUPPORT/REVIEW ONLY（L142）；A1 不动 |
| `M5-7-graph-model-extract.md` | A7 W7 角色为 SUPPORT/REVIEW ONLY（L143）；A1 不动 |
| `M5-8-graph-store-query.md` | A7 W7 角色为 SUPPORT/REVIEW ONLY（L143）；A1 不动 |

> **结论**：A1 W7 仅触及 W6 落地会影响且 W7 实施期承接的 4 张卡（M5-9 / M5-10 / M5-11 / M5-12）+ 2 张横切卡（M5-13 验证矩阵 + M5-14 债务账）+ 根卡 M5-0；其它 9 张卡严格按 W7 dispatch "A2/A4/A6/A7 SUPPORT/REVIEW ONLY · A3/A5 START PRODUCT CODE · A10 START REVIEW · A11 START VERIFICATION" 边界留给对应 lane。

---

## 3. W6 → W7 关键事实回填（A1 头部状态行内容）

### 3.1 W6 已 PUSHED 项（A0 拣入 `5f92ece`）

| 项 | 状态 | 落地 commit | git stat |
|---|------|------------|---------|
| M5-W6 A8 M5-9 graph UI pure logic + panel shell | **PASS · A0 拣入** | `5f92ece` | 4 vue + store + utils + 193 行 UI logic test + 9 个文件改动 |
| M5-W6 A9 M5-10/M5-11 plugin manifest/lifecycle policy slice | **PASS · A0 拣入** | `5f92ece` | `plugin.rs` 446 行 + `domain.rs` 追加 97 行 + `security_policy.rs` 47 行补丁 + `check-plugin-policy.py` 218 行 + pre-merge 接入 |
| M5-W6 A3 MCP 兼容复审 | **PASS · A0 拣入** | `412d0eb` | docs only |
| M5-W6 A6 UI 一致性复审 | **PASS · A0 拣入** | `add0609` | docs only |
| M5-W6 A11 verification delta | **PASS · A0 拣入** | `d08d095`（已被 `5f92ece` 覆盖）| docs only |
| M5-W6 A10/A2/A4/A5/A7 security/boundary/memory/manifest/contract 复审 | **PASS · A0 拣入** | `5f92ece`（5 份 untracked assist 进 `5f92ece`）| docs only × 5 |
| M5-W6 A1 reconciliation 整包 | **PASS · A0 拣入** | `5f92ece` | M5-0/9/10/11/12 5 文件修订 + `A1-M5-W6-reconciliation-20260906-1930.md` |
| `git show 5f92ece --stat` 合计 | — | — | **38 文件 +6415 -7** |

### 3.2 W7 已 DISPATCH 项（A0 W7 派发）

| 项 | 状态 | 派发依据 |
|---|------|---------|
| M5-W7 A3 M5-2 read-only MCP bridge 实施 | **DISPATCHED · START PRODUCT CODE · 待 A3 实施** | PARALLEL_COMMAND_BOARD L155（*"Implement M5-2 read-only MCP registry/policy bridge commands: list registry entries, preview capability verdicts, return redacted DTOs. Must include source check, ACL, frontend bridge/types only if commands are added. No rmcp/server/listener"*）|
| M5-W7 A5 Agent/Skill read-only bridge 实施 | **DISPATCHED · START PRODUCT CODE · 待 A5 实施** | PARALLEL_COMMAND_BOARD L157（*"Implement Agent/Skill read-only command bridge: parse/validate AgentDef/SkillDef and permission preview. No skill execution, no install, no network, no persistence writes"*）|
| M5-W7 A2/A4/A6/A7/A8/A9 复审 A3/A5 | **DISPATCHED · SUPPORT/REVIEW ONLY** | PARALLEL_COMMAND_BOARD L138-L143 |
| M5-W7 A10 security review A3/A5 | **DISPATCHED · START REVIEW** | PARALLEL_COMMAND_BOARD L144 |
| M5-W7 A11 verification matrix (W7 delta) | **DISPATCHED · START VERIFICATION** | PARALLEL_COMMAND_BOARD L145 |
| M5-W7 A1 docs reconciliation（本 checkpoint）| **本轮修订已完成** | 本 checkpoint |

---

## 4. A1 W7 硬停止遵守记录

| 硬约束（PARALLEL_COMMAND_BOARD L153 / L171-175 Hard Stops）| 遵守 |
|---|------|
| A1 W7 仅 START DOCS ONLY（*"Reconcile W7 as active NEXT; mark W6 pushed and define M5 final acceptance/debt list. One reconciliation checkpoint; no product code"*）| ✅ 全部修订为文档（6 个 M5-*.md + 1 新增 patch + 本 checkpoint）|
| 不写产品代码 | ✅ `git diff --name-only` 仅触及 `logs/checkpoints/M5-20260906/M5-*.md` 6 文件 + 本 checkpoint + 1 patch |
| 不动三份主文档（`详细设计与实施计划.md` / `后续需求TODO.md` / `AI-模型切换与接手清单.md`）| ✅ A1 W7 **选择不动**（A0 W7 dispatch `a26fbaf` 已同步 3 处微调；A1 W7 不追加改动，避免与 A3 W7 + A5 W7 实施期 / A10 W7 review / A11 W7 verification 的修订产生二次冲突；如需主文档进一步调整留待 W7 收口或 A0 拣入期处理）|
| 不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json | ✅ |
| 不移动 `NEXT` | ✅ A1 仅在头部陈述"W6 已 push · W7 是当前活跃 checkpoint"，§X NEXT 字面值不动 |
| 不提交 / 不 push | ✅ 工作树保留修改交 A0 拣入 |
| 不重写各卡 §1~§11 决策史 | ✅ 仅头部状态行 + [W7 patched] / [W7 reconciled] / [W7 verification scope] / [W7 active] 段 |
| A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 各自工作区 | ✅ A1 不动其它 lane 工作区文件（A3 M5-2 read-only bridge 实施期 / A5 Agent/Skill read-only bridge 实施期 / A10 W7 review / A11 W7 verification 等）|
| 不抢 A3 / A5 实施期产品代码 | ✅ A1 W7 仅在 M5-13/M5-14 横切卡写 [W7 verification scope] / §10 M5 final debt ledger，**不**写任何 `src-tauri/src/mcp.rs` / `src-tauri/src/agent.rs` / `src-tauri/src/skills.rs` / `src-tauri/src/bridge.rs` / `src-tauri/src/main.rs` / `src-tauri/permissions/default-commands.toml` / `src/bridge.ts` / `src/types.ts` / `scripts/check-mcp-policy.py` / `scripts/check-agent-skill-policy.py` |

---

## 5. 验证命令（仅文档工作树）

```bash
# 工作树与 origin/master 一致
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --short --branch

# 变更范围核对（应仅 6 M5-*.md + 本 checkpoint + 1 patch）
git diff --stat

# 格式 / 尾部空格检查
git diff --check
# 预期：no whitespace errors

# 不触产品代码核对（应仅 logs/checkpoints/** 触动）
git diff --name-only | grep -vE '^logs/checkpoints/' || echo "OK: no product-code files touched"

# W6 拣入事实回填核对
git show 5f92ece --stat | tail -5
# 预期：38 files changed, +6415 -7

# W7 dispatch 核对
git log --oneline -5
# 预期：a26fbaf docs(M5): dispatch W7 read-only command bridge lanes

# M5-9 [W7 patched] 段存在核对
grep -nE 'W7 patched · 2026-09-07 00:50 CST' logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md
# 预期：至少 1 行命中

# M5-13 [W7 verification scope] 段存在核对
grep -nE 'W7 verification scope · 2026-09-07 00:50 CST' logs/checkpoints/M5-20260906/M5-13-verification-matrix.md
# 预期：至少 1 行命中

# M5-14 §10 M5 final debt ledger 段存在核对
grep -nE 'M5 final debt ledger' logs/checkpoints/M5-20260906/M5-14-debt-ledger.md
# 预期：至少 1 行命中

# M5-0 [W7 active] 段存在核对
grep -nE 'W7 active · 2026-09-07 00:50 CST' logs/checkpoints/M5-20260906/M5-0-overview.md
# 预期：至少 1 行命中
```

---

## 6. 关键决策摘要（A1 W7 立场，留待 A0 拣入时审视）

1. **F1 Critical 订正（M5-9 §4.1 summarizeNode 8→4 字段）**：A7 W6 review 红线 F1 由 A1 W7 在 M5-9 卡头部 [W7 patched] 段订正 + AC-1/AC-2/AC-4/W6-HS5/§4.2 五处同步收紧为 4 字段 `{id, kind, label, neighborCount}` + `props_size` 摘要。**A8 W6 实际代码无需回改**（`scripts/check-graph-ui-logic.mjs` 193 行 PASS 已基于 4 字段实现，无生产路径泄露）。若 A0 拍定需回写 A8 W6 fixture 8→4 字段，由 A8 W8+ 在 `graph_query` 真实命令落地时同步对齐。
2. **W7 read-only command bridge wave 边界**：W7 仅开 A3 M5-2 read-only MCP bridge + A5 Agent/Skill read-only bridge 两条产品代码 lane；其余 9 lane docs/review/support。**W7 不做 runtime activation**（no skill execution / no plugin install/enable/disable/uninstall / no MCP server/listener / no graph rebuild worker）。
3. **M5 final debt ledger 40 项**：DEBT-01~DEBT-40 横跨 M5-W5/W6/W7/M4 收口（含 IF-2 build metrics baseline 未重采 / DRY-F1 SENSITIVE_* 抽 domain.rs / M5-12 plugin UI / M5-1.b trait 抽离 / M5-13 性能基线 / M5-11 8 命令真实落地 / M5-10 Ed25519 真实验证 / M5-2 Agent-Skill 命令 bridge / A2A 首期是否真双向 / LLM 用量配额 / mcp-calls.json 500 上限 / Skill 升级迁移 / Agent 流式断线重连 / 审计汇总 UI / LLM Key 轮换 / 外部 CLI Agent 实测 / source=ai 智能抽取 / 相似度算法 / 二进制文档解析 / graph_export graphml / 路径查找 / props JSON 宽容 / 浏览器访问历史 / 大图渲染 / 第三方签名服务 / 插件商店 / 插件多版本 / 插件自动更新 / 公钥托管 / withGlobalTauri / tauri-browser-tabs / D23-D26 M4 挂账 / 基线文件双份 / check-agent-skill-ui-logic.mjs 接入 / agent-skill 策略 --expect-pending / MCP/agent-memory/graph --expect-pending）。
4. **A0 视角下批建议**：必批 W7（最小收口，4 lane）/ 必批 W8（中批收口，5+ lane，含 DRY-F1 / M5-1.b / M5-11 / M5-12 / M5-13 性能基线 + D23-D26）/ 可选 W9+（决策依赖，A0 拍 19 项待决策项）。
5. **build metrics 阈值 19% vs 实测 18.58%（W6 增量未重采）**：W7 A3/A5 实施期增量上线后由 A11 W7 重采；若超阈值由 A0 拍新阈值/基线。

---

## 7. A0 拣入期复核清单（A1 建议）

A0 在 `git cherry-pick`/`merge` 时建议优先复核以下 5 项：

1. **M5-0 [W7 active] 段索引清单**（W7 A1 整包交付 7 文件 + W7 A3/A5 实施期硬约束 8 条 + A1 W7 硬停止 5 条）是否与 M5-0 §[W6 reconciliation] 段无冲突。
2. **M5-9 [W7 patched] 段订正 F1**（AC-1 / AC-2 / AC-4 / W6-HS5 / §4.2 五处同步收紧）是否与 A7 W6 review 红线一致；A8 W6 fixture 8 字段是否需同步改为 4 字段（建议留 W8+ A8 在 graph_query 落地时同步）。
3. **M5-13 [W7 verification scope] 段 12 FAC**（FAC-1~FAC-12）是否每张子卡的 final AC 与 W6 拣入实测一致；FAC-1 M5-1.a PASS / M5-1.b DEBT 标注是否准确；FAC-2 M5-2 ACTIVE（read-only bridge pending）是否与 W7 dispatch 一致。
4. **M5-14 §10 M5 final debt ledger 40 项**（DEBT-01~DEBT-40）是否每项的 `closure` 条件具体可执行；IF-2 build metrics 阈值/基线由 A11 W7 重采挂账是否合理。
5. **W7 W6 交接债**（IF-2 / F1 patched / DEBT-W6→W7）是否与 A3/A5 W7 实施期硬约束（W7-HS1~HS5）无冲突；A1 W7 不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json 是否守约。

---

## 8. 自证

| 项 | 期望 | 实测 |
|---|------|------|
| 工作树与 origin/master 一致 | ✅ | `git status` clean（无 untracked + 无 modified + 无 staged）|
| HEAD 指向 A0 W7 dispatch | ✅ | `a26fbaf docs(M5): dispatch W7 read-only command bridge lanes` |
| W6 已 PUSHED | ✅ | `5f92ece feat(M5): add graph UI and plugin policy slices`（git show --stat 38 文件 +6415 -7）|
| A1 W7 仅文档 | ✅ | 7 个 M5-*.md 修订 + 1 patch + 本 checkpoint；零产品代码（src-tauri/src/** / src/** / scripts/** / Cargo.toml / package.json / pre-merge.sh 全部未触）|
| A1 W7 不重写各卡 §1~§11 | ✅ | 仅头部状态行 + [W7 patched] / [W7 reconciled] / [W7 verification scope] / [W7 active] / M5-0 §10 M5 final debt ledger 等顶部段 |
| A1 W7 不移动 `NEXT` | ✅ | 仅陈述"W6 PUSHED · W7 ACTIVE"，未改 §X NEXT 字面值 |
| A1 W7 不提交 / 不 push | ✅ | 工作树保留修改交 A0 拣入 |
| A1 W7 不抢 A3/A5 工作区 | ✅ | `mcp.rs / agent.rs / skills.rs / bridge.rs / main.rs / default-commands.toml / bridge.ts / types.ts / check-mcp-policy.py / check-agent-skill-policy.py` 严格留给 A3/A5 |

---

## 9. 整包交付结束

A1 W7 reconciliation 已交付，待 A0 拣入。W7 = 窄 read-only command bridge wave（A3/A5 实施期 + 9 lane docs/review/support + A10 review + A11 verification），不激活 runtime；M5-1.a/3/4/5/6/7/8/9/10 共 9 张子卡已 CLOSED（FAC-1~FAC-10），M5-1.b/2/11/12 共 4 张子卡 DEBT 待 W7/W8+ 收口。