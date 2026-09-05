# A10 · Batch SCHED-1 安全复核（A6 + A7）

> Lane: `A10` — M4 security review（`AI:DEEP / R:xhigh`）
> 时间: 2026-09-05 23:19 CST
> Base: `85d2d7b`（`master`；`git fetch` + `git pull --ff-only` → 已经是最新的）
> 批次: **Batch SCHED-1**（board §Batch Implementation Dispatch → Lane A10 → Batch SCHED-1: A6+A7 together）
> 范围: **只读复核**。本轮**未改任何产品代码、未改任何策略脚本**（board：A10「May edit policy scripts only if A0 explicitly assigns a fix」；A0 未指派，故 S1-2 我**只报不修**）
> 前序: `A10-M4-security-review-20260905-2240.md`（G-1~G-12）、`A10-M4-security-recheck-A1A2A6-20260905-2345.md`（R-1~R-11）。本文件为增量。

---

## 1. 批次状态

| Lane | 本批产出 | 评审结论 |
|---|---|---|
| **A6**（M4-5.d 夹具） | `scripts/check-scheduler-policy.py`（34 319 B）+ `pre-merge.sh` 接入 + `logs/checkpoints/A6-M4-5.d-20260905-2350.md` | **PASS**（自检 20 坏样本全检出） |
| **A7**（M4-6 后端） | `src-tauri/src/tasks.rs`（新，含 cron 解析/校验/测试）、`domain.rs`、`workspace.rs` 改动；**无 `scheduler.rs`、无 `task_*` 命令** | **PASS_WITH_DEBT**：纯逻辑层安全达标，但 `cargo` 警告激增 |

**批次裁定：`PASS_WITH_DEBT`**。5 条发现（S1-1 ~ S1-5），其中 **1 条【阻断】**（S1-2 会让 pre-merge 默认模式变红）。

---

## 2. 时间线说明（重要，避免误判为结论反复）

本批评审期间工作树被并发 lane 持续改写，我的取证按**观测时点**如实记录，不以后续状态倒填：

| 时点 | `check-scheduler-policy.py` | `--self-test` | 默认扫描 |
|---|---|---|---|
| 23:0x（首次取证） | 29 660 B | **EXIT=1**，3 个码位无坏样本覆盖（`SCHED_AUDIT_PER_RUN_EVENT` / `SCHED_TOKIO_CRON_DEP` / `SCHED_TOKIO_PROMOTED`）；连跑 3 次 md5 一致，确定性失败 | EXIT=0 |
| 23:07（被并发修复后） | 34 319 B | **EXIT=0**，`PASS: 2 好样本 + 20 坏样本全检出（含变异防呆）；ACTIVE=14 PENDING=6` | **EXIT=1**（见 S1-2） |

→ S1-1 记录为「**观测到的真实缺陷、已被他人在我复核期间修复**」，保留证据链；S1-2 为**当前仍然生效的阻断项**。

---

## 3. 发现

### S1-1【已修复·留档】夹具自检曾确定性失败（3 码位无坏样本覆盖）

**观测**（23:0x，连跑 3 次输出 md5 完全相同，`EXIT=1`）：

```text
SCHED_SELF_TEST_RESULT=FAIL
  x 以下码位没有任何坏样本覆盖：['SCHED_AUDIT_PER_RUN_EVENT', 'SCHED_TOKIO_CRON_DEP', 'SCHED_TOKIO_PROMOTED']
```

**严重性（当时）**：board §Batch Implementation Rule 第 4 条要求交付「self-verifying」包；且 `pre-merge.sh` 已接入 `--self-test`，该项失败会直接打红集成门禁。
**现状**：23:07 被并发 lane 修复（文件 29 660 → 34 319 B），现为 `PASS … 20 个坏样本全部检出（含变异防呆）`。
**留档理由**：证据链完整（3 次确定性复现），若后续回归可据此定位；同时提醒 A0：该文件在本轮内被**同一 lane 二次改写**，集成时须确认取到的是最终版。

### S1-2【阻断】`SCHED_CRON_MACRO_SUPPORT` 误报：默认扫描命中 A7 测试中的拒绝用例，pre-merge 默认模式变红

**实测**：

```text
$ python3 scripts/check-scheduler-policy.py
SCHED_POLICY_RESULT=FAIL
  SCHED_CRON_MACRO_SUPPORT:tasks:"@daily"
EXIT=1
```

**根因（已定位到行）**：`src-tauri/src/tasks.rs:591` 是

```rust
    #[test]
    fn t_sched_c2_invalid_cron_expressions_are_rejected() {
        for bad in [
            "0 0 0 0 0 0",   // 6 段（带秒）
            "@daily",          // 宏        ← 第 591 行
            …
```

该处位于 `#[[cfg(test)]]`（模块起始于 `tasks.rs:523`）**之内**，是 A7 为验证「宏一律拒绝」而写的**非法输入清单**。夹具的 `CRON_MACRO_RE` 对 `tasks` 源码做**原文正则扫描**，把测试里的坏样本字符串当成实现，遂误报。

**为什么这是误报而非真实违规**：A6 契约 §4.4 与 A7 实现一致要求「不支持 `@reboot`/`@daily` 等宏，非法输入定义期拒绝」；`tasks.rs` 中出现该串恰恰证明**拒绝逻辑存在且有测试**，与码位意图相反。

**影响**：pre-merge 中 `python3 check-scheduler-policy.py || pm_fail "check-scheduler-policy.py（调度契约被破坏）"` 会**持续失败**，A0 集成被阻断。

**修复建议（须 A0 指派；A6 夹具与 A7 实现均可改，建议改夹具）**：
1. **首选**：源码扫描前剔除 `#[cfg(test)]` 区域（夹具已有 `strip_comments`，需补 `strip_test_modules`；`rust_fn_body` 一类取函数体的辅助也可复用）。
2. 次选：宏检测只作用于**非字符串字面量**的上下文（避免命中任何 `"@daily"` 字面量）。
3. 无论选哪种，**必须为这条规则补一个「测试区含宏字面量 → 不报」的好样本**，防止回归（夹具 `--self-test` 目前只验证「真实仓库 + 合成参考实现」两个好样本，未覆盖此场景）。

### S1-3【低】`pre-merge.sh` 帮助文本与日志中的码位计数已过期

**事实**：两处均写「**9 ACTIVE 码 + 8 pending 码位**」（`scripts/pre-merge.sh:91`、`:349`），而脚本实际为 **`ACTIVE_CODES` 14 条、`PENDING_CODES` 6 条**（`check-scheduler-policy.py:75-105`，且自检输出自证 `ACTIVE=14 PENDING=6`）。
**成因**：A6 契约 §8.1/§8.2 初版为 9+8，后按 board 23:55 派工追加了 `SCHED_TOKIO_PROMOTED`、`SCHED_TOKIO_CRON_DEP`、`SCHED_SHUTDOWN_NOT_REGISTERED`、`SCHED_SHUTDOWN_ORDER`、`SCHED_AUDIT_PER_RUN_EVENT` 共 5 条 ACTIVE，但 pre-merge 文案未同步。
**风险**：低（仅文案），但会误导后续 lane 判断覆盖范围。建议 A6 或 A0 顺手订正为「14 ACTIVE + 6 pending」。

### S1-4【中高】`cargo` 警告 2 → 27，触发 build-metrics 门禁；且与 A2 既有约定不一致

**实测**：

```text
$ bash scripts/pre-merge.sh
…
[pre-merge] FAIL: build metrics regression vs logs/m0-build-metrics/build-metrics-4f0e8ab.json
[pre-merge] PRE_MERGE_RESULT=FAIL          （EXIT=1，全批唯一 FAIL）

$ python3 scripts/measure-build-metrics.py --compare … --skip-build
{ "exceeds_growth_limit": false, "deltas": { "total_bytes_pct": 14.6, "cargo_warnings": 25 },
  "warnings_increased": true }              （EXIT=1）

$ cargo check --manifest-path src-tauri/Cargo.toml | grep ^warning
… `mvp-browser-os` (bin "mvp-browser-os") generated 27 warnings
```

**新增警告几乎全部是 `never used` 死代码**，来源为 A7 新落地的契约类型：

```text
struct `TaskDef` / `TaskRunRecord` / `RetryPolicy` is never constructed
enum `TaskTrigger` / `TaskKind` / `MissedRunPolicy` / `RetryBackoff` / `TaskRunTrigger` is never used
function `default_task_enabled` / `default_misfire_grace_secs` / `default_catch_up_limit` is never used
method `delay_secs` / `as_str` is never used
```

**关键不一致**：A2 在 `domain.rs` 为同类「暂无消费者的契约类型」统一加了 `#[allow(dead_code)]`（**26 处**），而 A7 的 `tasks.rs` 中 `allow(dead_code)` 数量为 **0**。
**评估**：这不是安全问题，但**当前确实打红了 build-metrics 门禁**，属集成阻塞项。
**建议**：
1. A7 落地 `task_*` 命令后这些类型即有消费者，警告会自然消失 —— 若 A7 即将接线，可暂不处理并登记；
2. 若接线尚早，按 A2 约定补 `#[allow(dead_code)]`（保持全仓一致）；
3. **不建议**为过门禁而放宽 `warnings_increased` 判定。

### S1-5【中】二进制体积 +14.6%，M4 体积预算已近耗尽

`total_bytes_pct: 14.6`，门禁上限 15%（`exceeds_growth_limit: false`，**未超但余量 0.4pt**）。
**风险**：`rusqlite`(**bundled**) + `mysql` + `postgres` 三个依赖在**功能尚未接线**时即吃掉几乎全部预算（对照：M4-2 无 `database.rs`、无命令）。A4/A5 后续必然再增，届时 `build metrics` 硬失败，可选退路很少。
**建议（A0 决策）**：① 上调阈值并书面说明理由；或 ② `rusqlite` 改非 bundled（依赖系统 sqlite3，A6 实测项 17 已记系统 sqlite3 在）；或 ③ 延后引入 `mysql`/`postgres` 之一，待 M4-2 真正需要时再补。**勿等红灯。**
**关联债务**：D24（M0-0.b 吞吐基线未重采）会削弱吞吐维度的对比可信度，但**不豁免**体积与警告维度。

---

## 4. 正向确认（红线达标项）

| 检查项 | 结论 | 证据 |
|---|---|---|
| **无第二执行路径**（F6 红线） | ✅ | `grep -n "std::process::Command\|Command::new\|sh -c\|bash -c" tasks.rs workspace.rs` → **空**；`scheduler.rs` 尚未创建，当前无触发逻辑 |
| **secret 参数不落盘**（R-3 / 我上一轮 R-4 的**定义期**部分） | ✅ 已实现且有测试 | `tasks.rs:369-381`（`p.secret && (supplied.contains_key(\|\| p.required)` ⇒ 拒绝；必填 secret 亦可拒绝，注释明写「无人值守任务无法交互输入凭据」），测试 `tasks.rs:698-734` |
| **cron 方言收敛** | ✅ | 仅 5 段；6 段 / 宏 / 越界 / 空串 / 步长 0 等 9 类非法输入在测试中全部断言拒绝（`tasks.rs:588-600`）—— **也正是这批用例触发了 S1-2 误报** |
| **凭据不入调度侧** | ✅ | `tasks.rs` 无任何 Keyring / `db:` / password 引用；与 A6 §7「任务不持有凭据」一致 |
| **A6 采纳我上一轮 R-1** | ✅ 已转码位 | `ACTIVE_CODES` 含 `SCHED_AUDIT_PER_RUN_EVENT`，注释明写「A10 R-1（审计冲刷）：契约 §7 据此修订」；`PER_RUN_AUDIT_EVENTS = ("task.run.start","task.run.finish")`（`:167`）。**契约 §7 的实际修订权在 A0**，A6 未擅改，处置得当 |

> **R-1 现状跟踪**：码位已就位，但**契约 §7 仍把 `task.run.start`/`task.run.finish` 列为审计事件**。A7 尚未产生审计写入代码，故审计冲刷**尚未成为实债**；一旦 A7 接线，风险立即生效。请 A0 在 A7 落地前完成 R-1 裁决（把两个事件移出 `audit.json`，明细留 `task-runs.json`）。

---

## 5. 红线对照（Batch SCHED-1）

| 红线 | 结论 | 依据 |
|---|---|---|
| 写权限 / 无人值守 | ✅ 达标 | `enabled` 默认 false（`default_task_enabled`）；A6 裁定 R-A6-1 已被 A7 类型层承接 |
| 凭据 | ✅ 达标 | `tasks.rs` 零凭据引用；R-3 定义期拒绝已实现 |
| 第二执行路径 | ✅ 达标（`scheduler.rs` 已落地并复用 `script_runner::start_run`/`start_command`，无第二进程路径） | `scheduler.rs:32,659,685`；`Command::new`/`sh -c` 命中仅出现在注释 |
| 无限增长 | ✅ 边界已在契约层封闭（200 任务 / 500 历史 / 512 扫描 / catch_up ≤10 / 间隔 ≥60s）；**审计冲刷为已落地的实债（R-1 见 §7 S1-8，待 A0 修订契约 §7 后由 A7 改代码）** | A6 §3.4 / §5 |
| source check / ACL | ✅ 达标 | A7 落地 5 条 `task_*` 命令：ACL 插在 `list_artifact_images` 之前、每条命令首行 `check_invocation_source`（bridge.rs:949/975/1027/1065/1089） |

---

## 6. 声明

- 本轮**未修改任何产品代码或策略脚本**。S1-2 与 S1-3 的修复建议**均需 A0 指派**（board §Batch Implementation Dispatch → Lane A10：「May edit policy scripts only if A0 explicitly assigns a fix. Otherwise no product-code edits.」）。我刻意**不自行修复**，以免与 A6/A7 并发写入同一文件造成覆盖。
- `pre-merge.sh` 实跑 **EXIT=1**，全批唯一 FAIL 为 build metrics（S1-4/S1-5）；S1-2 在 pre-merge 的**默认模式**分支中亦会失败，二者叠加均为红灯。
- 因工作树被并发改写，S1-1 与 S1-2 的取证**按观测时点分别记录**，未以后续状态倒填前序结论。

---

## 7. 复核补遗（2026-09-06，A7 落命令后触发）

评审期间 A7 在 Batch SCHED-1 内继续落地了命令层与运行器：`bridge.rs`（5 条 `task_*`，约 207 行新增）、`main.rs`（注册）、`default-commands.toml`（ACL）、以及 `src-tauri/src/scheduler.rs`（35 072 B，23:23 创建）。据此**修正 §4/§5 中"source check / 第二执行路径 本批不适用/未落地"的临时表述**，并落实我上一轮 R-1 的最终判定。

### S1-6【已达标】新增 5 条 `task_*` 命令的来源校验与 ACL 插位（G-10 / R-9 / `SCHED_CMD_NOT_REGISTERED` / `SCHED_ACL_ORDER`）

- **ACL 插位正确**：`default-commands.toml` 5 条 `task_*` 插入在末条 `list_artifact_images` **之前** ✅（契合坑位备忘②"ACL 末条恒为 list_artifact_images"，也满足 `SCHED_ACL_ORDER`）。
- **注册完整**：`main.rs:1413-1417` 注册 `task_list`/`task_add`/`task_update`/`task_remove`/`task_run_now` ✅。
- **来源校验置首行**：`bridge.rs` 每条命令**函数体第一行**即 `check_invocation_source(&webview, "task_*", None, &app)?`（task_list:949 / task_add:975 / task_update:1027 / task_remove:1065 / task_run_now:1089）✅ —— 远程 webview 伪造 label 无法调用，满足护栏"所有新命令必须过来源校验"。
- **结论**：原 §5 "source check / ACL ➖ 本批不适用" **作废**，改判 **✅ 达标**。

### S1-7【已达标】执行通道复用（F6 红线）

- `scheduler.rs:7-8` 文档声明"本模块只做触发""禁止第二套进程/spawn 路径（`std::process::Command` / `sh -c` / `bash -c`）"；`:32` 仅 `use crate::script_runner::{RunError, RunSnapshot, ScriptProcessTable}`。
- 实际触发走 `script_runner::start_run`（`:659`）/ `start_command`（`:685`）✅ —— 复用 M2-4 既有通道，**无第二执行路径**。
- 全仓对 `std::process::Command` / `Command::new` / `sh -c` / `bash -c` 的命中**仅出现在注释**（`:8`），运行代码零独立 spawn ✅。
- 审计 detail 已脱敏：`scheduler.rs:777-778` 明写仅含 `task_id`/`run_id`/`reason`/`status`/`error_code`，**不含参数值、命令正文、脚本正文、输出、任何凭据** ✅ —— 落实 G-3 与我上轮 R-1 的"定义期拒绝/脱敏"部分。
- **结论**：原 §5 "第二执行路径（scheduler.rs 未落地，接线后须复审）" **改判为 ✅ 达标（已落地并验证）**。

### S1-8【高·实债】R-1（审计冲刷）已由 A7 落地为真实代码路径，待 A0 修订契约 §7 后由 A7 改代码

- **证据链**：`scheduler.rs:779-793` `record_run_start` / `record_run_finish` 调用 `crate::workspace::log_audit(app, "task.run.start" / "task.run.finish", ...)`；`workspace.rs:384` 落盘 **`audit.json`**，`:393-394` **cap 1000 FIFO**（超则 `list.drain(0..len-1000)`）。
- **与 A6 裁定一致**：A6 在 M4-5.d §5 已"接受 R-1，建议修订契约 §7，契约修订权在 A0，本轮不改"。但 **A0 尚未修订契约 §7**，A7 已按未修订的 §7 把每次 tick 的 per-run 事件写入 `audit.json`。即 R-1 从"建议"变为"已落地、待闭环"。
- **风险量化**：单任务每 1 分钟 = 1440 次/天 → 日增 2880 条 audit 条目；`audit.json` cap 1000，故生命周期与失败事件（`task.add`/`update`/`remove`/`enable`/`auto_disabled`/`clock.rewind`/`run.reject`/`run.missed(over_limit)`）会在调度器开始 tick 后**数分钟内被冲刷**，审计不可用于事后取证。
- **处理建议（待 A0 裁决，归属 A6/A7，非 A10 权限）**：
  1. 契约 §7 把 `task.run.start` / `task.run.finish` **移出 `audit.json`**，明细留 `task-runs.json`（run 历史）；`audit.json` 仅保留生命周期与失败事件。
  2. 代码侧 `scheduler.rs` 的 `record_run_start` / `record_run_finish` 改为写 `task-runs.json`（或等价 history），不动 `audit.json`。**该改动属产品代码，超出我 A10 本轮权限（board：A10 仅 A0 指派时改策略脚本，产品代码零改），须 A0 指派 A7 执行。**
  3. `SCHED_AUDIT_PER_RUN_EVENT` 码位（A6 已加，self-test 现 PASS）应随之从"检测是否漏记 per-run 审计"改为"检测是否误把 per-run 审计写进 `audit.json`"——随契约 §7 修订一并归属 A0/A6。

### 修正汇总（覆盖 §4/§5 临时表述）

| 红线 | 原表述 | 现判 |
|---|---|---|
| 第二执行路径 | ✅（scheduler.rs 未落地，接线后须复审） | **✅ 达标**（已落地并复用 script_runner，S1-7） |
| source check / ACL | ➖ 本批不适用 | **✅ 达标**（S1-6） |
| 审计冲刷 R-1 | 待 A0 裁决 | **【高·实债】已落地为真实路径，待 A0 修订契约 §7 后由 A7 改代码（S1-8）** |
