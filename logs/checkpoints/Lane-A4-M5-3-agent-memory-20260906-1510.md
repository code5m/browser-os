# Lane A4 · M5-3 agent memory KV 契约切片（W4 产品代码整包）

> 续 W0(`0755`) / W1(`0825-w1-delta`) / W2(`1336-w2-slice`) / W3(`1410-w3-delta`)。
> 本文件是 **M5-W4 Parallel Dispatch（board 17:55，mainline `f7ad35a`）** 下 Lane A4 的产品代码整包交付。
> 依据 board §M5-W4：`A4 = START PRODUCT CODE`，任务 = “Implement first M5-3 A2A/agent memory KV contract slice: DTOs, validation, capacity/privacy policy, pure store helpers or JSON persistence shell; no network protocol and no background runtime.”

## 0. 调度匹配自检

| 项 | 实测 | 结论 |
|---|---|---|
| `WORKSPACE_IDENTITY` / `pwd` / branch | `BACKV3_MAIN` / 匹配 / `master` | ✅ |
| `git pull --ff-only` | 已拉到 `f7ad35a`（W4）+ `f8f1f49`（A2 seam）+ `12f1cff`（A3 M5-2） | ✅ |
| NEXT/M5 | board §M5-W4：仅 A4 + A5 可动产品代码；A4 拥有 M5-3 KV 契约切片 | ✅ 本包合规 |
| 是否越界 | 仅改 `domain.rs` / `main.rs` / `agent_memory.rs`（bin 侧，非 `core/`）/ `check-agent-memory-policy.py` / `pre-merge.sh`；**无命令、无网络、无后台 runtime、无新依赖** | ✅ |

## 1. W3→W4 依赖清算（历史性解锁）

| 阻塞项 | W3 状态 | W4 现状 | 结论 |
|---|---|---|---|
| **U-2**（seam） | `core/` 无 seam | A2 `f8f1f49` 落地 `core/seam.rs`（含 `ProgressSink`/`PathResolver`/`RootsProvider`），`core/mod.rs` 已 `pub mod seam` | **已解**：`agent_kv_default_path` 用 `PathResolver::base_dir`（A2 评审 seam 用法）；clock 改参数化 `now_secs`（A2 未提供 `Clock` seam，本切片不依赖） |
| **U-4**（capability 真源） | `capability.rs` 不存在 | A3 `12f1cff` 把 `MCP_CAPABILITY_V1` 落 `domain.rs:1540` + `mcp.rs` 注册表（bin 侧） | W4 A4 只做 KV 契约（**无命令**），不触发 capability 闸门 → **不影响本切片**；M5-3.b 后续消费 `McpGlobalPolicy` |
| **C-5/C-6**（A1 卡文案错误） | 卡仍错 | 本切片**直接在实现里用正确语义**，不再依赖卡文案修正 | **已解（代码态）**：C-5 字段名+字符串值双扫；C-6 per-agent=1MiB 字节 + agent 数上限 32 独立 |

## 2. 交付物

- `src-tauri/src/agent_memory.rs`（新）：M5-3 agent memory KV 契约层
  - DTO：`AgentKvRecord` / `AgentKvRecordSummary`（列表出口无 value）/ `AgentKvAuditEntry`（仅 key 哈希）/ `AgentKvNamespace`（`self`/`shared`/`peer`）。
  - 隐私三重闸（C-5 修正）：`scan_json_value` 递归扫**字段名 + 字符串值**（`sk-/AKIA/Bearer /eyJ/-----BEGIN/...`），任一命中 fail-closed 拒绝。
  - 容量三不变量（C-6 修正）：总 5MiB / 5000 条 / per-namespace 1000 / **per-agent 1MiB 字节（软配额，仅影响淘汰优先级）** / **agent 总数 32（独立不变量）**；超限按 LRU（最久未更新）淘汰；**永久记录计入字节上限、超容优先淘汰**。
  - TTL：`expires_at_secs`，读取按注入 `now` 判过期；`collect_garbage` 主动回收。
  - 审计：`redact_for_audit` 仅产 `key_hash`，**绝不序列化 value**。
  - 持久化 shell：`to_json`/`from_json`/`save`/`load` + `agent_kv_default_path(resolver)`（seam）。
- `src-tauri/src/domain.rs`：新增 `AGENT_KV_*` 容量常量（单一真源，受策略脚本守门）。
- `src-tauri/src/main.rs`：注册 `mod agent_memory;`。
- `scripts/check-agent-memory-policy.py`（新）：5 个 ACTIVE 码位 + 自测/默认/`--expect-pending` 三模式，把 C-5/C-6 修正钉成机器门禁。
- `scripts/pre-merge.sh`：在 `run_pre_merge` 与 `run_self_test` 接入该门禁（M5-2 与 M5-4/5 之间）。

## 3. 自测（全绿）

- `cargo fmt --check`：clean。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过（仅 A5 文件 pre-existing 未用 import 警告，与本 Lane 无关）。
- `cargo test ... agent_memory`：**11/11 通过**
  - `constants_are_consistent`（T-self-1 跨模块常量一致性）、`rejects_sensitive_key_name`（T-priv-1）、`rejects_sensitive_string_value`（T-priv-2：C-5 修正，拒 `{"note":"sk-xxx"}`）、`rejects_jwt_value`（T-priv-3）、`agent_count_cap_is_32`（T-cap-2：C-6 修正）、`per_agent_byte_quota_evicts_lru`（T-cap-3：单 agent >1MiB 淘汰最旧）、`permanent_records_count_toward_byte_cap`（T-ttl-3b：永久计入 5MiB）、`ttl_expiry_returns_none`、`audit_contains_no_value`、`json_round_trip_preserves_record`、`default_path_uses_seam`（T-seam-1）。
- `python3 scripts/check-agent-memory-policy.py --self-test`：**PASS（ACTIVE=5）**。
- `python3 scripts/check-agent-memory-policy.py`（默认扫描真实仓库）：**PASS（无违规）**——即本实现已满足全部 5 个码位。
- `git diff --check HEAD -- src-tauri scripts`：exit 0（干净）。

## 4. 给各 Lane 的 NEXT

- **给 A0**：本切片可直接 drop（patch 见同目录 `.patch`）。与 A5 在 `domain.rs` 的冲突由 A0 按 merge 顺序处理（A4 改动仅为追加常量块，低冲突）。
- **给 A1**：A1 M5-3 卡 §4.3 的 C-5/C-6 文案仍错（W3 delta 已给 drop-in 修正）；本切片已用正确语义实现，建议把卡文案对齐实现。
- **给 A2**：`PathResolver` 已被 `agent_kv_default_path` 消费，可供 A2 评审 seam 用法。
- **给 A3**：M5-3.b（a2a/dialect 命令）后续可消费 `McpGlobalPolicy`（U-4 真源已在 `domain.rs`/`mcp.rs`）。
- **给 A5**：`domain.rs`/`security_policy.rs` 同在 W4 改动，集成时注意常量块与各自 policy 函数互不侵入。

## 5. LANE 输出模板

```
LANE=A4
STATUS=PASS
BASE=f7ad35a
HEAD=logs/checkpoints/Lane-A4-M5-3-agent-memory-20260906-1510.md (patch 同目录)
FILES=src-tauri/src/agent_memory.rs, scripts/check-agent-memory-policy.py, src-tauri/src/domain.rs, src-tauri/src/main.rs, scripts/pre-merge.sh
VERIFY=cargo fmt --check 干净；cargo check 通过；cargo test agent_memory 11/11；check-agent-memory-policy.py --self-test PASS 且默认扫描 PASS；git diff --check 干净
CHECKPOINT=logs/checkpoints/Lane-A4-M5-3-agent-memory-20260906-1510.md
MERGE_NOTES=M5-3.a KV 契约首期切片落地（DTO/隐私双扫/容量三不变量/TTL/审计脱敏/JSON 持久化 shell/策略门禁）；U-2 经 PathResolver 消费解、U-4 不影响本切片、C-5/C-6 已在代码态修正；无命令/无网络/无后台 runtime/无新依赖
NEXT=M5-3.b（a2a/dialect 命令闸门，依赖 A3 McpGlobalPolicy）待 W4/W5 产品代码波；本切片为 M5-3.a 完整实现包
```
