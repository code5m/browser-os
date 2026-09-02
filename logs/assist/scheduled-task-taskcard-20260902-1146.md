# scheduled-task-taskcard（2026-09-02 11:46）

> 批次：`free-model-prework-4-full` · 路由 `AI:FAST`
> 锚定：WBS §6.1 **M4-5 ~ M4-8**（需求 #11 定时任务调度）
> 状态：⏸ **任务卡 / 未实现调度器 / 未执行验收 / 不宣称 PASS**
> 关联：`M4-58.a-prework-20260902-1055.md`（契约草案）· `script-execution-safety-taskcard-20260902-1146.md`（执行通道）· `M3-terminal-shutdown-taskcard-20260902-1146.md`（**阻塞依赖**）

---

## 1. 目标

定义 M4 定时任务卡的完整方案：**调度模型、幂等键、失败重试、错过补偿、取消、审计、持久化、ShutdownCoordinator 接入**，以及验收命令。

---

## 2. 现状证据（2026-09-02 实测）

| 项 | 现状 |
|---|---|
| 调度器 | ❌ 无（`main.rs` 只有三条常驻线程：layout enforcer / hibernation sweeper / grid load retry） |
| 定时任务命令 | ❌ 59 个白名单命令中无 `task_*` |
| `ScheduledTask` 领域模型 | ❌ `src-tauri/src/domain.rs` 中无 |
| 持久化 | ✅ 有范式：`workspace.rs` 的 `save_repos` / `load_repos`（`repos.json`，**非原子写** ⚠️） |
| 审计 | ✅ `log_audit`，**1000 条上限** |
| 执行通道 | ❌ `run_script` 未实现 → **本任务依赖 `script-execution-safety-taskcard`** |
| 退出收口 | ❌ 无 `ShutdownCoordinator` → **本任务依赖 `M3-terminal-shutdown-taskcard`** |
| 已有 cron 解析能力 | ⚠️ 前端侧将有 `cron-tool.html`（见 `M2-tools-seed-html-prework`），**后端解析需另做**或复用 Rust cron crate |

### 2.1 可参考的常驻线程挂载点

```rust
// src-tauri/src/main.rs:598-605（.setup() 内）
bridge::start_layout_enforcer(app.handle().clone());
bridge::start_hibernation_sweeper(app.handle().clone());
bridge::start_grid_load_retry(app.handle().clone());
```

→ 调度器线程应挂在同一位置，`ShutdownCoordinator` 注册 `SchedulerShutdown`。

### 2.2 现有持久化的坑

```rust
// src-tauri/src/workspace.rs（save_repos）
pub fn save_repos(app: &AppHandle, repos: &[RepoConfig]) -> Result<(), String> {
    fs::write(repos_file(app), serde_json::to_string_pretty(repos).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
```

→ ⚠️ **非原子写**：写入中途崩溃 = `repos.json` 半截 = 数据全丢。
定时任务文件**必须**用原子写（**K6**：临时文件 + `rename`），不得沿用 `save_repos` 的写法。

---

## 3. 必改文件候选

| 文件 | 改动 | 必要性 |
|---|---|---|
| **新增** `src-tauri/src/scheduler.rs` | 调度器：cron 解析、到期判定、触发、错过补偿 | 必须 |
| `src-tauri/src/domain.rs` | 新增 `ScheduledTask` / `TaskRunRecord` / `TaskStatus` | 必须 |
| `src-tauri/src/bridge.rs` | 新增 `task_list` / `task_create` / `task_update` / `task_delete` / `task_run_now` / `task_cancel` / `task_history` | 必须 |
| `src-tauri/permissions/default-commands.toml` | 新增 7 个命令（**K1**） | 必须 |
| **新增/改** `src-tauri/src/workspace.rs` | 定时任务持久化（**原子写**） | 必须 |
| `src-tauri/src/shutdown.rs` | 注册 `SchedulerShutdown` | 必须 |
| `src-tauri/src/main.rs` | `.setup()` 内启动调度线程 | 必须 |
| 前端 `TaskPanel.vue` 等 | UI（**本卡不涉及**） | 后续 |
| `Cargo.toml` | 是否引入 cron 解析 crate（决策点，见 §7） | 待定 |

---

## 4. 契约 / 数据结构

### 4.1 `ScheduledTask`

```rust
// src-tauri/src/domain.rs
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTask {
    pub id: String,                     // uuid v4
    pub name: String,
    /// 标准 5 段 cron（分 时 日 月 周）；不支持 6 段与 @宏
    pub cron: String,
    pub timezone: String,               // IANA，如 "Asia/Shanghai"；默认取系统
    /// 要执行的脚本（复用 run_script 通道）
    pub script_id: String,
    pub params: Vec<crate::bridge::ScriptParamValue>,
    pub enabled: bool,

    /// 错过补偿策略
    pub misfire_policy: MisfirePolicy,
    /// 失败重试
    pub retry: RetryPolicy,
    /// 超时（秒），透传给 run_script
    pub timeout_sec: u64,

    pub created_at: String,             // ISO 8601
    pub updated_at: String,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,    // 计算得出，持久化便于 UI 展示
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MisfirePolicy {
    /// 跳过（默认）：错过的触发点不补跑
    Skip,
    /// 立即补跑一次（不论错过了多少次，只补一次）
    FireOnce,
    /// 补跑全部错过的触发点（有上限，见 max_catch_up）
    CatchUp,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug)]
pub struct RetryPolicy {
    pub max_attempts: u32,              // 含首次，默认 1（不重试）
    pub backoff: BackoffKind,           // Fixed | Exponential
    pub base_delay_sec: u64,            // 默认 5
    pub max_delay_sec: u64,             // 默认 300
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackoffKind { Fixed, Exponential }
```

### 4.2 `TaskRunRecord`（历史）

```rust
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskRunRecord {
    pub run_id: String,                 // 与 run_script 的 runId 一致，便于串联
    pub task_id: String,
    /// 幂等键：格式 "<task_id>:<计划触发时刻的 unix 秒>"，唯一
    pub idempotency_key: String,
    pub scheduled_at: String,           // 计划触发时刻
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: RunStatus,              // 复用 script 的 RunStatus
    pub attempt: u32,                   // 第几次尝试（1 = 首次）
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub trigger: TriggerKind,           // Scheduled | Manual | CatchUp | Retry
    pub error: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind { Scheduled, Manual, CatchUp, Retry }
```

### 4.3 幂等键规则（核心）

```
idempotency_key = "{task_id}:{scheduled_at_unix_secs}"
```

| 规则 | 说明 |
|---|---|
| 唯一性 | 同一任务在同一**计划触发时刻**只允许产生一条执行记录 |
| 存储 | 内存 `HashSet` + 持久化 `task-runs.json`（最近 N 条） |
| 判重时机 | 触发前先查；已存在且状态非 `Failed` → 跳过 |
| 重试 | 重试**不产生新幂等键**（同一 `scheduled_at`），只递增 `attempt` |
| 手动触发（`task_run_now`） | 使用 `manual:{timestamp}` 前缀，**不占用**计划触发的幂等键 |

**反向用例**：应用崩溃重启后，调度器**不得**因 `last_run_at` 丢失而重复执行同一时刻的任务 → 依赖持久化的 `task-runs.json` 判重。

### 4.4 调度模型

```
调度线程（单线程，tick = 1s）
  loop {
      if shutdown_signal { break }
      let now = Utc::now();
      for task in tasks where enabled {
          // 1) 计算 next_run_at（若为空则算）
          // 2) 若 now >= next_run_at → 触发
          // 3) 按 misfire_policy 决定是否补跑
          // 4) 更新 last_run_at / next_run_at 并原子持久化
      }
      sleep(1s)  // 或 sleep 到下一个 next_run_at
  }
```

| 项 | 决策 |
|---|---|
| tick 粒度 | 1 秒（cron 最小粒度就是分钟，1 s tick 足够；CPU 开销可忽略） |
| 并发 | 串行触发判定，**执行**交给 `run_script`（异步，不阻塞调度线程） |
| 并发上限 | 同时 in-flight 的任务数上限（默认 3），超出排队 |
| 时区 | 存 IANA 名；默认系统时区；**明确不做 DST 特殊处理**，但文档标注 |
| cron 语法 | 仅 5 段标准式（`*`、`*/n`、`a-b`、`a,b`、数字）；**不支持** `@yearly` 等宏与 6 段 |

### 4.5 失败重试

```
attempt 1 失败 → wait(base_delay) → attempt 2 → wait(base_delay * 2) → attempt 3 → ...
最大间隔 max_delay_sec（默认 300s）
达到 max_attempts → 终态 Failed，写审计
```

| 规则 | 说明 |
|---|---|
| 重试范围 | 仅 `Failed`（退出码非 0）与 `Timeout`；`Cancelled` **不重试** |
| 幂等 | 重试沿用同一 `idempotency_key`，递增 `attempt` |
| 退避 | `Exponential`：`base * 2^(attempt-1)`，clamp 到 `max_delay_sec` |
| 抖动 | 建议加 ±10% 抖动（避免多任务同时重试造成尖峰） |
| 跨重启 | 重试状态持久化；重启后未完成重试的任务按 `misfire_policy` 处理 |

### 4.6 错过补偿（misfire）

| 场景 | `Skip`（默认） | `FireOnce` | `CatchUp` |
|---|---|---|---|
| 应用关闭期间错过 5 次 | 全部跳过，`next_run_at` 跳到下一次 | 启动时立即跑 1 次 | 启动时补跑，但**上限 3 次**（防雪崩） |
| 系统休眠 2 小时 | 同上 | 唤醒后跑 1 次 | 补跑（上限 3） |
| 单 tick 延迟几秒 | 直接跑（不算 misfire） | 直接跑 | 直接跑 |

**Misfire 判定阈值**：实际触发时间比计划晚 > 60 秒 → 视为 misfire。

**CatchUp 上限（硬约束）**：最多补 3 次，**不得**无上限补跑（关了一周的电脑开机后跑 1000 次是灾难）。

### 4.7 取消

| 操作 | 行为 |
|---|---|
| `task_cancel(run_id)` | 透传给 `cancel_script(run_id)`；状态 → `Cancelled`；**不重试** |
| `task_delete(id)` | 若有 in-flight 执行 → 先取消再删；删除持久化条目 + 历史（历史另存或按保留策略） |
| `task_update` 改 cron | 重算 `next_run_at`；in-flight 执行不受影响 |
| `enabled = false` | 停止后续触发；in-flight 执行**不受影响**（让它跑完） |
| 应用退出 | `SchedulerShutdown`：停止 tick → 取消全部 in-flight（走 `cancel_script`）→ 等宽限 |

### 4.8 审计（**K5 关键**）

定时任务会高频产生事件，若全写 `audit.json` 会**刷爆 1000 条上限**。

| 事件 | 落哪里 |
|---|---|
| 任务的 创建 / 更新 / 删除 / 启用 / 禁用 | `audit.json`（低频，动作级） |
| 每次执行开始 / 结束 | **独立 `task-runs.json`**（明细） |
| 执行失败 / 重试 | `task-runs.json` + `audit.json` 摘要（仅失败，低频） |
| 每次 tick | ❌ **不记**（否则每秒 1 条） |
| Misfire 补偿 | `audit.json`（低频） |

```rust
log_audit(&app, "task.run.failed", json!({
    "taskId": t.id, "runId": run_id, "attempt": attempt,
    "exitCode": exit_code,
    // ⚠️ 不记输出正文、不记参数明文
}).to_string());
```

### 4.9 持久化

| 文件 | 内容 | 写策略 |
|---|---|---|
| `<app_data>/mvp-browser-os/tasks.json` | `Vec<ScheduledTask>` | **原子写**（temp + rename） |
| `<app_data>/mvp-browser-os/task-runs.json` | `Vec<TaskRunRecord>`，最近 **500 条** | **原子写** + 环形裁剪 |

```rust
/// 原子写（K6），必须的范式
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 读取：损坏时备份而非静默清空（K6）
pub fn load_or_backup<T: DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(c) => match serde_json::from_str(&c) {
            Ok(v) => v,
            Err(e) => {
                let bak = path.with_extension("json.corrupt");
                let _ = fs::rename(path, &bak);          // 备份，不静默清空
                eprintln!("[scheduler] 数据损坏已备份: {bak:?}: {e}");
                T::default()
            }
        },
        Err(_) => T::default(),
    }
}
```

---

## 5. 实现要点（步骤化）

1. **先完成两个依赖**：`M3-terminal-shutdown-taskcard`（ShutdownCoordinator）+ `script-execution-safety-taskcard`（`run_script`）。
2. `domain.rs` 加 `ScheduledTask` / `TaskRunRecord` / `MisfirePolicy` / `RetryPolicy` / `TriggerKind`。
3. 新 `scheduler.rs`：
   - `Scheduler` struct：任务表 + in-flight 表 + 幂等键集合 + 关闭信号（`AtomicBool`）。
   - `tick()`：单趟判定。
   - `spawn_tick_thread(app)`：常驻线程，挂 `main.rs:598-605` 区域。
4. cron 解析：**决策点**（§7）——自研轻量解析 or 引入 crate。
5. `bridge.rs` 加 7 个命令 → `default-commands.toml`（**K1**）。
6. 持久化用 `atomic_write` + `load_or_backup`（§4.9）。
7. 注册 `SchedulerShutdown`。
8. 审计按 §4.8 拆分落盘。
9. 前端 UI（本卡不涉及，另开）。

---

## 6. 禁止事项

| # | 禁止 | 原因 |
|---|---|---|
| 1 | ❌ 调度器自己拼命令执行 | 必须复用 `run_script` 通道（否则绕过全部安全校验） |
| 2 | ❌ 每次 tick 写 `audit.json` | 1000 条上限会被秒刷爆（K5） |
| 3 | ❌ 非原子写 `tasks.json` | 崩溃 = 定时任务全丢（K6） |
| 4 | ❌ 数据损坏时静默清空 | 用户数据全丢且无痕（K6） |
| 5 | ❌ `CatchUp` 无上限补跑 | 关机一周开机跑 1000 次 |
| 6 | ❌ 对 `Cancelled` 自动重试 | 用户明确取消的意图 |
| 7 | ❌ 新增命令忘进 ACL | K1，静默拒绝极难排查 |
| 8 | ❌ 无 `ShutdownCoordinator` 就上调度线程 | 退出时线程与子进程都收不掉 |
| 9 | ❌ cron 支持 6 段 / `@宏` | 与 `cron-tool.html` 口径不一致，增加解析面积 |

---

## 7. 风险

| 级别 | 风险 | 缓解 |
|---|---|---|
| **高** | 两个上游依赖未完成 | 本任务**不得**先于 `ShutdownCoordinator` 与 `run_script` 开工 |
| 中 | **cron 解析实现决策**：自研 vs 引入 crate（如 `cron` / `tokio-cron-scheduler`） | 自研：零依赖但边界多；引入 crate：省事但增体积与锁文件。建议：先自研 5 段最小集（约 150 行），与 `cron-tool.html` 共用同一套语义；若边界过多再评估引入 |
| 中 | 时区与 DST | 明确「不做 DST 特殊处理」并文档标注；用 IANA 名存储 |
| 中 | 多任务同时到期造成资源尖峰 | 并发上限 3 + 重试抖动 |
| 中 | 长任务阻塞后续触发 | 执行异步化；同任务**串行**（前一未完不触发下一次，记录 skip） |
| 中 | `task-runs.json` 无限增长 | 环形裁剪，保留最近 500 条 |
| 低 | 1 s tick 的 CPU 开销 | 可优化为「sleep 到下一个 next_run_at」，但 1 s tick 实现更简单，先保留 |

---

## 8. 反向用例

| # | 用例 | 期望 |
|---|---|---|
| R1 | 任务 `*/1 * * * *`，应用关闭 5 分钟后启动（`Skip`） | 不补跑，`next_run_at` 跳到下一个整分 |
| R2 | 同一任务改 `FireOnce` 后重启 | 启动后跑 **1 次**（不是 5 次） |
| R3 | 同一任务改 `CatchUp` 后关闭 1 小时再启动 | 最多补跑 **3 次** |
| R4 | 任务执行中应用崩溃，重启 | 不因 `last_run_at` 丢失而重复执行（幂等键判重） |
| R5 | 脚本退出码 1，`max_attempts=3` | 共尝试 3 次，间隔递增，最终 `Failed` |
| R6 | 脚本执行中用户取消 | 立即 `Cancelled`，**不重试** |
| R7 | 删除正在运行的任务 | 先取消执行，再删除；无残留进程 |
| R8 | `enabled=false` 时有 in-flight 执行 | 不打断，跑完为止 |
| R9 | 快速连续点「立即运行」5 次 | 幂等键不同（manual:timestamp），但并发上限生效，不炸 |
| R10 | 制造 `tasks.json` 半截损坏 | 启动时备份为 `.corrupt`，**不静默清空**，应用可正常启动 |
| R11 | 跑 200 次任务后 `wc -l audit.json` | `audit.json` 条目不超 1000；明细在 `task-runs.json` |
| R12 | 应用退出时有 in-flight 任务 | 走 `SchedulerShutdown`，无孤儿进程 |
| R13 | 任务 cron 填 `*/1 * * * * *`（6 段） | 创建时拒绝，给出可读错误 |
| R14 | 任务引用不存在的 `script_id` | 创建时拒绝（或触发时报可读错误并禁用） |
| R15 | 系统休眠 2 小时后唤醒 | 按 misfire 策略处理，不雪崩 |

---

## 9. 验收命令（**均未执行**）

```bash
cd /home/ainfinit/Documents/极智简单/V3/mvp-browser-os-v3

# 9.1 调度器存在
ls -l src-tauri/src/scheduler.rs
grep -n "mod scheduler" src-tauri/src/main.rs
# 期望：均命中

# 9.2 复用执行通道（不得自己拼命令）
grep -rn '"\-c"' src-tauri/src/scheduler.rs | wc -l       # 期望 0
grep -n "run_script\|script_runner" src-tauri/src/scheduler.rs   # 期望命中

# 9.3 命令已进 ACL（K1）
grep -c "task_list\|task_create\|task_update\|task_delete\|task_run_now\|task_cancel\|task_history" \
  src-tauri/permissions/default-commands.toml
# 期望 7

# 9.4 原子写（K6）
grep -n "atomic_write\|json.tmp\|rename" src-tauri/src/scheduler.rs src-tauri/src/workspace.rs
# 期望命中

# 9.5 损坏备份而非清空（K6）
grep -n "corrupt" src-tauri/src/scheduler.rs src-tauri/src/workspace.rs   # 期望命中

# 9.6 幂等键
grep -n "idempotency_key\|idempotencyKey" src-tauri/src/scheduler.rs src-tauri/src/domain.rs
# 期望命中

# 9.7 CatchUp 上限
grep -n "CatchUp" -A 3 src-tauri/src/scheduler.rs | grep -n "3\|MAX_CATCH"
# 期望命中（≤3）

# 9.8 审计不刷爆（K5）
grep -rn "log_audit" src-tauri/src/scheduler.rs | wc -l
# 期望：少量（仅生命周期与失败，tick 内应无 log_audit）
sed -n '/fn tick/,/^}/p' src-tauri/src/scheduler.rs | grep -c "log_audit"
# 期望 0

# 9.9 ShutdownCoordinator 接入
grep -n "SchedulerShutdown" src-tauri/src/shutdown.rs      # 期望命中

# 9.10 动态（人工）
#   R1/R2/R3 misfire 三策略
#   R5 重试
#   R12 退出收口：ps -ef | grep -c "<script>"  # 期望 0

# 9.11 编译门槛
cargo clippy --manifest-path src-tauri/Cargo.toml 2>&1 | tail -20
# 对照 logs/baseline-2026-08-27.md（13 warning）
```

---

## 10. 失败动作

| 失败 | 动作 |
|---|---|
| 调度器自己拼命令行 | 立即改为复用 `run_script`；这是绕过安全防线的 P0 缺陷 |
| `audit.json` 被刷爆 | 按 §4.8 拆独立 JSON；不得靠「任务不多」搪塞 |
| `tasks.json` 损坏后数据静默丢失 | 补 `load_or_backup`；不得静默清空 |
| 重启后任务重复执行 | 修幂等键判重（依赖持久化的 `task-runs.json`） |
| CatchUp 补跑过多 | 加硬上限 3；不得移除上限 |
| 退出时任务进程残留 | 补 `SchedulerShutdown`；不得只停 tick 不杀进程 |
| cron 解析边界错误 | 与 `cron-tool.html` 对齐语义；加契约测试 |

---

## 11. 推荐模型

`AI:DEEP`（调度语义 + 持久化 + 进程生命周期 + 并发）。
**人工 GUI 验收必做**：R1~R3（misfire）、R5（重试）、R10（损坏恢复）、R12（退出收口）。
cron 解析部分若决定自研，可单独 `AI:BALANCED`；若决定引入 crate，需 `AI:DEEP` 评估依赖影响。
