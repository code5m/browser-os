# M5-A11 · W2 验证基线 + 前向验收准则（Verification Baseline & Forward Criteria）

> Lane：**A11（M5 verification）** ｜ 路由 `AI:BALANCED / R:medium`
> 时间：2026-09-06 13:42 CST ｜ 作者：CodeBuddy Hy4
> 依据：`PARALLEL_COMMAND_BOARD.md`（M5-W1 Implementation Dispatch @08:35 已完成；**无独立 M5-W2 dispatch 小节**）+ 用户指令「按 M5-W1/M5-W2 职责继续推进，优先做自己 Lane 的下一张最小可集成切片」。
> 范围声明：本 checkpoint **只产出验证文档，零产品代码改动**（严守 board：A11 不得编辑产品代码/策略脚本，策略脚本修复须 A0 指派）。
> BASE = `854bc40`（feat(M5): add core boundary gate）；HEAD（本车道）= 本文件，未 push。
> 姊妹件：M5-W1 已交付 `logs/checkpoints/M5-A11-W1-verification-delta-20260906-0830.md`。

---

## 0. 启动门禁与调度匹配确认（WORKSPACE_IDENTITY 要求）

```bash
cat .workspace-identity            # WORKSPACE_ID=BACKV3_MAIN / EXPECTED_BRANCH=master  ✅
pwd                                # /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3  ✅
git status --short --branch        # ## master...origin/master [领先 4]；含他 lane 未提交 M5 文档  ✅(见下)
git log --oneline -3               # 854bc40 / 0d08016 / 404f514  ✅
```

**调度匹配核对（用户要求项）**：
- **工作树干净度**：本车道自身 A11 提交（`0d08016` W1 delta）已干净落地；但当前工作树含**他 lane 未提交 M5 文档**（`M5-0-overview.md`、`M5-1-core-workspace-split.md`、`M5-A1-expansion`、`M5-1.b-seam-trait-injection-and-b-extract.md` 等由 A1；`A4-M5-a2a-memory-*w2-slice*`、`A7-M5-graph-core-*`、`A5-M5-agent-skill-*` 等由 A4/A5/A7）。属并行多 lane 常态，非本车道引入，按 board「Merge Rule」归 A0 集成，本车道不动。
- **NEXT/M5 调度匹配**：board 头部 `Current NEXT: M5-W1 core boundary implementation`（行 7）——**M5-W1 已完成**（A2 落边界门、A11 已验证 0830）。board **尚无独立 M5-W2 Implementation Dispatch 小节**（仅 M5-W1 08:35、M5 Wave 0 07:55、Integration Fix Wave）。用户以「M5-W2」指代「边界门之后的下一批抽取波」（M5-1.b seam / M5-3 agent_kv / M5-2 MCP 等）。本 checkpoint 据此推进，作为 A11 的 W2 最小可集成切片（验证基线 + 前向准则），待 A0 正式签发 M5-W2 dispatch。

---

## 1. M5-W2 起始基线（全门实跑，非 stale）

> 准则：吸取 IF-5 教训，本次以当前实跑为准，不复用陈旧计数。

| 门 | 命令 | 结果 | 判定 |
|---|---|---|---|
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | **329 passed；0 failed** | ✅ |
| core 边界门 self-test | `python3 scripts/check-core-boundary.py --self-test` | `CORE_POLICY_SELF_TEST=PASS`（ACTIVE=7，坏样本=9） | ✅ |
| core 边界门 default | `python3 scripts/check-core-boundary.py` | `all invariants hold`（core 文件=2） | ✅ |
| core 边界门 pending | `python3 scripts/check-core-boundary.py --expect-pending` | `CORE_PENDING_RESULT=NONE` | ✅ |
| tools 策略自检 | `python3 scripts/check-tools-policy.py --self-test` | `self-test OK`（好样本零违规 + 16 坏样本全检） | ✅ |
| db 策略 self-test | `python3 scripts/check-database-policy.py --self-test` | `DB_SELF_TEST_RESULT=PASS`（ACTIVE=14，PENDING=1） | ✅ |
| db 策略 default | `python3 scripts/check-database-policy.py` | `all invariants hold`（ACTIVE=14） | ✅ |
| db 策略 pending | `python3 scripts/check-database-policy.py --expect-pending` | `DB_PENDING_RESULT=NONE` | ✅ |
| scheduler 策略 self-test | `python3 scripts/check-scheduler-policy.py --self-test` | `SCHED_SELF_TEST_RESULT=PASS`（ACTIVE=23，PENDING=0） | ✅ |
| scheduler 策略 default | `python3 scripts/check-scheduler-policy.py` | `all invariants hold`（ACTIVE=23） | ✅ |
| 工作树 diff 检查 | `git diff --check` | `DIFF_CHECK_CLEAN` | ✅ |
| 前端构建 | `npm run build` | ✓ built；`index-*.js`=162.50 kB（gzip 58.28 kB），低于 IF-2 ~197 kB 红线 | ✅ |
| **集成门禁** | `bash scripts/pre-merge.sh` | **`PRE_MERGE_RESULT=FAIL`** | ❌ 见 §2 |

**基线小结**：除 `pre-merge.sh` 外，所有单门绿；`cargo test` 329、`npm run build` 通过、`git diff --check` 干净。pre-merge 红 = 两项非本车道问题（§2）。

---

## 2. pre-merge FAIL 根因（精确归因，A11 不修）

`pre-merge.sh` 输出关键行：
```
[pre-merge] FAIL: cargo fmt main
[pre-merge] FAIL: check-script-exec-policy.py --self-test
[pre-merge] FAIL: check-script-exec-policy.py（执行通道安全边界被破坏）
[pre-merge] PRE_MERGE_RESULT=FAIL
```

### 原因 A — `cargo fmt main`（IF-1 残留，DB lane 债）

```bash
$ cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
Diff in src-tauri/src/database.rs:1132   # let mut deadline = QueryDeadline::new(...) 未折行
Diff in src-tauri/src/domain.rs:1699    # assert_eq!(DB_MAX_SQL_BYTES, ...) 未折行
$ git status --short -- src-tauri/src/database.rs src-tauri/src/domain.rs
 M src-tauri/src/database.rs
 M src-tauri/src/domain.rs
```
- 归属：`database.rs`/`domain.rs` 为 M4-1/2（DB lane A3/A4）未提交改动，未格式化。
- 对应 board `Integration Fix Wave` 红灯 **IF-1**（"`cargo fmt main` fails on M4 Rust files"，owner A3/A4→A0）。
- **非 M5-W2 引入、非本车道可修**（A11 不碰产品代码）。建议 A3/A4 跑 `cargo fmt` 或 A0 终合时统一 fmt。

### 原因 B — `check-script-exec-policy.py` 自检坏 + 默认误报（策略脚本漂移）

```bash
$ python3 scripts/check-script-exec-policy.py --self-test
SELF_TEST_RESULT=FAIL
FAIL: 好样本本应零命中，实际命中：['EXEC_TIMEOUT_NOT_LAYERED']
FAIL: [EXEC_TIMEOUT_NOT_LAYERED] 变异原文片段不存在，夹具已失效（必须同步更新 MUTATIONS）：'pub const HARD_GRACE_SECS: u32 = 5;'
$ python3 scripts/check-script-exec-policy.py
EXEC_POLICY_RESULT=FAIL
  EXEC_TIMEOUT_NOT_LAYERED
```
根因（代码侧）：常量收口（`script_runner.rs` 的 `HARD_GRACE_SECS` 已迁至 `domain.rs:1017` 并 re-export）：
```rust
// src-tauri/src/script_runner.rs:48
pub use crate::domain::{HARD_GRACE_SECS, MAX_TIMEOUT_SECS};   // 现在是 re-export，非本地 const 定义
// src-tauri/src/domain.rs:1017
pub const HARD_GRACE_SECS: u32 = 5;
```
而策略脚本检测逻辑（行 189）仍要求 `script_runner.rs` 内存在**本地** `pub const HARD_GRACE_SECS\s*:`：
```python
# scripts/check-script-exec-policy.py:189
re.search(r"pub const HARD_GRACE_SECS\s*:", runner)
# 行 311 变异夹具锚点仍是 'pub const HARD_GRACE_SECS: u32 = 5;'
```
- 结论：检测与「常量收口到 domain + re-export」的新形态失配 → 默认扫描对真实仓库**误报** `EXEC_TIMEOUT_NOT_LAYERED`，且 self-test 变异锚点失效（合成好样本不再含本地 const 定义）。
- 该「常量收口」属 **M5-1.b 切片 0b**  groundwork（见 `src-tauri/src/core/mod.rs:15` 注释），故本次漂移由 M5-1.b 前置工作触发，非本车道引入。
- **A11 不修**（策略脚本修复须 A0 指派；board：A11「policy scripts only if A0 explicitly assigns」）。建议 A0/A10：将 `EXEC_TIMEOUT_NOT_LAYERED` 检测扩展为「`script_runner.rs` 含 `pub const HARD_GRACE_SECS` **或** `pub use crate::domain::HARD_GRACE_SECS`」，并同步更新 self-test 变异锚点/合成好样本。

---

## 3. M5-W2 前向验收准则（各下一张切片落盘后，A11 将逐一实跑）

> 总闸门：每个 M5-W2 切片必须**保持 pre-merge ALL_PASS**（即先消 §2 两项红灯），且**行为零回归**（cargo test 不减）。

### 3.1 M5-1.b seam 特质注入 + B-抽取（owner A2/A1，卡片已 staged）
- [ ] `mvp_core` 边界门 `check-core-boundary.py` self-test/default 仍 PASS（不得引入 tauri / AppHandle / crate::bridge / 第二执行路径）。
- [ ] 常量收口（`HARD_GRACE_SECS`/`MAX_TIMEOUT_SECS`/`MAX_TEXT_FIELD_BYTES` → `domain.rs`）**必须同步修 §2 原因 B 的策略脚本漂移**，否则 pre-merge 持续红。
- [ ] `cargo fmt --check` 干净（消 IF-1）。
- [ ] 行为不变：cargo test 仍 329；`cargo check` 无新增警告。

### 3.2 M5-3.a agent_kv 纯存储（owner A4，W2 slice 当前 docs-only）
- [ ] 落 `src-tauri/src/core/agent_kv.rs`，经边界门；**依赖 seam（U-2：Clock/AuditSink）须已存在**（A1 M5-1.b-seam 文件在计划内），否则 M5-3.a 代码态阻塞。
- [ ] C-5 值扫描须含字符串值（`{"note":"sk-xxx"}` 类）、C-6 per-agent=1MiB 字节上限（非 32=agent 数误用）——见 A4 W2 slice。
- [ ] 行为不变；边界门 PASS。A4 当前 W2 patch **仅文档、0 行产品代码**（已核验 `grep -c '^+++ b/src-tauri'` = 0），待 seam 就绪后交代码 patch。

### 3.3 M5-3.b a2a/dialect 命令闸门（owner A4 + A5，依赖 capability.rs 真源 U-4）
- [ ] `capability.rs` 须落 core（U-4 未决，仅阻塞 M5-3.b）；命令经 source check + ACL（末条前插）。
- [ ] 复用 M2-4 执行通道，无第二执行路径。

### 3.4 M5-2 MCP（owner A3，阶段二独立 crate）
- [ ] 独立 crate/bin 仅依赖 `mvp-core`；主二进制依赖图零变化（D-5 缓解：`check-core-boundary.py` 的 `CORE_TREE_TAURI`/`CORE_DEP_NOT_ALLOWLISTED` 转 ACTIVE 后须 PASS，禁 `tokio`/`rmcp` 直引主二进制）。
- [ ] 无新 Cargo/npm 依赖进入主包；MCP 专用来源通道为 core 内抽象（D-6）。

### 3.5 通用硬停止（沿用 M5-W1 + board M4 Guardrails）
- [ ] 无新 Cargo/npm 依赖（除非 A0 裁决）；无新 Tauri 命令/ACL（除既定末条前插）；无第二执行路径。
- [ ] `check-core-boundary.py` 持续挂 pre-merge；`git diff --check` 干净。

---

## 4. A11 下一张切片（待办）

| 触发条件 | A11 动作 |
|---|---|
| M5-1.b seam 代码落盘 | 重跑 §1 全门 + §3.1 准则；重点确认 §2 原因 B 被 A2/A0 修掉、pre-merge 转 ALL_PASS |
| M5-3.a agent_kv patch 落盘 | 重跑边界门 + cargo test；核对 C-5/C-6 修正；产 M5-3 验证增量 |
| M5-2 MCP crate 落盘 | 重跑边界门（CORE_TREE_TAURI/DEP 转 ACTIVE 后）；核对 D-5/D-6 |
| A0 修 §2 两红灯 | 复跑 pre-merge 确认 ALL_PASS，回填本 checkpoint §2 状态 |

---

## 5. 声明（避免误读）

- 本车道**零产品代码、零策略脚本改动**（board：A11 不得编辑产品代码；策略脚本修复须 A0 指派）。§2 两红灯为**归因报告**，非本车道修复项。
- 未 rebase / 未 push（board Merge Rule：仅 A0 可推送）。本 checkpoint 为新增独立文档。
- 结论基于实跑：cargo test / pre-merge.sh / check-*.py 三模式 / npm run build / git diff --check（§1 全表），非文档互证。
- 提交动作：仅 `git add` 本文件后提交（不带入他 lane 未提交 M5 文档），不 push。
