# Lane A3 · M5-W8 MCP 策略相位债收口（POLICY FIX ONLY）

> LANE=A3　WAVE=M5-W8 Full-Lane Follow-up Dispatch（board `PARALLEL_COMMAND_BOARD.md` §M5-W8，行 145-196）　ROLE=**START POLICY FIX ONLY**
> Task（行 188-190）：Close W7 MCP policy phase debt —— `check-mcp-policy.py --expect-pending` 因 MCP 产物已存在而失败；将 W1 pending 语义转为 W7/W8 active 检查，或用「当前相位断言」替换该模式；default/self-test 保持绿。
> Allowed Scope = `scripts/check-mcp-policy.py`、`scripts/pre-merge.sh`（如需要）、`logs/checkpoints/A3-M5-W8-*.md`、可选 policy-only fixtures
> Must Deliver = **check-mcp-policy.py --self-test PASS、default PASS、current-phase mode PASS；focused MCP Rust tests still PASS；no product runtime expansion**
> BASE=`a26fbaf`（origin/master 仍停在 W7 dispatch 基）　STATUS=PASS（三模式 + rust 测试全绿）

---

## 0. 调度自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git fetch && pull --ff-only` | origin 停在 `a26fbaf`（W7 dispatch 基），本地 4 领先（A6/A11/A3 W7 产物均未集成），pull 为 no-op | ✅ 同步 |
| 仅 A3 可写本 lane 产物 | ✅ 仅动 `check-mcp-policy.py` + `pre-merge.sh`，**零产品代码** | ✅ |
| 是否触碰 `domain.rs` / 产品 Rust / TS / ACL | 否（W8 纯策略收口） | ✅ 严守 |
| 红线（rmcp/tokio/监听/网络/第二执行路径） | 本波**新增并强化** `MCP_NO_RMCP_SERVER` 永久禁止 rmcp/server/listener/tokio | ✅ |

---

## 1. 相位债根因

W7 A3 以**只读 Tauri 命令桥**落地 MCP（`mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview`，注册于 `main.rs`，ACL + `bridge.ts` 齐备），并在 `domain.rs` 落地 `MCP_CAPABILITY_V1` 常量。

但 `check-mcp-policy.py` 仍保留 **W1 的 `--expect-pending` 模式与 7 个 PENDING 码位**，其语义是「MCP 产物尚未出现（W1 守门）」——一旦 `domain.rs` 出现 `MCP_CAPABILITY_V1`，`_mcp_present()` 即返回 True，导致：
- `--expect-pending` 现**必然 FAIL**（board 行 177 已记录「正确地标志 W1 pending 假设应退役」）；
- 7 个 PENDING 码位守的是「独立 `mcp_server` 二进制 + `mcp_tools/` 目录」架构，该架构在 W7 被 **W7 Hard Stop 永久否决**（A3 走的是 Tauri 命令桥，非 rmcp server）。

故 W8 收口：退役 W1 pending 语义，将其关心的「禁止 server 架构」转为单一 **ACTIVE** 守门，并用「当前相位断言」替换 `--expect-pending`。

---

## 2. 改动（2 文件，纯策略，无产品运行时扩张）

### 2.1 `scripts/check-mcp-policy.py`
- **移除 `--expect-pending` 模式** → 新增 **`--expect-current-gaps` 当前相位断言**：扫描真实仓库，`detect_hits` 后断言 (a) 只读桥已落地（`_mcp_bridge_present`：3 条命令在 `main.rs` 注册且 ACL+`bridge.ts` 齐备）；(b) 红线/奇偶/只读 8 个 ACTIVE 码位零命中。满足则 `MCP_CURRENT_GAPS_RESULT=PASS`，否则 FAIL。
- **退役 7 个 PENDING server 架构码位**（`MCP_LISTEN_PORT`/`MCP_RUNTIME_LEAK`/`MCP_OPTIONAL_DEP`/`MCP_BIN_GATED`/`MCP_TOOL_CALLS_COMMAND`/`MCP_PATH_POLICY_MISSING`/`MCP_URL_NOT_REDACTED`）及其 gate 函数。
- **退役 `MCP_TREE_TAURI`**（cargo-tree no-op；core 纯洁性由 `check-core-boundary.py` 守）。
- **新增 ACTIVE 守门 `MCP_NO_RMCP_SERVER`**（8 行核心逻辑）：永久禁止
  - `Cargo.toml` 出现 `rmcp` 依赖 / `tokio` 依赖 / `[[bin]] mcp_server`；
  - `src` 出现 `axum` / `hyper::Server` / `tokio::net::TcpListener` / `std::net::TcpListener`；
  - 存在 `src-tauri/src/mcp_tools/` 目录。
  - 该 regex 特意**避开裸 `TcpListener` 字样**（因 `mcp.rs:7` 注释「无 `TcpListener`」会误命中），只匹配真实监听形态。此守门把 W7 Hard Stop「无 rmcp/server/listener/tokio」**提升到策略层永久执行**。
- **`MCP_PARITY` 由 PENDING 升 ACTIVE**（桥已存在，奇偶守门应始终生效）。
- `PENDING_CODES` 现为空列表 → **相位债关闭**（self-test 打印 `ACTIVE=8，PENDING=0`）。
- **清理死代码**：`_CODE_REX` 移除 3 条不再引用的条目；删除 `_MCP_FILE_RE`/`_is_mcp_file`/`_with_artifact`/`_MCP_ARTIFACT`；所有 `_mcp_present` 引用清除；更新顶部用法/docstring 与过时注释。
- **self-test 同步**：7 个 PENDING 坏样本 → 1 个 `MCP_NO_RMCP_SERVER` 坏样本（`Cargo.toml` 引入 `rmcp`）；`MCP_FS_TOOL_PATH_POLICY`/`MCP_PARITY`/`MCP_BRIDGE_READONLY` 坏样本改用 `mutate`（移除 `_with_artifact` 依赖）；删除「无 MCP 产物 gate 测试」（PENDING 已空）。

### 2.2 `scripts/pre-merge.sh`
- MCP 段（行 411-421）接入 **`--expect-current-gaps`** 当前相位断言（与 lifecycle/security lane 既有的 `--expect-current-gaps` 约定对齐），并把码位数注释更正为「8 ACTIVE / 0 PENDING」。
- `bash -n` 语法校验通过。

---

## 3. 验证（全绿）

```bash
python3 scripts/check-mcp-policy.py --self-test        # MCP_POLICY_SELF_TEST=PASS（ACTIVE=8，PENDING=0）
python3 scripts/check-mcp-policy.py                     # 无违规 → MCP_POLICY=PASS
python3 scripts/check-mcp-policy.py --expect-current-gaps  # MCP_CURRENT_GAPS_RESULT=PASS（W8 相位）
cd src-tauri && cargo test mcp_policy_tests            # 9 passed（focused MCP Rust tests 仍 PASS）
bash -n scripts/pre-merge.sh                           # 语法 OK
read_lints scripts/check-mcp-policy.py                 # 0 diagnostics
```

> 本波**未改动任何产品代码**（Rust/TS/ACL/domain 均无 diff），故「no product runtime expansion」天然满足；MCP 桥行为完全由 W7 提交 `6c1f30e` 决定，本波仅强化其策略守门。

---

## 4. 改动文件清单（本 lane 专属，2 文件）

```
scripts/check-mcp-policy.py   +90  -180   退役 W1 pending 语义、新增 MCP_NO_RMCP_SERVER、--expect-current-gaps、清理死代码、self-test 同步
scripts/pre-merge.sh           +6   -4    MCP 段接入 --expect-current-gaps 当前相位断言
```

> 工作树另有 A1/A2/A4/A5/A6/A7/A8/A9/A11 的 WIP（未跟踪/未暂存），本 lane 仅暂存并交付上述 2 文件，不触碰他 lane 产物（见 `git status`）。

---

## 5. 补丁 / checkpoint

- 补丁：`logs/checkpoints/A3-M5-W8-mcp-policy-phase-debt-20260907-0807.patch`（仅本 lane 2 文件，`git diff -- <2 files>` 生成）
- checkpoint：`logs/checkpoints/A3-M5-W8-mcp-policy-phase-debt-20260907-0807.md`

---

## 6. NEXT / 合并须知

- 本 patch 为**纯策略收口**：把 W1 的「独立 rmcp server」假设彻底退役，W7 的只读 Tauri 命令桥被 `MCP_NO_RMCP_SERVER`（永久禁 rmcp/server/listener/tokio）+ `MCP_PARITY`（奇偶）+ `MCP_BRIDGE_READONLY`（写副作用）+ `--expect-current-gaps`（相位断言）四重守门。
- 与 A5/A7 只读桥（Agent/Skill/Graph）**互补无冲突**：各 lane 自守其命令的 source-check/ACL/前端；本波只动 MCP 策略脚本。
- `pre-merge.sh` 现对 MCP 同时跑 `--self-test` + 默认 + `--expect-current-gaps`，W8「current-phase mode PASS」在 CI 层强制。
- 挂账（非本波范围）：① 能力单源碎片化（MCP/Skill/Agent/未来 Plugin/Graph）仍属 A3 M5-2.b + A0 裁决（R1/G2）；② `bridge.rs:6276` 既有测试 fmt 差异（A4/A5 W7 产物），集成时由 A0 统一 fmt。
- 未 push（仅 A0 可 push master）。

```
LANE=A3
STATUS=PASS
WAVE=M5-W8 (POLICY FIX ONLY)
BASE=a26fbaf
HEAD=logs/assist/A3-M5-W8-mcp-policy-phase-debt-20260907-0807.md
FILES=scripts/check-mcp-policy.py,scripts/pre-merge.sh
VERIFY=check-mcp-policy.py --self-test PASS(ACTIVE=8,PENDING=0) / 默认 PASS / --expect-current-gaps PASS；cargo test mcp_policy_tests 9 passed；bash -n pre-merge.sh OK；read_lints 0
CHECKPOINT=logs/checkpoints/A3-M5-W8-mcp-policy-phase-debt-20260907-0807.{patch,md}
MERGE_NOTES=纯策略收口：退役 W1 pending 语义(7 PENDING 码位+MCP_TREE_TAURI+--expect-pending)，新增 ACTIVE MCP_NO_RMCP_SERVER(永久禁 rmcp/server/listener/tokio)+--expect-current-gaps 当前相位断言，MCP_PARITY 升 ACTIVE，PENDING 清空；pre-merge 接入 --expect-current-gaps；零产品代码改动
NEXT=A3 M5-2.b 收口 core/capability.rs + 泛化 CAPABILITY_SINGLE_SOURCE 门；plugin/agent 未来经 MCP 暴露时复用 evaluate_mcp_command(G3)
```
