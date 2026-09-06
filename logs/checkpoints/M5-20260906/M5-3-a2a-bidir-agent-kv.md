# M5-3 A2A 双向 + agent_kv（多方言归一 + 记忆层）

> 子卡 ID：**M5-3** · 需求 #7（A2P/A2A）· `[S3|LEVERAGE:2|COMPLEX|AI:DEEP|R:xhigh]`
> 责任 Lane 候选：**A15**（A9 提案；A0 签发时定）
> 父卡：`详细设计与实施计划.md` L564（`M5-3 A2A 双向与记忆`）
> 主预研：`logs/assist/M5-7.a-prework-20260902-1055.md` §4.3 A2A 草案；`logs/assist/A4-M5-a2a-memory-20260906-0755.md`（A4 v1 冻结 F-A4-1~14）
> 配套：`M5-2-rmcp-mcp-policy.md`（能力层基础）· `M5-4-agent-skill-runtime.md`（Agent 侧消费）

---

## [W1 patched · 2026-09-06 08:50 CST] agent_kv 三层嵌套 + 隐私三重闸 + TTL/迟滞水位

> **修订来源**：A0 M5-W1 dispatch（`logs/checkpoints/A0-M5-W1-dispatch-20260906-0835.md`）A1 行 reconcile + A4 prework（`logs/assist/A4-M5-a2a-memory-20260906-0755.md` F-A4-6/7/9 + N1~N6 + 25 单测）+ A10 复审（`logs/assist/A10-M5-security-review-20260906-1410.md` §104 "A1 卡 `agent_kv` 为扁平 `HashMap`（且其要求 LRU 淘汰但无 `updated_at`，内部自相矛盾）；A4 建议改三层嵌套（更严）"）。
> **修订原则**：A1 W1 接受 A4 更严方案（两层选一，"两者均安全，A4 更严"），修订**仅 §4.3 整段**与本顶部 `[W1 patched]` 段，**不重写 §1/§2/§4.1/§4.2/§4.4/§4.5 与 §5~§11 决策史**。

### W1 修订点

| # | W0 现状 | W1 修订 |
|---|---|---|
| W1-3-1 | §4.3 "存储" `HashMap<String, serde_json::Value>` 扁平 | 改**三层嵌套** `agent_kv[<agent_id>][<namespace>][<key>]`；namespace 白名单 `memory`/`facts`/`prefs`/`session`/`draft` |
| W1-3-2 | §4.3 "上限 5 MB / 5000 条（超限按 LRU 淘汰）" | 改 5 MiB 总 / 5000 条总 + **per-namespace 1000 条** + **per-agent 32 条**软配额 |
| W1-3-3 | §4.3 "LRU 淘汰但无 `updated_at`"（A10 §104 内部矛盾） | 加 `updated_at` 必填 + 90% 迟滞水位触发 + 按 `updated_at` 升序淘汰；TTL：`session` 30d / `prefs`/`facts` 365d / `memory`/`draft` 永久 |
| W1-3-4 | §4.3 "键名黑名单（复用 M2-3.a §4.4）" | 改**三重隐私闸**：① 键名黑名单 ② 值 JSON 字段名前缀关键字 ③ 出口脱敏（不入审计/前端/日志） |
| W1-3-5 | §4.3 未提 storage backend | 钉死**不引 SQLite**（沿用 M3 范式 `atomic_write` 单文件 JSON） |

### 引用与一致性

- 与 A4 预研对齐（`logs/assist/A4-M5-a2a-memory-20260906-0755.md` §5/§6 F-A4-6/7/9 + N1~N6 机器可检红线 + 25 单测）
- 与 A1 整包对齐（`M5-A1-expansion-20260906-0800.md` §"M5-3 与 A4"）
- 不与 A1 其它卡冲突（`M5-2`/`M5-4`/`M5-5`/`M5-6` 未涉 `agent_kv` 细节）

---

## [W4 next-card acceptance criteria · 2026-09-06 17:55 CST] A4 实施期 acceptance criteria（首切片：DTOs + 校验 + 容量/隐私 policy + JSON 持久化壳）

> **依据**：`PARALLEL_COMMAND_BOARD.md` L153（**A4 M5-W4** *"Implement first M5-3 A2A/agent memory KV contract slice: DTOs, validation, capacity/privacy policy, pure store helpers or JSON persistence shell if already patterned; no network protocol and no background runtime."*）+ L134-168 硬约束 + A4 W3 delta（`logs/assist/A4-M5-a2a-memory-20260906-1410-w3-delta.md`）。
> **消费依赖（已落地，A4 W4 可直接接入）**：
> - **U-2 seam 已落**（`f8f1f49`）—— `mvp_core::seam::{ProgressSink, PathResolver, RootsProvider}` 可用于 agent_kv 的 path 解析与 root 校验（若需路径策略）。
> - **U-4 capability 真源已落**（`12f1cff`）—— `MCP_CAPABILITY_V1` + `MCP_COMMAND_REGISTRY` + `evaluate_mcp_policy`；M5-3.b（A2A/dialect 命令闸门）W5+ 才消费，本卡 W4 暂不触。
> **A1 W4 角色**：A1 W4 **不**改 §1~§11 决策史；仅在头部加本 `[W4 next-card acceptance criteria]` 段，**明确 A4 W4 实施期 4 项 AC + 5 项 hard stops**，供 A4 / A10 / A11 / A0 验收。

### W4 A4 M5-3.a 实施期 acceptance criteria（4 项）

| AC | 描述 | 验收证据 |
|----|------|----------|
| AC-1 **DTOs 冻结** | `agent_kv` 三层（**per-agent**：`agent_id` / **per-scope**：`scope` / **per-key**：`key`）结构体与序列化形态在 `domain.rs` 或新 `agent_memory.rs`/`a2a.rs` 冻结；不允许在桥接/UI 层临时重定义 | `cargo test domain::agent_memory` 编译期 + 单测 + A11 抽查 |
| AC-2 **校验完整** | 字段名校验拒 `{"note":"sk-xxx"}`（A1 M5-3 卡 C-5 已识别错误——W3 delta §4 修正）；per-agent 条数上限 + 总条数上限（A1 卡 C-6 已识别 "per-agent 32 条"使 5000 总上限形同虚设——W3 delta §4 修正）；TTL/迟滞水位三层 + privacy 三重闸 | `scripts/check-agent-memory-policy.py` self-test + default + `--expect-pending` 三模式 + focused Rust 单测 |
| AC-3 **容量/隐私 policy 守门** | 容量上限 + privacy filter（无 token/cookie/Authorization/body/日志 prompt secrets —— W4 Hard Stop W4-HS4）；policy 脚本接入 `pre-merge.sh` | `pre-merge.sh` ALL_PASS |
| AC-4 **pure store helpers 或 JSON 持久化壳** | 若 core 内已 patterning：写 `core/agent_kv.rs` 纯 helper（无 Tauri 依赖）；若 binary 内：写 `agent_memory.rs` 配 `core::seam::PathResolver`（注入 base_dir）；持久化走 `session::atomic_write`（沿用唯一原子写原语）；**W4 不接 Tauri 命令**（W4-HS3 优先不加 command）| `cargo test agent_memory::` 或 `cargo test core::agent_kv::` |

### W4 A4 M5-3.a 实施期 hard stops（5 项）

| HS | 约束 | 来源 |
|----|------|------|
| W4-HS1 | **无 network protocol listener / 无 background runtime / 无 background scheduler** | PARALLEL_COMMAND_BOARD L153 + L165 |
| W4-HS2 | **无新 Tauri 命令**（除非 source check + ACL + 前端 bridge/types + policy coverage + tests 同包完整；W4 优先不加） | PARALLEL_COMMAND_BOARD L153 + L166 |
| W4-HS3 | **无新 Cargo 依赖**（除非已存在且有理由；A4 W3 delta 已声明 `serde_json` / `serde` 复用即可） | PARALLEL_COMMAND_BOARD L153 + A4 W3 delta |
| W4-HS4 | **core 内常量自拥**（A2 0b 已统一到 `domain.rs` 副本零容忍 —— `R-B3`）；agent_kv 自己的常量应定义在 `core/agent_kv.rs`（不引 `crate::domain::AGENT_KV_*` 假货）| A2 W2 `712a14c` + A4 W3 delta §"常量落点修正" |
| W4-HS5 | **所有 lane 必须从 `origin/master` pull，不 push** | PARALLEL_COMMAND_BOARD L168 |

### W4 验证清单（供 A11 收口）

- `cargo test --manifest-path src-tauri/Cargo.toml agent_memory`（或 `core::agent_kv`）PASS
- `python3 scripts/check-agent-memory-policy.py --self-test` PASS
- `python3 scripts/check-agent-memory-policy.py` PASS
- `python3 scripts/check-agent-memory-policy.py --expect-pending` PASS（如有 PENDING）
- `bash scripts/pre-merge.sh` ALL_PASS
- `git diff --check` CLEAN
- **A10 复审 PASS**（credential leakage / unbounded maps / command exposure / source-check/ACL drift / duplicate execution path 全否）
- **A11 verification delta** 产出 `logs/checkpoints/M5-A11-W4-*.md`

### W4 A1 不修订范围

- **§1 GOAL / §2 READ / §3 WRITE / §4 关键契约 / §5 FORBID / §6 COMMANDS / §7 PASS_CRITERIA / §8 FAIL_ACTION / §9 DOC_BACKWRITE / §10 COMMIT / §11 FORBID 遵守记录**：A1 W4 **不动**（决策史保持 W1 原文；W4 AC 在本顶部段单列）。
- **三份主文档 / ACL / Capability / pre-merge.sh / scripts/**：A1 W4 不动（policy 脚本由 A4 落地）。
- **`NEXT` 标记**：A0 调度权；A1 不改字面值。
- **本卡顶部"四 vs M5-3 拆 a/b"已由 A4 W3 delta 给出**（`agent_kv` 纯逻辑 = M5-3.a 待 U-2 ✅；A2A/dialect 命令闸门 = M5-3.b 待 U-4 ✅）——A1 W4 在本段承接，不重写 §0。

---

## 0. 编号与锚定

- 批次任务号 `M5-3`；需求号 #7；WBS L564 一致。
- 依赖：M5-2 ✅（`McpGlobalPolicy` + capability.rs） + M5-4 部分（Agent dialect 复用）
- 关键决策交 A0 拍：① A2A 首期是否真双向（仅被调 or 既可被调又可委派） ② `agent_kv` 容量上限策略

---

## 1. GOAL

实现 A2A（Agent ↔ Agent）协议兼容层与 Agent 记忆层（`agent_kv`）。首期支持多 Agent 方言归一（OpenAI 兼容 HTTP API + 外部 CLI Agent 子进程），构建可委派、可被委派的 A2A 任务流，记忆层仅作 AI 上下文（**与用户成果库严格分离**）。

---

## 2. READ

1. `logs/assist/M5-7.a-prework-20260902-1055.md` §4.3 A2A 草案（**全读**）
2. `M5-2-rmcp-mcp-policy.md`（能力层 / 确认闸门 / 审计范式）
3. `M5-4-agent-skill-runtime.md`（Agent 侧消费 `agent_kv`）
4. `src-tauri/src/keyring_store.rs`（LLM API Key 存放范式，27 行全读）
5. `src-tauri/src/bridge.rs:688-790`（两段式确认闸门，A2A 任务发起同样走）
6. `M5-0-overview.md` §7 红线（K3 凭据 / K5 审计）

---

## 3. WRITE

| 文件 | 性质 | 说明 |
|---|---|---|
| `src-tauri/src/a2a.rs` | **新增** | A2A 消息 schema + 状态机 + 协议兼容层 + 幂等键去重 |
| `src-tauri/src/agent_dialect.rs` | **新增** | 多方言归一：`OpenAiCompatible { base_url, model }` / `ExternalCli { program }` |
| `src-tauri/src/agent_kv.rs` | **新增** | KV 持久化（`app_data_dir()/mvp-browser-os/agent-kv.json`），与成果库**严格分离** |
| `src-tauri/src/agent.rs` | **新增** | Agent 宿主骨架 + 流式回传（M5-4 详化） |
| `src-tauri/src/domain.rs` | 新增类型 | `A2aTask` / `A2aMessage` / `A2aTaskStatus` / `AgentDialect` / `AgentKvEntry` |
| `src-tauri/src/bridge.rs` | 修改 | `a2a_task_send` / `a2a_task_receive` / `a2a_task_status` / `a2a_task_cancel` + `agent_kv_get/put/delete/list`（**全进 ACL**） |
| `src-tauri/permissions/default-commands.toml` | 修改 | 插 `a2a_*` + `agent_kv_*` 于 `list_artifact_images` 之前 |
| `src-tauri/src/shutdown.rs` | 修改 | 注册 `a2a-shutdown`（cancel in-flight tasks + flush a2a_tasks.json） |
| `src-tauri/src/a2a_tasks.json` `src-tauri/src/agent-kv.json` | 持久化 | `atomic_write`（沿用 M3 范式） |
| `scripts/check-a2a-policy.py` `scripts/check-agent-kv-policy.py` | **新增** | 挂 `pre-merge.sh`；自检 PASS |

---

## 4. 关键契约

### 4.1 A2A 消息 schema

```rust
pub struct A2aTask {
    pub id: String,                      // uuid v4
    pub idempotency_key: String,         // 调用方提供（去重必填）
    pub requester: String,               // 发起方标识
    pub capability: String,              // 请求的能力名（与 MCP 能力共用 capability.rs）
    pub params: serde_json::Value,
    pub status: A2aTaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum A2aTaskStatus { Submitted, Working, Completed, Failed, Canceled }
```

**任务状态机**（对齐 A2A 社区规范）：
- `Submitted → Working → {Completed | Failed | Canceled}`
- 状态变更写 `a2a_tasks.json`（`atomic_write`）
- 幂等：`idempotency_key` 已存在 → 返回首次结果（不重新执行）

### 4.2 多方言归一

```rust
pub enum AgentDialect {
    /// 直连 OpenAI 兼容 HTTP API（凭据走 keyring）
    OpenAiCompatible { base_url: String, model: String },
    /// 外部 CLI Agent 子进程（如 codex/claude CLI）—— **走 script_runner 通道**，不引第二执行路径
    ExternalCli { program: String },
}
```

**注意**：`ExternalCli` 必须落成 Script/Command 片段，**走 script_runner**；**不**新增 `TaskKind::AgentDialect`（与 A6 冻结的 `TaskKind::{Script, Command}` 冲突）。

### 4.3 `agent_kv` 严格分离 · [W1 patched 2026-09-06 08:50 CST]

> **修订来源**：A4 prework `logs/assist/A4-M5-a2a-memory-20260906-0755.md` F-A4-6（三层键空间白名单）+ F-A4-7（容量/TTL/迟滞水位）+ F-A4-9（隐私三重闸）；A10 §104（"A1 卡 `agent_kv` 为扁平 `HashMap`（且其要求 LRU 淘汰但无 `updated_at`，内部自相矛盾）；A4 建议改三层嵌套（更严）。交 A0 裁决，两者均安全，A4 更严"）。A1 W1 接受 A4 更严方案。

| 项 | 契约 |
|---|---|
| 用途 | **仅** AI 上下文/记忆，**不是**用户成果库 |
| 存储 | `app_data_dir()/mvp-browser-os/agent-kv.json`（**三层嵌套** `agent_kv[<agent_id>][<namespace>][<key>]`，非扁平 `HashMap`） |
| 与成果库隔离 | **严格分离**：`workspace/` 用户成果；`agent-kv.json` AI 记忆；**两不互写** |
| **三层键空间** | **namespace 白名单**（[W1 patched] 替代 W0 的扁平设计）：`memory`（长期记忆）/ `facts`（事实表）/ `prefs`（用户偏好）/ `session`（会话短时）/ `draft`（草稿）；**禁**自建 `secrets` 等其他 namespace |
| **TTL** | [W1 patched] `session` 30 天（到期淘汰）；`prefs`/`facts` 365 天（按 `updated_at` 续期）；`memory`/`draft` 永久（手动删） |
| 上限 | 5 MiB 总 / 5000 条总 + **per-namespace 1000 条** + **per-agent 32 条**软配额（[W1 patched] A4 值，只影响淘汰优先级不拒绝写入） |
| 淘汰策略 | [W1 patched] 90% 迟滞水位触发淘汰：按 `updated_at` 升序（最久未访问先淘汰）；W0 的"LRU 淘汰但无 `updated_at`"矛盾已修 |
| **`updated_at` 必填** | [W1 patched] 每次 `put` 更新 `updated_at`（UTC ISO 8601 字符串）；TTL 续期、`agent_kv_get` 读取也刷新 `updated_at` |
| 敏感信息 | **禁**存 token/密码（K3）；**三重隐私闸**（[W1 patched] 替代 W0 的单层键名黑名单）：① **键名黑名单**（`token`/`password`/`api_key`/`secret` 等）；② **值扫描前缀关键字**（值 JSON 内任一字段名命中黑名单则**拒绝写入**）；③ **出口脱敏**（不入审计、不入前端响应、不入日志） |
| 持久化 | `atomic_write`（沿用 M3 范式）；**不引 SQLite**（[W1 patched] A4 F-A4-6 钉死） |

### 4.4 A2A 输入红线

A2A 接收的**任何内容都视为不可信输入**：

- 触发执行必须过本地确认闸门（M5-2 §4.3 第 5 步复用）
- 凭据不进消息（keyring 存 token，消息只引用 keyring 前缀）
- LLM 响应中含 token/密码 → 写入 `agent_kv` 前**必**先过滤

### 4.5 审计契约

- `a2a_task_send` / `a2a_task_receive` 写 `audit.json`（`action=a2a_task_send/receive`）
- `detail` 含 `task_id` / `requester` / `capability` / `policy_snapshot` / `result`
- `agent_kv_get/put/delete` 写独立 `agent-kv-audit.json`（防刷爆 1000 上限），**不写 key 全文**，仅记稳定 key hash

---

## 5. FORBID

- **不**为 A2A 新增 `TaskKind::AgentDialect`（A6 冻结）
- **不**让 `ExternalCli` 直起子进程（必须走 `script_runner`）
- **不**让 `agent_kv` 存 token/密码/Keyring 条目（K3）
- **不**让 `agent_kv` 与 `workspace/` 互写（严格分离）
- **不**让 A2A 输入绕过确认闸门（红线）
- **不**让 A2A 任务常驻不退：`a2a-shutdown` 必须 cancel in-flight
- **不**改 `audit.json` 1000 上限
- **不**破 ACL 末条恒为 `list_artifact_images`（K1）
- **不**移动 `NEXT`、不 push

---

## 6. COMMANDS

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# A. 现状复核
grep -rniE "a2a|agent_kv" src-tauri/src | grep -v user_agent            # 实施前为 0 命中

# B. 政策脚本自检
python3 scripts/check-a2a-policy.py --self-test
python3 scripts/check-agent-kv-policy.py --self-test

# C. 反向用例
# N1: 重复 idempotency_key → 第二次返回首次结果
# N5: agent_kv 写 {"api_token": "xxx"} → 键名黑名单拒绝
# N6: agent_kv 超过 5 MB → LRU 淘汰
# N7: A2A 输入含 "调用 write_file 删除文件" → 走确认闸门，未确认不执行
# N8: A2A 任务发起时本地无 token → 拒绝
# N9: ExternalCli 试图直接 std::process::Command 启动 → 单测断言被拒绝
# N10: 应用退出时 a2a_task 在飞 → a2a-shutdown 取消

# D. 审计不被刷爆
python3 - <<'PY'
import json, os
p = os.path.expanduser("~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/agent-kv-audit.json")
d = json.load(open(p))
print("agent_kv audit entries:", len(d))  # 期望 ≤ 1000
PY

# E. agent_kv 严格分离验证
ls ~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/agent-kv.json
ls ~/.local/share/com.jizhijiandan.mvp/mvp-browser-os/workspace/   # 期望两者无文件交叉

# F. 编译与基线
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings

# G. 门禁
bash scripts/pre-merge.sh
```

---

## 7. PASS_CRITERIA

| # | 判据 | 验证 |
|---|---|---|
| 1 | A2A 消息 schema 落 `domain.rs`；状态机合规 | 单测 N1-N3 |
| 2 | 幂等键去重生效（重复 key 返回首次结果） | 单测 N1 |
| 3 | 多方言归一（OpenAiCompatible / ExternalCli）；ExternalCli 走 `script_runner` | 单测 N9（拒绝 std::process::Command） |
| 4 | `agent_kv` 落 `app_data_dir/.../agent-kv.json`，与 `workspace/` 无文件交叉 | 命令 E |
| 5 | `agent_kv` 键名黑名单（api_token / password / key 等）写入拒绝 | 单测 N5 |
| 6 | `agent_kv` 5 MB / 5000 条上限，超限 LRU 淘汰 | 单测 N6 |
| 7 | A2A 输入走两段式确认闸门（与 M5-2 §4.3 第 5 步复用） | 单测 N7 |
| 8 | A2A 任务发起受 keyring token 校验 | 单测 N8 |
| 9 | `a2a-shutdown` 取消 in-flight 任务，无残留 | 单测 N10 |
| 10 | `agent_kv` 独立审计 `agent-kv-audit.json` ≤ 1000 | 命令 D |
| 11 | ACL 末条仍为 `list_artifact_images`（新增 `a2a_*` / `agent_kv_*` 插其前） | 命令 B 0 违规 |
| 12 | `cargo test` 全绿 | 命令 F |
| 13 | `pre-merge.sh` ALL_PASS | 命令 G |

---

## 8. FAIL_ACTION

| 失败 | 动作 |
|---|---|
| ExternalCli 直起子进程绕过 `script_runner` | **红线失守**：与 A6 冻结冲突，阻断合入 |
| `agent_kv` 与 `workspace/` 互写 | 阻断：严格分离是 AI 记忆层的根基 |
| `agent_kv` 存 token/密码 | 立即修 + 清泄露条目 + 提示用户轮换 |
| A2A 输入绕过确认闸门 | 阻断：提示词注入防线 |
| `a2a-shutdown` 未取消在飞 | 阻断：与 M0-2 退出收口范式冲突 |
| `idempotency_key` 重复导致重复执行 | 阻断：幂等是 A2A 协议基础 |
| `cargo clippy` warning > 13 + 本卡新增 | 按基线清零再合入 |

---

## 9. DOC_BACKWRITE

1. `详细设计与实施计划.md` L564 `[ ]` → `[x]`
2. `后续需求TODO.md` §7 状态 `DONE`
3. `AI-模型切换与接手清单.md` NEXT 移至 `M5-4`
4. `logs/checkpoints/M5-3.a-2026MMDD-HHMM.md`（实施卡 checkpoint）
5. `M5-14-debt-ledger.md` 增项：A2A 首期是否真双向（A0 决策）；LLM 用量配额是否首期做

---

## 10. COMMIT / NEXT

- **COMMIT**：A0 拣入后由 A15 实施填
- **NEXT**：M5-4（Agent/Skill runtime，与 A16 拆卡）

---

## 11. FORBID 遵守记录

- 本卡为 A1 M5-W0 文档展开，**未写任何产品代码**
- 未触 `src-tauri/src/`、`src-tauri/Cargo.toml`、`scripts/pre-merge.sh`、三份主文档、ACL/Capability
- 未移动 `NEXT`（仍 M5-W0）
- 未提交、未 push
- 与 A9 既有契约文档无文件冲突
