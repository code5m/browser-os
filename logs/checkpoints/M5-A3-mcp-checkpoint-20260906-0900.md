# A3 · M5-2 MCP 策略门禁 · W2 最小可集成切片 checkpoint

> LANE=A3　WAVE=M5-2.a（契约冻结 / 门禁先行）　STATUS=PASS_WITH_DOCS+GATE
> BASE=854bc40（master 领先 origin/master 4；A2 的 `mvp_core` 边界已提交；本切片在其之上）
> DELIVERABLES=`scripts/check-mcp-policy.py`(新增) + 本 checkpoint
> 生成：2026-09-06 09:00 CST

---

## 0. 本切片是什么 / 不是什么

**是**：一个**静态准入门禁脚本**，把 M5-2 的 13 条安全不变量暴露成可复现的 `pre-merge` 守门。
**不是**：MCP 运行时代码。本切片**不引入** `rmcp` / `tokio` / 任何 npm 包 / MCP server / 新 Tauri 命令 /
ACL 条目——即不触碰 W1 硬停止列出的任何红线，与 A0 W1 硬停止（`PARALLEL_COMMAND_BOARD.md` §M5-W1 Hard Stops）一致。

> 关于调度：本机 `PARALLEL_COMMAND_BOARD.md` 头部仍写 `Current NEXT: M5-W1 ... A3 SUPPORT DOCS ONLY`，
> 且板上**无 M5-W2 章节**（仅 `## Dispatch Waves` 下的 M4 `Wave 2`，非 M5）。
> 但用户指令明确要求「按 M5-W1/M5-W2 职责继续推进，做自己 Lane 的下一张最小可集成切片」。
> 在板子尚未翻到 W2 的前提下，本 Lane 选择**最低风险、最贴合 A3 既有职责**的推进方式：
> 先交付 A11 D53 / A10 D-1 早已指派给 A3/A14 的 `check-mcp-policy.py` 门禁（A3 主篇 §8 已设计 10 个码位）。
> 若 A0 尚未正式把板子翻到 W2，该脚本作为**零运行时代价的就绪补丁**，随时可接入。

---

## 1. 交付物：`scripts/check-mcp-policy.py`

对齐 A2 的 `scripts/check-core-boundary.py` 范式：「**产物存在才判**」——M5-2 代码未落地时 PENDING 码位自动 no-op。

### 1.1 码位登记表（4 ACTIVE + 9 PENDING）

| 码位 | 类型 | 守什么 | 来源 |
|---|---|---|---|
| `MCP_NPM_SDK_PRESENT` | ACTIVE | `package.json` 不得含 `@modelcontextprotocol/*`（禁 npm 分包） | A3 主篇 §3.3 / A11 D51 |
| `MCP_NPM_IN_CARGO` | ACTIVE | `Cargo.toml` 不得含 `npm` 字样（纯 Rust rmcp） | A11 D51 |
| `MCP_NODE_RUNTIME_PRESENT` | ACTIVE | src 内不得 `Command::new("node"\|"npx")` | A3 主篇 §3.3 |
| `MCP_CAPABILITY_DRIFT` | ACTIVE | `MCP_CAPABILITY_V1` 单一真源（不得两处定义） | A11 D46 |
| `MCP_LISTEN_PORT` | PENDING | MCP 文件不得出现 TCP 监听（首期仅 stdio） | A3 主篇 §3.1 / A1 卡 §4.1 |
| `MCP_RUNTIME_LEAK` | PENDING | `tokio::` 不得漏进非 MCP 专属文件 | A3 W1 delta §2.2 / §8 |
| `MCP_OPTIONAL_DEP` | PENDING | `rmcp`/`tokio` 若存在须 `optional=true` 且由 `mcp` feature 启用、不在 `default` | A3 W1 delta §2.2 |
| `MCP_BIN_GATED` | PENDING | `[[bin]] mcp_server` 须 `required-features=["mcp"]` | A3 W1 delta §2.2 |
| `MCP_TOOL_CALLS_COMMAND` | PENDING | `mcp_tools/` 不得调 `bridge::`/`terminal::`（须调 core 内部 API） | A3 补篇 §6 / B3 |
| `MCP_PATH_POLICY_MISSING` | PENDING | 暴露 `read_file`/`list_dir` 须 `check_path_within_roots` | A3 补篇 §3 / B8 |
| `MCP_URL_NOT_REDACTED` | PENDING | 回传 URL 须 `redact_sensitive_url` | A3 补篇 §3 / B9 |
| `MCP_PARITY` | PENDING | `mcp_*` 命令须 handler ∧ ACL ∧ `bridge.ts` 三处同现 | A3 主篇 §2.2 F-1 教训 |
| `MCP_TREE_TAURI` | PENDING | `cargo tree -p mvp-core` 无 tauri（阶段一同 package 必 FAIL，阶段二转 ACTIVE） | A3 W1 delta §3 |

### 1.2 用法（与 `check-core-boundary.py` 同族）

```text
python3 scripts/check-mcp-policy.py                 默认扫描（无违规 → EXIT 0；有 → EXIT 2）
python3 scripts/check-mcp-policy.py --self-test     好/坏样本双向自检（含变异防呆）
python3 scripts/check-mcp-policy.py --expect-pending 验证 MCP 产物尚未落地（W1 守门）
```

### 1.3 自测证据（2026-09-06 09:00，本机 python3）

```text
$ python3 scripts/check-mcp-policy.py --self-test
MCP_POLICY_SELF_TEST=PASS（ACTIVE=4，PENDING=9）        exit=0

$ python3 scripts/check-mcp-policy.py
MCP_POLICY=PASS（无违规）                               exit=0

$ python3 scripts/check-mcp-policy.py --expect-pending
MCP_PENDING_RESULT=NONE（9 个 pending 码位均未实现，W1 守门通过）  exit=0
```

自测覆盖：每个码位至少一个坏样本（且「变异防呆」断言内容确有改动，防漏检）；
好样本零误报；无 MCP 产物时 PENDING gate 不误触发。

### 1.4 接入 `pre-merge.sh`（建议，本切片**未改** pre-merge.sh —— 属 A2 W1 范围）

在 `scripts/pre-merge.sh` 的 M5 段（紧邻 `check-core-boundary.py` 三处调用之后）补：

```sh
  python3 "$SCRIPT_DIR/check-mcp-policy.py" --self-test >/dev/null 2>&1 \
    || pm_fail "check-mcp-policy.py --self-test"
  python3 "$SCRIPT_DIR/check-mcp-policy.py" >/dev/null 2>&1 \
    || pm_fail "check-mcp-policy.py（M5-2 MCP 安全不变量被破坏）"
  python3 "$SCRIPT_DIR/check-mcp-policy.py" --expect-pending >/dev/null 2>&1 \
    || pm_fail "check-mcp-policy.py --expect-pending（MCP 产物已现，应翻 PENDING 为 ACTIVE 并由 W2 实现接管）"
```

接入时机：A0 把板子正式翻到 W2、且确认 M5-2 实现开始时（届时本脚本的 PENDING gate 已自然满足，
`--expect-pending` 会提醒「翻 ACTIVE」）。

---

## 2. 对既有交付的回溯修正（已落实）

- 本 Lane W1 delta §4 已确认 lib 名为 **`mvp_core`**（A2 实际提交 `src-tauri/Cargo.toml` `[lib] name="mvp_core"`）。
  本脚本的 gate 口径（core 边界、re-export shim）与此一致，无需改。
- A1 的 `M5-2` 卡截至本切片落盘**仍未修订**（仍含 `mcp_server_start/stop` 命令、未处理 B1/B7/B8）。
  本脚本的 PENDING 码位（`MCP_OPTIONAL_DEP` / `MCP_BIN_GATED` / `MCP_RUNTIME_LEAK` / `MCP_PATH_POLICY_MISSING` /
  `MCP_PARITY`）正是为修订后的实现守门——但 A1 卡本身不在本 Lane 修改范围。

---

## 3. 剩余阻塞项（仍需 A0 裁决，未因本切片改变）

| ID | 阻塞项 | 与本切片关系 |
|---|---|---|
| B1 | `rmcp`→`tokio` 与 F-1 冲突 | 本脚本 `MCP_OPTIONAL_DEP`/`MCP_BIN_GATED` 已把**推荐解**（optional+required-features）编码为机器守门；但「字面触碰 F-1」仍须 A0 书面选 (a) 豁免 / (b) 推迟到阶段二独立 package |
| B7 | stdio 独立进程 vs GUI 内 `mcp_server_start/stop` | 本脚本 `MCP_PARITY` 守「若保留 start/stop 则三处同现」；仍建议首期砍掉 start/stop |
| B8 | `read_file`/`list_dir` 无路径根策略 = 远程任意文件读 | 本脚本 `MCP_PATH_POLICY_MISSING` 守门；但白名单是否移出首期仍待 A0 决 |
| B3 | MCP 无 `Webview`，`db_query` 首行即 `check_invocation_source` | 本脚本 `MCP_TOOL_CALLS_COMMAND` 守「MCP 工具调 core 内部 API，不得调 `bridge::db_query`」 |

---

## 4. 输出模板回填（Lane Output Template）

```text
LANE=A3
STATUS=PASS_WITH_DOCS+GATE（脚本已落地并通过 self-test/default/expect-pending 三模式）
BASE=854bc40
HEAD=scripts/check-mcp-policy.py（新增）+ 本 checkpoint
FILES=scripts/check-mcp-policy.py
      logs/checkpoints/M5-A3-mcp-checkpoint-20260906-0900.md
VERIFY=python3 scripts/check-mcp-policy.py --self-test → MCP_POLICY_SELF_TEST=PASS（ACTIVE=4,PENDING=9）
      python3 scripts/check-mcp-policy.py → MCP_POLICY=PASS
      python3 scripts/check-mcp-policy.py --expect-pending → MCP_PENDING_RESULT=NONE
      python3 -m py_compile scripts/check-mcp-policy.py → OK
      git diff 产品代码/Cargo.toml/package.json 零改动（仅新增脚本+文档）
CHECKPOINT=logs/checkpoints/M5-A3-mcp-checkpoint-20260906-0900.md
MERGE_NOTES=①本切片是 M5-2.a「契约冻结/门禁先行」，零运行时代价，不触碰 W1 红线；
            ②板上暂无 M5-W2 章节、A3 仍标 SUPPORT DOCS ONLY，本 Lane 按用户 W2 指令推进最低风险切片；
            ③pre-merge.sh 接线建议见 §1.4（本切片未改，属 A2 W1 范围，待 A0 翻 W2 时接入）；
            ④B1/B7/B8/B3 仍待 A0 裁决；⑤A1 M5-2 卡仍待修订
NEXT=A0 翻板到 W2 并裁决 B1 → A3 据修订后的 M5-2 卡落地 M5-2.b（rmcp server bin + capability.rs + McpGlobalPolicy），
     届时本脚本 PENDING gate 满足、--expect-pending 提示翻 ACTIVE，门禁即生效
```
