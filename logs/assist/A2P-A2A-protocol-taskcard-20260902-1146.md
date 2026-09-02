# A2P-A2A-protocol-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §7 **M5-1 ~ M5-3**（需求 #7 A2P / A2A 等 Agent 协议）
> 状态：⏸ **任务卡 / 未实现协议栈 / 未执行验收 / 不宣称 PASS**
> 关联：`M5-7.a-prework-20260902-1055.md`（能力清单 / stdio 传输 / McpGlobalPolicy）· `M5-12.a-prework-20260902-1055.md`（Agent-Skill 契约，共享同一份能力白名单）

---

## 1. 目标

整理 M5 A2P / A2A 协议卡：**消息结构、能力声明、权限、超时、取消、错误码、审计、反向用例**。

| 术语 | 含义 |
|---|---|
| **A2A**（Agent-to-Agent） | 本应用与外部 Agent 之间的通信协议 |
| **A2P**（Agent-to-Platform） | 外部 Agent 调用本平台（宿主）能力的协议 |
| **MCP** | 既有的事实标准（Model Context Protocol），上一批次 `M5-7.a` 已选定 **stdio 传输 + MCP 风格消息** |

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 现状 |
|---|---|
| 协议实现 | ❌ 无（`Cargo.toml` 无 MCP / JSON-RPC 相关依赖） |
| 协议命令 | ❌ 59 个白名单命令中无 `a2a_*` / `mcp_*` |
| 领域模型 | ❌ `domain.rs` 无协议相关结构 |
| 子进程能力 | ⚠️ 有终端 PTY（`portable-pty`），**但**无 `ShutdownCoordinator`（依赖 TASK-10） |
| 出网能力 | ⚠️ `capabilities/default.json` 未显式授予 HTTP 权限；前端无 `fetch` 到宿主代理的通道 |
| 审计 | ✅ `log_audit`，1000 条上限（**K5**） |
| 已有 AI 前端参照 | ✅ `src/components/browser/AINavPanel.vue`（AI 导航面板） |

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| **新增** `src-tauri/src/protocol/mod.rs` | 协议模块入口 | 必须 |
| **新增** `src-tauri/src/protocol/message.rs` | 消息结构（请求/响应/通知/错误） | 必须 |
| **新增** `src-tauri/src/protocol/capability.rs` | 能力声明与白名单 | 必须 |
| **新增** `src-tauri/src/protocol/transport_stdio.rs` | stdio 传输（子进程 + JSON Lines） | 必须 |
| **新增** `src-tauri/src/protocol/session.rs` | 会话管理、超时、取消 | 必须 |
| `src-tauri/src/domain.rs` | 新增协议相关结构 | 必须 |
| `src-tauri/src/bridge.rs` | 新增 `a2a_spawn` / `a2a_send` / `a2a_cancel` / `a2a_list_agents` | 必须 |
| `src-tauri/permissions/default-commands.toml` | 新增命令（**K1**） | 必须 |
| `src-tauri/src/shutdown.rs` | 注册 `AgentShutdown` | 必须（依赖 TASK-10） |

---

## 4. 契约 / 数据结构

### 4.1 传输层

| 项 | 决策 |
|---|---|
| 传输 | **stdio**（子进程 + 换行分隔的 JSON，即 JSON Lines / NDJSON） |
| 编码 | UTF-8；**每条消息一行**；行尾 `\n` |
| 单行上限 | 1 MB（超出即协议错误，防止 OOM） |
| 保活 | 无（stdio 天然长连接） |
| 并发 | 单个 Agent 一个子进程；多 Agent 多子进程 |

**为什么不用 HTTP/WebSocket**：stdio 无需端口、无需鉴权暴露面、进程隔离天然，且与 MCP 生态一致（`M5-7.a` 已定）。

### 4.2 消息结构（JSON-RPC 2.0 兼容）

```jsonc
// ---- 请求（A2P：宿主 → Agent；A2A 同构） ----
{
  "jsonrpc": "2.0",
  "id": "req-<uuid>",           // 字符串，非数字（便于幂等与去重）
  "method": "skill/invoke",
  "params": { /* 方法专属 */ },
  // 扩展字段（非标准 JSON-RPC，但显式声明便于网关实现）
  "meta": {
    "sessionId": "sess-<uuid>",
    "traceId": "trace-<uuid>",     // 贯穿审计
    "timeoutMs": 30000
  }
}

// ---- 成功响应 ----
{
  "jsonrpc": "2.0",
  "id": "req-<uuid>",           // 必须与请求一致
  "result": { /* 方法专属 */ },
  "meta": { "traceId": "trace-<uuid>", "elapsedMs": 1234 }
}

// ---- 错误响应 ----
{
  "jsonrpc": "2.0",
  "id": "req-<uuid>",
  "error": {
    "code": -32001,
    "message": "能力未授予: shell:execute",
    "data": { "capability": "shell:execute", "retriable": false }
  },
  "meta": { "traceId": "trace-<uuid>" }
}

// ---- 通知（无 id，无需响应）----
{
  "jsonrpc": "2.0",
  "method": "progress",
  "params": { "sessionId": "sess-<uuid>", "done": 3, "total": 10 }
}
```

| 字段约束 | 说明 |
|---|---|
| `id` | 必须是**字符串**且在会话内唯一；数字 id 一律拒绝（防精度丢失与重放） |
| `method` | 小写 + `/` 分层命名空间（`skill/invoke`、`fs/read`、`a2a/ping`） |
| `params` | 必须是 **object**（数组一律拒绝，防位置参数歧义） |
| `meta.traceId` | 请求方生成；缺失则由网关补；贯穿全部日志与审计 |
| 未知字段 | **忽略**（前向兼容），但记录 debug 日志 |

### 4.3 能力声明（Capability）

```jsonc
// Agent 启动时通过 initialize 声明
{
  "jsonrpc": "2.0", "id": "req-init",
  "method": "initialize",
  "params": {
    "agentId": "code-reviewer",
    "agentVersion": "1.2.0",
    "protocolVersion": "1.0",
    "declaredCapabilities": [
      { "name": "fs:read",  "scope": ["/home/u/projects/**"], "dangerous": false },
      { "name": "shell:execute", "scope": ["git status", "git diff"], "dangerous": true },
      { "name": "net:fetch", "scope": ["https://api.github.com/**"], "dangerous": true }
    ],
    "skills": [
      { "name": "review_pr", "description": "审查一个 PR", "dangerous": false }
    ]
  }
}
```

**能力白名单（与 `M5-12.a` Agent-Skill 契约共用同一份）**：

| 能力 | 默认 | 说明 |
|---|---|---|
| `fs:read` | 需授权 | 路径 scope 必需 |
| `fs:write` | 需授权 | 路径 scope 必需 |
| `shell:execute` | **默认拒绝** | 与 `script-execution-safety-taskcard` 同一通道 |
| `net:fetch` | **默认拒绝** | 域名白名单必需 |
| `db:query` | 需授权 | 只读优先 |
| `ui:notify` | 允许 | 向用户展示通知（低危） |

**fail-closed（与 `M5-15.a` §5 一致）**：声明但未授予的能力 → **拒绝执行**，不降级放行。

### 4.4 权限模型

```
宿主全局策略 (McpGlobalPolicy)
   ├── allowUntrustedAgents: false        // 默认不允许未签名/未登记的 Agent
   ├── maxConcurrentAgents: 4
   ├── defaultTimeoutMs: 30000
   ├── maxTimeoutMs: 300000
   ├── capabilities: { 全局默认授予集 }
   └── perAgent: { "<agentId>": { 覆写 } }   // 用户显式授予，存配置
```

| 层 | 判定顺序 |
|---|---|
| 1 | Agent 是否登记（`agentId` 在已知列表） |
| 2 | 方法是否需要能力（查方法 → 能力映射表） |
| 3 | 该能力是否被授予（全局默认 + perAgent 覆写） |
| 4 | `scope` 是否覆盖本次参数（路径前缀 / 命令白名单 / 域名前缀） |
| 5 | 危险能力是否有当次会话的**用户显式同意**（可带「本次会话记住」） |

任一层不通过 → 返回 error，**不执行**。

### 4.5 超时

| 项 | 规则 |
|---|---|
| 请求级 | `meta.timeoutMs`，缺失用 `defaultTimeoutMs`（30 s），上限 `maxTimeoutMs`（300 s） |
| 硬超时 | `timeoutMs + 5 s` → 强制终止（SIGTERM → SIGKILL，进程组） |
| 初始化超时 | 10 s（Agent 启动后必须在此时间内发 `initialize`） |
| 空闲超时 | 300 s 无消息 → 关闭会话（可配） |
| 超时响应 | `error.code = -32004`，`data.retriable = true` |

### 4.6 取消

```jsonc
// 宿主 → Agent
{ "jsonrpc":"2.0", "method":"$/cancel", "params": { "id": "req-<uuid>" } }
```

| 规则 | 说明 |
|---|---|
| 通知形态 | `$/cancel` 是**通知**（无 id），Agent 可异步响应 |
| 幂等 | 对已终态的 id 发 cancel → 无副作用 |
| 尽最大努力 | Agent 可能已执行完；宿主应仍以最终 result/error 为准 |
| 硬取消 | 超时 + 5 s 后仍未响应 → 进程组终止 |
| 审计 | 记 `a2a.cancel`，含 traceId 与 reason（user / timeout / shutdown） |

### 4.7 错误码

| 区间 | 归属 | 说明 |
|---|---|---|
| `-32700` ~ `-32600` | 标准 JSON-RPC | 解析错误 / 无效请求 / 方法未找到 / 无效参数 / 内部错误 |
| `-32001` ~ `-32019` | **本平台保留（A2P/A2A）** | 见下表 |
| `-32100` 以上 | Agent 自定义 | Agent 业务错误 |

**平台保留错误码表**：

| code | 名称 | `retriable` | 触发 |
|---|---|---|---|
| `-32001` | `CAPABILITY_DENIED` | false | 能力未授予 / scope 不匹配 |
| `-32002` | `AGENT_NOT_REGISTERED` | false | `agentId` 未登记 |
| `-32003` | `AGENT_SPAWN_FAILED` | true | 子进程启动失败 |
| `-32004` | `TIMEOUT` | true | 请求超时 |
| `-32005` | `CANCELLED` | false | 用户/系统取消 |
| `-32006` | `RATE_LIMITED` | true | 并发/频率超限 |
| `-32007` | `MESSAGE_TOO_LARGE` | false | 单行 > 1 MB |
| `-32008` | `PROTOCOL_VERSION_MISMATCH` | false | `protocolVersion` 不兼容 |
| `-32009` | `TRANSPORT_CLOSED` | true | stdio 管道关闭 |
| `-32010` | `INVALID_ID` | false | id 非字符串或重复 |
| `-32011` | `INVALID_PARAMS` | false | `params` 非 object |
| `-32012` | `SHUTTING_DOWN` | true | 宿主正在退出 |

**规则**：`retriable` 必须由返回方显式给出；**不允许**客户端靠 code 猜。

### 4.8 审计

| 事件 | 落哪里 | 内容 |
|---|---|---|
| Agent 启动 / 停止 / 崩溃 | `audit.json` | agentId、版本、退出码 |
| **每次方法调用** | ⚠️ **独立 `a2a-audit.json`**（**K5**） | traceId、method、能力、耗时、结果码 |
| 能力被拒（`CAPABILITY_DENIED`） | `audit.json` | agentId、capability、traceId |
| 危险能力的用户同意 | `audit.json` | agentId、capability、同意范围（一次/本次会话/永久） |
| 消息正文 | ❌ **不记** | 隐私 + 体积（可能含用户数据） |

**红线**：A2A 的每次调用**不得**写 `audit.json`（会刷爆 1000 条上限 —— K5），必须独立文件承载明细 + 摘要进 audit。

---

## 5. 实现要点（步骤化）

1. **先完成 `M3-terminal-shutdown-taskcard`**（ShutdownCoordinator / 进程组回收）。
2. `protocol/message.rs`：请求/响应/通知/错误的结构 + 序列化 + 校验（id 字符串、params object、单行 ≤1 MB）。
3. `protocol/capability.rs`：能力白名单 + scope 匹配（路径前缀 / 命令白名单 / 域名前缀）+ fail-closed 判定。
4. `protocol/transport_stdio.rs`：子进程 spawn（**`setsid` 建进程组**，同 `script-execution-safety-taskcard` §4.7）+ 行读取 + 写入 + 背压。
5. `protocol/session.rs`：会话表、id 去重、超时、取消、并发上限。
6. `bridge.rs` 加命令 → `default-commands.toml`（**K1**）。
7. 注册 `AgentShutdown`。
8. 审计按 §4.8 拆分落盘。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ 接受数字 `id` / 数组 `params` | 精度丢失与语义歧义 |
| 2 | ❌ 能力未授予时降级放行 | 必须 fail-closed |
| 3 | ❌ 无超时 / 无硬超时 | 卡死宿主 |
| 4 | ❌ 只 kill 直接子进程 | 孤儿进程（Agent 常派生子进程） |
| 5 | ❌ 每次调用写 `audit.json` | 刷爆 1000 条上限（K5） |
| 6 | ❌ 把消息正文写进审计/日志 | 隐私泄露 |
| 7 | ❌ 新增命令忘进 ACL | K1 |
| 8 | ❌ 让 Agent 自己决定能力边界 | 能力由宿主判定，Agent 只能**声明** |
| 9 | ❌ 无 `ShutdownCoordinator` 就上 Agent 进程 | 退出收不掉 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | 依赖 `ShutdownCoordinator` 与进程组回收 | 未完成时不得开工 |
| 高 | Agent 是不可信代码 → 提示注入 / 越权 | 能力白名单 + scope + fail-closed + 危险能力用户显式同意 |
| 中 | stdio 管道背压（Agent 狂输出） | 行读取限速 + 单行 1 MB 上限 + 环形缓冲 |
| 中 | 协议版本演进 | `protocolVersion` 显式协商；未知字段忽略 |
| 中 | 并发 Agent 资源竞争 | `maxConcurrentAgents = 4` + 队列 |
| 中 | MCP 生态快速演进，自研协议可能脱节 | 消息结构**保持 JSON-RPC 2.0 兼容**，便于后续对齐或替换 |
| 低 | `traceId` 缺失导致排障困难 | 网关强制补全 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 发送 `id: 123`（数字） | `-32010 INVALID_ID` |
| R2 | 发送 `params: [1,2]`（数组） | `-32011 INVALID_PARAMS` |
| R3 | 发送单行 2 MB 消息 | `-32007 MESSAGE_TOO_LARGE`，不 OOM |
| R4 | 未登记的 agentId 调 `initialize` | `-32002 AGENT_NOT_REGISTERED` |
| R5 | Agent 声明 `shell:execute` 但未授予 → 调用 | `-32001 CAPABILITY_DENIED` + 审计记录 |
| R6 | `fs:read` scope 为 `/home/u/projects/**`，请求 `/etc/passwd` | `-32001`（scope 不匹配） |
| R7 | Agent 30 s 不响应 | `-32004 TIMEOUT`，`retriable = true` |
| R8 | 超时后 5 s 仍不退出 | 进程组 SIGKILL，无残留 |
| R9 | 对已完成的 id 发 `$/cancel` | 无副作用 |
| R10 | 并发启动 10 个 Agent（上限 4） | 6 个进队列或 `-32006 RATE_LIMITED` |
| R11 | Agent 崩溃（stderr 输出后退出） | 会话关闭，in-flight 请求返回 `-32009`，审计记录退出码 |
| R12 | 宿主退出时有 in-flight 请求 | `-32012 SHUTTING_DOWN` + 进程组回收 |
| R13 | 协议版本不兼容 | `-32008`，给出支持版本列表 |
| R14 | 高频调用 1000 次 | `audit.json` 不爆（明细在 `a2a-audit.json`） |
| R15 | Agent 返回含 `retriable` 缺失的错误 | 视为 `false`（不重试） |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 协议模块存在
ls src-tauri/src/protocol/                       # 期望 message.rs / capability.rs / transport_stdio.rs / session.rs

# 9.2 id 必须是字符串
grep -n "INVALID_ID\|is_string\|as_str" src-tauri/src/protocol/message.rs   # 期望命中

# 9.3 单行上限
grep -n "1_048_576\|1 * 1024 * 1024\|MAX_LINE" src-tauri/src/protocol/transport_stdio.rs  # 期望命中

# 9.4 能力 fail-closed
grep -n "CAPABILITY_DENIED\|fail.closed\|fail_closed" src-tauri/src/protocol/capability.rs  # 期望命中

# 9.5 超时与硬超时
grep -n "timeoutMs\|HARD_TIMEOUT\|killpg" src-tauri/src/protocol/session.rs  # 期望命中

# 9.6 进程组
grep -n "setsid\|killpg" src-tauri/src/protocol/transport_stdio.rs            # 期望命中

# 9.7 审计不刷爆（K5）
grep -rn "log_audit" src-tauri/src/protocol/*.rs | wc -l                      # 期望：少量（生命周期 + 拒绝）
grep -n "a2a-audit\|a2a_audit" src-tauri/src/protocol/*.rs                    # 期望命中（独立明细文件）

# 9.8 命令已进 ACL（K1）
grep -c "a2a_spawn\|a2a_send\|a2a_cancel\|a2a_list_agents" src-tauri/permissions/default-commands.toml  # 期望 4

# 9.9 退出收口
grep -n "AgentShutdown" src-tauri/src/shutdown.rs                             # 期望命中

# 9.10 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 logs/baseline-2026-08-27.md（13 warning）
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 能力未授予却被放行 | 视为 P0 安全缺陷，立即整改 |
| 超时后进程残留 | 补硬超时 + 进程组 kill；不得只 kill 直接子进程 |
| `audit.json` 被刷爆 | 拆独立 JSON；不得靠「Agent 调用不多」搪塞 |
| 消息正文进日志 | 立即移除；不得「只记前 200 字符」这类折中 |
| Agent 崩溃导致宿主卡死 | 补 `TRANSPORT_CLOSED` 处理与 in-flight 清理 |
| clippy warning 增加 | 对照 baseline 回退 |

---

## 11. 推荐模型

`AI:DEEP`（协议设计 + 子进程 + 并发 + 安全边界）。
方案评审建议 `AI:DEEP-xhigh`（能力与权限模型）。
**人工验收必做**：R4/R5/R6（权限）、R11（崩溃）、R12（退出收口）。
