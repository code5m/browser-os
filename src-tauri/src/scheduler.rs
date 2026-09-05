//! M4-7 安全触发与退出：调度循环、重试、退出收口。
//!
//! 契约源：`logs/checkpoints/A6-M4-5-scheduler-contract-20260905-2330.md` §4 / §5（Lane A6 冻结）。
//!
//! 三条硬约束（由 `scripts/check-scheduler-policy.py` 机器守护）：
//!
//! - **F6**：本模块**只做触发**。执行唯一入口是 `script_runner::start_run` / `start_command`，
//!   禁止第二套进程/spawn 路径（`std::process::Command` / `sh -c` / `bash -c`）。
//! - **F7**：退出收口注册在 `bridge::register_shutdown_tasks`，`stop-scheduler` 的索引
//!   必须小于 `kill-running-scripts`（先停触发，再杀运行中的）。
//! - **§7**：tick / 主循环内**不得**逐轮写 `audit.json`（上限 1000 FIFO，每秒 1 条约
//!   17 分钟即把审计冲光）。审计调用一律下沉到 `record_*` 辅助函数，且只在真正
//!   触发 / 跳过 / 错过 / 拒绝 / 自动禁用时写（低频、动作级事件）。
//!
//! 时钟：判定逻辑（`next_fire_after` / `collect_missed_slots` / `is_missed` / `should_fire`）
//! 走 `tasks.rs` 的「时间入参」纯函数（可测性等价且更强）；本模块的 `Clock` trait
//! 供**主循环与 fire 路径**取当前时间，契约 §4.1 允许裸时钟的位置仅限 `SystemClock`
//! 实现与调度主循环。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Manager};

use crate::bridge::AppState;
use crate::domain::{
    MissedRunPolicy, RunStatus, TaskDef, TaskRunRecord, TaskRunTrigger, SCHED_MAX_ATTEMPTS,
    SCHED_RETRY_TOTAL_BUDGET_SECS,
};
use crate::script_runner::{RunError, RunSnapshot, ScriptProcessTable};
use crate::tasks;

/// 主循环节拍：每轮等待的上界（契约 §4.3：`cap` = 1 秒）。
/// 停止信号由此获得 ≤1 秒（实际 ≤50 ms，见 `SystemClock::sleep_until_or_stop`）的响应上界。
const TICK_SECS: u64 = 1;
/// 停止轮询分片（在 `cap` 内再切分，进一步压低停止延迟）。
const STOP_POLL_MS: u64 = 50;

/// 同任务正在收尾（stop 已置位）时的跳过原因。
const SKIP_STOPPING: &str = "stopping";

// ----------------------------- 时钟（契约 §4.1） -----------------------------

/// 可注入时钟。**调度主循环与 fire 路径只经本 trait 取时间**。
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
    /// 可中断等待。返回 `true` = 到期；`false` = 被停止信号唤醒（应立即退出循环）。
    /// `cap`：单次等待硬上界，实现必须把 `deadline - now` 截断到 `cap` 以内。
    fn sleep_until_or_stop(&self, deadline: Instant, stop: &AtomicBool, cap: Duration) -> bool;
}

/// 唯一允许读系统时钟的实现。
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn sleep_until_or_stop(&self, deadline: Instant, stop: &AtomicBool, cap: Duration) -> bool {
        let now = Instant::now();
        let target = if deadline.duration_since(now) > cap {
            now + cap
        } else {
            deadline
        };
        loop {
            if stop.load(Ordering::SeqCst) {
                return false;
            }
            let now = Instant::now();
            if now >= target {
                return true;
            }
            std::thread::sleep((target - now).min(Duration::from_millis(STOP_POLL_MS)));
        }
    }
}

// ----------------------------- 触发计划（纯函数，可单测） -----------------------------

/// 一个待触发的计划触发点。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedFire {
    pub slot: DateTime<Utc>,
    /// true = 补跑（trigger 记为 `CatchUp`）
    pub catch_up: bool,
}

/// 本轮触发决策：应触发的 slot + 应记录的跳过/错过原因。
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SlotPlan {
    pub fire: Vec<PlannedFire>,
    /// 记 `task.run.missed` 的原因（`skipped` / `over_limit`）
    pub missed: Vec<&'static str>,
    /// 记 `task.run.skipped` 的原因（`reentrant`）
    pub skipped: Vec<&'static str>,
}

/// 把收集到的 slot 转成「本轮该做什么」。
///
/// 纯函数（时间全部由入参给出），契约 §5.1 / §5.2 的决策集中在此，便于假时钟单测。
///
/// **O-A7-2（契约张力，已登记交 A0 裁定）**：契约 §5.1 允许 `CatchUp` 在一轮内补跑多个
/// slot，但 §5.2 同时规定「同任务 in_flight 非空则跳过、且**不排队**」。二者在满足
/// 「F9 同任务不并发」的前提下不可兼得：本函数按契约产出**意图**（`fire` 最多
/// `catch_up_limit` 个），执行层在每次真正触发后再查占用，超出的部分在下一轮
/// 按 `reentrant` 跳过（不消耗重试配额）。
pub fn plan_slots(
    slots: &[DateTime<Utc>],
    now: DateTime<Utc>,
    grace_secs: u64,
    policy: MissedRunPolicy,
    catch_up_limit: u32,
    last_fired_at: Option<DateTime<Utc>>,
    occupied: bool,
) -> SlotPlan {
    let mut plan = SlotPlan::default();
    let pending: Vec<DateTime<Utc>> = slots
        .iter()
        .copied()
        .filter(|s| last_fired_at.map(|l| *s > l).unwrap_or(true))
        .filter(|s| tasks::should_fire(now, *s))
        .collect();
    if pending.is_empty() {
        return plan;
    }
    // 同任务 in_flight：全部记 reentrant（F9；不排队、不消耗重试配额）
    if occupied {
        for _ in &pending {
            plan.skipped.push(tasks::SKIP_REENTRANT);
        }
        return plan;
    }

    let on_time: Vec<DateTime<Utc>> = pending
        .iter()
        .copied()
        .filter(|s| !tasks::is_missed(now, *s, grace_secs))
        .collect();
    let missed: Vec<DateTime<Utc>> = pending
        .iter()
        .copied()
        .filter(|s| tasks::is_missed(now, *s, grace_secs))
        .collect();

    if let Some(first) = on_time.first() {
        // 到点的触发点：正常触发最早的一个（触发后同任务即被占用，其余记 reentrant）
        plan.fire.push(PlannedFire {
            slot: *first,
            catch_up: false,
        });
        for _ in on_time.iter().skip(1) {
            plan.skipped.push(tasks::SKIP_REENTRANT);
        }
        return plan;
    }

    // 全部为错过：按策略决定补跑
    match policy {
        MissedRunPolicy::Skip => {
            for _ in &missed {
                plan.missed.push(tasks::MISSED_SKIPPED);
            }
        }
        MissedRunPolicy::RunOnce => {
            // 只补**最新**那个错过的触发点
            if let Some(latest) = missed.last() {
                plan.fire.push(PlannedFire {
                    slot: *latest,
                    catch_up: true,
                });
                for _ in missed.iter().take(missed.len().saturating_sub(1)) {
                    plan.missed.push(tasks::MISSED_SKIPPED);
                }
            }
        }
        MissedRunPolicy::CatchUp => {
            let limit = catch_up_limit.max(1) as usize;
            for (i, slot) in missed.iter().enumerate() {
                if i < limit {
                    plan.fire.push(PlannedFire {
                        slot: *slot,
                        catch_up: true,
                    });
                } else {
                    plan.missed.push(tasks::MISSED_OVER_LIMIT);
                }
            }
        }
    }
    plan
}

// ----------------------------- 运行态 -----------------------------

/// 某个任务的在飞运行（同任务不并发的判据）。
#[derive(Debug, Clone)]
struct InFlight {
    run_id: String,
    scheduled_at: DateTime<Utc>,
    attempt: u32,
    trigger: TaskRunTrigger,
}

/// 等待重试的运行。
#[derive(Debug, Clone)]
struct PendingRetry {
    task_id: String,
    scheduled_at: DateTime<Utc>,
    attempt: u32,
    due_at: DateTime<Utc>,
}

fn in_flight() -> &'static Mutex<HashMap<String, InFlight>> {
    static IN_FLIGHT: OnceLock<Mutex<HashMap<String, InFlight>>> = OnceLock::new();
    IN_FLIGHT.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 触发结果。错误码用稳定字符串（`RunError::code()` 或 `TASK_*`）。
enum FireOutcome {
    Started(String),
    /// 跳过（reentrant / target_busy / global_limit）：不消耗重试配额
    Skipped(&'static str),
    /// 持久性错误：自动 `enabled = false`
    Reject(&'static str),
    /// 可重试错误
    Retryable(&'static str),
}

// ----------------------------- 生命周期 -----------------------------

/// 调度器句柄：停止位。注册在 `ShutdownCoordinator`（任务名 `stop-scheduler`）。
pub struct SchedulerHandle {
    stop: AtomicBool,
}

fn handle_slot() -> &'static OnceLock<SchedulerHandle> {
    static HANDLE: OnceLock<SchedulerHandle> = OnceLock::new();
    &HANDLE
}

/// 供 `bridge::register_shutdown_tasks` 调用：置停止位（**不 join 线程、不取消在飞运行**）。
pub fn request_stop() {
    if let Some(h) = handle_slot().get() {
        h.stop.store(true, Ordering::SeqCst);
    }
}

fn stop_requested(app: &AppHandle) -> bool {
    if let Some(h) = handle_slot().get() {
        if h.stop.load(Ordering::SeqCst) {
            return true;
        }
    }
    app.state::<AppState>()
        .shutdown_requested
        .load(Ordering::SeqCst)
}

/// 启动调度线程（整个进程只启动一次）。
pub fn start(app: AppHandle) {
    let slot = handle_slot();
    if slot.get().is_some() {
        return;
    }
    let _ = slot.set(SchedulerHandle {
        stop: AtomicBool::new(false),
    });
    std::thread::spawn(move || run_loop(&app));
}

// ----------------------------- 主循环 -----------------------------

/// 调度主循环。
///
/// ⚠️ 契约 §7：本函数体内**不得**直接写审计（审计下沉到 `record_*`）。
fn run_loop(app: &AppHandle) {
    let clock = SystemClock;
    let mut retries: Vec<PendingRetry> = Vec::new();
    loop {
        if stop_requested(app) {
            break;
        }
        let now = clock.now_utc();
        tick(app, now, &mut retries);
        let stop = handle_slot().get().map(|h| &h.stop);
        match stop {
            Some(stop) => {
                let deadline = Instant::now() + Duration::from_secs(TICK_SECS);
                if !clock.sleep_until_or_stop(deadline, stop, Duration::from_secs(TICK_SECS)) {
                    break;
                }
            }
            None => break,
        }
    }
    eprintln!("[scheduler] 调度线程已退出");
}

/// 一轮扫描：回收已结束运行 → 到期的重试 → 逐任务触发 → 落盘。
///
/// ⚠️ 契约 §7：本函数体内**不得**直接写审计（审计下沉到 `record_*`）。
fn tick(app: &AppHandle, now: DateTime<Utc>, retries: &mut Vec<PendingRetry>) {
    let path = tasks::tasks_file(app);
    let mut list = tasks::load_tasks_at(&path);
    let mut dirty = false;

    // 1) 回收已结束的运行，并挑出需要自动禁用的任务（持久性错误）
    for task_id in reap_finished(app, &list, retries, now) {
        if let Some(task) = list.iter_mut().find(|t| t.id == task_id) {
            if task.enabled {
                task.enabled = false;
                task.updated_at = now;
                dirty = true;
                record_auto_disabled(app, &task.id, tasks::TASK_TARGET_NOT_FOUND);
            }
        }
    }

    // 2) 到期的重试
    let mut deferred: Vec<PendingRetry> = Vec::new();
    for retry in retries.drain(..) {
        if retry.due_at > now {
            deferred.push(retry);
            continue;
        }
        let task = match list.iter().find(|t| t.id == retry.task_id) {
            Some(t) => t.clone(),
            None => continue,
        };
        if !task.enabled {
            continue;
        }
        if is_occupied(&task.id) {
            record_skip(app, &task.id, tasks::SKIP_REENTRANT);
            continue;
        }
        match fire(
            app,
            &task,
            retry.scheduled_at,
            TaskRunTrigger::Retry,
            retry.attempt,
        ) {
            // 在飞登记已在 `fire` 内完成（slot / attempt 由本次重试携带）
            FireOutcome::Started(_) => {}
            FireOutcome::Skipped(reason) => record_skip(app, &task.id, reason),
            FireOutcome::Reject(code) | FireOutcome::Retryable(code) => {
                record_reject(app, &task.id, code)
            }
        }
    }
    *retries = deferred;

    // 3) 逐任务扫描
    for task in list.iter_mut() {
        if !task.enabled {
            continue;
        }
        let anchor = task.last_fired_at.unwrap_or(task.created_at);
        // 时钟回拨：不追补、不倒退（契约 §4.5）
        if let Some(last) = task.last_fired_at {
            if now < last {
                record_clock_rewind(app, &task.id);
                let next = tasks::next_fire_after(&task.trigger, now);
                if task.next_run_at != next {
                    task.next_run_at = next;
                    task.updated_at = now;
                    dirty = true;
                }
                continue;
            }
        }
        if is_occupied(&task.id) {
            continue;
        }
        let slots = tasks::collect_missed_slots(&task.trigger, anchor, now);
        if !slots.is_empty() {
            let plan = plan_slots(
                &slots,
                now,
                task.misfire_grace_secs,
                task.missed_run_policy,
                task.catch_up_limit,
                task.last_fired_at,
                is_occupied(&task.id),
            );
            for reason in &plan.missed {
                record_missed(app, &task.id, reason);
            }
            for reason in &plan.skipped {
                record_skip(app, &task.id, reason);
            }
            for planned in &plan.fire {
                // 每触发一次后任务即被占用：后续计划点下一轮按 reentrant 跳过（O-A7-2）
                if is_occupied(&task.id) {
                    record_skip(app, &task.id, tasks::SKIP_REENTRANT);
                    continue;
                }
                let trigger = if planned.catch_up {
                    TaskRunTrigger::CatchUp
                } else {
                    TaskRunTrigger::Scheduled
                };
                let slot = planned.slot;
                match fire(app, task, slot, trigger, 1) {
                    FireOutcome::Started(_) => {
                        // last_fired_at 与 next_run_at 同一次原子落盘（契约 §5.1 不变量 3）
                        let fired = task.last_fired_at.map(|l| l.max(slot)).unwrap_or(slot);
                        task.last_fired_at = Some(fired);
                        task.updated_at = now;
                        dirty = true;
                    }
                    FireOutcome::Skipped(reason) => record_skip(app, &task.id, reason),
                    FireOutcome::Reject(code) => {
                        record_reject(app, &task.id, code);
                        task.enabled = false;
                        task.updated_at = now;
                        dirty = true;
                        record_auto_disabled(app, &task.id, code);
                    }
                    FireOutcome::Retryable(code) => {
                        record_reject(app, &task.id, code);
                        let attempt = 1;
                        if task.retry.max_attempts > attempt {
                            let delay = retry_delay(app, task, attempt);
                            retries.push(PendingRetry {
                                task_id: task.id.clone(),
                                scheduled_at: slot,
                                attempt: attempt + 1,
                                due_at: now + chrono::Duration::seconds(delay as i64),
                            });
                        }
                    }
                }
            }
        }
        // 推进 next_run_at（永远指向未来的第一个触发点）
        let base = now.max(anchor);
        let next = tasks::next_fire_after(&task.trigger, base);
        if task.next_run_at != next {
            task.next_run_at = next;
            task.updated_at = now;
            dirty = true;
        }
    }

    if dirty {
        let _ = tasks::save_tasks_at(&path, &list);
    }
}

fn is_occupied(task_id: &str) -> bool {
    in_flight()
        .lock()
        .map(|m| m.contains_key(task_id))
        .unwrap_or_else(|p| p.into_inner().contains_key(task_id))
}

fn mark_in_flight(
    task_id: &str,
    run_id: String,
    scheduled_at: DateTime<Utc>,
    attempt: u32,
    trigger: TaskRunTrigger,
) {
    let mut guard = in_flight().lock().unwrap_or_else(|p| p.into_inner());
    guard.insert(
        task_id.to_string(),
        InFlight {
            run_id,
            scheduled_at,
            attempt,
            trigger,
        },
    );
}

/// 回收已结束的运行：写 `task-runs.json` + 审计，并安排重试。
/// 返回**需要自动禁用**的任务 id（持久性错误）。
fn reap_finished(
    app: &AppHandle,
    list: &[TaskDef],
    retries: &mut Vec<PendingRetry>,
    now: DateTime<Utc>,
) -> Vec<String> {
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    let mut finished: Vec<(String, InFlight, Option<RunSnapshot>)> = Vec::new();
    {
        let guard = in_flight().lock().unwrap_or_else(|p| p.into_inner());
        for (task_id, flight) in guard.iter() {
            let snapshot = table.snapshot(&flight.run_id);
            let done = match &snapshot {
                Some(s) => s.status.is_terminal(),
                // 进程表按容量淘汰终态记录：查不到即视为已结束
                None => true,
            };
            if done {
                finished.push((task_id.clone(), flight.clone(), snapshot));
            }
        }
    }
    let mut to_disable = Vec::new();
    for (task_id, flight, snapshot) in finished {
        let mut guard = in_flight().lock().unwrap_or_else(|p| p.into_inner());
        guard.remove(&task_id);
        drop(guard);

        let status = snapshot
            .as_ref()
            .map(|s| s.status)
            .unwrap_or(RunStatus::Failed);
        let exit_code = snapshot.as_ref().and_then(|s| s.exit_code);
        let error_code = snapshot.as_ref().and_then(|s| s.error.clone());
        let started_at = snapshot
            .as_ref()
            .map(|s| s.started_at)
            .unwrap_or(flight.scheduled_at);
        let record = TaskRunRecord {
            task_id: task_id.clone(),
            run_id: flight.run_id.clone(),
            trigger: flight.trigger,
            scheduled_at: flight.scheduled_at,
            attempt: flight.attempt,
            started_at,
            finished_at: Some(now),
            status,
            exit_code,
            error_code: error_code.clone(),
        };
        let _ = tasks::append_task_run(&tasks::task_runs_file(app), &record);

        // 重试判定（契约 §5.2 / §5.3）
        let failed = matches!(status, RunStatus::Failed | RunStatus::Timeout);
        if failed {
            let task = match list.iter().find(|t| t.id == task_id) {
                Some(t) => t,
                None => continue,
            };
            let max = task.retry.max_attempts.min(SCHED_MAX_ATTEMPTS);
            if flight.attempt < max {
                let delay = retry_delay(app, task, flight.attempt);
                let due_at = now + chrono::Duration::seconds(delay as i64);
                // R-11：重试总时长硬上界（自最初触发点起算）。超出则放弃后续重试，
                // 失败任务保持终态（不自动禁用——属可重试错误，非持久性错误）。
                if retry_within_budget(flight.scheduled_at, due_at, SCHED_RETRY_TOTAL_BUDGET_SECS) {
                    retries.push(PendingRetry {
                        task_id: task_id.clone(),
                        scheduled_at: flight.scheduled_at,
                        attempt: flight.attempt + 1,
                        due_at,
                    });
                }
            }
        }
        // 持久性错误（目标失效/路径被拒/参数非法）→ 自动禁用，避免每轮刷同一条失败
        if let Some(code) = &error_code {
            if is_persistent_error(code) {
                to_disable.push(task_id.clone());
            }
        }
    }
    to_disable
}

fn is_persistent_error(code: &str) -> bool {
    matches!(
        code,
        "SCRIPT_PATH_REJECTED"
            | "CWD_REJECTED"
            | "TIMEOUT_TOO_LARGE"
            | "INVALID_PARAM_NAME"
            | "PARAM_REQUIRED"
            | "DANGEROUS_NEWLINE"
            | "DANGEROUS_CONTROL_CHAR"
            | tasks::TASK_TARGET_NOT_FOUND
    )
}

fn retry_delay(app: &AppHandle, task: &TaskDef, attempt: u32) -> u64 {
    let base = tasks::retry_delay_secs(&task.retry, attempt);
    let max = task.retry.max_delay_secs.max(base);
    // seed 取运行计数，保证同一轮不同任务的抖动不整齐对齐
    let seed = app.state::<AppState>().script_runs.running_count() as u64 + attempt as u64;
    tasks::jitter_secs(base, seed, max)
}

/// R-11：重试总时长硬上界（自最初触发点 `scheduled_at` 起算）。
/// `due_at` 落在 `budget_secs` 内才允许继续重试；超出则放弃后续重试，
/// 失败任务保持终态（不自动禁用——属可重试错误，非持久性错误）。
fn retry_within_budget(
    scheduled_at: DateTime<Utc>,
    due_at: DateTime<Utc>,
    budget_secs: u64,
) -> bool {
    (due_at - scheduled_at).num_seconds() <= budget_secs as i64
}

// ----------------------------- 触发（F6：唯一执行入口） -----------------------------

/// 触发一次运行。**只做调度**：真正执行交给 `script_runner`。
fn fire(
    app: &AppHandle,
    task: &TaskDef,
    slot: DateTime<Utc>,
    trigger: TaskRunTrigger,
    attempt: u32,
) -> FireOutcome {
    // 调用 start_* 之前最后一次查停止位（契约 §5.5：TOCTOU 最小化）
    if stop_requested(app) {
        return FireOutcome::Skipped(SKIP_STOPPING);
    }
    let roots = crate::bridge::allowed_roots(app);
    let home = match app.path().home_dir() {
        Ok(h) => h,
        Err(_) => return FireOutcome::Reject("HOME_UNAVAILABLE"),
    };
    let table: Arc<ScriptProcessTable> = Arc::clone(&app.state::<AppState>().script_runs);
    let records = Some(crate::workspace::script_runs_file(app));

    let result = match task.kind {
        crate::domain::TaskKind::Script => {
            let script = match crate::workspace::load_scripts(app)
                .into_iter()
                .find(|s| s.id == task.target_id)
            {
                Some(s) => s,
                None => return FireOutcome::Reject(tasks::TASK_TARGET_NOT_FOUND),
            };
            if !script.enabled {
                return FireOutcome::Reject("SCRIPT_DISABLED");
            }
            // A10 R-4：复跑 R-3 定义期校验（secret 参数 / 必填 / 键集）。
            // 目标脚本事后把某参数改标 secret，或新增必填参数，既有任务持有明文值
            // 或缺值会在真正执行前被拒，并走 §5.2 持久性错误处置（自动禁用 + 审计）。
            if let Err(e) = tasks::validate_params(&script.params, &task.params) {
                return FireOutcome::Reject(e.code());
            }
            let body = match crate::workspace::script_body_path(app, &script.path) {
                Ok(p) => p,
                Err(_) => return FireOutcome::Reject("SCRIPT_PATH_REJECTED"),
            };
            let mut meta = script;
            if task.timeout_secs > 0 {
                meta.timeout_secs = task.timeout_secs;
            }
            crate::script_runner::start_run(
                &table,
                &meta,
                &body,
                &task.params,
                &roots,
                &home,
                Some(app.clone()),
                records,
            )
        }
        crate::domain::TaskKind::Command => {
            let snippet = match crate::workspace::load_snippets(app)
                .into_iter()
                .find(|s| s.id == task.target_id)
            {
                Some(s) => s,
                None => return FireOutcome::Reject(tasks::TASK_TARGET_NOT_FOUND),
            };
            if !snippet.enabled {
                return FireOutcome::Reject("SNIPPET_DISABLED");
            }
            // A10 R-4：同上，命令片段也复跑 R-3 定义期校验。
            if let Err(e) = tasks::validate_params(&snippet.params, &task.params) {
                return FireOutcome::Reject(e.code());
            }
            let mut snippet = snippet;
            if task.timeout_secs > 0 {
                snippet.timeout_secs = task.timeout_secs;
            }
            crate::script_runner::start_command(
                &table,
                &snippet,
                &task.params,
                &roots,
                &home,
                Some(app.clone()),
                records,
            )
        }
    };

    match result {
        Ok(run_id) => {
            record_run_start(app, &task.id, &run_id, trigger.as_str());
            // 触发成功后**立即**登记在飞（含计划触发点与第几次尝试）：
            // 后续 slot 在下一轮按 reentrant 跳过，不排队、不消耗重试配额（F9）。
            mark_in_flight(&task.id, run_id.clone(), slot, attempt, trigger);
            FireOutcome::Started(run_id)
        }
        Err(e) => {
            let code = e.code();
            match e {
                RunError::AlreadyRunning { .. } => {
                    record_skip(app, &task.id, tasks::SKIP_TARGET_BUSY);
                    FireOutcome::Skipped(tasks::SKIP_TARGET_BUSY)
                }
                RunError::TooManyRuns { .. } => {
                    record_skip(app, &task.id, tasks::SKIP_GLOBAL_LIMIT);
                    FireOutcome::Skipped(tasks::SKIP_GLOBAL_LIMIT)
                }
                RunError::InvalidParam(_)
                | RunError::ScriptPathRejected(_)
                | RunError::CwdRejected(_)
                | RunError::TimeoutTooLarge { .. }
                | RunError::UnknownRun(_) => {
                    record_reject(app, &task.id, code);
                    FireOutcome::Reject(code)
                }
                RunError::SpawnFailed(_) | RunError::UnsupportedPlatform => {
                    FireOutcome::Retryable(code)
                }
            }
        }
    }
}

// ----------------------------- 命令层入口 -----------------------------

/// `task_run_now`：立即触发一次（`trigger = Manual`）。
///
/// **不推进** `last_fired_at` / `next_run_at`（契约 §5.4：不干扰计划）。
pub fn fire_now(app: &AppHandle, id: &str) -> Result<RunSnapshot, String> {
    crate::images::validate_id(id).map_err(|_| tasks::TASK_NOT_FOUND.to_string())?;
    let task = tasks::load_tasks_at(&tasks::tasks_file(app))
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| tasks::TASK_NOT_FOUND.to_string())?;
    let now = SystemClock.now_utc();
    match fire(app, &task, now, TaskRunTrigger::Manual, 1) {
        FireOutcome::Started(run_id) => app
            .state::<AppState>()
            .script_runs
            .snapshot(&run_id)
            .ok_or_else(|| "UNKNOWN_RUN".to_string()),
        FireOutcome::Skipped(_) => Err(tasks::TASK_ALREADY_RUNNING.to_string()),
        FireOutcome::Reject(code) | FireOutcome::Retryable(code) => Err(code.to_string()),
    }
}

/// `task_remove` 前的收口：取消该任务的在飞运行（契约 §5.4）。
/// 返回被取消的运行数。
pub fn cancel_in_flight(app: &AppHandle, id: &str) -> usize {
    let target = {
        let guard = in_flight().lock().unwrap_or_else(|p| p.into_inner());
        guard.get(id).map(|f| f.run_id.clone())
    };
    let Some(run_id) = target else {
        return 0;
    };
    let table = Arc::clone(&app.state::<AppState>().script_runs);
    let ok = table.cancel(&run_id).is_ok();
    if ok {
        let mut guard = in_flight().lock().unwrap_or_else(|p| p.into_inner());
        guard.remove(id);
        1
    } else {
        0
    }
}

// ----------------------------- 审计（契约 §7，低频动作级） -----------------------------

/// 审计 detail 只含 `task_id` / `run_id` / `reason` / `status` / `error_code`，
/// **不含**参数值、命令正文、脚本正文、输出、任何凭据。
/// 契约 §7（A10 R-1）：每次执行都产生的 start/finish **不进** `audit.json`——
/// 最小间隔 60s 的任务单任务即 1440 次/天，会把 cap 1000 的环形缓冲整体冲掉。
/// 执行明细一律落 `task-runs.json`（见 `reap_finished` 的 `append_task_run`）。
/// 仅「手工触发」是低频用户动作，记一条 `task.run.manual`（契约 §6）。
fn record_run_start(app: &AppHandle, task_id: &str, run_id: &str, trigger: &str) {
    if trigger == "manual" {
        crate::workspace::log_audit(
            app,
            "task.run.manual",
            format!("task_id={task_id} run_id={run_id} trigger=manual"),
        );
    }
}

fn record_skip(app: &AppHandle, task_id: &str, reason: &str) {
    crate::workspace::log_audit(
        app,
        "task.run.skipped",
        format!("task_id={task_id} reason={reason}"),
    );
}

fn record_missed(app: &AppHandle, task_id: &str, reason: &str) {
    crate::workspace::log_audit(
        app,
        "task.run.missed",
        format!("task_id={task_id} reason={reason}"),
    );
}

fn record_reject(app: &AppHandle, task_id: &str, error_code: &str) {
    crate::workspace::log_audit(
        app,
        "task.run.reject",
        format!("task_id={task_id} error_code={error_code}"),
    );
}

fn record_auto_disabled(app: &AppHandle, task_id: &str, error_code: &str) {
    crate::workspace::log_audit(
        app,
        "task.auto_disabled",
        format!("task_id={task_id} error_code={error_code}"),
    );
}

fn record_clock_rewind(app: &AppHandle, task_id: &str) {
    crate::workspace::log_audit(app, "task.clock.rewind", format!("task_id={task_id}"));
}

// ---------------------------------------------------------------------------
// 单测：假时钟下的错过执行 / 重入 / 重试决策（T-sched-c5~c7、c10、c13）
// 真实进程取证归 M4-7.d（`T-trig-*`，需 /proc 与真实子进程，本卡不代签）。
// ---------------------------------------------------------------------------
#[cfg(test)]
mod scheduler_tests {
    use super::*;

    fn slots_from(base: DateTime<Utc>, every_secs: i64, count: usize) -> Vec<DateTime<Utc>> {
        (1..=count)
            .map(|i| base + chrono::Duration::seconds(every_secs * i as i64))
            .collect()
    }

    // ---------- T-sched-c5：Skip 策略不补跑 ----------

    #[test]
    fn t_sched_c5_skip_policy_never_catches_up() {
        let base = Utc::now();
        let slots = slots_from(base, 60, 5);
        let now = base + chrono::Duration::seconds(600);
        let plan = plan_slots(&slots, now, 60, MissedRunPolicy::Skip, 3, None, false);
        assert!(plan.fire.is_empty(), "Skip 策略不得补跑");
        assert_eq!(plan.missed.len(), 5, "5 个错过的触发点各记一条 missed");
        assert!(plan.missed.iter().all(|r| *r == tasks::MISSED_SKIPPED));
    }

    // ---------- T-sched-c6：RunOnce 只补最新一次 ----------

    #[test]
    fn t_sched_c6_run_once_fires_latest_slot_only() {
        let base = Utc::now();
        let slots = slots_from(base, 60, 5);
        let now = base + chrono::Duration::seconds(600);
        let plan = plan_slots(&slots, now, 60, MissedRunPolicy::RunOnce, 3, None, false);
        assert_eq!(plan.fire.len(), 1, "RunOnce 只补跑一次");
        assert_eq!(plan.fire[0].slot, slots[4], "必须补**最新**那个错过的点");
        assert!(plan.fire[0].catch_up);
        assert_eq!(plan.missed.len(), 4);
    }

    // ---------- T-sched-c7：CatchUp 受 catch_up_limit 上界 ----------

    #[test]
    fn t_sched_c7_catch_up_is_bounded_by_limit() {
        let base = Utc::now();
        let slots = slots_from(base, 60, 10);
        let now = base + chrono::Duration::seconds(3600);
        let plan = plan_slots(&slots, now, 60, MissedRunPolicy::CatchUp, 3, None, false);
        assert_eq!(plan.fire.len(), 3, "catch_up_limit=3 → 至多补 3 次");
        assert_eq!(plan.missed.len(), 7);
        assert!(plan.missed.iter().all(|r| *r == tasks::MISSED_OVER_LIMIT));

        let plan_all = plan_slots(&slots, now, 60, MissedRunPolicy::CatchUp, 50, None, false);
        assert_eq!(plan_all.fire.len(), 10, "上限放大后全部补跑");
    }

    // ---------- T-sched-c10：同任务并发触发 → reentrant 跳过 ----------

    #[test]
    fn t_sched_c10_same_task_is_never_reentrant() {
        let base = Utc::now();
        let slots = slots_from(base, 60, 3);
        let now = base + chrono::Duration::seconds(600);
        let plan = plan_slots(&slots, now, 60, MissedRunPolicy::CatchUp, 5, None, true);
        assert!(plan.fire.is_empty(), "占用中不得触发");
        assert_eq!(plan.skipped.len(), 3);
        assert!(plan.skipped.iter().all(|r| *r == tasks::SKIP_REENTRANT));
    }

    // ---------- last_fired_at 判重：已触发的 slot 不再出计划 ----------

    #[test]
    fn already_fired_slots_are_filtered_out() {
        let base = Utc::now();
        let slots = slots_from(base, 60, 5);
        let now = base + chrono::Duration::seconds(600);
        let plan = plan_slots(
            &slots,
            now,
            60,
            MissedRunPolicy::CatchUp,
            5,
            Some(slots[2]),
            false,
        );
        assert!(
            plan.fire.iter().all(|f| f.slot > slots[2]),
            "已触发的 slot 必须被过滤（判重真相源 = last_fired_at）"
        );
    }

    // ---------- 未错过的到点点：正常触发且只触发一次 ----------

    #[test]
    fn on_time_slot_fires_once_and_rest_are_reentrant() {
        let base = Utc::now();
        let slots = slots_from(base, 60, 3);
        // now 距最早 slot 仅 10 秒（≤ grace 60），判为「到点」而非「错过」
        let now = slots[0] + chrono::Duration::seconds(10);
        let plan = plan_slots(&slots, now, 60, MissedRunPolicy::Skip, 3, None, false);
        assert_eq!(plan.fire.len(), 1);
        assert!(!plan.fire[0].catch_up, "到点触发不是补跑");
        assert_eq!(plan.fire[0].slot, slots[0]);
    }

    // ---------- 契约 §4.3：可中断等待在 stop 置位后立即返回 ----------

    #[test]
    fn interruptible_sleep_returns_false_when_stopped() {
        let clock = SystemClock;
        let stop = AtomicBool::new(true);
        let start = Instant::now();
        let fired = clock.sleep_until_or_stop(
            Instant::now() + Duration::from_secs(10),
            &stop,
            Duration::from_secs(1),
        );
        assert!(!fired, "stop 已置位时必须立即返回 false");
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "停止响应必须远快于 cap"
        );

        let stop2 = AtomicBool::new(false);
        let fired2 = clock.sleep_until_or_stop(
            Instant::now() + Duration::from_millis(30),
            &stop2,
            Duration::from_secs(1),
        );
        assert!(fired2, "未 stop 且到期应返回 true");
    }

    // ---------- 持久性错误分类 ----------

    #[test]
    fn persistent_errors_are_recognized() {
        assert!(is_persistent_error("SCRIPT_PATH_REJECTED"));
        assert!(is_persistent_error(tasks::TASK_TARGET_NOT_FOUND));
        assert!(
            !is_persistent_error("SPAWN_FAILED"),
            "可重试错误不得自动禁用"
        );
        assert!(!is_persistent_error("TIMEOUT"));
    }

    // ---------- R-11：重试总时长硬上界 ----------

    #[test]
    fn retry_total_duration_is_bounded() {
        use crate::domain::SCHED_RETRY_TOTAL_BUDGET_SECS;
        let base = Utc::now();
        // 超过预算的 due_at 必须被拒
        let too_late = base + chrono::Duration::seconds(SCHED_RETRY_TOTAL_BUDGET_SECS as i64 + 1);
        assert!(
            !retry_within_budget(base, too_late, SCHED_RETRY_TOTAL_BUDGET_SECS),
            "超出总时长预算必须放弃重试"
        );
        // 边界内（含等号）允许
        let on_edge = base + chrono::Duration::seconds(SCHED_RETRY_TOTAL_BUDGET_SECS as i64);
        assert!(
            retry_within_budget(base, on_edge, SCHED_RETRY_TOTAL_BUDGET_SECS),
            "预算边界内应允许重试"
        );
        // 更小的预算同样生效
        assert!(
            !retry_within_budget(base, base + chrono::Duration::seconds(100), 60),
            "更小的预算必须拒绝超界重试"
        );
    }
}
