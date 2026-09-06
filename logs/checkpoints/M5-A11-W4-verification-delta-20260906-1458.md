# M5-A11 · W4 验证增量（Verification Delta After W4 Outputs）

```text
LANE=A11
STATUS=PASS（验证；集成门禁 ALL_PASS；2 项非阻断提示：① MCP --expect-pending 按设计 FAIL（产物已落地→PENDING→ACTIVE 翻转待 M5-2.b）；② A4/A5 W4 产品代码尚未入主仓，待 A0 集成后再验证）
BASE=f7ad35a（W4 dispatch HEAD；本地 master 与 origin/master 已同步）
HEAD=logs/checkpoints/M5-A11-W4-verification-delta-20260906-1458.md
FILES=logs/checkpoints/M5-A11-W4-verification-delta-20260906-1458.md
VERIFY=见 §1 全门表（实跑，非 stale）
CHECKPOINT=本文件
MERGE_NOTES=见 §5（W3 发现②已关闭：pre-merge 接入 check-mcp-policy；U-2 已关闭→M5-3.a 解锁；U-4 已立项→M5-3.b 路径开启；A4/A5 待集成）
NEXT=待 A0 集成 A4/A5 W4 产品代码后，A11 再出补验证增量；并建议 M5-2.b 把 check-mcp-policy 的 PENDING 码位翻 ACTIVE
```

> 依据：`PARALLEL_COMMAND_BOARD.md` §M5-W4 Parallel Dispatch（行 134-168）→ **A11 = START VERIFICATION：「One verification delta」**（after A4/A5 outputs）；scope `logs/assist/M5-A11-W4-*.md`、`logs/checkpoints/M5-A11-W4-*.md`。
> 范围声明：本增量**只产出验证文档，零产品代码、零策略脚本改动**（W4 的产品代码属 A2 `f8f1f49` 与 A3 `12f1cff`，由 A0 集成）。未 push。
> 姊妹件：W1 `M5-A11-W1-verification-delta-20260906-0830.md`、W2 `M5-A11-W2-verification-baseline-20260906-1342.md`、W3 `M5-A11-W3-verification-delta-20260906-1413.md`。

---

## 0. 启动门禁与调度匹配

```bash
cat .workspace-identity              # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                  # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git fetch origin && git pull --ff-only   # 已经是最新的（本地==origin/master，领先差额随 A0 推送清零）
git status --short --branch          # master...origin/master；仅 M5-0-overview / M5-1-core-workspace-split / M5-1.b-seam 三处 doc 脏（他 lane）+ ?? A8 W4 delta（他 lane），非本车道
git log --oneline -8                 # f7ad35a / f8f1f49 / 12f1cff / bdb0602 / e96c902 / 98a3b01 / 712a14c / a654f0c
```
- **调度匹配**：board 头部 `Current NEXT: M5-W4 parallel implementation`（行 7）与本增量一致。W4 产品代码 lane 为 A4/A5；但本波实际入主仓的产品代码是 A2 `f8f1f49`（seam，W3 的 M5-1.b 切片 1 收口）与 A3 `12f1cff`（M5-2 MCP 注册表/全局策略，W3 的 M5-2 切片）。本车道按 W4 A11 行交付单一验证增量。
- **工作树**：本车道 W3 提交 `bdb0602` 已随 A0 推送（领先清零）；当前他 lane 脏文件不属于本车道，按 Merge Rule 不动。

---

## 1. W4 验证矩阵（全门实跑，非 stale）

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **337 passed；0 failed**（329 + 8：seam.rs 2 + mcp.rs 6） | ✅ |
| core 边界门 self-test | `python3 scripts/check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7，坏样本=9） | ✅ |
| core 边界门 default | `python3 scripts/check-core-boundary.py` | `all invariants hold`（ACTIVE=7，**core 文件=3**） | ✅ |
| core 边界门 pending | `python3 scripts/check-core-boundary.py --expect-pending` | `CORE_PENDING_RESULT=NONE` | ✅ |
| M5-2 MCP 门 self-test | `python3 scripts/check-mcp-policy.py --self-test` | `MCP_POLICY_SELF_TEST=PASS`（ACTIVE=5，PENDING=9） | ✅ |
| M5-2 MCP 门 default | `python3 scripts/check-mcp-policy.py` | `MCP_POLICY=PASS`（无违规，EXIT 0） | ✅ |
| M5-2 MCP 门 pending | `python3 scripts/check-mcp-policy.py --expect-pending` | `MCP_PENDING_RESULT=FAIL`（按设计：已检出 M5-2 产物→应翻转 PENDING→ACTIVE，见 §3 提示①） | ⚠️ 设计内 |
| M2-4 执行通道 self-test | `python3 scripts/check-script-exec-policy.py --self-test` | `SELF_TEST_RESULT=ALL_PASS`（23 坏样本+1 好样本+码位完整性） | ✅ |
| tools 策略 self-test | `python3 scripts/check-tools-policy.py --self-test` | `self-test OK`（好样本零违规+16 坏样本全检） | ✅ |
| db 策略 self-test | `python3 scripts/check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（ACTIVE=14，PENDING=1） | ✅ |
| db 策略 default | `python3 scripts/check-database-policy.py` | `all invariants hold`（ACTIVE=14） | ✅ |
| db 策略 pending | `python3 scripts/check-database-policy.py --expect-pending` | `DB_PENDING_RESULT=NONE`（1 pending） | ✅ |
| scheduler 策略 self-test | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（ACTIVE=23，PENDING=0） | ✅ |
| scheduler 策略 default | `python3 scripts/check-scheduler-policy.py` | `all invariants hold`（ACTIVE=23） | ✅ |
| scheduler 策略 pending | `python3 scripts/check-scheduler-policy.py --expect-pending` | `SCHED_PENDING_RESULT=NONE` | ✅ |
| Rust fmt | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | `FMT_CLEAN` | ✅ |
| 工作树 diff | `git diff --check` | `DIFF_CHECK_CLEAN` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-*.js`=162.50 kB（gzip 58.28 kB），低于 IF-2 ~197 kB 红线 | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=ALL_PASS`（EXIT=0）** | ✅ |

---

## 2. A2 seam（切片 1，`f8f1f49`）验证 —— U-2 已交付，M5-3.a 解锁

**交付内容**：`src-tauri/src/core/seam.rs`（95 行）+ `core/mod.rs`（+7）+ `bridge.rs`（+44，Tauri 侧 trait 落地）。seam.rs 定义 `Progress`/`ProgressSink`/`PathResolver`/`RootsProvider` 三个薄抽象 trait + mock 实现测试。

| 验收项 | 验证结果 |
|---|---|
| 零 Tauri 依赖（core 边界） | ✅ seam.rs 仅 `use std::path::PathBuf; use serde::Serialize;`，无 `tauri`/`AppHandle`/`crate::bridge`；`check-core-boundary.py` 现扫描 **core 文件=3** 仍 PASS |
| 行为零变化 | ✅ `cargo test` 337（seam 增 2 测试，无行为改动）；其余 329 不变 |
| 对象安全 / 可用 | ✅ 测试 `progress_sink_is_object_safe`（`&dyn ProgressSink`）、`path_and_roots_seams_are_usable` 通过 |
| 越界落地在 bin 侧 | ✅ `TauriProgressSink`/`TauriPathResolver`/`TauriRootsProvider` 在 `bridge.rs`（bin crate），core 不反向依赖命令层 |
| 无新命令/ACL/UI | ✅ `git diff f8f1f49~1 f8f1f49 -- main.rs ACL bridge.ts types.ts` = 空 |

> **债务结清（U-2）**：本切片即 W3 时 A2 NEXT 所规划的「切片 1·2 注入 ProgressSink/PathResolver/RootsProvider」。**M5-3.a（agent_kv 纯存储逻辑）的独占阻塞 U-2 现关闭** → 待 A4 在 `core/agent_kv.rs` 注入 seam 即可推进（A4 W3 delta §3 已规划 `core` 自拥 `AGENT_KV_*` 常量，守 R-B3 不 `use crate::domain`）。

---

## 3. A3 MCP 注册表/全局策略（`12f1cff`）验证 —— U-4 已立项，M5-3.b 路径开启

**交付内容**：`src-tauri/src/mcp.rs`（193 行，纯注册表+纯策略）+ `domain.rs`（+63，`MCP_CAPABILITY_V1` 等 DTO/能力常量）+ `main.rs`（+2，`mod mcp;` 模块声明）+ `check-mcp-policy.py`（+16，PENDING→ACTIVE 过渡）+ `pre-merge.sh`（+14，接入门禁）。**零 rmcp / 零 tokio / 零 TcpListener / 零 server runtime / 零新 Tauri 命令**。

| 验收项（W4/W3 硬停止） | 验证结果 |
|---|---|
| 纯注册表+纯策略，无运行时 | ✅ `mcp.rs` 仅 `MCP_COMMAND_REGISTRY`（7 能力，映射 `core_api` 如 `workspace.read_file`）、`lookup_mcp_command`、`evaluate_mcp_command`（fail-closed）、`redact_mcp_url`（复用 `security_policy::redact_sensitive_url` 单一真源）；`#[allow(dead_code)]` 标记尚未被 server 消费 |
| 能力白名单单一真源 | ✅ `MCP_CAPABILITY_V1` 唯一定义于 `domain.rs`；注册表每项均在白名单内且双向覆盖（测试 `registry_entries_are_subset_of_capability_whitelist` 守奇偶） |
| fail-closed | ✅ 未知能力→Deny；`touches_fs` 且越路径根→Deny（测试 `unknown_capability_is_denied`/`fs_capability_outside_roots_is_denied`） |
| URL 脱敏单一真源 | ✅ `redact_mcp_url` 复用 `security_policy`，测试 `url_is_redacted_before_return` 验证 token/密码不回传 |
| 无新 Tauri 命令（守 W4 硬停止"新命令须原子含 ACL/源检/bridge/types"） | ✅ `main.rs` 仅 `mod mcp;`（非命令注册）；`git diff 12f1cff~1 12f1cff -- main.rs ACL bridge.ts types.ts` 仅 `main.rs`（模块声明），**无 ACL / bridge.ts / types.ts 改动** → 未暴露任何 `mcp_*` 命令 |
| 门禁三模式 | ✅ self-test `PASS`（ACTIVE=5，PENDING=9）/ default `PASS`（EXIT 0） |
| **W3 发现②已关闭：pre-merge 接入** | ✅ `pre-merge.sh` 现含 `check-mcp-policy.py --self-test` + default（行 416-419、534-536）；`bash scripts/pre-merge.sh` → `PRE_MERGE_RESULT=ALL_PASS` |

> **提示①（设计内，非阻断）**：`check-mcp-policy.py --expect-pending` 现返回 `MCP_PENDING_RESULT=FAIL`——脚本按设计在「检出 M5-2 产物（`MCP_CAPABILITY_V1`/`mcp.rs`）」时要求把 PENDING 码位翻为 ACTIVE，由 M5-2.b 的 rmcp/server 实现接管。该模式**不在 pre-merge 门禁内**，故不影响 `ALL_PASS`。建议 A0 在翻牌 M5-2.b 时落实此翻转。
>
> **债务立项（U-4）**：能力白名单单一真源（`MCP_CAPABILITY_V1`）+ 路径/URL 奇偶 + `check-mcp-policy.py` 的 `MCP_CAPABILITY_DRIFT`/`MCP_FS_TOOL_PATH_POLICY`/`MCP_TOOL_CALLS_COMMAND` 已就位，**M5-3.b（A2A/dialect 命令闸门）依赖的 U-4 路径已开启**（A4 W3 delta §3）。完整 server 集成（B1/B7/B8）仍待 A0 裁决、属 M5-2.b。

---

## 4. A4 / A5 W4 产品代码状态（待集成）

W4 dispatch 打开 A4（M5-3 agent_kv 契约切片）与 A5（M5-4/5 Agent/Skill 域+命令策略壳）产品代码 lane，但**截至本增量，主仓尚未见其产品代码落地**：

- `ls src-tauri/src/ | grep -iE 'agent|a2a|skill|mcp|capability|seam'` → 仅 `mcp.rs`（A3）；无 `agent_memory.rs`/`a2a.rs`/`agent.rs`/`skills.rs`。
- `ls scripts/ | grep -iE 'agent|skill'` → 无 `check-agent-memory-policy.py`/`check-agent-skill-policy.py`。
- 工作树无 A4/A5 未提交产品文件（仅他 lane doc 脏 + A8 W4 delta 未跟踪）。

**结论**：A4/A5 的 W4 产出尚未进入本主仓（应仍在各自 worktree / 待 A0 集成）。本车道对 A4/A5 的验证**推迟到 A0 集成后**再出补增量；届时重点核：bounded/privacy store、no credential/body leakage、no new dependency、命令须原子含源检/ACL/bridge/types/策略/tests。

---

## 5. 集成门禁 + Before-A0 冲突扫描

- **集成门禁**：`PRE_MERGE_RESULT=ALL_PASS`（EXIT=0）。相对 W3 baseline 的 `ALL_PASS` 维持不变；W4 新增 8 测试不影响红灯状态。
- **命令表面扫描**：seam `f8f1f49` 未碰 main.rs/ACL/bridge.ts/types.ts；mcp `12f1cff` 仅 `main.rs`（`mod mcp;`），无新 Tauri 命令、无 ACL 顺序问题、无 bridge/main/types 命令奇偶（因无命令新增，平凡一致）。
- **空文件扫描**：W4 两产品提交均含实质内容（seam.rs 95 / mcp.rs 193 / domain.rs +63 等），无空文件；他 lane 脏 doc 与 A8 delta 不属本车道范围。
- **NEXT 一致性**：board 头部 `Current NEXT: M5-W4` 与 §M5-W4 dispatch 一致。
- **lane scope 漂移**：W4 实际产品代码仅 A2（seam，属 W3 M5-1.b 授权）/A3（MCP 注册表，属 W3 M5-2 授权），无越界；A4/A5 尚未落地，待核。

---

## 6. 债务台账更新（相对 W3）

| 项 | W3 状态 | W4 状态 |
|---|---|---|
| U-2（core seam：ProgressSink/PathResolver/RootsProvider） | 未交付（M5-3.a 阻塞） | **已关闭**（f8f1f49）→ M5-3.a 解锁 |
| U-4（capability 真源） | open（A3 仅落门禁） | **已立项**（12f1cff：单一真源+奇偶+策略）→ M5-3.b 路径开启 |
| W3 发现①（seam 未交付→M5-3.a 阻塞） | 待解 | **已关闭**（seam 已交付） |
| W3 发现②（check-mcp-policy 未接 pre-merge） | 待 A0 收口 | **已关闭**（12f1cff 接入 pre-merge） |
| MCP --expect-pending | NONE（产物未落地） | 按设计 FAIL（产物已落地→PENDING→ACTIVE 翻转待 M5-2.b） |
| 集成门禁 | ALL_PASS | ALL_PASS（稳） |
| cargo test | 329 | 337（+8） |

---

## 7. 声明（避免误读）

- 本车道**零产品代码、零策略脚本改动**（W4 的 seam 属 A2、MCP 注册表属 A3，均由 A0 集成）。本增量仅新增验证文档。
- 未 rebase、未 push（board Merge Rule：仅 A0 推送）。
- 全部结论基于 §1 实跑证据；未引用旧报告（遵守 IF-5 不 stale 要求）。
- 提交动作：仅 `git add` 本文件后提交，不带入他 lane 脏文件（`M5-0-overview.md` 等）与 A8 未跟踪 delta，不 push。
