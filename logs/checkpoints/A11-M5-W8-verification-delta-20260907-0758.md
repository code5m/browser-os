# M5-A11 · W8 验证增量（Full-Lane Follow-up；A3 W7 已提交、A3 W8 策略债未闭）

```text
LANE=A11
STATUS=功能就绪 GREEN；集成卫生 2 红灯（fmt / git-diff-check，均源于 A3 W7 交付）+ A3 W8 策略债 1 红灯（MCP --expect-pending）。
  A0 在「A3 闭 W8 策略债 + 两处卫生修复」三事完成后方可 push。
BASE=6c1f30e（A3 W7 MCP 只读桥已提交，含 5f92ece A8/A9 W6）；本地领先 origin 3（daa10f6/A11-W7、a29b796/A6-W7、6c1f30e/A3-W7）
HEAD=logs/checkpoints/A11-M5-W8-verification-delta-20260907-0758.md
FILES=logs/checkpoints/A11-M5-W8-verification-delta-20260907-0758.md
VERIFY=见 §1 W8 全门表（当前工作树 = 已提交 + 未提交 A0 集成修复）
CHECKPOINT=本文件
MERGE_NOTES=§6 三红灯修复配方（交 A0/A3；A11 不改产品代码）；§9 push 裁决
NEXT=待 A3 闭 W8 MCP 策略债 + A0 跑 cargo fmt 全量 + 剥离 A3 W7 .patch 行尾空格 → 重跑 pre-merge 预期 ALL_PASS + cargo test 绿 → A0 push
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W8 Full-Lane Follow-up Dispatch（L174-225）→ **A11 = START VERIFICATION BATCH**（L200）：「Maintain W8 verification matrix including A3. Record exact commands, pass/fail, residual debt, and whether A0 may push after A3 policy phase debt closes.」scope `logs/assist/A11-M5-W8-*.md`、`logs/checkpoints/A11-M5-W8-*.md`。
> 用户指令「M5-W8 Excluding-A3 Dispatch」解读：W8 全量 lane 中，A3 的 W8 职责（策略债闭合）是唯一功能门；其 W7 交付还遗留两项卫生债（fmt / 行尾空格）。除 A3 三项外，W8 验证全绿。
> 范围声明：本增量**只产出验证文档，零产品代码改动**。W8 仅 A3（POLICY FIX ONLY）/A5（PRODUCT CODE，硬化）可写产品代码；其余 lane docs/review/support。
> 姊妹件：W7 `M5-A11-W7-verification-delta-20260907-0732.md`（标注 3 红灯 + A3/A5 待交付）。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已是最新（origin/master 未动；本地领先 3）
git status --short --branch          # 多 lane 共享工作树：plugin.rs(MM) + check-agent-skill-policy.py(M) + 多 docs(A/M)
git log --oneline -6                 # 6c1f30e(A3 W7) / daa10f6(A11 W7) / a29b796(A6 W7) / a26fbaf(W7 dispatch) / 5f92ece(A8/A9 W6)
```
- **调度匹配**：board 头部 `Current NEXT: M5-W8 full-lane follow-up; A3 W7 code is present and must close MCP policy pending-mode debt before further MCP runtime work`（L7）。W8 目标（L178）：闭 A3 MCP 策略 pending-mode 债、硬化 Agent/Skill 只读桥、接线/打磨 UI 消费、刷新 graph/plugin 计划、产一批验证包后 A0 push。
- W8 硬停止（L202-209）：A3 仅可改 MCP 策略/checkpoint 文件（不新增运行时命令）；无 rmcp/server/listener/plugin 安装/网络；每命令须 source check+ACL+前端 bridge/types+策略覆盖+测试同包；仅 A0 push。

---

## 1. W8 验证矩阵（当前工作树实跑）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **389 passed（387 bin + 2），0 failed** | ✅ |
| cargo check 告警 | `cargo check --locked` | **2 warnings**（grid_process.rs:76 index/comms；grid_process.rs:103 new）= 基线 | ✅ |
| cargo fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | **FAIL**；`bridge.rs:6276` + `plugin.rs:16` 未格式化 | ❌ RED |
| core 边界门 | `check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| M5-2 MCP 门 | `check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS`（ACTIVE=6，PENDING=9） | ✅ |
| M5-2 MCP 门 | `check-mcp-policy.py`（default） | `MCP_POLICY=PASS`（无违规） | ✅ |
| M5-2 MCP 门 | `check-mcp-policy.py --expect-pending` | **`MCP_PENDING_RESULT=FAIL`**（9 PENDING 码须翻 ACTIVE） | ❌ RED（A3 W8 债） |
| agent-memory 门 | `check-agent-memory-policy.py --self-test` | `AGENT_KV_POLICY_SELF_TEST=PASS`（ACTIVE=5） | ✅ |
| agent-skill 门 | `check-agent-skill-policy.py --self-test` | `AGENT_SKILL_POLICY_SELF_TEST=PASS`（ACTIVE=3，PENDING=4） | ✅ |
| agent-skill 门 | `check-agent-skill-policy.py`（default） | `AGENT_SKILL_POLICY=PASS` | ✅ |
| graph 门 | `check-graph-policy.py --self-test` | `GRAPH_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| graph 门 | `check-graph-policy.py`（default） | `GRAPH_POLICY=PASS` | ✅ |
| graph UI 逻辑 | `node scripts/check-graph-ui-logic.mjs` | **通过 34，失败 0** | ✅ |
| A9 plugin 门 | `check-plugin-policy.py --self-test` | `PLUGIN_SELF_TEST=ALL_PASS`（ACTIVE=1，PENDING=5） | ✅ |
| A9 plugin 门 | `check-plugin-policy.py`（default） | `PLUGIN_POLICY=PASS` | ✅ |
| A9 plugin 门 | `check-plugin-policy.py --expect-pending` | `PLUGIN_PENDING_OK` | ✅ |
| tools 门 | `check-tools-policy.py --self-test` | self-test OK（16 坏样本全检） | ✅ |
| database 门 | `check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（ACTIVE=14，PENDING=1） | ✅ |
| scheduler 门 | `check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（ACTIVE=23） | ✅ |
| script-exec 门 | `check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-C2Y5r8GX.js`=164.58 kB（gzip 58.97）；面板懒加载分块 | ✅ |
| 构建指标 | `measure-build-metrics.py --compare 4f0e8ab --skip-build` | `total_bytes_pct≈20.66`（**<21% 阈值**），`cargo_warnings=0 delta`（2=baseline）→ PASS | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=FAIL`**（EXIT=1；FAIL：cargo fmt main + git diff --check branch range） | ❌ RED（2 项） |

> 注：pre-merge 不跑 `cargo test`（仅 `cargo check`），故 cargo test 绿灯需 A0「before push」清单单独确认；W7 的 build metrics 红灯本波已消解（warnings 回 2=baseline → `warnings_increased=false`）。

---

## 2. W7 三红灯闭合回顾

| W7 红灯 | 根因 | W8 状态 |
|---|---|---|
| **W7-1** cargo test 编译中断（plugin.rs 测试模块漏导 `PluginCapability`） | 顶层 import 含 `PluginCapability` 仅测试用；测试模块未导入 | **已闭** ✅：工作树 staged（顶层 import 规范化去 `PluginCapability`）+ unstaged（测试模块补 `use crate::domain::{PluginCapability, PluginEntry, PluginSignature}`）→ cargo test 389/0 |
| **W7-3** build metrics `warnings_increased`（2→3） | 同 W7-1 根：顶层 import 含未用 `PluginCapability` | **已闭** ✅：warnings 回 2（仅 grid_process.rs 两条既有）；build metrics PASS |
| **W7-2** cargo fmt 未过 | `bridge.rs` +231 行（A3 W7 的 mcp 命令封装，未格式化）+ `plugin.rs:16`（W7-1 修复 staged 版手动折行，rustfmt ≤100 列要求单行→非 canonical） | **未闭** ❌：见 §6 RED-1 |

> 更正 W7 增量记录：W7 我据 `agent.rs`/`skills.rs` 无 `#[tauri::command]` 判定「A5 W7 未交付」有误——命令封装实际在 `bridge.rs`，且 `main.rs:1434-1445` 已注册 `bridge::agent_parse/agent_validate/agent_permission_preview/skill_parse/skill_validate/skill_permission_preview` **及** 3 个 `mcp_*`。故 **A5 W7 与 A3 W7 命令桥均已落地并注册**（见 §3/§4）。

---

## 3. A3 W7 MCP 只读桥验证（提交 `6c1f30e`）

- **交付面**（10 文件，+1224/−8）：`bridge.rs`(+231 命令封装) / `main.rs`(+9 invoke) / `default-commands.toml`(+9 ACL) / `mcp.rs`(+81) / `src/bridge.ts`(+24) / `src/types.ts`(+31) / `check-mcp-policy.py`(+65) / `.md`+.patch 交付件。
- **命令**：`mcp_policy_get` / `mcp_registry_list` / `mcp_capability_preview`（bin-side、只读、无 rmcp/server/listener/网络/运行时执行）。
- **验证**：cargo test 含其单测，389/0 全绿 ✅；`check-mcp-policy.py` self-test PASS（ACTIVE=6，PENDING=9）+ default PASS ✅；ACL 末条 `list_artifact_images`（L127）未变，mcp_* 插于 L124-126（末条之前）✅。
- **遗留债（W8 红灯来源）**：
  - `bridge.rs` +231 行未格式化 → §6 RED-1（fmt 主因）。
  - `logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.patch` 行尾空格 → §6 RED-2（git-diff-check 主因）。
  - `check-mcp-policy.py --expect-pending` FAIL（9 PENDING 码）→ §6 RED-3（A3 W8 策略债，board L193）。

---

## 4. A5 W7 Agent/Skill 只读桥验证（命令已落地并注册）

- **命令**：`agent_parse` / `agent_validate` / `agent_permission_preview` / `skill_parse` / `skill_validate` / `skill_permission_preview` —— `bridge.rs` 封装，`main.rs:1434-1439` 注册，ACL `default-commands.toml` L118-123 预声明（均在末条 `list_artifact_images` 之前）✅。
- **策略**：`check-agent-skill-policy.py` self-test PASS（ACTIVE=3，PENDING=4）+ default PASS ✅；`permission_preview` 仅回 gate+capabilities id（无 secret），`validate` 跑 `contains_credential_leak`，能力白名单 `AGENT_CAPABILITY_V1`/`SKILL_CAPABILITY_V1` 空集合 fail-closed。
- **W8 状态**：`check-agent-skill-policy.py` 有 staged 更新（W8 硬化，self-test 仍绿）；A5 W8 须确保命令桥 + ACL + 前端 bridge/types + 策略覆盖 + 聚焦测试原子同包（board L194），当前命令已注册、策略绿，硬化进行中。

---

## 5. A8/A9 W6（提交 `5f92ece`）仍绿

- **A8 W6 graph UI 逻辑**：`check-graph-ui-logic.mjs` → **34 passed / 0 failed** ✅。
- **A9 W6 plugin 策略**：`check-plugin-policy.py` → `ALL_PASS`（ACTIVE=1 `PLUGIN_NO_SECRETS`，PENDING=5）/ default PASS / `--expect-pending` OK ✅。
- 前端 `npm run build` PASS（主 chunk 164.58 kB；GraphPanel/AgentManagerPanel/SkillManagerPanel/PermissionPreviewModal/TaskPanel/DatabasePanel 均懒加载分块；xterm 334 kB 独立）✅。

---

## 6. 剩余红灯（A0 闭门前必修，均源于 A3 W7/W8）

### RED-1 · cargo fmt 未过（`bridge.rs:6276` + `plugin.rs:16`）
- **根因**：① `bridge.rs` 的 +231 行（A3 W7 mcp 命令封装）未格式化；② `plugin.rs:16` 为 W7-1 修复的 staged 版手动折行——该 import 在 ≤100 列内 rustfmt 要求单行，staged 版折多行属非 canonical。
- **修复**（A0）：`cargo fmt --manifest-path src-tauri/Cargo.toml --all`（+ `cargo fmt --manifest-path tauri-browser-tabs/Cargo.toml --all`）全量格式化，消解 `bridge.rs` 与 `plugin.rs` 两项。

### RED-2 · git diff --check (branch range) 未过
- **根因**：A3 W7 提交的 `logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.patch`（含于 `6c1f30e`，当前为 HEAD）含行尾空格；`git diff --check origin/master..HEAD` 报多行 `trailing whitespace`（L6/7/32/33/103/397/402/407/418/513/518/527/529）。工作树与暂存区 `git diff --check` 均干净。
- **修复**（A0/A3）：剥离该 `.patch` 行尾空格（`sed -i 's/[[:space:]]*$//' logs/checkpoints/A3-M5-W7-mcp-readonly-bridge-20260907-0739.patch`）后重新提交（amend `6c1f30e` 或 fixup），消 `git diff --check` 红灯。

### RED-3 · MCP 策略 `--expect-pending` 债（A3 W8 职责，board L193）
- **现象**：`check-mcp-policy.py --expect-pending` → `MCP_PENDING_RESULT=FAIL：已检测到 M5-2 产物（rmcp / mcp_tools / mcp_server bin / MCP_CAPABILITY_V1），应把本脚本 PENDING 码位翻为 ACTIVE 并由 W2 实现接管`。
- **修复**（A3 W8）：将 9 个 PENDING 码翻为 ACTIVE（或把 `--expect-pending` 模式替换为当前阶段断言），保持 self-test/default 绿；**不新增** rmcp/server/listener/网络/运行时执行。

---

## 7. 集成门禁结论

- `pre-merge.sh` → `PRE_MERGE_RESULT=FAIL`（EXIT=1），**仅 2 FAIL**（cargo fmt main + git diff --check branch range）。build metrics 本波已 PASS（warnings 2=baseline，size 164.58 kB ≈20.66% <21% 阈值）。
- `cargo test` → 389/0 ✅（pre-merge 不覆盖，但 A0 before-push 清单必拦）。
- 三条红灯（§6）全部可经卫生修复 + A3 W8 策略债闭合消解，无功能缺陷。

---

## 8. Before-A0 冲突扫描

- **空文件**：无。
- **stale STOPPED 冒充 PASS**：无（各脚本真实 PASS/FAIL）。
- **重复命令名**：`mcp_*` 3 个、`agent_*`/`skill_*` 各 3 个，均唯一；无与既有冲突。
- **ACL 奇偶**：末条恒为 `list_artifact_images`（L127）；A3 W7 `mcp_*`（L124-126）与 A5 W7 `agent_*`/`skill_*`（L118-123）均正确插于末条之前 ✅ 无漂移。
- **bridge/main/types 命令奇偶**：`main.rs:1434-1445` 注册 9 命令，与 `default-commands.toml` ACL、`src/types.ts`/`src/bridge.ts`（A3 W7 +24/+31）一致 ✅。
- **docs NEXT 一致性**：board 头部 `Current NEXT: M5-W8` 与 §M5-W8 dispatch 一致 ✅。
- **lane scope 漂移**：W8 仅 A3（POLICY FIX ONLY）/A5（PRODUCT CODE 硬化）可写产品代码；A1/A2/A4/A6/A7/A8/A9/A10 均 docs/review/support。无漂移。

---

## 9. Push 裁决

> **A0 可 push 当且仅当**以下三事完成：
> 1. **A3 W8 闭 MCP 策略债**（RED-3）：9 PENDING→ACTIVE，`check-mcp-policy.py` self-test/default/`--expect-pending` 全绿。
> 2. **`cargo fmt --all` 全量格式化**（RED-1）：消解 `bridge.rs`/`plugin.rs` 两项，pre-merge 的 `cargo fmt main` 转绿。
> 3. **剥离 A3 W7 `.patch` 行尾空格并重提**（RED-2）：消解 `git diff --check`。

三事完成后重跑 `pre-merge.sh` 预期 `ALL_PASS` + `cargo test` 绿 → A0 可 push。

> 「Excluding-A3」解读：除 A3 W8 策略债（唯一功能门）与 A3 W7 两项卫生债（fmt/行尾空格）外，W8 全量验证 **GREEN**；若 A3 W8 策略债单列后续 wave，则 (2)(3) 两卫生修复后即达 push 就绪。

---

## 10. 债务台账更新（相对 W7 增量）

| 项 | W7 状态 | W8 状态 |
|---|---|---|
| W7-1 cargo test 编译中断 | ❌ RED（4 errors） | ✅ **已闭**（工作树 plugin.rs 修复；389/0） |
| W7-3 build metrics warnings_increased | ❌ RED（2→3） | ✅ **已闭**（warnings 回 2=baseline） |
| W7-2 cargo fmt | ❌ RED | ❌ **仍 RED**（归因 A3 W7 bridge.rs +231 未格式化 + plugin.rs:16 非 canonical） |
| A3 W7 交付卫生：bridge.rs 未格式化 | — | ❌ RED（RED-1） |
| A3 W7 交付卫生：.patch 行尾空格 | — | ❌ RED（RED-2） |
| A3 W8 MCP 策略 `--expect-pending` 债 | — | ❌ RED（RED-3，board L193） |
| A3 W7 MCP 只读桥功能 | pending（W7 误判） | ✅ 已交付+注册+测试绿（更正 W7） |
| A5 W7 Agent/Skill 只读桥 | pending（W7 误判） | ✅ 已交付+注册+策略绿（更正 W7） |
| 构建指标阈值 | 21%（W7 A0 抬 19→21） | 维持；实测 20.66%，余量 0.34% |
| grid_process.rs 两条 warning（index/comms, new） | 既有债 | 维持（与 W7/W8 无关） |
| agent-skill-ui-logic.mjs 未接 pre-merge | 小缺口 | 维持（测试本身 57/57 PASS） |
| agent-skill 策略缺 --expect-pending 模式 | 不一致 | 维持（4 stale PENDING 码位） |
| mcp/agent-memory/graph --expect-pending | FAIL by design | 维持（mcp 待 A3 W8 翻转） |
| A8/A9 W6 graph UI + plugin 策略 | PENDING（W7 标注） | ✅ **已验绿**（34/0；ALL_PASS） |

---

## 11. 声明（避免误读）

- 本车道**零产品代码改动**；三红灯修复配方（§6）为交 A0/A3 的指引，A11 **未**修改 plugin.rs / bridge.rs / .patch / 任何脚本。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；针对**含未提交 A0 集成修复的当前工作树**复跑，未引用旧报告（遵守 IF-5 不 stale）。W7 增量的「A3/A5 W7 待交付」结论经本波核准为**误判**（命令封装在 bridge.rs 而非 agent.rs/skills.rs），已于 §2/§3/§4 更正。
- 仅 `git add` 本文件，不带入他 lane 改动（含 A3 W7 的 .patch/commit、A5 W8 的 check-agent-skill-policy.py staged 更新、多 lane 的 assist/checkpoint 笔记均不提交）。
