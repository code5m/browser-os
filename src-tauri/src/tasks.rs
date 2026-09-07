//! M4-6 定时任务：校验 / cron 解析 / 触发点计算 / 持久化 的**纯函数层**。
//!
//! 契约源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md`（Lane A6 冻结）。
//! 设计口径沿用 `scripts.rs` / `snippets.rs`：本文件只做纯函数与落盘原语，
//! 命令层归 `bridge.rs`，触发与退出收口归 `scheduler.rs`。
//!
//! 四条硬约束（由 `scripts/check-scheduler-policy.py` 机器守护）：
//!
//! - **F6**：禁止第二套进程/spawn 路径。执行唯一入口是 `script_runner`。
//! - **F8**：判定函数（`next_fire_after` / `collect_missed_slots` / `is_missed` /
//!   `should_fire` / `validate_trigger` / `validate_task`）**不得读系统时钟**。
//!   本层采用「时间由入参传入」而非「注入 Clock」——可测性等价且更强（纯函数），
//!   `Clock` trait 仍按契约 §4.1 实现于 `scheduler.rs`，供主循环与 fire 路径使用。
//! - **F10**：落盘必须走 `session::atomic_write`（tmp + rename），不新造第二条原子写路径。
//! - **§3.3 R-3**：`ScriptParam.secret` 参数值一律不得落盘（定义期拒绝）。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use chrono::{DateTime, Datelike, Local, NaiveDateTime, Timelike, Utc};

use crate::domain::{
    RetryPolicy, ScriptParam, TaskDef, TaskRunRecord, TaskTrigger, SCHED_MAX_ATTEMPTS,
    SCHED_MAX_CATCH_UP, SCHED_MAX_HISTORY, SCHED_MAX_SLOT_SCAN, TASK_INTERVAL_MAX_SECS,
    TASK_INTERVAL_MIN_SECS, TASK_MAX_NAME_BYTES, TASK_MAX_PARAMS,
};

static TASK_STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Serialize every tasks.json load-modify-save transaction across the scheduler
/// and task commands. Callers must hold this guard until their save completes.
pub fn task_store_lock() -> MutexGuard<'static, ()> {
    TASK_STORE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

// ----------------------------- 错误类型 -----------------------------

/// 定时任务错误。**稳定字符串码**（沿用 `scripts.rs` 20 码范式），前端据此选文案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskError {
    pub code: &'static str,
}

impl TaskError {
    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}

fn err(code: &'static str) -> TaskError {
    TaskError { code }
}

/// 契约 §3.3 首期 10 个稳定错误码。
pub const TASK_NOT_FOUND: &str = "TASK_NOT_FOUND";
pub const TASK_INVALID_CRON: &str = "TASK_INVALID_CRON";
pub const TASK_INTERVAL_OUT_OF_RANGE: &str = "TASK_INTERVAL_OUT_OF_RANGE";
pub const TASK_TARGET_NOT_FOUND: &str = "TASK_TARGET_NOT_FOUND";
pub const TASK_SECRET_PARAM_FORBIDDEN: &str = "TASK_SECRET_PARAM_FORBIDDEN";
pub const TASK_PARAM_INVALID: &str = "TASK_PARAM_INVALID";
pub const TASK_LIMIT_REACHED: &str = "TASK_LIMIT_REACHED";
pub const TASK_ALREADY_RUNNING: &str = "TASK_ALREADY_RUNNING";
pub const TASK_PERSIST_FAILED: &str = "TASK_PERSIST_FAILED";
pub const TASK_NAME_TOO_LONG: &str = "TASK_NAME_TOO_LONG";

/// 跳过的三种原因（契约 §5.2：均**不消耗**重试配额）。
pub const SKIP_REENTRANT: &str = "reentrant";
pub const SKIP_TARGET_BUSY: &str = "target_busy";
pub const SKIP_GLOBAL_LIMIT: &str = "global_limit";
/// 错过执行的两类记录原因（契约 §5.1）。
pub const MISSED_SKIPPED: &str = "skipped";
pub const MISSED_OVER_LIMIT: &str = "over_limit";

// ----------------------------- cron 5 段解析 -----------------------------

/// 5 段 cron 的解析结果。每个字段一张位图：第 n 位为 1 表示值 n 命中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CronExpr {
    /// 分：bit 0..=59
    pub minutes: u64,
    /// 时：bit 0..=23
    pub hours: u64,
    /// 日：bit 0..=30 表示 1..=31
    pub days: u64,
    /// 月：bit 0..=11 表示 1..=12
    pub months: u64,
    /// 周：bit 0..=6，0 = 周日（输入 7 归一为 0）
    pub weekdays: u64,
}

impl CronExpr {
    /// 判定某个**本地**时刻是否命中。
    ///
    /// **dom / dow 取交集（AND）**：标准 Vixie cron 在两者都非通配时取并集（OR），
    /// 本实现刻意取交集 —— 触发更少即 fail-closed，且语义简单可测。契约 §4.4
    /// 未规定该项，本收窄登记为 **O-A7-1**，交 A0 裁定。
    pub fn matches_naive(&self, at: &NaiveDateTime) -> bool {
        let minute = at.minute();
        let hour = at.hour();
        let day = at.day();
        let month = at.month();
        let weekday = at.weekday().num_days_from_sunday();
        minute <= 59
            && hour <= 23
            && (1..=31).contains(&day)
            && (1..=12).contains(&month)
            && (self.minutes >> minute) & 1 == 1
            && (self.hours >> hour) & 1 == 1
            && (self.days >> (day - 1)) & 1 == 1
            && (self.months >> (month - 1)) & 1 == 1
            && (self.weekdays >> weekday) & 1 == 1
    }
}

/// 解析 5 段 cron。语法：`*` / `*/n` / `a-b` / `a,b,c` / 字面量。
///
/// **定义期拒绝**（`TASK_INVALID_CRON`）：段数非 5（含 6 段带秒、宏、空串）、
/// 值越界、步长嵌套、`*/0`、非法字符。一律**不静默降级/截断**。
pub fn parse_cron(expr: &str) -> Result<CronExpr, TaskError> {
    let fields: Vec<&str> = expr.split_whitespace().collect();
    if fields.len() != 5 {
        return Err(err(TASK_INVALID_CRON));
    }
    Ok(CronExpr {
        minutes: parse_field(fields[0], 0, 59)?,
        hours: parse_field(fields[1], 0, 23)?,
        days: parse_field(fields[2], 1, 31)?,
        months: parse_field(fields[3], 1, 12)?,
        weekdays: parse_weekday_field(fields[4])?,
    })
}

fn parse_num(raw: &str) -> Result<u64, TaskError> {
    let t = raw.trim();
    if t.is_empty() || !t.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err(TASK_INVALID_CRON));
    }
    t.parse::<u64>().map_err(|_| err(TASK_INVALID_CRON))
}

fn parse_field(field: &str, min: u64, max: u64) -> Result<u64, TaskError> {
    let mut mask = 0u64;
    for part in field.split(',') {
        if part.is_empty() {
            return Err(err(TASK_INVALID_CRON));
        }
        let (range, step_raw) = match part.split_once('/') {
            Some((r, s)) => (r, Some(s)),
            None => (part, None),
        };
        if range.is_empty() {
            return Err(err(TASK_INVALID_CRON));
        }
        let (lo, hi) = if range == "*" {
            (min, max)
        } else if let Some((a, b)) = range.split_once('-') {
            (parse_num(a)?, parse_num(b)?)
        } else {
            let v = parse_num(range)?;
            (v, v)
        };
        if lo < min || hi > max || lo > hi {
            return Err(err(TASK_INVALID_CRON));
        }
        let step = match step_raw {
            None => 1,
            Some(s) => {
                let st = parse_num(s)?;
                // `*/0` 无意义且会导致死循环，定义期拒绝
                if st == 0 {
                    return Err(err(TASK_INVALID_CRON));
                }
                st
            }
        };
        let mut v = lo;
        while v <= hi {
            mask |= 1u64 << v;
            v += step;
        }
    }
    if mask == 0 {
        return Err(err(TASK_INVALID_CRON));
    }
    Ok(mask)
}

/// 周字段：值域 0–7，**0 与 7 均为周日**（归一到 bit 0）。
fn parse_weekday_field(field: &str) -> Result<u64, TaskError> {
    let mut mask = 0u64;
    for part in field.split(',') {
        if part.is_empty() {
            return Err(err(TASK_INVALID_CRON));
        }
        let (range, step_raw) = match part.split_once('/') {
            Some((r, s)) => (r, Some(s)),
            None => (part, None),
        };
        if range.is_empty() {
            return Err(err(TASK_INVALID_CRON));
        }
        let (lo, hi) = if range == "*" {
            (0u64, 7u64)
        } else if let Some((a, b)) = range.split_once('-') {
            (parse_num(a)?, parse_num(b)?)
        } else {
            let v = parse_num(range)?;
            (v, v)
        };
        if lo > 7 || hi > 7 || lo > hi {
            return Err(err(TASK_INVALID_CRON));
        }
        let step = match step_raw {
            None => 1,
            Some(s) => {
                let st = parse_num(s)?;
                if st == 0 {
                    return Err(err(TASK_INVALID_CRON));
                }
                st
            }
        };
        let mut v = lo;
        while v <= hi {
            mask |= 1u64 << (v % 7);
            v += step;
        }
    }
    if mask == 0 {
        return Err(err(TASK_INVALID_CRON));
    }
    Ok(mask)
}

// ----------------------------- 触发点判定（F8：无时钟） -----------------------------

/// 返回严格大于 `after` 的第一个计划触发点。
///
/// 纯函数：时间由入参给出，函数体**不读系统时钟**（F8）。
pub fn next_fire_after(trigger: &TaskTrigger, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match trigger {
        TaskTrigger::Interval { every_secs } => {
            let step = (*every_secs).max(TASK_INTERVAL_MIN_SECS);
            after.checked_add_signed(chrono::Duration::seconds(step as i64))
        }
        TaskTrigger::Cron { expr } => {
            let cron = parse_cron(expr).ok()?;
            next_cron_fire(&cron, after)
        }
    }
}

/// 逐分钟推进找第一个命中的本地时刻（cron 最小粒度 1 分钟，不做秒级对齐，契约 §4.3）。
///
/// 只做**一次**时区转换（在入参上），循环内操作 `NaiveDateTime`，避免逐分钟
/// `with_timezone` 的开销（1 年扫描 = 52 万次迭代）。
fn next_cron_fire(cron: &CronExpr, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let mut naive = after
        .with_timezone(&Local)
        .naive_local()
        .with_second(0)?
        .with_nanosecond(0)?
        + chrono::Duration::minutes(1);
    // 上界：366 天的分钟数。超出视为无解（如「2 月 31 日」这类永不命中的组合）。
    const MAX_MINUTES: i64 = 366 * 24 * 60;
    for _ in 0..MAX_MINUTES {
        if cron.matches_naive(&naive) {
            match naive.and_local_timezone(Local) {
                chrono::LocalResult::Single(dt) => return Some(dt.with_timezone(&Utc)),
                // DST 回拨：同一本地时刻出现两次 —— 只取第一次（契约 §4.5）
                chrono::LocalResult::Ambiguous(earliest, _later) => {
                    return Some(earliest.with_timezone(&Utc))
                }
                // DST 前进：该本地时刻不存在 —— 跳过该点，不补跑（契约 §4.5）
                chrono::LocalResult::None => {
                    naive += chrono::Duration::minutes(1);
                    continue;
                }
            }
        }
        naive += chrono::Duration::minutes(1);
    }
    None
}

/// 生成 `(anchor, now]` 区间内的全部计划触发点，最多 `SCHED_MAX_SLOT_SCAN` 个。
///
/// 扫描上限是**防雪崩**护栏：时钟前跳数月时，不得生成数十万个补跑请求。
pub fn collect_missed_slots(
    trigger: &TaskTrigger,
    anchor: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let mut slots = Vec::new();
    if now <= anchor {
        return slots;
    }
    let mut cursor = anchor;
    while slots.len() < SCHED_MAX_SLOT_SCAN {
        match next_fire_after(trigger, cursor) {
            Some(next) if next <= now => {
                slots.push(next);
                cursor = next;
            }
            _ => break,
        }
    }
    slots
}

/// 是否判定为「错过」：迟到**超过** `grace_secs` 才算（等于不算）。
pub fn is_missed(now: DateTime<Utc>, slot: DateTime<Utc>, grace_secs: u64) -> bool {
    now.timestamp().saturating_sub(slot.timestamp()) > grace_secs as i64
}

/// 到点判定（含正好相等）。
pub fn should_fire(now: DateTime<Utc>, slot: DateTime<Utc>) -> bool {
    now >= slot
}

// ----------------------------- 定义期校验（契约 §3.3） -----------------------------

/// R-2：触发方式合法。cron 5 段且字段在值域内；interval ∈ [60, 2592000]。
pub fn validate_trigger(trigger: &TaskTrigger) -> Result<(), TaskError> {
    match trigger {
        TaskTrigger::Cron { expr } => parse_cron(expr).map(|_| ()),
        TaskTrigger::Interval { every_secs } => {
            if *every_secs < TASK_INTERVAL_MIN_SECS || *every_secs > TASK_INTERVAL_MAX_SECS {
                Err(err(TASK_INTERVAL_OUT_OF_RANGE))
            } else {
                Ok(())
            }
        }
    }
}

/// R-1 / R-2 / R-5 / R-6：任务自身的字段校验（**不含**目标存在性与参数校验，
/// 二者分别需要 app 与目标参数定义，见 `validate_params`）。
pub fn validate_task(task: &TaskDef) -> Result<(), TaskError> {
    if crate::images::validate_id(&task.id).is_err() || task.target_id.trim().is_empty() {
        return Err(err(TASK_PARAM_INVALID));
    }
    if task.name.trim().is_empty() || task.name.len() > TASK_MAX_NAME_BYTES {
        return Err(err(TASK_NAME_TOO_LONG));
    }
    validate_trigger(&task.trigger)?;
    // R-5
    if task.catch_up_limit < 1 || task.catch_up_limit > SCHED_MAX_CATCH_UP {
        return Err(err(TASK_PARAM_INVALID));
    }
    if task.timeout_secs > crate::script_runner::MAX_TIMEOUT_SECS {
        return Err(err(TASK_PARAM_INVALID));
    }
    if task.retry.max_attempts < 1 || task.retry.max_attempts > SCHED_MAX_ATTEMPTS {
        return Err(err(TASK_PARAM_INVALID));
    }
    if task.retry.max_delay_secs > crate::domain::SCHED_MAX_DELAY_SECS {
        return Err(err(TASK_PARAM_INVALID));
    }
    // R-6
    if task.params.len() > TASK_MAX_PARAMS {
        return Err(err(TASK_PARAM_INVALID));
    }
    Ok(())
}

/// R-3 / R-4：参数校验。
///
/// - **R-3**：`params` 中禁止出现目标 `ScriptParam.secret == true` 的参数值；
///   若目标存在必填 secret 参数，该任务**直接拒绝创建**（无人值守任务无法交互输入凭据）。
/// - **R-4**：键集必须与目标参数名一致（缺必填 / 多未知键均拒绝），值过
///   `security_policy::check_text_field`。
pub fn validate_params(
    meta: &[ScriptParam],
    supplied: &HashMap<String, String>,
) -> Result<(), TaskError> {
    for p in meta {
        if p.secret && (supplied.contains_key(&p.name) || p.required) {
            return Err(err(TASK_SECRET_PARAM_FORBIDDEN));
        }
        if !p.secret && p.required {
            match supplied.get(&p.name).map(|v| v.trim()) {
                Some(v) if !v.is_empty() => {}
                _ => return Err(err(TASK_PARAM_INVALID)),
            }
        }
    }
    for key in supplied.keys() {
        if !meta.iter().any(|p| &p.name == key) {
            return Err(err(TASK_PARAM_INVALID));
        }
    }
    for value in supplied.values() {
        if crate::security_policy::check_text_field("value", value, crate::scripts::MAX_ARG_BYTES)
            .is_err()
        {
            return Err(err(TASK_PARAM_INVALID));
        }
    }
    Ok(())
}

/// R-6：任务总数上限。超出**拒绝新增**（与 `MAX_SCRIPTS` 同口径，不是静默裁剪）。
pub fn check_capacity(count: usize) -> Result<(), TaskError> {
    if count >= crate::domain::SCHED_MAX_TASKS {
        Err(err(TASK_LIMIT_REACHED))
    } else {
        Ok(())
    }
}

// ----------------------------- 重试退避（契约 §5.3） -----------------------------

/// 退避延迟（秒）：`base * 2^(attempt-1)`（Exponential）或 `base`，**clamp 到 `max_delay_secs`**。
///
/// 抖动由 `jitter_secs` 施加，且抖动后仍不得超过 `max_delay_secs`（契约 §5.3 抖动条款）。
pub fn retry_delay_secs(retry: &RetryPolicy, attempt: u32) -> u64 {
    retry.delay_secs(attempt)
}

/// ±10% 抖动（避免多任务同时重试造成尖峰）。
///
/// 纯函数（**不引入 `rand` 依赖**：M0-4 依赖清理口径）——用 xorshift 从 seed 派生，
/// 同一 seed 结果稳定，便于单测。
pub fn jitter_secs(base: u64, seed: u64, max_delay_secs: u64) -> u64 {
    if base == 0 {
        return 0;
    }
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let ratio = (x % 21) as i64 - 10; // [-10, +10]
    let delta = (base as i64 * ratio) / 100;
    let jittered = (base as i64 + delta).max(0) as u64;
    jittered.min(max_delay_secs)
}

// ----------------------------- 持久化（F-4 / F-10） -----------------------------

/// 任务列表文件：`data_dir/tasks.json`。
pub fn tasks_file(app: &tauri::AppHandle) -> PathBuf {
    crate::workspace::data_dir(app).join("tasks.json")
}

/// 运行历史文件：`data_dir/task-runs.json`（环形保留 `SCHED_MAX_HISTORY`，**不参与判重**）。
pub fn task_runs_file(app: &tauri::AppHandle) -> PathBuf {
    crate::workspace::data_dir(app).join("task-runs.json")
}

/// 任务列表落盘（原子写：tmp + rename）。
pub fn save_tasks_at(path: &Path, list: &[TaskDef]) -> Result<(), String> {
    let content =
        serde_json::to_string_pretty(list).map_err(|e| format!("序列化任务列表失败: {e}"))?;
    crate::session::atomic_write(path, &content)
}

/// 任务列表读取。
///
/// **损坏处理（O-A6-7）**：解析失败**不得静默清空**（既有 `load_scripts_at` 就是静默丢弃，
/// 等于定时任务全丢且无痕）。此处把损坏文件 rename 为 `.corrupt` 并打日志后返回空列表。
pub fn load_tasks_at(path: &Path) -> Vec<TaskDef> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    match serde_json::from_str::<Vec<TaskDef>>(&content) {
        Ok(list) => list,
        Err(e) => {
            eprintln!(
                "[tasks] 任务文件损坏，已备份为 .corrupt（任务将全部停用，需人工介入）：{}（{e}）",
                path.display()
            );
            let mut backup = path.as_os_str().to_os_string();
            backup.push(".corrupt");
            let _ = fs::rename(path, PathBuf::from(backup));
            Vec::new()
        }
    }
}

/// 运行历史落盘（原子写 + 容量 FIFO）。
pub fn save_task_runs_at(path: &Path, list: &[TaskRunRecord]) -> Result<(), String> {
    let mut kept = list.to_vec();
    if kept.len() > SCHED_MAX_HISTORY {
        let keep_from = kept.len() - SCHED_MAX_HISTORY;
        kept.drain(..keep_from);
    }
    let content =
        serde_json::to_string_pretty(&kept).map_err(|e| format!("序列化运行历史失败: {e}"))?;
    crate::session::atomic_write(path, &content)
}

/// 运行历史读取（损坏同样走 `.corrupt` 备份，不静默丢弃）。
pub fn load_task_runs_at(path: &Path) -> Vec<TaskRunRecord> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };
    match serde_json::from_str::<Vec<TaskRunRecord>>(&content) {
        Ok(list) => list,
        Err(e) => {
            eprintln!(
                "[tasks] 运行历史损坏，已备份为 .corrupt：{}（{e}）",
                path.display()
            );
            let mut backup = path.as_os_str().to_os_string();
            backup.push(".corrupt");
            let _ = fs::rename(path, PathBuf::from(backup));
            Vec::new()
        }
    }
}

/// 追加一条运行记录（读 → 追加 → 容量裁剪 → 原子落盘）。
pub fn append_task_run(path: &Path, record: &TaskRunRecord) -> Result<(), String> {
    let mut list = load_task_runs_at(path);
    list.push(record.clone());
    save_task_runs_at(path, &list)
}

// ---------------------------------------------------------------------------
// 单测（T-sched-c1~c4 / c13 的纯函数部分 + T-task-1~5 持久化 + R-A6-1）
// ---------------------------------------------------------------------------
#[cfg(test)]
mod task_domain_tests {
    use super::*;
    use crate::domain::{MissedRunPolicy, RetryBackoff, RunStatus, TaskKind, TaskRunTrigger};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "m4-6-{}-{}-{}",
            tag,
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建临时目录");
        dir
    }

    fn sample_task(id: &str, trigger: TaskTrigger) -> TaskDef {
        let now = Utc::now();
        TaskDef {
            id: id.into(),
            name: "sample".into(),
            kind: TaskKind::Script,
            target_id: "11111111-1111-4111-8111-111111111111".into(),
            params: HashMap::new(),
            enabled: false,
            trigger,
            missed_run_policy: MissedRunPolicy::Skip,
            catch_up_limit: 3,
            misfire_grace_secs: 60,
            retry: RetryPolicy::default(),
            timeout_secs: 0,
            last_fired_at: None,
            next_run_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn task_store_lock_serializes_concurrent_transactions() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Arc, Barrier};
        use std::thread;

        let start = Arc::new(Barrier::new(8));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..8 {
            let start = Arc::clone(&start);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            handles.push(thread::spawn(move || {
                start.wait();
                let _store_guard = task_store_lock();
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                thread::yield_now();
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            handle.join().expect("并发事务线程不应 panic");
        }

        assert_eq!(
            max_active.load(Ordering::SeqCst),
            1,
            "调度器与任务命令的 load-modify-save 临界区必须串行"
        );
    }

    // ---------- T-sched-c1：合法 5 段解析 ----------

    #[test]
    fn t_sched_c1_valid_cron_expressions_parse() {
        for expr in [
            "*/15 * * * *",
            "0 9 * * 1-5",
            "0 0 1,15 * *",
            "0 0 * * 0",
            "30 6 * jan-jun mon-fri",
        ] {
            // 第 5 例含月份别名，当前方言只接受数字，必须被拒（定义期拒绝，不静默降级）
            let ok = parse_cron(expr).is_ok();
            if expr == "30 6 * jan-jun mon-fri" {
                assert!(parse_cron(expr).is_err(), "月份别名不得被接受");
            } else {
                assert!(ok, "{expr} 应解析成功");
            }
        }
        // 「0 0 * * 0」与「0 0 * * 7」等价（均为周日）
        assert_eq!(
            parse_cron("0 0 * * 0").unwrap(),
            parse_cron("0 0 * * 7").unwrap()
        );
    }

    // ---------- T-sched-c2：非法表达式全部拒绝 ----------

    #[test]
    fn t_sched_c2_invalid_cron_expressions_are_rejected() {
        for bad in [
            "0 0 0 0 0 0",  // 6 段（带秒）
            "@daily",       // 宏
            "60 * * * *",   // 分 60
            "",             // 空串
            "* * * 0 *",    // 月 0
            "*/0 * * * *",  // 步长 0
            "* * * * 8",    // 周 8
            "1-60 * * * *", // 区间越界
            "* * * *",      // 4 段
        ] {
            assert_eq!(
                parse_cron(bad).unwrap_err().code(),
                TASK_INVALID_CRON,
                "{bad} 应被拒"
            );
        }
    }

    // ---------- T-sched-c3：next_fire_after 单调且严格大于入参 ----------

    #[test]
    fn t_sched_c3_next_fire_after_is_monotonic_and_future() {
        let trigger = TaskTrigger::Cron {
            expr: "*/15 * * * *".into(),
        };
        let start = Utc::now();
        let mut cursor = start;
        let mut seen: Vec<DateTime<Utc>> = Vec::new();
        for _ in 0..100 {
            let next = next_fire_after(&trigger, cursor).expect("必须有下一触发点");
            assert!(next > cursor, "next 必须严格大于入参（不得原地打转）");
            seen.push(next);
            cursor = next;
        }
        // 严格递增：无重复值
        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 100, "100 次推进不得出现重复触发点");
        // 间隔必须是 15 分钟的整数倍（cron */15 语义）
        for w in seen.windows(2) {
            let gap = (w[1] - w[0]).num_minutes();
            assert_eq!(gap, 15, "相邻触发点间隔应为 15 分钟");
        }
    }

    // ---------- T-sched-c4：Interval 边界 ----------

    #[test]
    fn t_sched_c4_interval_bounds() {
        assert_eq!(
            validate_trigger(&TaskTrigger::Interval { every_secs: 59 })
                .unwrap_err()
                .code(),
            TASK_INTERVAL_OUT_OF_RANGE
        );
        assert!(validate_trigger(&TaskTrigger::Interval { every_secs: 60 }).is_ok());
        assert!(validate_trigger(&TaskTrigger::Interval {
            every_secs: 2592000
        })
        .is_ok());
        assert_eq!(
            validate_trigger(&TaskTrigger::Interval {
                every_secs: 2592001
            })
            .unwrap_err()
            .code(),
            TASK_INTERVAL_OUT_OF_RANGE
        );
    }

    // ---------- is_missed / should_fire ----------

    #[test]
    fn missed_and_fire_predicates_match_contract() {
        let now = Utc::now();
        let slot = now - chrono::Duration::seconds(30);
        assert!(!is_missed(now, slot, 60), "迟到 30s ≤ grace 60s，不算错过");
        assert!(should_fire(now, slot), "已到点应触发");
        let old = now - chrono::Duration::seconds(120);
        assert!(is_missed(now, old, 60), "迟到 120s > grace，算错过");
        assert!(!should_fire(now, now + chrono::Duration::seconds(1)));
    }

    // ---------- collect_missed_slots 扫描上限（防雪崩） ----------

    #[test]
    fn collect_missed_slots_is_capped() {
        let trigger = TaskTrigger::Interval { every_secs: 60 };
        let anchor = Utc::now() - chrono::Duration::days(400);
        let now = Utc::now();
        let slots = collect_missed_slots(&trigger, anchor, now);
        assert_eq!(slots.len(), SCHED_MAX_SLOT_SCAN, "必须被扫描上限截断");
        assert!(slots.windows(2).all(|w| w[0] < w[1]), "slot 必须严格递增");
    }

    // ---------- R-A6-1：默认 enabled=false（含缺字段的存量文件） ----------

    #[test]
    fn task_def_default_enabled_is_false() {
        let json = r#"{
            "id": "b3f1c2d4-0000-4000-8000-000000000001",
            "name": "n",
            "kind": "script",
            "target_id": "11111111-1111-4111-8111-111111111111",
            "trigger": { "interval": { "every_secs": 60 } },
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let task: TaskDef = serde_json::from_str(json).expect("缺字段的存量文件必须可解析");
        assert!(!task.enabled, "缺 enabled 字段必须按 false（fail-closed）");
        assert_eq!(task.missed_run_policy, MissedRunPolicy::Skip);
        assert_eq!(task.catch_up_limit, 3);
        assert_eq!(task.misfire_grace_secs, 60);
        assert_eq!(task.retry.max_attempts, 1, "默认不重试");
    }

    // ---------- 契约 §3.3 R-3：secret 参数值禁止落盘 ----------

    #[test]
    fn secret_params_are_rejected_at_definition_time() {
        let meta = vec![
            ScriptParam {
                name: "TOKEN".into(),
                label: "令牌".into(),
                param_type: crate::domain::ParamType::String,
                required: false,
                default: None,
                options: vec![],
                raw: false,
                secret: true,
            },
            ScriptParam {
                name: "DIR".into(),
                label: "目录".into(),
                param_type: crate::domain::ParamType::String,
                required: true,
                default: None,
                options: vec![],
                raw: false,
                secret: false,
            },
        ];
        let mut supplied = HashMap::new();
        supplied.insert("DIR".into(), "/tmp".into());
        assert!(validate_params(&meta, &supplied).is_ok());

        supplied.insert("TOKEN".into(), "abc".into());
        assert_eq!(
            validate_params(&meta, &supplied).unwrap_err().code(),
            TASK_SECRET_PARAM_FORBIDDEN
        );

        // 必填 secret 参数：即使不提供值也直接拒绝（无人值守任务无法交互输入凭据）
        let secret_required = vec![ScriptParam {
            name: "PWD".into(),
            label: "密码".into(),
            param_type: crate::domain::ParamType::String,
            required: true,
            default: None,
            options: vec![],
            raw: false,
            secret: true,
        }];
        let empty: HashMap<String, String> = HashMap::new();
        assert_eq!(
            validate_params(&secret_required, &empty)
                .unwrap_err()
                .code(),
            TASK_SECRET_PARAM_FORBIDDEN
        );

        // 未知键 / 缺必填 → TASK_PARAM_INVALID
        let mut unknown = HashMap::new();
        unknown.insert("DIR".into(), "/tmp".into());
        unknown.insert("NOPE".into(), "x".into());
        assert_eq!(
            validate_params(&meta, &unknown).unwrap_err().code(),
            TASK_PARAM_INVALID
        );
        let missing: HashMap<String, String> = HashMap::new();
        assert_eq!(
            validate_params(&meta, &missing).unwrap_err().code(),
            TASK_PARAM_INVALID
        );
    }

    // ---------- 契约 §3.3 R-5/R-6：字段与容量上限 ----------

    #[test]
    fn validate_task_enforces_field_bounds() {
        let mut task = sample_task(
            "b3f1c2d4-0000-4000-8000-000000000002",
            TaskTrigger::Interval { every_secs: 60 },
        );
        assert!(validate_task(&task).is_ok());

        task.name = "x".repeat(TASK_MAX_NAME_BYTES + 1);
        assert_eq!(validate_task(&task).unwrap_err().code(), TASK_NAME_TOO_LONG);

        task = sample_task(
            "b3f1c2d4-0000-4000-8000-000000000002",
            TaskTrigger::Interval { every_secs: 60 },
        );
        task.catch_up_limit = SCHED_MAX_CATCH_UP + 1;
        assert_eq!(validate_task(&task).unwrap_err().code(), TASK_PARAM_INVALID);

        task = sample_task(
            "b3f1c2d4-0000-4000-8000-000000000002",
            TaskTrigger::Interval { every_secs: 60 },
        );
        task.timeout_secs = crate::script_runner::MAX_TIMEOUT_SECS + 1;
        assert_eq!(validate_task(&task).unwrap_err().code(), TASK_PARAM_INVALID);

        task = sample_task(
            "b3f1c2d4-0000-4000-8000-000000000002",
            TaskTrigger::Interval { every_secs: 60 },
        );
        task.retry.max_attempts = SCHED_MAX_ATTEMPTS + 1;
        assert_eq!(validate_task(&task).unwrap_err().code(), TASK_PARAM_INVALID);

        assert_eq!(
            check_capacity(crate::domain::SCHED_MAX_TASKS)
                .unwrap_err()
                .code(),
            TASK_LIMIT_REACHED
        );
        assert!(check_capacity(crate::domain::SCHED_MAX_TASKS - 1).is_ok());
    }

    // ---------- 契约 §5.3：退避与抖动有硬上界 ----------

    #[test]
    fn retry_delay_is_clamped_and_jittered_within_max() {
        let retry = RetryPolicy {
            max_attempts: 3,
            backoff: RetryBackoff::Exponential,
            base_delay_secs: 10,
            max_delay_secs: 60,
        };
        assert_eq!(retry_delay_secs(&retry, 1), 10);
        assert_eq!(retry_delay_secs(&retry, 2), 20);
        assert_eq!(retry_delay_secs(&retry, 3), 40);
        // 超过 max_delay 后一律 clamp（含抖动后）
        for attempt in 1..=8 {
            let base = retry_delay_secs(&retry, attempt);
            for seed in 0..64u64 {
                let d = jitter_secs(base, seed, retry.max_delay_secs);
                assert!(d <= retry.max_delay_secs, "抖动后不得超过 max_delay_secs");
                assert!(d <= crate::domain::SCHED_MAX_DELAY_SECS);
            }
        }
        // 抖动幅度必须在 ±10% 内（非 clamp 情况下）
        let d = jitter_secs(100, 7, 600);
        assert!((90..=110).contains(&d), "抖动幅度应落在 ±10%：{d}");
    }

    // ---------- T-task-1~5：持久化（真实临时目录） ----------

    #[test]
    fn t_task_persistence_roundtrip_is_atomic() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("tasks.json");
        let list = vec![sample_task(
            "b3f1c2d4-0000-4000-8000-000000000003",
            TaskTrigger::Interval { every_secs: 60 },
        )];
        save_tasks_at(&path, &list).expect("落盘");
        // 原子写：目录内无 .tmp 残留
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "atomic_write 后不得残留 .tmp");
        let back = load_tasks_at(&path);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, list[0].id);
        assert!(!back[0].enabled, "往返后默认仍为 false");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t_task_corrupt_file_is_backed_up_not_silently_dropped() {
        let dir = temp_dir("corrupt");
        let path = dir.join("tasks.json");
        fs::write(&path, "{ this is not json").expect("写入损坏文件");
        let loaded = load_tasks_at(&path);
        assert!(loaded.is_empty(), "损坏文件不得产出任务");
        assert!(
            dir.join("tasks.json.corrupt").exists(),
            "损坏文件必须备份为 .corrupt（O-A6-7，不得静默丢弃）"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t_task_missing_file_loads_empty_without_backup() {
        let dir = temp_dir("missing");
        let loaded = load_tasks_at(&dir.join("tasks.json"));
        assert!(loaded.is_empty());
        assert!(
            !dir.join("tasks.json.corrupt").exists(),
            "文件不存在是正常首启，不得误生成 .corrupt 备份"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t_task_run_history_is_capped_and_appended() {
        let dir = temp_dir("history");
        let path = dir.join("task-runs.json");
        let now = Utc::now();
        for i in 0..(SCHED_MAX_HISTORY + 25) {
            let rec = TaskRunRecord {
                task_id: format!("t{i}"),
                run_id: format!("r{i}"),
                trigger: TaskRunTrigger::Scheduled,
                scheduled_at: now,
                attempt: 1,
                started_at: now,
                finished_at: Some(now),
                status: RunStatus::Succeeded,
                exit_code: Some(0),
                error_code: None,
            };
            append_task_run(&path, &rec).expect("追加记录");
        }
        let list = load_task_runs_at(&path);
        assert_eq!(list.len(), SCHED_MAX_HISTORY, "历史必须按容量裁剪");
        // FIFO：保留最新
        assert_eq!(
            list.last().unwrap().task_id,
            format!("t{}", SCHED_MAX_HISTORY + 24)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn t_task_run_history_survives_corrupt_file() {
        let dir = temp_dir("history-corrupt");
        let path = dir.join("task-runs.json");
        fs::write(&path, "[]]]").expect("写入损坏文件");
        assert!(load_task_runs_at(&path).is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
