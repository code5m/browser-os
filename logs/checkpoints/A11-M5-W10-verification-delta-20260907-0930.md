# M5-A11 · W10 验证增量（Controlled Runtime Prep；默认门全 ALL_PASS；`mcp` 特性门待 A3 落地）

```text
LANE=A11
STATUS=默认构建/测试全绿；W10 刚开启（board 16:00），A3 的 `mcp` stdio-prep 特性尚未提交 → 特性门 N/A（已定义复核程序）；其余运行时面按 W10 硬停止全部锁定。A0 可 push 当前树（W9 集成 + W10 调度文档），A3 特性落地后须先过特性门再纳入 push。
BASE=3792115（A0 W9 集成：feat(M5): integrate W9 runtime-free polish）
HEAD=logs/checkpoints/A11-M5-W10-verification-delta-20260907-0930.md
FILES=logs/checkpoints/A11-M5-W10-verification-delta-20260907-0930.md ; logs/assist/A11-M5-W10-push-readiness-20260907-0930.md
VERIFY=见 §1 W10 验证矩阵（当前工作树 = W9 集成后干净基线 + A4 W10 隐私复核基线文档未跟踪）
CHECKPOINT=本文件
MERGE_NOTES=§3 `mcp` 特性门复核程序（A3 落地后必跑）；§7 锁定运行时面清单；§8 push 就绪度
NEXT=A3 提交 `mcp` 特性后，A11 复跑 §3 特性门（cargo test --features mcp + 默认 + 策略 + 指标）→ 全绿则 A0 纳入 W10 push
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W10 Controlled Runtime Prep Dispatch（L176-227）→ **A11 = START VERIFICATION**（L202）：维护 W10 验证矩阵——默认构建/测试、**若 A3 加 `mcp` 则特性构建/测试**、策略脚本、UI 脚本、pre-merge、构建指标 ≤22%、告警不变；产物 `logs/checkpoints/A11-M5-W10-*.md`、`logs/assist/A11-M5-W10-*.md`。
> 用户指令「M5-W10 Controlled Runtime Prep Dispatch」：W10 准备下一运行时波次但**不开不安全执行**；仅 A3 可碰 MCP 运行时准备代码（stdio-only、feature-gated、无 listener/network、无工具副作用），Plugin/Agent 执行保持锁定。
> 范围声明：本增量**只产出验证文档，零产品代码改动**。W10 仅 A3（PRODUCT CODE NARROW）可写 MCP 准备代码；A1/A2/A4/A7/A9/A10 为 docs/review；A5/A6/A8 为 test/policy/UI 小改。
> 姊妹件：W9 `M5-A11-W9-verification-delta-20260907-0905.md`、W8 `M5-A11-W8-verification-delta-20260907-0758.md`。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已最新（origin/master 未动）
git status --short --branch          # 干净基线 + 1 个未跟踪 A4 W10 隐私复核基线文档
git log --oneline -5                 # 3792115(A0 W9 集成) / 0d86a19(A11 W9) / 770e22c(A6 W9) / ef87401(A3 W9 MCP 当前相位) / 97118d6(A0 W8 归一化)
```
- **调度匹配**：board 头部 `Current NEXT: M5-W10 controlled runtime prep; W9 outputs are ready for A0 integration and W10 opens only narrow MCP stdio shell preparation plus review/polish lanes`（L7）。W10 目标（L180）：准备下一运行时波次但**不开不安全执行**；仅 A3 可碰 MCP 运行时准备（stdio-only、feature-gated、无 listener/network、无工具副作用），Plugin/Agent 执行锁定。
- W10 硬停止（L204-211）：仅 A3 可碰 MCP 准备产品代码；禁 TCP listener/HTTP/network/daemon/plugin-install/skill-exec/model-call/隐藏 script-db 执行；任何 `rmcp`/`tokio` 须可选 + feature-gated + 不污染默认构建 + 策略自检守护；命令面须 source-check + ACL 同步；**构建指标阈值 22%**；cargo 告警不增；仅 A0 push。

---

## 1. W10 验证矩阵（当前工作树实跑 = W9 集成后干净基线）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测（默认） | `cargo test --manifest-path src-tauri/Cargo.toml` | **410 passed（408 bin + 2），0 failed**（CARGO_TEST_EXIT=0） | ✅ |
| cargo check 告警 | `cargo check --locked` | **2 warnings**（grid_process.rs:76 index/comms；grid_process.rs:103 new）= 基线 | ✅ |
| cargo fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | **0 diff** | ✅ |
| `mcp` 特性构建/测试 | `cargo test --features mcp`（或等价） | **N/A** —— 当前无 `mcp` Cargo 特性（见 §3） | ⏸ 待 A3 |
| core 边界门 | `check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| M5-2 MCP 门 | `check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS`（**ACTIVE=8，PENDING=0**） | ✅ |
| M5-2 MCP 门 | `check-mcp-policy.py`（default） | `MCP_POLICY=PASS` | ✅ |
| M5-2 MCP 门 | `check-mcp-policy.py --expect-current-gaps` | **`MCP_CURRENT_GAPS_RESULT=PASS`** | ✅ |
| agent-memory 门 | `check-agent-memory-policy.py --self-test` | `AGENT_KV_POLICY_SELF_TEST=PASS`（ACTIVE=5） | ✅ |
| agent-skill 门 | `check-agent-skill-policy.py --self-test` | `AGENT_SKILL_POLICY_SELF_TEST=PASS`（ACTIVE=3，PENDING=5） | ✅ |
| agent-skill 门 | `check-agent-skill-policy.py`（default） | `AGENT_SKILL_POLICY=PASS` | ✅ |
| graph 门 | `check-graph-policy.py --self-test` | `GRAPH_POLICY_SELF_TEST=PASS`（ACTIVE=7） | ✅ |
| graph 门 | `check-graph-policy.py`（default） | `GRAPH_POLICY=PASS` | ✅ |
| graph UI 逻辑 | `node scripts/check-graph-ui-logic.mjs` | **通过 43，失败 0** | ✅ |
| Agent/Skill UI 逻辑 | `node scripts/check-agent-skill-ui-logic.mjs` | **99 assertions passed, 0 failed** | ✅ |
| A9 plugin 门 | `check-plugin-policy.py --self-test` | `PLUGIN_SELF_TEST=ALL_PASS`（ACTIVE=1，PENDING=5） | ✅ |
| A9 plugin 门 | `check-plugin-policy.py`（default） | `PLUGIN_POLICY=PASS` | ✅ |
| A9 plugin 门 | `check-plugin-policy.py --expect-pending` | `PLUGIN_PENDING_OK` | ✅ |
| tools 门 | `check-tools-policy.py --self-test` | self-test OK（16 坏样本全检） | ✅ |
| database 门 | `check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（ACTIVE=14，PENDING=1） | ✅ |
| scheduler 门 | `check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（ACTIVE=23） | ✅ |
| script-exec 门 | `check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-CCrBduX4.js`=164.82 kB（gzip 59.04）；面板懒加载分块 | ✅ |
| 构建指标 | `measure-build-metrics.py --compare logs/m0-build-metrics/build-metrics-4f0e8ab.json --skip-build` | **`total_bytes_pct=21.38`（≤22% 阈值）**，`cargo_warnings=0`，`warnings_increased=false`，`exceeds_growth_limit=false` | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=ALL_PASS`**（无 FAIL；build metrics 未回归） | ✅ |

> board W10 事实（L179）引述：Agent/Skill 桥测试 26 passed、MCP 测试 9 passed、plugin 测试 10 passed；Agent/Skill UI 99、Graph UI 43；npm build 通过；MCP/Agent 策略脚本通过。本矩阵实测 **UI 99/43 + 全部策略 PASS**，与 board 事实一致（26+9+10=45 为集成子计数，含于 410 单测总数）。
> legacy `check-mcp-policy.py --expect-pending` 已退役；现行 W10 相位门 `--expect-current-gaps` → PASS。

---

## 2. W10 硬停止合规复核（当前工作树）

| W10 硬停止（L204-211） | 当前树证据 | 判定 |
|---|---|---|
| 仅 A3 可碰 MCP 运行时准备 | 无 MCP 运行时代码（仅 W7/W8 只读桥 `mcp.rs` 命令）；A3 W10 特性未提交 | ✅ |
| 禁 TCP listener/HTTP/network/daemon/plugin-install/skill-exec/model-call/隐藏 script-db 执行 | `grep` 全仓无 `tokio`/`rmcp`/`TcpListener`/`bind(`/`reqwest`/`hyper` 等；策略脚本（script-exec/tools/core/mcp/agent-skill/plugin）全 PASS 守护 | ✅ |
| `rmcp`/`tokio` 须可选 + feature-gated + 不污染默认 | grep `Cargo.toml` 无 `[features]`/`mcp`/`tokio`/`rmcp`；默认构建零此类依赖 | ✅（尚未引入） |
| 命令面 source-check + ACL 同步 | core 边界 + agent-skill/plugin/graph 策略 PASS；本波无新增命令（ACL 末条恒 `list_artifact_images`，无越界） | ✅ |
| 构建指标 ≤22% | **21.38%** | ✅ |
| cargo 告警不增 | `cargo_warnings=0` delta（基线 2） | ✅ |
| 仅 A0 push | 本 Lane 不 push | ✅ |

---

## 3. `mcp` 特性门复核程序（A3 落地后必跑；当前 N/A）

**现状**：A3 的 W10 `mcp` stdio-prep 特性在验证时刻**尚未提交**。证据：
- `grep -nE '\[features\]|^mcp|mcp =|tokio|rmcp' src-tauri/Cargo.toml` → 空（无 `[features]` 段，无 `mcp`/`tokio`/`rmcp` 引用）；
- 无 `src-tauri/src/bin/mcp_server.rs`、无 `src-tauri/src/mcp_tools/**`；
- 最近提交 `3792115` 为 W9 集成，其后无 A3 W10 产品代码提交；
- 工作树仅 1 个未跟踪文档（`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`，docs）。

**A3 提交后 A11 须复跑的特性门（board L194 接受条件）：**
1. `cargo test --features mcp`（或 A3 实际命名）→ 编译 + 全 PASS（stdio-only 骨架、无工具副作用）；
2. 默认 `cargo test`（无 features）→ 仍 410/0、告警仍 2（**默认构建零污染**）；
3. `rmcp`/`tokio` 须 `optional` + `required-features` gated，且 `cargo tree`（默认 features）不含二者；
4. `check-mcp-policy.py --self-test/default/--expect-current-gaps` 仍 PASS（策略自检守护特性门控）；
5. pre-merge 仍 `ALL_PASS`；
6. 构建指标仍 **≤22%**（stdio-only 编译隔离壳不应推高默认包体；`total_bytes_pct` 不越 22%）；
7. 无 listener/network/daemon/工具副作用（由 core/tools/mcp 策略 + grep 复核）；
8. 若 A3 新增 MCP 命令：须同包落地 source-check + ACL + bridge/types + policy/tests（board L209）。

> 上述任一项失败 → 阻塞 A0 将该特性提交纳入 push，直至 A3 修复。当前（特性未落地）此项 = ⏸ 待 A3，不阻塞默认波次 push。

---

## 4. 构建指标复核

- `total_bytes_pct = 21.38`（W9-0905 实测 21.09；本波 +0.29，仍 **≤ 22%** W10 阈值）✅。
- `cargo_warnings` delta = 0 → 告警数与基线持平（2 条，grid_process.rs 既有债）✅。
- `warnings_increased = false`、`exceeds_growth_limit = false` → 构建指标 **通过**（pre-merge 内部 "build metrics 未回归"）。
- 主 chunk `index-CCrBduX4.js` = 164.82 kB（gzip 59.04）；xterm/vue-vendor 独立分块；面板懒加载。

---

## 5. 集成门禁结论

- `pre-merge.sh` → **`PRE_MERGE_RESULT=ALL_PASS`**（EXIT=0），**零 FAIL**。覆盖：baseline/verify-resources self-test、npm build、cargo check、cargo fmt、全部 policy 不变量夹具（CORE/MCP/AGENT-KV/AGENT-SKILL/GRAPH/PLUGIN）、git diff --check。
- `cargo test` → 410/0 ✅（pre-merge 不覆盖单测，但 A0 before-push 必拦；本波已独立确认）。
- **W10 硬停止全部满足**：无运行时（无 MCP server/rmcp/plugin-install/skill-exec/model-call/network）；构建指标 21.38% ≤ 22%；cargo 告警未增；仅 A0 可 push。

---

## 6. 各 Lane W10 可观测状态（A11 仅验证门禁，不审内容）

- **A1**（DOCS ONLY）：W10 协调/标 active NEXT；更新 M5 卡片声明哪些运行时面仍锁定、A3 的 `mcp` stdio-prep 切片已开。未提交（W10 进行中）。
- **A2**（BOUNDARY REVIEW ONLY）：复核 A3 W10 `mcp` stdio-prep 计划/代码的核心/bin 边界（core 无 tauri、mcp 工具不调 bridge::*、无重复 script/db/plugin 执行路径）。待 A3 代码落地后出 verdict。
- **A3**（PRODUCT CODE NARROW）：W10 `mcp` stdio-prep 骨架——feature-gated、复用现有 `mcp.rs` registry/policy、**无 TCP listener/network/rmcp 工具副作用/无 file/db/script/plugin 执行**。**验证时刻尚未提交**（见 §3）。
- **A4**（PRIVACY REVIEW ONLY）：`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`（未跟踪，基线）。复核 W10 MCP stdio-prep + 既有 Agent/Skill/Graph/Plugin 错误/审计/日志的 secret 回显（工具结果 URL、capability reason、序列化错误）。
- **A5**（TEST/POLICY ONLY）：Agent/Skill 只读桥仍锁执行；仅当 A10/A4 指出具体泄漏才补策略/测试，否则出 readiness 备注。无执行/运行时。
- **A6**（UI SMALL ONLY）：Agent/Skill UI 无执行按钮；打磨 disabled/preview、loading/error 确定性、无 secret 回显。
- **A7**（GRAPH DOCS ONLY）：后续 graph 实时查询实现卡（命令名/结果上限/取消/隐私）；不实现命令。
- **A8**（GRAPH UI SMALL ONLY）：无后端命令前提下继续 graph UI 打磨（无后端空态、确定性选择、有界渲染）；当前 graph-UI 逻辑 **43/0**。
- **A9**（PLUGIN RUNTIME PLAN ONLY）：plugin 运行时调度卡（install/enable/delete/list 序列、签名失败模式、存储上限、审计脱敏、UI 依赖）；不实现 plugin 运行时。
- **A10**（SECURITY REVIEW）：批量复核 W10 输出（重点 A3 feature-gated MCP prep）；对任何 listener/network/默认依赖污染/副作用工具零容忍。
- **A11**（本 Lane）：本验证增量。

---

## 7. W10 锁定运行时面（按硬停止，未来波次，非 W10 阻塞）

- **Plugin install/enable/delete/download 运行时** → 锁定（A9 W10 仅出运行时卡，不实现）。
- **Skill/Agent 执行 / model call / network** → 锁定（A5 W10 无执行）。
- **MCP server / rmcp 实际运行工具** → A3 仅做 stdio-prep 编译隔离骨架（无工具副作用）；实际 rmcp/server 留待 M5-2.b（A3 W9 `ef87401` 已出 forward card）。本波 `mcp` 特性尚未提交（§3）。
- 非 W10 范围基线债：`grid_process.rs` 2 warnings（早前 M5 外）；残留 PENDING 策略码（agent-skill 5 / plugin 5 / database 1，未来相位夹具，非阻塞）；终端 GUI 债 D23-D26（M3 时代，独立于 M5）。

---

## 8. Push 就绪度

### Default 波次（当前树 = W9 集成 + W10 调度文档）
> **A0 可 push 当前树**。判定依据：默认门全绿（pre-merge `ALL_PASS`、cargo test 410/0、fmt 0 diff、构建指标 21.38% ≤ 22%、cargo 告警不变、W10 硬停止全满足）。board L179 明言「W9 still needs A0 final pre-merge/push after this dispatch is committed」→ A0 在提交 W10 调度文档后即可 push。

### A3 `mcp` 特性（增量）
> **待 A3 落地后先过 §3 特性门**再纳入 W10 push。特性未提交前，不阻塞上述 default 波次 push；特性提交后若 §3 全绿则可随 W10 一并 push，否则阻塞至修复。

### 工作树在制（验证时刻）
- 未跟踪：`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`（A4 隐私基线，docs）。
- A1/A2/A3/A5/A6/A7/A8/A9/A10 的 W10 产品代码/复核（除 A4 基线文档）在验证时刻尚未提交或属进行中；A0 集成前按 board 各自 scope 收口。

---

## 9. 债务台账更新（相对 W9 增量）

| 项 | W9-0905 | W10-0930 |
|---|---|---|
| cargo test | 402/0 | ✅ **410/0**（+8） |
| cargo 告警 | 2=baseline | ✅ 2=baseline（未增） |
| cargo fmt | 0 diff | ✅ 0 diff |
| pre-merge | ALL_PASS | ✅ **ALL_PASS** |
| 构建指标 total_bytes_pct | 21.09% | ✅ **21.38%**（≤22%） |
| MCP 策略相位 | ACTIVE=8/PENDING=0 | ✅ 同（`--expect-current-gaps` PASS） |
| Graph UI 逻辑 | 41/0 | ✅ **43/0** |
| Agent/Skill UI 逻辑 | 79/0 | ✅ **99/0** |
| `mcp` 特性门 | N/A（W9 无特性） | ⏸ **N/A（A3 未提交）**，程序已定义（§3） |
| MCP 运行时准备代码 | 无 | ⏸ A3 W10 未提交（锁定中，仅 A3 可碰） |
| Plugin/Agent/Skill 执行 | 锁定 | ✅ 维持锁定（W10 硬停止） |
| 残留 PENDING 码 | agent-skill 5 / plugin 5 / database 1 | ✅ 维持（非阻塞） |
| grid_process.rs 2 warning | 既有债 | ✅ 维持（非 W10 范围） |

---

## 10. 声明（避免误读）

- 本车道**零产品代码改动**；仅产出验证文档（checkpoint + assist）。W9 集成（`3792115`）由 A0 完成；W10 调度文档由 A0 添加（L176 标注 16:00 CST）。所有修复/集成均非 A11 所为，A11 仅验证。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；针对**W9 集成后干净工作树**（含 1 个 A4 隐私基线文档未跟踪）复跑，未引用旧报告（遵守 IF-5 不 stale）。
- A3 的 W10 `mcp` 特性在验证时刻未提交 → 特性门标 N/A 并预定义复核程序（§3），不阻塞 default 波次 push。
- 仅 `git add` 本 Lane 两文件，不带入他 lane 在制改动（A4 隐私基线文档不提交）。
