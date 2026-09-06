# A10 · M5-W3 安全复审（A2 / A3 W3 产出）

> Lane: A10（M4/M5 独立安全复审）
> Dispatch 依据：`PARALLEL_COMMAND_BOARD.md` → "M5-W3 Parallel Dispatch" → A10 = START REVIEW
> 复审对象（W3 真实产品代码）：
>   - A2 W3：`712a14c`（常量收口，已提交）+ 工作树暂存 seam 注入（`core/seam.rs` + `core/mod.rs` + `bridge.rs` Tauri 实现）
>   - A3 W3：`a654f0c`（MCP 策略门，已提交）+ `12f1cff`（MCP 命令注册表 + 全局策略 + DTO + pre-merge 接线，已提交）
> BASE（复审起点）：本地 master 头 `12f1cff`，与 `origin/master` 差 3 个本地提交未 push
> 复审时间：2026-09-06 17:40 CST
> 交付物：本文档（仅 docs，未改动任何产品代码，未 push）
> 说明：本文件**取代**同目录 1730 初稿——初稿仅在 `12f1cff` 落库前看到 `a654f0c`/`712a14c`，结论关于 D1 "未接线" 已过时；现据完整 W3 范围重写。

## 0. Lane Output Template（机器可读结论）

```text
LANE=A10
STATUS=PASS
BASE=12f1cff
HEAD=logs/assist/A10-M5-W3-security-review-20260906-1730.md（docs only）
FILES=logs/assist/A10-M5-W3-security-review-20260906-1730.md
VERIFY=cargo test bin 335/329 基线通过(+6 mcp) + mvp_core lib 2 通过(seam)；check-core-boundary --self-test PASS(ACTIVE=7,core 文件=3)；check-mcp-policy --self-test PASS(ACTIVE=5/PENDING=9)；默认 PASS；--expect-pending=NONE；Cargo 无 rmcp/tokio/npm；core 无 tauri/bridge；A2/A3 均无新 Tauri 命令
CHECKPOINT=logs/assist/A10-M5-W3-security-review-20260906-1730.md
MERGE_NOTES=A2 W3=PASS（常量收口+seam 纯 trait 注入，零行为变更，无 tauri 入 core，无新命令）；A3 W3=PASS（注册表 fail-closed、能力白名单单一真源在 domain.rs、复用 check_path_within_roots/redact_sensitive_url、无 rmcp/tokio/网络、无新命令、pre-merge 已接线 D1 解决）
NEXT=A1/A0 确认 M5-2 §3 白名单真源取 domain.rs(MCP_CAPABILITY_V1) 而非 capability.rs(U-4 仍缺)；A3 M5-2.b 须保证 MCP 工具走 core 内部 API 而非 bridge::*；A2 切片2 消费 seam 时改掉 base_dir "." 兜底与 Progress.detail 前端事件脱敏
```

## 1. 复审范围与方法

按 W3 调度，A10 复审 A2 / A3 的 W3 产出，聚焦 5 类：
**边界泄漏 / 重复执行路径 / ACL·来源校验漂移 / 敏感日志 / 策略误否**。

实证方法（非仅凭读码）：实跑 `cargo test`、`check-core-boundary.py`、`check-mcp-policy.py` 三模式；`git show`/`git diff HEAD` 核对 A2/A3 净差；grep `core/` 是否引入 tauri/bridge；grep `Cargo.toml` 依赖；grep `main.rs`/`.invoke_handler` 是否新增 Tauri 命令。

## 2. 实证自检结果（机器证据）

| 检查项 | 命令/来源 | 结果 |
|---|---|---|
| 行为回归（bin） | `cargo test --manifest-path src-tauri/Cargo.toml` | **335 passed; 0 failed**（M4 基线 329；A3 `mcp.rs` +6 测试）|
| 行为回归（lib） | `cargo test … mvp_core`（core::） | **2 passed**（A2 seam 测试），0 failed |
| core 边界门 | `scripts/check-core-boundary.py --self-test` | **PASS（ACTIVE=7，core 文件=3：keyring_store/mod/seam）** |
| core 边界默认 | `scripts/check-core-boundary.py` | **all invariants hold** |
| MCP 策略门自测 | `scripts/check-mcp-policy.py --self-test` | **PASS（ACTIVE=5，PENDING=9）** |
| MCP 默认扫描 | `scripts/check-mcp-policy.py` | **PASS（无违规）** |
| MCP pending 守门 | `scripts/check-mcp-policy.py --expect-pending` | **NONE（9 个 pending 码位均未实现，W1 守门通过）** |
| 依赖纯洁 | `grep -nE 'rmcp\|^tokio\|tokio =' src-tauri/Cargo.toml` | **无 rmcp / tokio** |
| MCP 无网络/tokio | `grep -nE 'TcpListener\|\.bind\(|axum|hyper::Server|tokio::net|rmcp' src-tauri/src/` | **无命中（mcp.rs 注释外零出现）** |
| core 未漏 tauri | `grep -rnE 'use tauri\|crate::bridge\|AppHandle' src-tauri/src/core/` | **NO**（seam.rs 零 Tauri 依赖）|
| 无新 Tauri 命令 | `grep -rnE 'fn mcp_[a-z_]+\(' src-tauri/src/` 与 `main.rs` invoke_handler 净差 | **无**（A3 仅 `mod mcp;`；`mcp_server_start` 仅是拒绝测试串）|
| pre-merge 接线 | `grep 'check-mcp-policy' scripts/pre-merge.sh` | **已接线**（416-419 三段 + 534-536 守卫）→ D1 解决 |

## 3. A2 W3 — 结论 PASS

W3 任务："trait/seam 注入 only，no behavior change"。实际交付分两块：

### 3.1 常量收口 `712a14c`（已提交）
- `domain.rs` 增权威常量 `MAX_TIMEOUT_SECS/HARD_GRACE_SECS/MAX_TEXT_FIELD_BYTES`；`database.rs`/`script_runner.rs`/`security_policy.rs` 改 `pub use crate::domain::*`（值逐字不变，DB 侧补 `as u64` 转型）。
- 跨模块对齐单测 `t_db_c6` 改为「字面值锁定 + 契约别名同值」双保险，仍防 A3/A4 改值漂移。
- 实证：`cargo test` 329→329 零回归（该提交本身）；边界门未破。

### 3.2 seam 注入（工作树暂存，未提交）
- `src-tauri/src/core/seam.rs`（新，95 行）：3 个纯 trait——`ProgressSink`/`PathResolver`/`RootsProvider`，**零 Tauri 依赖、零行为**，2 个测试（NoopSink/FixedResolver mock）。
- `src-tauri/src/core/mod.rs`：再导出 seam 模块。
- `src-tauri/src/bridge.rs`（+44）：`TauriProgressSink`/`TauriPathResolver`/`TauriRootsProvider` 在 `AppHandle` 上的具体实现，**`#[allow(dead_code)]`**（切片1仅声明，切片2才注入消费）。标准 Tauri 写法（`app.emit`/`app.path().data_dir()`/`allowed_roots`）。
- 实证：`check-core-boundary.py` 现报 core 文件=3 且 PASS（seam 未破边界）；`cargo test` lib 2 测试通过；`main.rs` 无新 `.invoke_handler` 项 → **未新增 Tauri 命令**。

**5 类核对（A2 两块合并）：**
- 边界泄漏：core/ 无 tauri/bridge（grep + 边界门双证）；常量收口未触碰 core/。
- 重复执行路径：seam 为纯抽象；bridge 实现只调既有 `allowed_roots`/`emit`/`data_dir`，无 `std::process::Command`、无第二执行路径。
- ACL·来源校验漂移：W3 未新增任何 Tauri 命令 → 无漂移。
- 敏感日志：无新增日志；`Progress.detail` 经 `script:progress` 事件上送，属进度相位描述，非凭据。
- 策略误否：无新策略码位。

**A2 切片2 待办（watch，非 W3 阻塞）：**
- `TauriPathResolver::base_dir` 在 `data_dir()` 错误时 `unwrap_or_else(|| PathBuf::from("."))` 兜底到 cwd——切片2 消费时应改抛错或落到安全固定根，避免越出工作区。
- `Progress.detail: Option<String>` 进 `script:progress` 前端事件，切片2 须确认 detail 不夹带 SQL/路径/凭据。

## 4. A3 W3 — 结论 PASS

W3 任务："MCP 命令注册表 / 全局策略切片，无 rmcp / 无 server / 无网络监听；新命令须含 source-check+ACL+bridge/types+策略覆盖"。实际交付分两块：

### 4.1 策略门 `a654f0c`（已提交）
- 新增 `scripts/check-mcp-policy.py`（504 行）。自测原 FAIL（`MCP_RUNTIME_LEAK` 误触）已被本提交修复 → 现 ACTIVE=5/PENDING=9、默认 PASS。

### 4.2 注册表 + 全局策略 + DTO + pre-merge 接线 `12f1cff`（已提交）
- `src-tauri/src/mcp.rs`（新，193 行）：**纯冻结注册表** `MCP_COMMAND_REGISTRY`（6 项：file_read/file_list/tab_query/history_query/bookmarks_query/downloads_query/console_query，各标 `touches_fs`/`returns_url`）+ **fail-closed 全局策略** `evaluate_mcp_command`（未知能力→`Deny`；`touches_fs` 必须经 `check_path_within_roots`）+ `redact_mcp_url` 复用 `redact_sensitive_url` + `current_policy_snapshot`。6 个 Rust 测试。
- `src-tauri/src/domain.rs`（+63）：`MCP_CAPABILITY_V1` 能力白名单（**单一真源**，7 项）、`McpCommandDef`/`McpDecision`/`is_known_mcp_capability`/`McpPolicySnapshot`。全部 `#[allow(dead_code)]`（契约层，由 bin 侧 `mcp.rs` 消费）。
- `src-tauri/src/main.rs`（+2）：`mod mcp;`（bin 专属，未进 `mvp_core` lib 边界）。
- `scripts/check-mcp-policy.py`（+16）：新增 `MCP_FS_TOOL_PATH_POLICY` ACTIVE 码位 + 自测样本 → ACTIVE 升 5。
- `scripts/pre-merge.sh`（+14）：**已接线** `check-mcp-policy.py`（416-419 三段：self-test/default/`--expect-pending` 守门；534-536 缺失文件+自测守卫）。

**5 类核对（A3 两块合并）：**
- 边界泄漏：无 `rmcp`/`tokio`/`TcpListener`/`npm`/`网络监听`（grep 零命中）；`mcp.rs` 模块注释明示守 W3 硬停止；`MCP_RUNTIME_LEAK` 为 PENDING（见 §5）。
- 重复执行路径：`mcp.rs` 不 spawn 进程、不起监听、不调 `bridge::*`；`core_api` 字段为**文档性标注**（首期无 server，不真调用），从设计上杜绝 MCP 工具绕过来源校验（B3）。
- ACL·来源校验漂移：**未新增 Tauri 命令**（`mcp_server_start` 仅为拒绝测试串；`main.rs` 仅 `mod mcp;` 无新 invoke 项）→ W3 硬停止"无新 Tauri 命令"守住，故 ACL/source-check 不适用。
- 敏感日志：`mcp.rs` 不记录路径/URL/token；URL 回传强制走 `redact_sensitive_url`（测试 `url_is_redacted_before_return` 守 `u:p@`/`token` 脱敏）。
- 策略误否：默认拒绝（fail-closed）；`evaluate_mcp_command("exec", …)`→Deny（测试守）；越路径根 `file_read /etc/passwd`→Deny（测试守）；注册表项⊆白名单且双向覆盖（测试守）。无已知误否/误报。

**→ 我 W2 复审两项 finding 全部关闭：**
- W2 #1（`check-mcp-policy --self-test` FAIL）→ 已解决（a654f0c）。
- W2 D1（未挂 pre-merge）→ 已解决（12f1cff 接线）。

## 5. 残留 / 后续切片 Watch-list（非 W3 阻塞）

1. **[W3 已解] D1**：pre-merge 接线完成（12f1cff）。✔
2. **[RECONCILE] M5-2 §3 白名单真源**：A3 把能力白名单单一真源落在 `domain.rs::MCP_CAPABILITY_V1`，而 M5-2 主篇 §3 / A4 卡 U-4 原设想在 `capability.rs`（`capability.rs` 至今仍 ABSENT）。`MCP_CAPABILITY_DRIFT` 守门确保 `domain.rs` 唯一；A1/A0 须确认 M5-2 采用 domain.rs 方案（则 U-4 对 M5-2 实际已闭合），避免与未来 M5-3 A2A 的 `capability.rs` 冲突。
3. **[W3 已解] D5**：`MCP_RUNTIME_LEAK` 仍 PENDING（9 之一），因 B1(a) 未决（核心采用 `optional=["mcp"]`+`required-features=["mcp"]` 时 tokio 仅现于 mcp 二进制）。现状零 tokio，风险 0；一旦 A0 决 B1(a) 并落 mcp 运行时，须升 `MCP_RUNTIME_LEAK`+`CORE_DEP_NOT_ALLOWLISTED` 为 ACTIVE。
4. **[W3 已解] 网络红线**：`MCP_LISTEN_PORT`（ACTIVE）守 TcpListener/bind/axum/hyper/tokio::net；当前零命中。✔
5. **[forward] A3 M5-2.b**：实现 stdio server / rmcp handler 时，须保证工具走 `core_api` 标注的**核心内部 API**，严禁 `bridge::*`；并实时经 `evaluate_mcp_command` + `redact_mcp_url`。
6. **[forward] A2 切片2**：消费 seam 时处理 §3.2 两处（base_dir "." 兜底、Progress.detail 前端事件脱敏）。
7. **[沿用 W2] B8/B9/D6/D7/B7**：路径根读取、URL 脱敏、工具结果字符封顶、跨进程句柄零拷贝、审计不落 token——A3 落运行时时逐条覆盖（B8/B9 已复用 `security_policy` 单一真源，✔）。

## 6. 对既有 A10 复审的更正/延续

- W2 #1 / D1 → **均关闭**（见 §4）。
- W2 D5（`MCP_RUNTIME_LEAK` PENDING）→ 仍为文档化残留（§5-3），随 B1(a) 决策升级。
- W1（A2 边界门 `854bc40`）由 A11 验 PASS；本次 A2 seam 未破该门（实证 §2）。

## 7. 声明

- 本文档为 A10 W3 复审交付，**未改动任何产品代码 / 未改 `pre-merge.sh` / 未 push**。
- 工作树其余未提交物（A1/A4/A5/A7/A8/A9 W3 文档与 patch、A2 seam 暂存）属他 lane，未触碰。
- 本地 master 头 `12f1cff`，领先 `origin/master` 3 个本地提交（A0 集成他 lane，未 push）。
