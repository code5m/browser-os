# M5-2 内嵌 rmcp + McpGlobalPolicy + 首组 MCP 工具

> 子卡 ID：**M5-2** · 需求 #7（A2P/A2A）· `[S3|LEVERAGE:3|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A14**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L563（`M5-2 内嵌 rmcp 与全局护栏`）
> 主预研：`logs/assist/M5-7.a-prework-20260902-1055.md`（A2P/A2A 协议草案，std #7）
> 配套：`M5-1-core-workspace-split.md`（capability.rs 必先决位置）· `M5-5-agent-skill-commands.md`（共用 capability.rs）
>
> **W1** PASS（2026-09-06 13:00 CST · `M5-1.a` core boundary 已就位）· **W2** PASS（2026-09-06 13:55 CST · commit `a654f0c`，M5-2.a MCP 政策门就位）· **W3** PASS（2026-09-06 14:24 CST · commit `12f1cff`，A3 `mcp.rs` 冻结 DTOs + `MCP_COMMAND_REGISTRY` + `evaluate_mcp_policy` fail-closed + `MCP_FS_TOOL_PATH_POLICY` ACTIVE）· **W4** ACTIVE（A3 转入 SUPPORT/REVIEW ONLY；详见本卡顶部 `[W4 active]` 段）

---

## [W3 active · 2026-09-06 17:30 CST] 当前活跃 checkpoint 切到 M5-W3（A3 实施期承接 · 仅 frozen DTOs/pure registry/policy）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L133-167（**M5-W3 Parallel Dispatch**，Added 2026-09-06 17:10 CST by A0 after pushing through `712a14c`）+ L151（"A3 | **START PRODUCT CODE** | Implement the first M5-2 MCP command-registry/policy slice without adding `rmcp`: frozen DTOs/pure registry/policy checks only; no server runtime and no network listener."）。
> **事实摘要**：本卡 M5-2.a 政策门在 W2 dispatch（`a654f0c`）由 A3 拣入；W3 是 A3 实施期承接本卡余下切片（frozen DTOs/pure registry/policy checks only）的唯一窗口。
> **A1 W3 角色**：START DOCS ONLY — *"mark W1/W2 complete and make M5-W3 the active checkpoint"*。A1 在本卡 W3 仅加本顶部 `[W3 active]` 段 + 头部状态行；**不重写 §0~§X 决策史**。

### W3 A3 实施期硬约束（与 W3 dispatch 承诺一致 · A3 必守）

| # | 约束 | 来源 |
|---|------|------|
| W3-1 | **不增 `rmcp` 依赖** | PARALLEL_COMMAND_BOARD L151 + Hard Stops L165 |
| W3-2 | 仅 frozen DTOs / pure registry / policy checks | PARALLEL_COMMAND_BOARD L151 |
| W3-3 | **无 server runtime、无 network listener** | PARALLEL_COMMAND_BOARD L151 + Hard Stops L165 |
| W3-4 | **无新 Tauri 命令**（除非 ACL/source check 同包；A3 W3 默认不加）| PARALLEL_COMMAND_BOARD L151 + Hard Stops L166 |
| W3-5 | `check-mcp-policy.py` PASS（已落 `a654f0c`；新增 DTO/registry/policy 仍不得引入 rmcp/tokio/MCP 服务器/Plugin runtime/Agent runtime）| L165 + L151 Must Deliver |
| W3-6 | 无 npm 依赖 / 无 background listener | PARALLEL_COMMAND_BOARD L165 |
| W3-7 | Policy self-test + default PASS；pre-merge hook 完整 | PARALLEL_COMMAND_BOARD L151 Must Deliver |

### W3 A1 不修订范围

- **§0 编号与锚定 / §1 GOAL / §2 READ / §3 WRITE / §4 FORBID / §5 COMMANDS / §6 PASS_CRITERIA / §7 FAIL_ACTION / §8 DOC_BACKWRITE / §9 COMMIT**：A1 W3 **不动**（这些是 A3 实施期工作卡，A1 仅冻结交付状态）。
- **三份主文档**、ACL、Capability、pre-merge 全局：均 A1 硬停止。
- **`NEXT` 标记**：A1 在本段陈述"事实已变 / 待 A3 实施"，但**不修改 §9 NEXT 字面值**（A0 调度权）。

### W3 状态（本卡）

| 项 | 状态 |
|---|------|
| W1 core boundary gate（前置）| **PASS**（`854bc40` / `0d08016`）|
| W2 M5-2.a MCP 政策门拣入 | **PASS**（`a654f0c`）|
| W3 A3 实施（M5-2 余下切片 · frozen DTOs/pure registry/policy）| **ACTIVE · 待 A3 实施** |
| W3 A1 文档 reconciliation（本段 + 头部状态行）| **本卡 W3 修订已完成** |

---

## [W3 reconciliation · 2026-09-06 17:55 CST] A3 M5-2 余下切片已 A0 拣入（`12f1cff`）—— `mcp.rs` 冻结 + `MCP_FS_TOOL_PATH_POLICY` ACTIVE

> **W3 拣入事实**（`12f1cff` A0 拣入）：
> - **`src-tauri/src/mcp.rs`（首期切片，150+ 行新增）**：
>   - `pub const MCP_COMMAND_REGISTRY: &[McpCommandDef]` 冻结 7 命令（`file_read` / `file_list` / `tab_query` / `history_query` / `bookmarks_query` / `downloads_query` / `console_query`），每项含 `capability` / `core_api` / `touches_fs` / `returns_url`
>   - `pub fn lookup_mcp_command(capability: &str) -> Option<&'static McpCommandDef>`（fail-closed：未知能力返回 None）
>   - `pub fn evaluate_mcp_policy(...)` 裁决（fail-closed：默认拒绝；复用 `security_policy::{check_path_within_roots, redact_sensitive_url}`）
>   - **6 Rust 单测**（路径 root 校验、URL 脱敏、capability drift、未在白名单、touches_fs/returns_url 策略）
> - **`src-tauri/src/domain.rs`（新增 6 项冻结 DTO/常量）**：
>   - `pub const MCP_CAPABILITY_V1: &[&str]` —— 单一真源白名单（`MCP_CAPABILITY_DRIFT` 守门）
>   - `pub struct McpCommandDef` / `pub enum McpCommandKind` / `pub struct McpToolCall` / `pub struct McpPolicyDecision`
>   - `pub fn is_mcp_capability_allowed(name: &str) -> bool`
> - **`src-tauri/src/main.rs`（bin only）**：`mod mcp;` —— 落在二进制侧，**不**触发 `mvp_core` core boundary gate（与 W2 `a654f0c` 守门一致）
> - **`scripts/check-mcp-policy.py`**：加 `MCP_FS_TOOL_PATH_POLICY` ACTIVE 码位 + self-test 用例
> - **`scripts/pre-merge.sh`**：wire `check-mcp-policy.py`（self-test + default 双跑）
> - **A3 W3 守"无 rmcp / 无 server / 无 network listener / 无 npm / 无新 Tauri 命令 / 无 Agent 运行时"硬约束**——全部遵守（bin-only 注册表，server 集成留待 M5-2.b 后续 dispatch）

### W3 拣入对本卡 §1~§X 的影响

| §/项 | W2 状态 | W3 reconciliation |
|------|---------|-------------------|
| §1 GOAL（"首期切片 = 政策门 + 注册表"）| 写卡中 | **已 PASS**（`a654f0c` 政策门 + `12f1cff` 注册表）|
| §3 WRITE 候选文件（`mcp.rs` / `domain.rs` DTOs / `check-mcp-policy.py`）| 写卡中 | **已 PASS**（`12f1cff`）|
| §4 FORBID（无 rmcp / 无 server / 无 listener / 无 npm）| 写卡中 | **遵守**（A11 W3 verification 报 cargo fmt 干净 + cargo check 0 new warnings）|
| §6 COMMANDS（无新 Tauri 命令，除非 source check/ACL/前端同包）| 写卡中 | **遵守**（A3 W3 未注册新 Tauri 命令）|
| §7 PASS_CRITERIA（policy self-test + default PASS + pre-merge wire）| 写卡中 | **PASS**（A11 W3 verification 报 policy self-test+default+pending ALL_PASS）|
| §9 COMMIT / NEXT | 写卡中 | 字面 NEXT 仍写"待 A3 实施余下切片"——A1 W3 不改字面值 |

### W3 A1 不修订范围（本卡）

- **§1 GOAL / §2 READ / §3 WRITE / §4 FORBID / §5 COMMANDS / §6 PASS_CRITERIA / §7 FAIL_ACTION / §8 DOC_BACKWRITE / §9 COMMIT / §10 实施步骤 / §11 反向边清单 / §12 FORBID 遵守记录**：A1 W3 reconciliation **不动**（决策史保持 W2 原文；事实回填在本段）。
- **三份主文档 / ACL / Capability / pre-merge.sh**：均 A1 硬停止（pre-merge.sh 已由 A3 在 `12f1cff` 修订，A1 不再二次改）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。
- **本卡顶部 W3 状态行**已从 "ACTIVE" 改为 "PASS"。

---

## [W4 active · 2026-09-06 17:55 CST] 当前活跃 checkpoint 切到 M5-W4（A3 转入 SUPPORT/REVIEW ONLY）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L134-168（**M5-W4 Parallel Dispatch**，Added 2026-09-06 17:55 CST by A0 after pushing through `f8f1f49`）。
> **W4 事实摘要**：W3 整包已 A0 拣入；A3 在 W4 转入 *SUPPORT/REVIEW ONLY*（L152）—— *"Review A4/A5 against MCP registry/policy decisions; no MCP server/runtime expansion."*
> **A1 W4 角色**：A1 仅在头部加本 `[W4 active]` 段做"当前活跃 checkpoint"标记 + 提示 W4 实施期 A4/A5 可**消费**本卡已落地的 `MCP_CAPABILITY_V1` 真源 + `MCP_COMMAND_REGISTRY` + `evaluate_mcp_policy` 裁决。

### W4 状态（本卡）

| 项 | 状态 | 来源 |
|---|------|------|
| W1 core boundary gate（前置）| **PASS** | `854bc40` / `0d08016` |
| W2 M5-2.a MCP 政策门拣入 | **PASS** | `a654f0c` |
| W3 M5-2 余下切片（`mcp.rs` + DTOs + 注册表 + 策略）| **PASS · A0 拣入** | `12f1cff` + A11 W3 verification |
| W4 A3 复审 A4/A5 消费 MCP 真源/注册表 | **ACTIVE · 待 A3 输出** | PARALLEL_COMMAND_BOARD L152 |
| W4 A1 文档 reconciliation（本段 + 头部状态行）| **本卡 W4 修订已完成** | 本 checkpoint |

### W4 A1 不修订范围（本卡）

- **§1~§12 决策史**：A1 W4 **不动**（W3 已 PASS 且未引入新事实影响本卡决策）。
- **三份主文档 / ACL / Capability / pre-merge.sh**：A1 W4 不动。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。

---

## 0. 编号与锚定

- 批次任务号 `M5-2`；需求号 #7；WBS L563 一致。
- 依赖：M5-1 ✅（capability.rs 已落 core 或二进制） + A3 DB API 已落盘（`database.rs` 1272 行） + A4 `db_*` 命令 + `security_policy.rs` SQL 分类器。

---

## 1. GOAL

内嵌纯 Rust `rmcp`（**禁 npm 分包**），实现 `McpGlobalPolicy` 全局护栏（read-only / 危险 SQL 闸门 / 连接白名单 / 能力白名单 / 确认闸门），首期只暴露**只读 + 低危** MCP 能力（artifact / audit / repo / workspace / tab / db.query 的受限子集），**不暴露任何执行/写盘/出网能力**。建立 `src-tauri/src/capability.rs` 公共白名单（A2P / A2A / Skill / Plugin / Agent 五处共用，防漂移）。

---

## 2. READ

1. `logs/assist/M5-7.a-prework-20260902-1055.md`（**全读**，A2P/A2A 协议草案；§4 capability/§4.4 policy/§4.5 错误码/§5 风险/§6 反向用例）
2. `src-tauri/src/bridge.rs:688-790`（两段式确认闸门范式，**协议层必须复用**）
3. `src-tauri/src/workspace.rs:89-110`（审计范式 + 1000 上限）
4. `src-tauri/src/security_policy.rs`（2041 行，**全读**；M4-3 数据库 SQL 分类器复用点）
5. `src-tauri/permissions/default-commands.toml`（尾条恒为 `list_artifact_images`，MCP 新命令插其前）
6. `src-tauri/permissions/remote-collect.toml`（远程 webview 仅回传类，MCP **不走 remote**）
7. `src-tauri/Cargo.toml`（依赖基线 + 现有 `tauri`/`tokio`/其他）
8. `M5-1-core-workspace-split.md` §3（capability.rs 位置）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/Cargo.toml` | 修改 | 加 `rmcp = { version = "...", default-features = false }`（纯 Rust，**不** 引 `npx` 任何东西） |
| `src-tauri/src/mcp_server.rs` | **新增** | `rmcp` server 骨架 + 传输（stdio）+ 能力注册 + `McpGlobalPolicy` 引用 |
| `src-tauri/src/mcp_policy.rs` | **新增** | `McpGlobalPolicy` 解析 + 判定顺序（§4 草案 6 步） |
| `src-tauri/src/mcp_tools/` | **新增目录** | 首组工具（db_query / artifact_list / audit_list / repo_list / workspace_list_dir / workspace_read_file / tab_list，全部**只读**） |
| `src-tauri/src/capability.rs` | **新增** | 公共白名单（M5-1 决位置后）；A2P / A2A / Skill / Plugin / Agent **共用** |
| `src-tauri/src/domain.rs` | 新增类型 | `CapabilityManifest` / `Capability` / `RiskLevel` / `McpPolicy` / `McpAuditRecord` |
| `src-tauri/src/bridge.rs` | 修改 | `mcp_server_start` / `mcp_server_stop` / `mcp_policy_get/set` 命令（**全进 ACL**） |
| `src-tauri/src/main.rs` | 修改 | 加 `--mcp-server` argv 分支（仿 `--grid-child` 模式） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 `mcp_server_start/stop` + `mcp_policy_get/set` 于 `list_artifact_images` 之前 |
| `src-tauri/src/shutdown.rs` | 修改 | 注册 `mcp-server-shutdown` 协调任务（先于 `kill-running-scripts`，与 `stop-scheduler` 序一致） |
| `src-tauri/src/mcp_calls.json` | 持久化 | 独立调用日志（防 1000 上限被刷爆），**详细字段见 §6** |
| `scripts/check-mcp-policy.py` | **新增** | 仿 `check-database-policy.py`；自检 PASS；挂 `pre-merge.sh` |
| `src/types.ts` `src/bridge.ts` | 新增类型 | TS 镜像 `Capability`/`McpPolicy`（snake_case） |

---

## 4. 关键契约（草案锁死，实施期严格照此）

### 4.1 传输：stdio only（首期）

- 启动：外部 Agent 以 `--mcp-server` 启动本应用；stdin/stdout 走 NDJSON / JSON-RPC 2.0
- **禁 TCP 端口**（避免局域网暴露；R2 极高）
- 单进程串行处理；并发留待稳定
- 单请求 ≤ 1 MB；单响应 ≤ 5 MB；超时 30s
- 与 M3-1.a 进程回收一致：协议进程退出即回收，无残留

### 4.2 能力清单（首期白名单 + 黑名单）

**白名单**（`capability.rs` 常量 `MCP_CAPABILITY_V1`）：

| 能力 | maps_to | risk | requires_confirm |
|---|---|---|---|
| `artifact.list` | `list_artifacts` | Read | 否 |
| `artifact.read` | `read_artifact` | Read | 否 |
| `audit.list` | `audit_log` | Read | 否 |
| `repo.list` | `list_repos` | Read | 否 |
| `workspace.list_dir` | `list_dir` | Read | 否 |
| `workspace.read_file` | `read_file` | Read | 否 |
| `tab.list` | `tab_list` | Read | 否 |
| `db.query` | `db_query` | Read | 否（受 §4.3 策略限制） |

**黑名单（首期明确不暴露，无论配置如何）**：

`write_file` / `create_file` / `delete_path` / `rename_path` / `launch_app` / `open_browser` / `request_sync` / `confirm_sync` / `term_write` / `term_spawn` / `script_*` / `run_script` / `db_*` 写路径 / `clipboard_*`

> **理由**：这些能力一旦暴露，"外部 Agent 提示词注入"可直接转化为本地任意操作。**M5 首期不开放执行类能力。**

### 4.3 `McpGlobalPolicy` 护栏（fail-closed）

```rust
pub struct McpGlobalPolicy {
    pub read_only: bool,                // 默认 true（全局只读，risk > Read 一律拒绝）
    pub allow_dangerous_sql: bool,      // 默认 false；与 A4 写闸门 `allow_write` 需同时为 true
    pub allowed_connection_ids: Vec<String>, // 默认空 = 不允许任何 db 能力
    pub allowed_capabilities: Vec<String>,  // 默认 = §4.2 白名单
    pub result_limit: usize,            // 默认 50（对齐 dbx EXECUTE_QUERY_LIMIT）
    pub auto_confirm: bool,             // 默认 false = 必须人工确认
}
```

**判定顺序（任一失败即拒绝，不短路跳过后续）**：

1. 能力在 `allowed_capabilities`？否 → `CAPABILITY_NOT_ALLOWED`
2. `read_only` 且 `risk > Read`？是 → `READ_ONLY_MODE`
3. 需 `requires_policy` 且该策略未开？是 → `POLICY_DISABLED`
4. 是 `db.query`？`allow_dangerous_sql` 与 `allowed_connection_ids` 双重校验
5. `requires_confirm` 且 `auto_confirm=false`？走两段式确认闸门（对齐 `request_sync`/`confirm_sync`）
6. 通过 → 执行 → 写 `mcp_calls.json` + 摘要进 `audit.json` → 按 `result_limit` 截断

### 4.4 错误码

| 码 | 含义 | 客户端动作 |
|---|---|---|
| `-32700` | JSON 解析失败 | 修正格式 |
| `-32601` | 未知 method | 查 manifest |
| `-32001` | `FORBIDDEN` | 不要重试（配置问题） |
| `-32002` | `CAPABILITY_NOT_ALLOWED` | 同上 |
| `-32003` | `READ_ONLY_MODE` | 同上 |
| `-32004` | `CONFIRM_REQUIRED` | 提示用户去 UI 确认 |
| `-32005` | `RESULT_TRUNCATED` | 收窄查询 |
| `-32006` | `RATE_LIMITED` | 退避重试 |
| `-32007` | `INTERNAL_ERROR` | 上报附 `run_id` 查审计 |

### 4.5 审计契约

- **每次调用**写 `mcp_calls.json`（独立文件，500 条上限 FIFO）
- 摘要进 `audit.json`（`action=mcp_call`）
- `detail` 含 `capability` / `requester` / `policy_snapshot` / `result`（success/denied/error）/ `run_id`
- **禁**记抽取文本正文 / 凭据 / DSN / SQL 正文（K3）

---

## 5. FORBID

- **不**引 `rmcp` 之外的任何 MCP 库（**禁 npm 分包**）
- **不**开 TCP 端口（首期仅 stdio）
- **不**暴露任何执行类能力（白名单外一律黑名单）
- **不**让 `capability.rs` 与各 Lane 私写白名单漂移（5 处共用一份）
- **不**破 K1（ACL 末条恒为 `list_artifact_images`）/K3（凭据）/K5（审计）
- **不**让 `mcp-server-shutdown` 改 `stop-scheduler` 索引（与 A7 冻结冲突）
- **不**改 `audit.json` 1000 上限；高频走 `mcp_calls.json`
- **不**移动 `NEXT`
- **不** push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
grep -rniE "rmcp|\bmcp\b" src-tauri/src src | grep -v user_agent    # 实施前为 0 命中
grep -c "tauri::command" src-tauri/src/bridge.rs                       # 基线 59+MCP 新增

# B. 政策脚本自检
python3 scripts/check-mcp-policy.py --self-test                       # PASS

# C. 能力黑名单校验（实施后跑）
for cap in write_file create_file delete_path run_script term_write db_connect; do
  printf '%s\n' "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$cap\",\"params\":{}}" \
    | ./target/release/mvp-browser-os --mcp-server 2>&1 | head -1
  # 期望含 CAPABILITY_NOT_ALLOWED
done

# D. 无监听端口
ss -lntp 2>/dev/null | grep -i mvp || echo OK                          # 应输出 OK

# E. 端到端冒烟
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"artifact.list","params":{}}' \
  | ./target/release/mvp-browser-os --mcp-server 2>&1 | head -5

# F. 协议进程退出无残留
pgrep -fa mvp-browser-os | grep mcp-server
# 退出协议 stdin EOF 后 5s 内应自动退出

# G. 编译与基线
cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
cargo clippy --all-targets 2>&1 | grep -c warning                       # 不得 > 13 + 本卡新增

# H. 门禁
bash scripts/pre-merge.sh                                               # ALL_PASS
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | `rmcp` 纯 Rust 嵌入，`Cargo.toml` 体现 | 命令 A 命中 `rmcp`（限 src-tauri） |
| 2 | `mcp_server_start/stop` + `mcp_policy_get/set` 进 ACL，**末条仍为 `list_artifact_images`** | 命令 B 0 违规 |
| 3 | `capability.rs` 落定（M5-1 决位置），白名单常量只此一份 | `grep -n "MCP_CAPABILITY_V1" src-tauri/src/capability.rs` 唯一命中 |
| 4 | `McpGlobalPolicy` 默认 `read_only=true` / `auto_confirm=false` | 单测 |
| 5 | 黑名单 10+ 能力经 `capability.rs` 拒（命令 C 全 `CAPABILITY_NOT_ALLOWED`） | 命令 C |
| 6 | `db.query` 受 `allow_dangerous_sql` + `allowed_connection_ids` 双重校验 | 单测：N5/N6 |
| 7 | `requires_confirm` 能力走两段式确认闸门；未确认前不执行 | 单测：N7 |
| 8 | `mcp_calls.json` 独立上限 500；`audit.json` 仅记摘要 | 命令 B 0 违规 + 单测 |
| 9 | `cargo test` 全绿（含 `mcp_*` 单测） | 命令 G |
| 10 | `pre-merge.sh` ALL_PASS | 命令 H |
| 11 | `--mcp-server` 启动无 TCP 监听（命令 D） | 命令 D |
| 12 | `mcp-server-shutdown` 注册在 `stop-scheduler` 之后（与 A7 冻结一致） | 单测索引断言 |
| 13 | 错误响应不泄露能力清单存在性（防探测，N13） | 单测：未授权方调未知能力返回 `-32001 FORBIDDEN` 而非 `-32601` |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| 出现 TCP 监听 | **红线失守**：回到 stdio-only；未关闭前不发布 |
| 能力白名单在两处定义 | 阻断：必须收口到 `capability.rs` |
| 能力被自动执行而绕过确认闸门（N7） | 阻断：确认闸门是协议层红线 |
| Key/token 出现在日志/审计/响应 | 立即修 + 清泄露条目 + 提示用户轮换 |
| `mcp-server-shutdown` 改 `stop-scheduler` 索引 | 阻塞：与 A7 冻结冲突，先解决协调器序 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

实施期（不在 A1 W0 范围）需回写：

1. `详细设计与实施计划.md` L563 `[ ]` → `[x]`，补 rmcp 版本与 capability.rs 位置
2. `后续需求TODO.md` §7 状态 `PARTIAL` → `DONE: MCP/rmcp`；留 `A2A` 给 M5-3
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-3`
4. `logs/checkpoints/M5-2.a-2026MMDD-HHMM.md`（实施卡 checkpoint）
5. `M5-14-debt-ledger.md` 增项：`withGlobalTauri` 全局暴露（仅影响 M5-10，本卡不涉及）

---

## 10. COMMIT / NEXT

- **COMMIT**：本卡文档无 commit；A0 拣入后由 A14 实施填
- **NEXT**：
  - 成功 → `M5-3`（A2A + agent_kv）
  - 成功 → 与 `M5-4`（Agent/Skill runtime）并行（capability.rs 共用）

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
- 与 A9 既有契约文档无文件冲突
