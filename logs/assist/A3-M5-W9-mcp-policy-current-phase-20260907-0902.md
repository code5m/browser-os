# Lane A3 · M5-W9 Runtime-Free Polish — MCP 策略相位保持 + M5-2.b 前瞻卡

> LANE=A3　WAVE=M5-W9 Runtime-Free Polish Dispatch（board `PARALLEL_COMMAND_BOARD.md` §M5-W9，行 175-201，A3 行 193）　ROLE=**START POLICY/REVIEW ONLY**
> Task（行 193）：Keep MCP policy current-phase green and prepare a later M5-2.b card for actual rmcp/server work. **Do not implement rmcp/server/listener/network.**
> Allowed Scope = `scripts/check-mcp-policy.py` only if a regression is found、`logs/assist/A3-M5-W9-*.md`
> Must Deliver = **Policy/review note；self-test / default / current-gaps 三模式命令结果**
> BASE=`97118d6`（A0 已集成 W8，本地 `master` 已含 W8 A3 提交 `94e763e`）　STATUS=PASS（三模式全绿，零回归，零产品代码改动）

---

## 0. 调度自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git fetch && pull --ff-only` | 已是最新（`97118d6` 即 origin 头），本地 W8 A3 `94e763e` 已集成 | ✅ 同步 |
| 仍属 A3 可写范围 | ✅ 仅产出 review note + 前瞻卡；未触碰任何产品代码 | ✅ |
| W9 Hard Stop 合规 | 未引入 rmcp/server/listener/network、未增/改命令、无 token/cookie/Authorization/body/prompt-secret 泄漏 | ✅ |
| 是否触碰 `domain.rs` / 产品 Rust / TS / ACL / mcp.rs | 否（W9 = POLICY/REVIEW ONLY） | ✅ |
| 红线（rmcp/tokio/监听/网络/第二执行路径） | 本波**零新增**；既有 `MCP_NO_RMCP_SERVER` 永久禁止形态仍生效 | ✅ |

---

## 1. 相位保持结论：MCP 策略当前相位仍全绿（无回归）

A0 在 W8 集成（`4d7be97` + `97118d6`）未改变 MCP 只读桥形态；复跑本 lane W8 收口后的门禁，三项全绿：

```bash
python3 scripts/check-mcp-policy.py --self-test          # MCP_POLICY_SELF_TEST=PASS（ACTIVE=8，PENDING=0）
python3 scripts/check-mcp-policy.py                       # MCP_POLICY=PASS（无违规）
python3 scripts/check-mcp-policy.py --expect-current-gaps  # MCP_CURRENT_GAPS_RESULT=PASS（W8 相位）
cd src-tauri && cargo test mcp_policy_tests              # 9 passed（产品代码零改动，与 W8 一致）
```

**`--expect-current-gaps` 断言内容（W8 相位）**：
- 只读桥已落地：`main.rs` 已注册 `mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview` 且 ACL（`default-commands.toml`）+ `bridge.ts` 齐备；
- 8 个 ACTIVE 码位零命中：含 `MCP_NO_RMCP_SERVER`（永久禁 `rmcp`/`tokio`/`mcp_server` bin/`mcp_tools/` 目录/`axum`/`hyper::Server`/`TcpListener`）、`MCP_PARITY`（奇偶）、`MCP_BRIDGE_READONLY`（写副作用）、`MCP_FS_TOOL_PATH_POLICY`、`MCP_CAPABILITY_DRIFT`、`MCP_NPM_SDK_PRESENT`、`MCP_NPM_IN_CARGO`、`MCP_NODE_RUNTIME_PRESENT`。

→ **无回归，故按 scope 约定（`scripts/check-mcp-policy.py` only if a regression is found）未修改策略脚本**。`PENDING_CODES` 仍为空（W8 相位债已关闭）。

---

## 2. M5-2.b 前瞻卡（实际 rmcp/server 工作 —— 当前被 W7 Hard Stop 否决，仅规划）

> 本卡为**前瞻规划**：记录「当产品确实需要对外暴露 MCP server（如对接 Claude Desktop / 外部 MCP 客户端，暴露本机命令与能力）时」所需满足的架构与策略翻转。当前（W9 及之前）**W7 Hard Stop 仍永久有效**——禁止引入 rmcp/server/listener，MCP 仅以只读 Tauri 命令桥存在。

### 2.1 触发条件（未来，非本波）
- 产品需要以 **MCP server 协议**向外部客户端暴露工具（而非仅前端经 `invoke` 调只读桥）；
- 或需要 streaming / 双向能力，只读桥的 request/response 范式无法满足。

### 2.2 架构约束（若未来实施，必须满足）
1. **依赖隔离**：`rmcp`（及必要 `tokio`）必须以 `optional = true` 声明于 `Cargo.toml`，并由独立的 `[features] mcp-server` 启用；**绝不进入 `default` features**，绝不污染主二进制依赖图（F-1 实质）。
2. **二进制隔离**：独立的 `[[bin]] mcp_server`（或 feature-gated mod），必须 `required-features = ["mcp-server"]`，使默认 `cargo build` / `cargo tauri build` 不编译 MCP server。
3. **能力单源（沿用 W8 契约）**：server 暴露的每个 tool 必须复用 `domain.rs` 的 `MCP_CAPABILITY_V1` 白名单 + `evaluate_mcp_command` 同源校验；**新增 tool 不得绕过 capability source check**（G3）。
4. **只读保证不降级**：server 暴露的命令仍须是只读（与现有 `mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview` 同口径）；任何写操作（fs write / exec / network / model call）一律拒绝，且 `MCP_BRIDGE_READONLY` 守门继续生效。
5. **ACL 对齐**：server 暴露的 tool 名必须与 `default-commands.toml` ACL 一一对应（奇偶守门 `MCP_PARITY` 继续生效）。
6. **脱敏不降级**：registry 中含 `returns_url` 必须带 `redact_sensitive_url`（`MCP_FS_TOOL_PATH_POLICY` 继续生效）；错误/显示面不得回显 secret（K7/A4 隐私审查结论继续适用）。
7. **无第二执行路径**：MCP server 不得新建命令执行/进程派生通道，仅复用既有 `core` 内部 API（与 W7 只读桥同源）。

### 2.3 策略脚本需翻转的项（若未来实施）
- 将当前单一 **`MCP_NO_RMCP_SERVER`（forbidden）** 收口为：
  - **`MCP_SERVER_GATED`**（ACTIVE）：允许 `rmcp`/`tokio`/`mcp_server` bin **仅当**满足上述 1–2（optional + required-features 隔离）；默认构建若出现 rmcp/tokio 依赖泄漏仍 FAIL；
  - 保留 `MCP_PARITY` / `MCP_BRIDGE_READONLY` / `MCP_FS_TOOL_PATH_POLICY` / `MCP_CAPABILITY_DRIFT` / `MCP_NPM_*` / `MCP_NODE_RUNTIME_PRESENT` 不变。
- `--expect-current-gaps` 模式相应更新为「server 已按 gated 约束落地、奇偶/只读/脱敏全绿」。
- 该翻转**只能由 A0 在显式解锁 M5-2.b（经产品决策确认需要 MCP server）后**执行；在此之前 `MCP_NO_RMCP_SERVER` 保持 forbidden，任何误引入 rmcp/server 的 diff 都会被 pre-merge 拦下。

### 2.4 挂账（W9 镜头下仍未解决，但均不阻塞当前只读桥）
- **R1/G2 能力单源碎片化**：MCP / Skill / Agent / 未来 Plugin / Graph 的能力白名单仍各自定义，待 A3 M5-2.b 收口 `core/capability.rs` + 泛化 `CAPABILITY_SINGLE_SOURCE` 门（与 §2.2-3 同源）。
- **`bridge.rs:6276` 既有测试 fmt 差异**（A4/A5 W7/W8 产物）：集成时由 A0 统一 fmt，本波不涉及。

---

## 3. 范围隔离

- 本波**零产品代码改动**（`mcp.rs`/`domain.rs`/`main.rs`/`default-commands.toml`/`bridge.ts`/`types.ts` 均无 diff）。
- 未修改 `scripts/check-mcp-policy.py`（无回归，依 scope 不触碰）。
- 仅新增本 note + checkpoint 文档；**无 `.patch`**（无代码 diff）。
- 未 push（仅 A0 可 push master）。

---

## 4. 验证汇总

| 检查 | 结果 |
|---|---|
| `check-mcp-policy.py --self-test` | PASS（ACTIVE=8，PENDING=0） |
| `check-mcp-policy.py` 默认 | 无违规 PASS |
| `check-mcp-policy.py --expect-current-gaps` | MCP_CURRENT_GAPS_RESULT=PASS（W8 相位仍成立） |
| `cargo test mcp_policy_tests`（src-tauri） | 9 passed（产品代码零改动） |
| W9 Hard Stop 合规 | ✅ 无 rmcp/server/listener/network、无命令增改、无 secret 泄漏 |

---

## 5. checkpoint / note 路径

- note：`logs/assist/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md`
- checkpoint：`logs/checkpoints/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md`

---

```
LANE=A3
STATUS=PASS
WAVE=M5-W9 (POLICY/REVIEW ONLY)
BASE=97118d6
HEAD=logs/assist/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md
FILES=logs/assist/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md,logs/checkpoints/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md
VERIFY=check-mcp-policy.py --self-test PASS(ACTIVE=8,PENDING=0) / 默认 PASS / --expect-current-gaps PASS；cargo test mcp_policy_tests 9 passed
REGRESSION=none → scripts/check-mcp-policy.py 未改动
MERGE_NOTES=W9 POLICY/REVIEW ONLY：MCP 策略当前相位仍全绿（零回归），未改脚本；产出 M5-2.b 前瞻卡（实际 rmcp/server 工作被 W7 Hard Stop 否决，仅规划架构约束+策略翻转路径）；零产品代码；无 .patch
NEXT=A3 M5-2.b 收口 core/capability.rs + 泛化 CAPABILITY_SINGLE_SOURCE 门（能力单源）；MCP server 解锁须 A0 显式决策后翻转 MCP_NO_RMCP_SERVER→MCP_SERVER_GATED
```
