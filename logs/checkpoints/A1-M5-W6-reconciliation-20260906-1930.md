# Lane A1 · M5-W6 reconciliation checkpoint（docs-only）

> CHECKPOINT=A1-M5-W6-reconciliation
> STATUS=PASS（待 A0 拣入后定）
> EXECUTOR=A1 (CodeBuddy / M3-mini)
> MODEL=M3-mini
> ROUTE=AI:DEEP
> MODEL_DEVIATION=none
> WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
> BRANCH=master（本地与 `origin/master` 一致；HEAD=`77b1e3e docs(M5): dispatch W6 graph UI and plugin lanes`，A0 W6 dispatch）
> WAVE=M5-W6 Parallel Dispatch（Added 2026-09-06 19:25 CST by A0 after pushing through `4b438ef`）
> VERIFY=无产品代码改动；`git diff --check` 干净；`git diff --stat` 仅触及 5 个 M5-*.md 文件 + 本 checkpoint
> NEXT=M5-W6 整包交付交 A0 拣入合并；A8 / A9 待 W6 dispatch 实施期承接 M5-9 / M5-10 / M5-11
> NATURE=纯文档展开（PARALLEL_COMMAND_BOARD L152 *"One reconciliation checkpoint; no product code"* 硬约束）

---

## 0. 一句话

按 `PARALLEL_COMMAND_BOARD.md` L136-171（M5-W6 Parallel Dispatch，A0 在 `77b1e3e` 后签发）的 A1 行指令 *"Reconcile W6 as active NEXT; mark W5 pushed and tighten M5-9/M5-10/M5-11/M5-12 acceptance criteria"*，A1 在 W6 仅做文档对账：**(a)** M5-0 头部时间戳加 W5 拣入行 + W6 active 行 + 基准补 `4b438ef` / `f99d2eb` / `1a2c9cd` / `77b1e3e`；**(b)** 根卡新增 `[W5 reconciliation]` 段（5 段核心 commit `4b438ef` / `f99d2eb` / `1a2c9cd` / `d3f11cd` / `77b1e3e` + 8 份 W5 assist + IF-2 build metrics 挂账 + A8/A9 W6 升级）；**(c)** 根卡新增 `[W6 active]` 段（7 文件交付清单 + 8 项 A8/A9 硬约束摘要）；**(d)** M5-9 / M5-10 / M5-11 三张 W6 实施期卡增补 `[W6 next-card acceptance criteria]` 段（4/4/4 项 AC + 5/5/5 项 hard stops 各段）；**(e)** M5-12（A19 W6 仍 SUPPORT DOCS ONLY）仅加 `[W6 status]` 轻量段说明，不动 §1~§11；**(f)** 写本 checkpoint + patch；不写产品代码；不重写 §1~§11 决策史；不移动 `NEXT`；不提交 / 不 push。

---

## 1. W6 一行 prompt

```
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3；继续 Lane A1，先 cd 到 WORKDIR，再 git fetch origin && git pull --ff-only，再读取 WORKSPACE_IDENTITY.md 和 PARALLEL_COMMAND_BOARD.md，按 M5-W6 Parallel Dispatch 完成自己的整包交付，自行整理补丁/checkpoint，不 push。
```

---

## 2. 整包交付清单（5 张子卡修订 + 1 张本 checkpoint + 1 patch）

| # | 文件 | 修订类型 | 状态 |
|---|------|----------|------|
| 1 | `logs/checkpoints/M5-20260906/M5-0-overview.md` | 头部标题加 W5 reconciliation + W6 active；时间戳加 W5 拣入 + W6 active 行；基准补 `4b438ef` / `f99d2eb` / `1a2c9cd` / `77b1e3e`；依据补 W6 dispatch；新增 `[W5 reconciliation]` 段（5 段核心 commit + 8 份 W5 assist + IF-2 挂账 + A8/A9 W6 升级 + W5→W6 状态切换总账表）+ 新增 `[W6 active]` 段（索引 7 文件 + 8 项 W6 硬约束 + 5 项 A1 硬停止）| DONE |
| 2 | `logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md` | 头部状态行 W3→W6；新增 `[W6 next-card acceptance criteria]` 段（**4 项 AC**：graph UI pure logic helper / list/search/filter + node detail summary / capacity+error+empty 状态 / headless logic test + 容量隐私双扫；**5 项 hard stops**：不加 commands / 不调 live agent / 不调 model / 无新 npm 依赖 / K7 props 零泄露）| DONE |
| 3 | `logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md` | 头部状态行 W3→W6；新增 `[W6 next-card acceptance criteria]` 段（**4 项 AC**：PluginManifest DTOs + validation / lifecycle state machine / permission manifest rules + capability 真源 / policy script + pre-merge wire；**5 项 hard stops**：不 install/uninstall/delete/download/execute / 不网络 / 不签名强制 / 不注册 10 命令 / 不破 capability.rs 漂移）| DONE |
| 4 | `logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md` | 头部状态行 W3→W6；新增 `[W6 next-card acceptance criteria]` 段（**4 项 AC**：commands_islolation shell 5 stub / 5 命令 ACL stub / capability 校验骨架 / plugin-invokes.json audit shape；**5 项 hard stops**：5 命令仅 stub / audit 仅 key_hash / capability 真源单点 / 不破 K1 ACL 末条 / 不接 M5-10 真实 lifecycle + 无网络 + 无 IO）| DONE |
| 5 | `logs/checkpoints/M5-20260906/M5-12-plugin-ui.md` | 头部状态行 W3→W6；新增 `[W6 status]` 轻量段（A19 W6 仍 SUPPORT DOCS ONLY；W6 无 plugin UI lane 承接；待 A9 W6 plugin backend 落地后 W7+ 派发；A1 W6 不写 next-card AC）| DONE |
| 6 | `logs/checkpoints/A1-M5-W6-reconciliation-20260906-1930.md`（本文件） | 新增 A1 W6 整包 checkpoint | DONE |
| 7 | `logs/checkpoints/Lane-A1-M5-W6-reconciliation-20260906-1930.patch` | 新增 A1 W6 整包 patch | 待 §5 命令生成 |

### 2.1 未触及的 10 张子卡（与 W6 dispatch A1 行硬约束一致）

| 未修订文件 | 不修订原因 |
|-----------|-----------|
| `M5-1-core-workspace-split.md` | A2 W6 角色为 SUPPORT/REVIEW ONLY（L138）；A1 不动 W5 已修订段 |
| `M5-1.b-seam-trait-injection-and-b-extract.md` | 同上 |
| `M5-2-rmcp-mcp-policy.md` | A3 W6 角色为 SUPPORT/REVIEW ONLY（L139）；A1 不动 |
| `M5-3-a2a-bidir-agent-kv.md` | A4 W6 角色为 SUPPORT/REVIEW ONLY（L140）；A1 不动 |
| `M5-4-agent-skill-runtime.md` | A5 W6 角色为 SUPPORT/REVIEW ONLY（L141）；A1 不动 |
| `M5-5-agent-skill-commands.md` | A5 W6 角色为 SUPPORT/REVIEW ONLY（L141）；A1 不动 |
| `M5-6-agent-skill-ui.md` | A6 W6 角色为 SUPPORT/REVIEW ONLY（L142）；A1 W5 已修订 next-card AC，本轮 **不** 重复改（避免与 A6 W6 assist / A10 W6 review 冲突）|
| `M5-7-graph-model-extract.md` | A7 W6 角色为 SUPPORT/REVIEW ONLY（L143）；A1 W5 已修订 next-card AC，本轮不重复改 |
| `M5-8-graph-store-query.md` | A7 W6 角色为 SUPPORT/REVIEW ONLY（L143）；A1 W5 已修订 next-card AC，本轮不重复改 |
| `M5-13-verification-matrix.md` | A11 W6 角色为 START VERIFICATION（L145）；A1 不动（避免与 A11 横切工作重叠）|
| `M5-14-debt-ledger.md` | 横切卡，W6 修订由其它 lane 触发；A1 不动 |

> **结论**：A1 W6 仅触及 W5 落地会影响且 W6 实施期承接的 4 张卡（M5-9 / M5-10 / M5-11 / M5-12）+ 根卡 M5-0；其它 11 张卡严格按 W6 dispatch "A2/A3/A4/A5/A6/A7 SUPPORT/REVIEW ONLY · A8/A9 START PRODUCT CODE · A10 START REVIEW · A11 START VERIFICATION" 边界留给对应 lane。

---

## 3. W5 → W6 关键事实回填（A1 头部状态行内容）

### 3.1 W5 已 PASS 项（A0 拣入）

| 项 | 状态 | 落地 commit |
|---|------|------------|
| M5-W5 A6 M5-6 Agent/Skill UI pure logic + panel shell | **PASS** | `f99d2eb` 拣入（前端 UI pure logic helper + Agent/Skill 面板壳 + `check-agent-skill-ui-logic.mjs`）|
| M5-W5 A7 M5-7/M5-8 graph model/store policy slice | **PASS** | `4b438ef` 拣入（`src-tauri/src/graph.rs` + `domain.rs` GraphNode/Edge + 7 容量常量 + `check-graph-policy.py` 7 ACTIVE 码 + pre-merge 接入）|
| M5-W5 A11 verification delta | **PASS** | `1a2c9cd` 拣入（cargo test 9/9 graph + UI logic test 25 断言 + 2 个 policy --self-test PASS + pre-merge ALL_PASS）|
| M5-W5 A3 MCP/AC assist | **PASS** | `d3f11cd` 拣入 A3 W5 assist |
| M5-W5 A1 reconciliation 整包 | **PASS** | （已 `1610939` 拣入的 W4 整包+未拣入的 W5 reconciliation patch 留 A0 集成）|
| M5-W5 A2/A4/A5/A8/A9/A10 W5 assist | **PASS**（untracked）| 各 lane W5 role = SUPPORT DOCS ONLY / REVIEW ONLY |

### 3.2 W6 已 DISPATCH 项（A0 W6 派发）

| 项 | 状态 | 派发依据 |
|---|------|----------|
| M5-W6 A8 M5-9 实施（graph UI pure logic + panel shell）| **DISPATCHED · 待 A8 实施** | PARALLEL_COMMAND_BOARD L159 |
| M5-W6 A9 M5-10/M5-11 实施（plugin manifest/lifecycle policy slice）| **DISPATCHED · 待 A9 实施** | PARALLEL_COMMAND_BOARD L160 |
| M5-W6 A2/A3/A4/A5/A6/A7 复审 A8/A9 | **DISPATCHED** | PARALLEL_COMMAND_BOARD L138-L143 |
| M5-W6 A10 security review A8/A9 | **DISPATCHED** | PARALLEL_COMMAND_BOARD L144 |
| M5-W6 A11 verification matrix | **DISPATCHED** | PARALLEL_COMMAND_BOARD L145 |
| M5-W6 A1 docs reconciliation（本 checkpoint）| **本轮修订已完成** | 本 checkpoint |

---

## 4. A1 W6 硬停止遵守记录

| 硬约束（PARALLEL_COMMAND_BOARD L152 / L164-171 Hard Stops）| 遵守 |
|---|------|
| A1 W6 仅 START DOCS ONLY | ✅ 全部修订为文档（5 个 M5-*.md + 1 新增 checkpoint）|
| 不写产品代码 | ✅ `git diff --name-only` 仅触及 `logs/checkpoints/M5-20260906/M5-*.md` 5 文件 + 本 checkpoint |
| 不动三份主文档（`详细设计与实施计划.md` / `后续需求TODO.md` / `AI-模型切换与接手清单.md`）| ✅ A1 W6 **选择不动**（PARALLEL_COMMAND_BOARD L152 虽允许 "three main docs"，但本轮 A1 选择仅文档对账 + 不动主文档，以避免与 A8 W6 graph UI 实施期 / A9 W6 plugin 实施期 / A10 W6 review / A11 W6 verification 的修订产生二次冲突；如需主文档调整留待 W6 收口或 A0 拣入期处理）|
| 不动 ACL / Capability / pre-merge.sh / Cargo.toml / package.json | ✅ |
| 不移动 `NEXT` | ✅ A1 仅在头部陈述"事实已变"，§X NEXT 字面值不动 |
| 不提交 / 不 push | ✅ 工作树保留修改交 A0 拣入 |
| 不重写各卡 §1~§11 决策史 | ✅ 仅头部 [W6 next-card acceptance criteria] / [W6 status] 段 + 头部状态行 |
| A2/A3/A4/A5/A6/A7/A8/A9/A10/A11 各自工作区 | ✅ A1 不动其它 lane 工作区文件（如 A6 W5 UI 面板 / A7 W5 graph store / A8/A9 W6 实施期 / A10 W6 review / A11 W6 verification 等）|
| 不抢 A8 / A9 实施期产品代码 | ✅ A1 W6 仅在 M5-9/10/11 顶部写 next-card AC 段（4 项 AC + 5 项 hard stops），**不**写任何 `src/components/**` / `src/stores/**` / `src/types.ts` / `src/bridge.ts` / `src-tauri/src/domain.rs` / `src-tauri/src/plugin.rs` / `src-tauri/src/security_policy.rs` / `scripts/check-graph-ui-logic.mjs` / `scripts/check-plugin-policy.py` |

---

## 5. 验证命令（仅文档工作树）

```bash
# 工作树与 origin/master 一致
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
git status --short --branch

# 变更范围核对（应仅 5 M5-*.md + 本 checkpoint）
git diff --stat

# 格式 / 尾部空格检查
git diff --check
# 预期：no whitespace errors

# 不触产品代码核对（应仅 logs/checkpoints/** 触动）
git diff --name-only | grep -vE '^logs/checkpoints/'
# 预期：无输出

# 不动三份主文档核对
git diff --name-only | grep -E '(详细设计与实施计划|后续需求TODO|AI-模型切换与接手清单)\.md'
# 预期：无输出

# 不动 ACL/Cap/pre-merge/Cargo/package 核对
git diff --name-only | grep -E '(ACL|default-commands|pre-merge|Cargo\.toml|package\.json)'
# 预期：无输出

# 不动 A8/A9 实施期工作区核对
git diff --name-only | grep -E '(src/components|src/stores|src/types\.ts|src/bridge\.ts|src-tauri/src/domain\.rs|src-tauri/src/plugin\.rs|src-tauri/src/security_policy\.rs|scripts/check-graph-ui|scripts/check-plugin)'
# 预期：无输出
```

---

## 6. W6 A1 不做的（A0 拣入期 / A8-A11 实施期再做）

| 项 | 谁做 | 何时 |
|---|------|------|
| 本 5 张子卡 + 本 checkpoint 拣入合并 | **A0** | A1 提交 patch/checkpoint 后，A0 拣入 master |
| M5-9 graph UI pure logic + panel shell | **A8** | A8 W6 实施期（PARALLEL_COMMAND_BOARD L159）|
| M5-10 PluginManifest DTOs + validation + lifecycle state machine + permission manifest rules + policy script | **A9** | A9 W6 实施期（PARALLEL_COMMAND_BOARD L160）|
| M5-11 commands_islolation shell + 5 命令 ACL stub + capability 校验骨架 + audit shape | **A9** | A9 W6 实施期（PARALLEL_COMMAND_BOARD L160）|
| A2/A3/A4/A5/A6/A7 复审 A8/A9 | **A2/A3/A4/A5/A6/A7** | W6 实施期（A8/A9 输出后）|
| A10 W6 security review A8/A9 | **A10** | W6 实施期（A8/A9 输出后）|
| A11 W6 verification | **A11** | W6 实施期（A8/A9 输出后）|
| M5-12 plugin UI（plugin 列表 / 安装向导 / 启用停用 / 审计查询 / 权限预览）| **A19** | **W7+**（W6 仍 SUPPORT DOCS ONLY；待 A9 W6 plugin backend 落地后由 A0 派发）|
| W6 → W7 dispatch | **A0** | W6 实施期收口后 |

---

## 7. 提交格式（给 A0 拣入参考）

本 checkpoint 与 5 张子卡修订均**未** `git add` / `git commit`（A1 硬约束"不提交"）；A0 拣入时建议：

1. `git add logs/checkpoints/M5-20260906/M5-0-overview.md logs/checkpoints/M5-20260906/M5-9-graph-ui-agent-consume.md logs/checkpoints/M5-20260906/M5-10-plugin-manifest-lifecycle.md logs/checkpoints/M5-20260906/M5-11-plugin-commands-isolation.md logs/checkpoints/M5-20260906/M5-12-plugin-ui.md logs/checkpoints/A1-M5-W6-reconciliation-20260906-1930.md`
2. `git commit -m "docs(A1): M5-W6 reconciliation — mark W5 pushed + tighten M5-9/M5-10/M5-11/M5-12 acceptance criteria"`（A0 commit 措辞参考）
3. A0 push 后，本轮 M5-W6 派发闭合；A8/A9 进入 W6 实施期。

> A1 patch 备用：参见同目录 `Lane-A1-M5-W6-reconciliation-20260906-1930.patch`（如 A0 拣入偏好 patch 应用）。

---

## 8. 与其他 lane 的边界

- **A0**：本轮 A1 整包交 A0 拣入；不冲突（master 与 origin 一致 `77b1e3e`，A1 工作树未推）。
- **A2/A3/A4/A5/A6/A7**：W6 角色为 *SUPPORT/REVIEW ONLY*（L138-L143）；本 checkpoint 不动 A2/A3/A4/A5/A6/A7 W6 工作区。
- **A8**：W6 角色为 *START PRODUCT CODE*（L159）—— M5-9 graph UI pure logic + panel shell；本 checkpoint 已为 A8 准备 4 项 AC + 5 项 hard stops，**不**与 A8 工作区冲突。
- **A9**：W6 角色为 *START PRODUCT CODE*（L160）—— M5-10/M5-11 plugin manifest/lifecycle policy slice；本 checkpoint 已为 A9 准备 4+4 项 AC（双卡）+ 5+5 项 hard stops，**不**与 A9 工作区冲突。
- **A10**：W6 角色为 *START REVIEW*（L144）；本轮 A1 修订均为头部状态行 + 顶部段，无新增红线；A10 W6 review 工作区独立。
- **A11**：W6 角色为 *START VERIFICATION*（L145）；本轮 A1 修订均已给出"§验证清单（供 A11 收口）"，**不**与 A11 验证矩阵横切工作重叠。
- **A19**：W6 角色为 *SUPPORT DOCS ONLY*（未在 L137-L145 明列但不在 2 条产品代码 lane 内）；M5-12 plugin UI 在 W6 仍 SUPPORT DOCS ONLY；本 checkpoint 仅在 M5-12 顶部加 `[W6 status]` 轻量段说明，**不**抢 A19 W7+ UI 派发。

---

## 9. W3 → W4 → W5 → W6 状态切换总账

| Wave | A1 checkpoint 文件 | A0 拣入 commit | 关键 commit 链 | 状态 |
|------|-------------------|----------------|---------------|------|
| W0 | `M5-0-overview.md`（初版）| `404f514` | `a1a2061` → `404f514` | PASS |
| W1 | `M5-0-overview.md` (W1 reconciliation) | `0d08016` | + `854bc40` + `0d08016` | PASS |
| W2 | `M5-0-overview.md` (W2 patched) | `712a14c` | + `a654f0c` + `712a14c` | PASS |
| W3 | `A1-M5-W3-reconciliation-20260906-1730.md` | `f8f1f49` | + `98a3b01` + `e96c902` + `bdb0602` + `12f1cff` + `f8f1f49` | PASS |
| W4 | `A1-M5-W4-reconciliation-20260906-1755.md` | `1610939` | + `d71f558` + `13c5279` + `562efb9` + `1610939` | PASS |
| W5 | `A1-M5-W5-reconciliation-20260906-1835.md` | `4b438ef` + `f99d2eb` + `1a2c9cd` | + `0e76a89`（A0 W5 dispatch） + `d3f11cd`（A3 W5 assist） + `4b438ef` + `f99d2eb` + `1a2c9cd` | **PASS · A0 拣入** |
| W6 | **本 checkpoint** `A1-M5-W6-reconciliation-20260906-1930.md` | <待 A0 拣入> | + `77b1e3e`（A0 W6 dispatch） | **本轮修订已完成** · 待 A0 拣入 |

---

## 10. W6 新增事实（A0 拣入前应确认）

1. **HEAD 自 `0e76a89` 已更新到 `77b1e3e`**：A0 在 A1 W5 拣入后签发了 W6 dispatch（`docs(M5): dispatch W6 graph UI and plugin lanes`）。
2. **W5 核心 3 commit 已落地**：
   - `4b438ef feat(M5): add graph model store policy slice`（A7 W5 graph.rs + domain.rs + check-graph-policy.py）
   - `f99d2eb feat(A6): M5-6 Agent/Skill UI pure logic + panel shell (W5)`（A6 W5 UI panel + check-agent-skill-ui-logic.mjs）
   - `1a2c9cd docs(A11): M5-W5 verification delta after A6/A7 W5 outputs`（A11 W5 验证 delta）
3. **A0 W5 期间 A3 W5 已产出**：A3 W5 assist 拣入 `d3f11cd`。
4. **A8 W6 升级为 START PRODUCT CODE**：M5-9 graph UI pure logic + panel shell；范围严格 `src/components/**` + `src/stores/**` + `src/types.ts` + `src/bridge.ts`（only if no new command）+ `scripts/check-graph-ui-logic.mjs`；不加 commands / 不调 live agent / 不调 model / 无 npm 依赖。
5. **A9 W6 升级为 START PRODUCT CODE**：M5-10/M5-11 plugin manifest/lifecycle policy slice；范围 `src-tauri/src/domain.rs` + optional `src-tauri/src/plugin.rs` + `src-tauri/src/security_policy.rs` + `scripts/check-plugin-policy.py` + `scripts/pre-merge.sh`；不 install/uninstall/delete/download/execute real plugins / 不网络 / 不签名强制 / 不注册 10 条 plugin_* 命令 / 不破 capability.rs 漂移。
6. **A19 W6 仍 SUPPORT DOCS ONLY**：M5-12 plugin UI 无产品代码 lane 承接；待 A9 W6 plugin backend 落地后 W7+ 派发。
7. **IF-2 挂账（build metrics 19%）**：W5 期间未在 W5 关闭，PARALLEL_COMMAND_BOARD L139 显式记挂账；W6 实施期 A8/A9 上线后 A11 重采，由 A0 拍定阈值/基线。
8. **A8 W6 d3 引入决议（IF-3）**：A8 W6 严禁引 d3 整包已在 W6-HS4 严守；W7+ 真接 layout 时由 A8 重新决议并经 A0 拣入。

---

## 11. CHECKPOINT 元数据

```
CHECKPOINT=A1-M5-W6-reconciliation
STATUS=PASS（待 A0 拣入后定）
EXECUTOR=A1 (CodeBuddy / M3-mini)
MODEL=M3-mini
ROUTE=AI:DEEP
MODEL_DEVIATION=none
WORKDIR=/home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3
BRANCH=master（HEAD=77b1e3e；与 origin/master 一致）
COMMIT=<待 A0 拣入后填>
VERIFY=见 §5 命令（仅文档工作树）；git diff --check 干净；5 张 M5-*.md 仅头部 + 顶部段，无 §1~§11 重写
NEXT=M5-W6 整包交付交 A0 拣入合并；A8 / A9 待 W6 dispatch 实施期承接 M5-9 / M5-10 / M5-11
NATURE=纯文档展开；零产品代码；不提交 / 不 push
```