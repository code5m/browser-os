# Lane A4 · M5-W10 Privacy Review — Baseline + A3 stdio-prep 落点后审查框架

> LANE=A4　WAVE=M5-W10 Controlled Runtime Prep Dispatch（board `PARALLEL_COMMAND_BOARD.md` 行 195）
> Status=**START PRIVACY REVIEW ONLY**（行 195 + 196 A4 角色：可提 policy 夹具但**不编辑产品代码**）
> A3 W10 当前进度：**尚未交付**（无 `bin/mcp_server.rs`、无 `mcp_tools/`、无 `[features] mcp-server`、Cargo.toml 仍 0 feature/0 rmcp/0 tokio）。本笔记为 A3 落点前的 **baseline 快照 + 落点后审查框架**，A3 提交后由 A4 二次复核。
> Base=`3792115`（A0 W9 整合）　HEAD=本地未推送
> 交付：`logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md`（仅文档，无产品代码）

---

## 0. 调度自检

| 项 | 实测 | 结论 |
|---|---|---|
| WORKSPACE_IDENTITY / pwd / branch | BACKV3_MAIN / 匹配 / master | ✅ |
| `git fetch && pull --ff-only` | HEAD=3792115（已是最新） | ✅ |
| 仍属 A4 可写范围 | ✅ 仅产出 review note；未触碰任何产品代码（**含 `src/utils/graphUi.ts` —— A8 lane 文件，不动**） | ✅ |
| 工作树 dirty | A8 staged 改 `src/utils/graphUi.ts`（已存在 working tree，未 commit）；A9 创建 0 byte 空文件 `A9-M5-W10-plugin-runtime-dispatch-card-20260907-1730.md`（违规 A4 不处理）；A4 仅新增本笔记（`??` 未跟踪）| A4 不碰非本 lane 文件 |
| W10 Hard Stop 合规 | 未引入 rmcp/server/listener/network；A3 是唯一 MCP prep writer | ✅ |
| 是否触碰 `domain.rs` / 产品 Rust / TS / ACL / mcp.rs | 否（A4 严格 REVIEW ONLY） | ✅ |

## 1. W10 baseline 现状（A0 W9 整合后，HEAD=3792115）

### 1.1 政策门禁（W10 baseline 全绿）

| 检查 | 结果 |
|---|---|
| `check-agent-memory-policy.py --self-test` | AGENT_KV_POLICY_SELF_TEST=PASS（ACTIVE=5） |
| `check-graph-policy.py --self-test` | GRAPH_POLICY_SELF_TEST=PASS（ACTIVE=7） |
| `check-plugin-policy.py --self-test` | ACTIVE=1 PENDING=5 |
| `check-agent-skill-policy.py --self-test` | AGENT_SKILL_POLICY_SELF_TEST=PASS（ACTIVE=3，PENDING=5，含 AGSK_CREDENTIAL_NOT_ECHOED） |
| `check-mcp-policy.py --self-test` | MCP_POLICY_SELF_TEST=PASS（ACTIVE=8，PENDING=0） |
| `check-mcp-policy.py --expect-current-gaps` | MCP_CURRENT_GAPS_RESULT=PASS（W8 相位：只读桥已落地，无 rmcp/server/listener/tokio，奇偶/只读/红线性门禁全绿） |
| `check-graph-ui-logic.mjs` | 通过 43，失败 0 |
| `check-agent-skill-ui-logic.mjs` | 99 assertions passed, 0 failed |

### 1.2 Cargo.toml 现状（A3 W10 预期落点）

```toml
# 当前 src-tauri/Cargo.toml 第 20-52 行
[dependencies]
tauri = { version = "2", features = ["unstable", "protocol-asset"] }
# ... 既有依赖 ...
# M4-2：sync DB 客户端（无 async 框架）
rusqlite = { version = "0.40.2", features = ["bundled"] }
mysql = "28.0.2"
postgres = "0.19.14"

[profile.release]
# 无 [features] 段，无 [[bin]] 段
```

→ **A3 W10 须新增**：
- `[features] mcp-server = ["dep:rmcp", "dep:tokio"]`（rmcp + tokio 必须 optional + feature-gated）
- `rmcp = { version = "...", optional = true }`
- `tokio = { version = "...", optional = true }`
- `[[bin]] mcp_server`，`required-features = ["mcp-server"]`
- 默认 `cargo build` / `cargo tauri build` 不编译 mcp_server

### 1.3 8 ACTIVE 码位（A3 W10 须全部仍绿）

| 码位 | 守门 |
|---|---|
| MCP_NPM_SDK_PRESENT | 禁 `@modelcontextprotocol/sdk`/`mcp-client` 等 npm MCP 客户端 |
| MCP_NPM_IN_CARGO | 禁 `npm:xxx` spec 出现在 Cargo.toml（避免双包管理污染） |
| MCP_NODE_RUNTIME_PRESENT | 禁 node-runtime 形态 |
| MCP_CAPABILITY_DRIFT | capability 白名单单源（`domain.rs::MCP_CAPABILITY_V1` + `mcp.rs::MCP_CAPABILITY_DEFS`） |
| MCP_FS_TOOL_PATH_POLICY | `touches_fs`/`returns_url` 项必须复用路径根 + URL 脱敏 |
| MCP_BRIDGE_READONLY | 禁写副作用（fs write / exec / network / model call） |
| MCP_NO_RMCP_SERVER | **当前永久 forbidden**——`rmcp`/`tokio`/`mcp_server` bin/`mcp_tools/`/`axum`/`hyper::Server`/`TcpListener` 全部禁止；A3 W10 须按 §2 翻转条件升级为 `MCP_SERVER_GATED` |
| MCP_PARITY | 桥奇偶（bridge/main/ACL/bridge.ts） |

### 1.4 A8 W10 working-tree 改动（仅观察，不审 A8 patch——A8 自行合并）

`git status --porcelain` 显示 `M src/utils/graphUi.ts`（A8 W10 staged，未 commit）。**A4 不审 A8 patch**（不属 A4 范围），但 A8 改动**直接扩大 R-W9-3（F1 DRY 常量化）缺口**——这是 A4 W10 必须记录的具体事实。

**A8 新增的 RENDER_* 常量**（`src/utils/graphUi.ts:237-238`）：
```typescript
export const RENDER_NODE_CAP = 5000;  // = GRAPH_MAX_NODES
export const RENDER_EDGE_CAP = 20000; // = GRAPH_MAX_EDGES
```

**R-W9-3 GRAPH_MAX_NODES/MAX_EDGES 现已扩散到 4 个源**（A4 W9 时是 3 源）：

| # | 位置 | 名称 | 值 | 类型 |
|---|---|---|---|---|
| 1（真源）| `src-tauri/src/domain.rs:2114` | `GRAPH_MAX_NODES` | `5_000` | Rust const |
| 1（真源）| `src-tauri/src/domain.rs:2116` | `GRAPH_MAX_EDGES` | `20_000` | Rust const |
| 2（W7 已有）| `src/utils/graphUi.ts:26` | `GRAPH_MAX_NODES` | `5000` | TS const |
| 2（W7 已有）| `src/utils/graphUi.ts:27` | `GRAPH_MAX_EDGES` | `20000` | TS const |
| 3（W7 已有）| `src/stores/useGraphStore.ts:28` | `MAX_NODES` | `5000` | TS const |
| 3（W7 已有）| `src/stores/useGraphStore.ts:29` | `MAX_EDGES` | `20000` | TS const |
| **4（A8 W10 新增）** | `src/utils/graphUi.ts:237` | `RENDER_NODE_CAP` | `5000` | TS const |
| **4（A8 W10 新增）** | `src/utils/graphUi.ts:238` | `RENDER_EDGE_CAP` | `20000` | TS const |

**A4 W10 评估**：
- 数值**当前一致**（5_000 / 20_000）。A8 在 `RENDER_NODE_CAP` 旁注释 `// = GRAPH_MAX_NODES`，A8 自觉对齐真源——这是好的意识，但**仍是 4 源硬编码**。
- **与 A4 隐私评审的相关性**：RENDER_* 用于 `clampRender` 截断 items 数组（`graphUi.ts:240-246`），A8 写到"正常数据不会触发截断，仅作为单点护栏"——这是 UI 防泄漏守门（避免超大节点/边 prop 一次性渲染时 DOM 内部消化可能的内容回显）。**有界渲染是隐私正面贡献**。✓
- **R-W9-3 缺口加剧**：A8 端应直接 `import` 上方 `GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES`（已在同文件 26-27 行），无需新增 `RENDER_*` 第二副本。A4 建议 A8 在最终 patch 里把 `RENDER_NODE_CAP` 改为 `export const RENDER_NODE_CAP = GRAPH_MAX_NODES;`（同源引用），但**A4 不改 A8 文件**（A4 不编辑产品代码）。A0 整合前 A8 自行收敛。
- **A4 W10 立场**：R-W9-3 从 W9 的"建议"升为 W10 的"已扩散观察点"——A0 整合前应有 A0/A1 显式决策（是否留 4 源硬编码 vs 收口为单一 TS 常量 + 跨语言 const 生成）。A4 仅文档化。

**A8 改后 graph UI logic 仍 PASS**（43/43）—— 表明 A8 patch 至少未破坏 A8 W9 已确认的 41 项断言 + W10 新增断言。

## 2. A3 W9 笔记（`logs/assist/A3-M5-W9-mcp-policy-current-phase-20260907-0902.md`）§2.2 给的 7 条架构约束（W10 端落地必须满足）

1. **依赖隔离**：`rmcp` + 必要 `tokio` 必须 `optional = true` + `[features] mcp-server` 启用；**绝不进入 `default` features**。
2. **二进制隔离**：独立 `[[bin]] mcp_server`，`required-features = ["mcp-server"]`，默认 `cargo build` / `cargo tauri build` 不编译。
3. **能力单源**：server 暴露的 tool 复用 `domain.rs::MCP_CAPABILITY_V1` + `evaluate_mcp_command` 同源校验；不得绕过 capability source check。
4. **只读保证不降级**：server 暴露命令仍只读（与 W7 `mcp_policy_get`/`mcp_registry_list`/`mcp_capability_preview` 同口径）；写操作一律拒绝；`MCP_BRIDGE_READONLY` 继续生效。
5. **ACL 对齐**：server 暴露的 tool 名必须与 `default-commands.toml` 一一对应（`MCP_PARITY` 继续生效）。
6. **脱敏不降级**：registry 含 `returns_url` 必须 `redact_sensitive_url`（`MCP_FS_TOOL_PATH_POLICY` 继续生效）；错误/显示面不得回显 secret（K7/A4 隐私审查结论继续适用）。
7. **无第二执行路径**：MCP server 不得新建进程派生通道；仅复用既有 core 内部 API。

## 3. A4 W10 评审目标（W10 看板行 195）

> "Review W10 MCP stdio-prep **and** existing Agent/Skill/Graph/Plugin surfaces for error/audit/log secret echo. **Pay attention to tool result URLs, capability reasons, and serialized errors.**"

→ A4 W10 评审**三层**：
- **层 1（stdio JSON-RPC 通道，新）**：A3 `bin/mcp_server.rs` 落点后，错误/响应走 stdin/stdout JSON-RPC。**与 W7 只读桥前端 toast 不同路径**——绕过前端 store。必须用 §4-A 审查脚本逐项断言。
- **层 2（tool result URLs）**：`returns_url: true` 的 tool（mcp.rs:40/46）必须经 `redact_sensitive_url`（mcp.rs:107）。**当前 `McpRegistryEntryView` 仅暴露 `returns_url: bool`（无 URL 文本）→ 无 secret 暴露面**。但 W10 A3 若要新增 tool 实际返回 URL 内容，**必须复用 `redact_mcp_url`（mcp.rs:107-110）**。→ 必须用 §4-B 断言。
- **层 3（capability reasons / serialized errors）**：当前 `McpDecision` 是 zero-arg enum（mcp.rs:123-126 仅 `Allow`/`Deny`），**无 reason 字段**。W10 A3 若给 server 错误对象加 reason（JSON-RPC 错误对象典型携带 `-32601 Method not found` 等），**新暴露面**：需确认 reason 文本不携带 secret（system prompt / token / cookie / Authorization）。→ 必须用 §4-C 断言。

## 4. A3 W10 落点后 A4 立即可用的审查脚本（REVIEW ONLY，无产品代码）

### 4-A. stdio JSON-RPC 通道错误脱敏审查

```bash
# A3 落点后跑：
cd src-tauri
# 1. 默认 build 仍不应含 rmcp（cargo tree 验证依赖隔离）
cargo tree --no-default-features 2>&1 | grep -E "rmcp|tokio" && echo "FAIL: 默认 build 泄漏 rmcp/tokio" || echo "PASS"
# 2. feature build 才能见到 rmcp
cargo tree --features mcp-server 2>&1 | grep -E "rmcp" | head -3
# 3. mcp_server bin 必须 required-features gate
grep -nE "required-features" src-tauri/Cargo.toml
# 4. bin/mcp_server.rs 全文无 println/eprintln/log_audit（error 文本仅走 stdout JSON-RPC）
grep -nE "println!|eprintln!|log_audit|info!|warn!|error!" src-tauri/src/bin/mcp_server.rs
# 5. mcp_tools/ 目录任何文件无 secret 引用模式（token、Authorization、cookie、sk-、password）
grep -rnE "(?i)(token|authorization|cookie|password|sk-[a-zA-Z0-9_-]{8,})" src-tauri/src/mcp_tools/ 2>&1 | head -10
```

### 4-B. tool result URL 脱敏审查

```bash
# 1. server 暴露的 tool 若 returns_url=true，URL 内容必经 redact_mcp_url
grep -nE "returns_url.*=.*true|redact_mcp_url|redact_sensitive_url" src-tauri/src/mcp_tools/ -r
# 2. evaluate_mcp_command / evaluate_mcp_tool 路径必须经 check_path_within_roots + redact_sensitive_url
grep -nE "check_path_within_roots|redact_sensitive_url|redact_mcp_url" src-tauri/src/mcp.rs src-tauri/src/bin/mcp_server.rs
# 3. capability 白名单与 domain.rs 单源
diff <(grep -oE "MCP_CAPABILITY_V1 = \[[^]]*\]" src-tauri/src/domain.rs) <(grep -oE "MCP_CAPABILITY_V1 = \[[^]]*\]" src-tauri/src/mcp.rs)
```

### 4-C. capability reasons / serialized errors 审查

```bash
# 1. server 错误对象结构（JSON-RPC 错误对象：code/message/data）必须 schema-bounded
grep -nE "serde.*Serialize.*Error|JsonRpcError|to_string|with_message" src-tauri/src/bin/mcp_server.rs src-tauri/src/mcp.rs
# 2. 任何 error.to_string() / Display 不得回显 secret：
#    a) PolicyError::CredentialLeak 现在已 <redacted>（security_policy.rs:118-120）
#    b) GraphError::SecretInProps 固定分类文本（graph.rs:62）
#    c) Agent/Skill validate errors 走 agent_validate_inner / skill_validate_inner → format!("{e}") → <redacted>
# 3. 检查 server 路径是否直接 e.to_string() raw error（而非走 Display 入口）
grep -nE "e\.to_string\(\)|: &\{.*err|display!|Display" src-tauri/src/bin/mcp_server.rs
# 4. 任何错误对象若含 data 字段，必须 schema 白名单（不能透传 raw error）
grep -nE "data:|error\.data|err\.data" src-tauri/src/bin/mcp_server.rs
```

### 4-D. 既有表面（Agent/Skill/Graph/Plugin）W10 范围内无回归确认

```bash
# 1. 5 政策全跑（baseline 已 PASS → 落点后仍应 PASS）
for s in check-agent-memory-policy check-graph-policy check-plugin-policy check-agent-skill-policy check-mcp-policy; do
  python3 scripts/$s.py --self-test
  python3 scripts/$s.py
done
# 2. UI logic
node scripts/check-graph-ui-logic.mjs
node scripts/check-agent-skill-ui-logic.mjs
# 3. CredentialLeak 脱敏回归
grep -nE "CredentialLeak" src-tauri/src/security_policy.rs
# 4. Graph SecretInProps 固定分类
grep -nE "SecretInProps" src-tauri/src/graph.rs
# 5. 全文 secret 引用模式扫描（默认 + features 双 build）
cargo build --no-default-features 2>&1 | grep -E "warning.*unused" | head -5
```

## 5. A3 W10 落点后 A4 二次复核清单（A3 commit 后套用）

- [ ] A3 提交 `bin/mcp_server.rs` 后，§4-A ~ §4-D 全跑 PASS
- [ ] 默认 `cargo build` 0 rmcp/0 tokio（`cargo tree --no-default-features | grep` 空）
- [ ] `cargo build --features mcp-server` 出现 rmcp；`bin mcp_server` 编译成功
- [ ] `MCP_NO_RMCP_SERVER` 按 A3 W9 笔记 §2.3 翻转条件升级为 `MCP_SERVER_GATED`（或 A0 显式决策后）
- [ ] 8 ACTIVE 码位在 feature build 下仍全绿（或 `MCP_NO_RMCP_SERVER` 翻转后其余 7 仍绿）
- [ ] `cargo test mcp_policy_tests` 9 passed + 新增 stdio-prep 单测 PASS
- [ ] `mcp_server` 错误对象（JSON-RPC）schema 文档化且不含 raw error
- [ ] `returns_url=true` 的 tool 经 `redact_mcp_url`
- [ ] 既有 4 域（Agent/Skill/Graph/Plugin）W10 范围内零回归
- [ ] Frontend 不读 stdin/stdout（stdio 是 server 进程内通信，**不**经 Tauri IPC）

## 6. 与 W10 Hard Stops 对齐

| Hard Stop | A4 评估 |
|---|---|
| No TCP listener, HTTP server, network bind, background daemon | 当前 mcp.rs 零网络；A3 落点须 stdio-only（行 207）|
| Plugin install/enable/delete/download runtime, skill/agent execution, model call | 不属 A4 范围；A9/A5 守门 |
| rmcp/tokio optional + feature-gated | 取决于 A3 落点是否符合 §2 7 条；A4 §4-A 验证 |
| Source check + ACL 同步 | 既有 9 命令（A4 W9 已核），新增 tool 须 MCP_PARITY 守门 |
| Build metrics 22% / cargo warnings 不增 | 取决于 A3 落点；A11 复核 |
| Only A0 push | A4 不 push |

## 7. A4 W9 残项在 W10 阶段的进展

| ID | W9 状态 | W10 阶段动作 / 观察 |
|---|---|---|
| R-W9-1 | 建议 A5 升 AGSK_CREDENTIAL_NOT_ECHOED PENDING→ACTIVE | 仍未升（PENDING=5 桶）；A5 W10 范围限定（行 196 "may add if A10/A4 identify a concrete leak; otherwise no product-code patch"）—— R-W9-1 不属"具体漏洞"，A4 不强制 A5 升，留 A5 决定 |
| R-W9-2 | 建议 A5 Rust 补"errors.join 不含 secret"断言 | 同 R-W9-1；A4 不强制 |
| R-W9-3 | F1 DRY 常量化 OPEN（SENSITIVE_* 双份 + 图谱容量常量 3 源拷贝） | **W10 升级为"已扩散观察点"**：A8 W10 working-tree 在 `src/utils/graphUi.ts:237-238` 新增 `RENDER_NODE_CAP=5000` / `RENDER_EDGE_CAP=20000` 两个硬编码副本，**4 源扩散**（Rust 真源 + 3 处 TS 硬编码）。A4 已在 §1.4 完整记录；建议 A8 在最终 patch 把 `RENDER_NODE_CAP`/`RENDER_EDGE_CAP` 改为同源引用 `GRAPH_MAX_NODES`/`GRAPH_MAX_EDGES`（同文件 26-27 行），**A4 不改 A8 文件**（A4 REVIEW ONLY）。A0 整合前应有 A0/A1 显式决策（4 源硬编码 vs 收口为单一 TS 常量 + 跨语言 const 生成）。 |

## 8. 与 M5-2.b 转发卡（A3 W9 §2）的 W10 接续

A3 W9 给的 M5-2.b 转发卡明确"该翻转**只能由 A0 在显式解锁 M5-2.b 后**执行"。A4 W10 评审**不**替 A0 决策，**不**触发 `MCP_NO_RMCP_SERVER` 翻转。本笔记仅产出审查框架与 baseline 快照。

## 9. 风险点速查（A3 W10 提交后逐项核）

| 风险 | 位置 | 审查断言 |
|---|---|---|
| rmcp/tokio 进入 default build | Cargo.toml | `cargo tree --no-default-features` 空 |
| mcp_server bin 漏 required-features | Cargo.toml | `[[bin]]` 块含 `required-features = ["mcp-server"]` |
| mcp_tools/ 目录含 secret 模式 | src-tauri/src/mcp_tools/ | §4-A 第 5 条 |
| tool 返回 URL 未脱敏 | bin/mcp_server.rs / mcp_tools/ | `redact_mcp_url` 必经 |
| 错误对象 reason 字段含 secret | bin/mcp_server.rs | §4-C |
| ACL 奇偶漂移 | default-commands.toml | `MCP_PARITY` 守门 |
| capability 白名单双源 | domain.rs vs mcp.rs | diff 应空 |
| secret 落 audit / log | 全文 | bin/mcp_server.rs 全文 0 log_audit / 0 println |
| 第二执行路径 | bin/mcp_server.rs | 仅复用 core API；无 `std::process::Command` / `tokio::process` / `Command::new` 直接派生子进程 |

## 10. 与 A2 W10（边界 review）的分工

A2 W10 任务（行 193）："Review A3 W10 MCP stdio-prep plan/code for **core/bin boundary**: no tauri in core, no bridge::* calls from mcp tools, no duplicate script/db/plugin execution path."

A4 W10 任务（行 195）："Review ... for **error/audit/log secret echo**. Pay attention to **tool result URLs, capability reasons, and serialized errors**."

→ **A2 看 core/bin 边界；A4 看 secret echo 边界。两者互补不重叠**。A3 提交后建议 A2 + A4 并行复核，A0 整合前合并结论。

---

## 11. LANE 输出模板

```text
LANE=A4
STATUS=PASS（REVIEW ONLY，无产品代码；A3 W10 落点后由 A4 二次复核套用 §4 脚本）
BASE=3792115
HEAD=logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md
FILES=logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md
VERIFY=git pull --ff-only @3792115；5 政策全 PASS（agent-memory ACTIVE=5 / graph ACTIVE=7 / plugin ACTIVE=1 PENDING=5 / agent-skill ACTIVE=3 PENDING=5 / mcp ACTIVE=8 PENDING=0）；mcp --expect-current-gaps PASS；graph UI logic 43/43；agent-skill UI logic 99/99；Cargo.toml 无 [features] 段、无 rmcp/tokio；McpDecision zero-arg enum（无 reason 字段）→ 现状无 capability reason 暴露面
CHECKPOINT=logs/assist/A4-M5-W10-privacy-review-baseline-20260907-0925.md
MERGE_NOTES=W10 A4 baseline 快照 + A3 stdio-prep 落点后审查框架（§4-A stdio JSON-RPC 通道 / §4-B tool result URL 脱敏 / §4-C capability reasons 序列化 / §4-D 既有表面无回归）；A3 W9 M5-2.b 转发卡 7 条架构约束已纳入；A4 不替 A0 决策 MCP_NO_RMCP_SERVER 翻转；与 A2 W10（边界）分工互补；W10 观察：A8 working-tree 在 graphUi.ts:237-238 新增 RENDER_NODE_CAP/RENDER_EDGE_CAP 硬编码副本 → R-W9-3 从 W9 3 源扩散到 W10 4 源（§1.4），A4 仅记录不修改 A8 文件；A9 W10 笔记 0 byte 空文件违规（与 A4 无关，A0 整合时处理）；W9 残项 R-W9-1/R-W9-2 留 A5 决定，R-W9-3 升级为已扩散观察点待 A0/A1 决策
NEXT=A3 W10 提交后由 A4 套用 §4 脚本二次复核 + 报 A2 并行；A10 终审；A11 W10 验证矩阵
```