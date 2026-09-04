//! M2-4.b：脚本执行的**进程组与生命周期内核**。
//!
//! 契约依据（唯一）：`logs/checkpoints/M2-4.a-20260903-2233.md`（冻结裁定书）
//! + `logs/checkpoints/M2-4.b-VERDICT-20260904-0705.md`（独立裁定书，与 a 卡冲突时以裁定为追加记录）。
//!
//! 本模块**只做进程/生命周期**，不含任何 `#[tauri::command]`（命令层归 M2-4.c），
//! 不含输出保留/背压/事件流/落盘（归 M2-4.d）。
//!
//! ## 三条不可动摇的红线
//! 1. **命令构造**：`Command::new(interpreter_bin).arg(path).args(values)` —— argv 数组，
//!    **任何 `sh -c`/`bash -c`/字符串拼接一律禁止**（a 卡 §8.1 P0）。
//! 2. **进程回收**：必须 `setsid` + `killpg` 杀**进程组**；禁止沿用
//!    `grid_process.rs:677/743` 的 `child.kill()`（只杀直接子进程，留孤儿）。
//! 3. **不加引号**：argv 模式下给值加 `'...'` 会把单引号作字面量传进脚本
//!    （裁定书 §3.1，实测 `printf` 收到 `'/tmp/x'`）。
//!
//! ## 线程模型
//! 每个 run 起 3 个线程：supervisor（轮询 wait + 超时/取消）与 2 个 drain（只读丢弃，
//! 防管道写满阻塞子进程）。**进程表不持有 `Child`**——`Child` 在 spawn 后立即移入
//! supervisor 线程，否则任何 `wait` 都得在锁内进行，会阻塞状态查询与并发判定
//! （同 M0-2.b「清理任务持业务锁」的死锁教训）。

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::domain::{ParamType, RunStatus, ScriptInterpreter, ScriptMeta, ScriptParam};
use crate::scripts::{check_required, validate_enum_value, validate_param_value, ScriptError};
use crate::security_policy::check_path_within_roots;

// ----------------------------- 常量（a 卡冻结值） -----------------------------

/// 全局默认超时（a 卡 §2：`ScriptMeta.timeout_secs == 0` 时取此值）。
pub const DEFAULT_TIMEOUT_SECS: u32 = 60;
/// 脚本级超时上限（a 卡 §2：超过即拒绝，**不静默封顶**，fail-closed）。
pub const MAX_TIMEOUT_SECS: u32 = 600;
/// soft 超时（SIGTERM）到 hard（SIGKILL）的宽限（a 卡 §2）。
pub const HARD_GRACE_SECS: u32 = 5;
/// 取消（SIGTERM）到 SIGKILL 的宽限（a 卡 §2）。
pub const CANCEL_GRACE_SECS: u32 = 5;
/// 全局并发护栏（a 卡 §3；R14 语义为「≤8 个**不同**脚本并发」）。
pub const MAX_CONCURRENT_RUNS: usize = 8;
/// 进程表容量（对齐 a 卡 §7 `MAX_RUN_RECORDS`；**只淘汰终态**）。
pub const MAX_TABLE_ENTRIES: usize = 200;
/// supervisor 轮询间隔。50 ms 对秒级超时（≥2 s）精度足够，且无需跨线程同步 wait 结果。
pub const POLL_INTERVAL_MS: u64 = 50;

/// env 固定最小集（a 卡 §5：`env_clear()` + 闭集，防 `LD_PRELOAD`/`SSH_AUTH_SOCK` 泄露）。
pub const MINIMAL_ENV_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

// ----------------------------- 运行期错误 -----------------------------

/// 运行期错误。**与 `scripts::ScriptError` 分工**：后者是定义期/校验期错误（M2-3 领域），
/// 本枚举只表达「进程能不能起、能不能收」的运行期问题。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// 同一个 `script_id` 已有 run 处于 `Running`（a 卡 §3）。
    AlreadyRunning { script_id: String },
    /// 全局并发已达 `MAX_CONCURRENT_RUNS`。
    TooManyRuns { limit: usize },
    /// 脚本级 `timeout_secs` 超过 `MAX_TIMEOUT_SECS`。
    TimeoutTooLarge { limit: u32 },
    /// 脚本正文路径不在允许根目录内，或不存在。
    ScriptPathRejected(String),
    /// cwd（脚本所在目录）锁定失败。
    CwdRejected(String),
    /// 非 Unix 平台：进程组能力缺失（a 卡 §4 显式降级，不做 Windows Job Object）。
    UnsupportedPlatform,
    /// `spawn` 失败。
    SpawnFailed(String),
    /// 参数校验未通过（包装 `ScriptError`，保留其稳定错误码）。
    InvalidParam(ScriptError),
    /// `run_id` 不存在。
    UnknownRun(String),
}

impl RunError {
    /// 稳定错误码（c 卡前端据此选文案，不得随意改名）。
    pub fn code(&self) -> &'static str {
        match self {
            RunError::AlreadyRunning { .. } => "SCRIPT_ALREADY_RUNNING",
            RunError::TooManyRuns { .. } => "TOO_MANY_RUNS",
            RunError::TimeoutTooLarge { .. } => "TIMEOUT_TOO_LARGE",
            RunError::ScriptPathRejected(_) => "SCRIPT_PATH_REJECTED",
            RunError::CwdRejected(_) => "CWD_REJECTED",
            RunError::UnsupportedPlatform => "UNSUPPORTED_PLATFORM",
            RunError::SpawnFailed(_) => "SPAWN_FAILED",
            RunError::InvalidParam(e) => e.code(),
            RunError::UnknownRun(_) => "UNKNOWN_RUN",
        }
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl From<ScriptError> for RunError {
    fn from(e: ScriptError) -> Self {
        RunError::InvalidParam(e)
    }
}

// ----------------------------- 状态与进程表 -----------------------------

/// 一次运行的状态快照（供 c 卡 `script_status` 查询；**不含输出正文**，输出归 d 卡）。
///
/// 消费者 = **M2-4.c**（`script_status` 返回）+ **M2-4.d**（`script-runs.json` 落盘）。
/// b 卡只写入、不外露命令层，故字段级标注 `#[allow(dead_code)]`（同 M2-3 口径）。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RunSnapshot {
    pub run_id: String,
    pub script_id: String,
    pub status: RunStatus,
    pub started_at: chrono::DateTime<Utc>,
    pub finished_at: Option<chrono::DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
}

struct RunEntry {
    script_id: String,
    /// Unix：`setsid` 成功后等于 `child.id()`（新会话组长，pid == pgid == sid）。
    /// 非 Unix / 假设不成立：None（降级路径见 `terminate_group`）。
    pgid: Option<i32>,
    /// 取消信号。**cancel 只置位、不阻塞**（ShutdownCoordinator 契约：清理任务不得阻塞）。
    cancel: Arc<AtomicBool>,
    state: Arc<Mutex<RunSnapshot>>,
}

/// 脚本进程表：`run_id -> RunEntry`。终态 entry **保留**（供状态查询），超容量只淘汰终态。
pub struct ScriptProcessTable {
    runs: Mutex<HashMap<String, RunEntry>>,
}

impl Default for ScriptProcessTable {
    fn default() -> Self {
        Self::new()
    }
}

/// 消费者 = **M2-4.c**（`script_status`/`cancel_script` 命令）。b 卡只提供内核、
/// 不接命令层（a 卡 §9 边界），故整块标注 `#[allow(dead_code)]`；
/// `evict_if_needed` 除外——它由 `start_run` 真实调用。
#[allow(dead_code)]
impl ScriptProcessTable {
    pub fn new() -> Self {
        Self {
            runs: Mutex::new(HashMap::new()),
        }
    }

    /// 当前处于 `Running` 的 run 数量（并发上限判定用）。
    pub fn running_count(&self) -> usize {
        self.lock_runs()
            .values()
            .filter(|e| {
                e.state
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .status
                    .is_terminal()
                    == false
            })
            .count()
    }

    /// 该 `script_id` 是否已有运行状态。
    pub fn has_running_script(&self, script_id: &str) -> bool {
        self.lock_runs().values().any(|e| {
            e.script_id == script_id
                && !e
                    .state
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .status
                    .is_terminal()
        })
    }

    /// 查询快照。
    pub fn snapshot(&self, run_id: &str) -> Option<RunSnapshot> {
        self.lock_runs()
            .get(run_id)
            .map(|e| e.state.lock().unwrap_or_else(|p| p.into_inner()).clone())
    }

    /// 请求取消：**只置信号位，立即返回，不阻塞**（真正的 kill 由 supervisor 线程执行）。
    /// 对已终态的 run 为幂等 no-op（R13）。
    pub fn cancel(&self, run_id: &str) -> Result<(), RunError> {
        let runs = self.lock_runs();
        let entry = runs
            .get(run_id)
            .ok_or(RunError::UnknownRun(run_id.to_string()))?;
        // 终态不可互转（a 卡 §2），且不得重复触发 kill。
        if entry
            .state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .status
            .is_terminal()
        {
            return Ok(());
        }
        entry.cancel.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// 淘汰超容量的**终态** entry（FIFO，丢最旧的）；`Running` 永不淘汰。
    fn evict_if_needed(&self) {
        let mut runs = self.lock_runs();
        if runs.len() <= MAX_TABLE_ENTRIES {
            return;
        }
        // 按 finished_at 排序只淘汰终态；未终态的 finished_at 为 None，排在最后（永不淘汰）。
        let mut terminal: Vec<(chrono::DateTime<Utc>, String)> = runs
            .iter()
            .filter_map(|(id, e)| {
                let st = e.state.lock().unwrap_or_else(|p| p.into_inner());
                st.finished_at.map(|t| (t, id.clone()))
            })
            .collect();
        terminal.sort();
        let excess = runs.len() - MAX_TABLE_ENTRIES;
        for (_, id) in terminal.into_iter().take(excess) {
            runs.remove(&id);
        }
    }

    fn lock_runs(&self) -> std::sync::MutexGuard<'_, HashMap<String, RunEntry>> {
        self.runs.lock().unwrap_or_else(|p| p.into_inner())
    }
}

// ----------------------------- 纯函数 -----------------------------

/// 计算有效超时：`0` → 全局默认 60；`>600` → 拒绝（不静默封顶）。
pub fn effective_timeout(script_timeout: u32) -> Result<u32, RunError> {
    match script_timeout {
        0 => Ok(DEFAULT_TIMEOUT_SECS),
        t if t <= MAX_TIMEOUT_SECS => Ok(t),
        _ => Err(RunError::TimeoutTooLarge {
            limit: MAX_TIMEOUT_SECS,
        }),
    }
}

/// 构造 argv（**纯函数、可单测**）。
///
/// 输出形如 `[interpreter_bin?, script_abs_path, v1, v2, ...]`：
/// - `Shebang` 无解释器位（直接 exec 脚本文件）。
/// - 参数严格按 `params` 声明顺序；**键名不进命令行**（a 卡 §8.1）。
/// - **每个值都是独立 argv 元素，一律不加引号**（裁定书 §3.1）。
pub fn build_argv(
    interpreter: ScriptInterpreter,
    script_abs_path: &Path,
    params: &[ScriptParam],
    values: &HashMap<String, String>,
    roots: &[PathBuf],
) -> Result<Vec<String>, RunError> {
    let mut argv: Vec<String> = Vec::with_capacity(params.len() + 2);

    // 1) 解释器来自枚举白名单，永不来自用户输入（a 卡 §8.1）。
    if let Some(bin) = interpreter.binary() {
        argv.push(bin.to_string());
    }

    // 2) 脚本绝对路径（调用方已 canonicalize + 根目录校验；此处再兜一次底）。
    let script_str = script_abs_path.to_string_lossy().to_string();
    check_path_within_roots(&script_str, roots)
        .map_err(|e| RunError::ScriptPathRejected(format!("脚本路径不在允许根目录内: {e}")))?;
    argv.push(script_str);

    // 3) 参数按声明顺序展开，逐个过校验。
    for param in params {
        let value: Option<&String> = values.get(&param.name).or(param.default.as_ref());
        let value: &str = match value {
            Some(v) => v,
            None => {
                // 未提供的必填参数由 check_required 统一判定，交给它给出稳定错误码。
                check_required(param, None).map_err(RunError::InvalidParam)?;
                continue;
            }
        };

        check_required(param, Some(value)).map_err(RunError::InvalidParam)?;
        validate_param_value(value, param.param_type).map_err(RunError::InvalidParam)?;

        match param.param_type {
            ParamType::Enum => {
                validate_enum_value(value, &param.options).map_err(RunError::InvalidParam)?;
            }
            ParamType::Path => {
                check_path_within_roots(value, roots)
                    .map_err(|_| RunError::InvalidParam(ScriptError::PathEscape))?;
            }
            ParamType::String | ParamType::Int | ParamType::Bool => {}
        }

        // 4) 一律不加引号（裁定书 §3.1）：argv 数组不经 shell，引号会变字面量。
        argv.push(value.to_string());
    }

    Ok(argv)
}

// ----------------------------- 平台分支（进程组） -----------------------------

/// 在新进程组（新会话）中 spawn，使后续可以整组 kill（a 卡 §8.4）。
#[cfg(unix)]
fn spawn_in_new_group(cmd: &mut Command) -> std::io::Result<Child> {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.spawn()
}

/// 非 Unix：`setsid`/`killpg` 不可用，显式降级（a 卡 §4）。
#[cfg(not(unix))]
fn spawn_in_new_group(_cmd: &mut Command) -> std::io::Result<Child> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "UNSUPPORTED_PLATFORM",
    ))
}

/// 向整个进程组发信号。
#[cfg(unix)]
fn kill_group(pgid: i32, sig: i32) {
    unsafe {
        libc::killpg(pgid, sig);
    }
}

#[cfg(not(unix))]
fn kill_group(_pgid: i32, _sig: i32) {}

/// 进程组是否仍有存活成员（信号 0 探测；ESRCH 即整组已消失）。
#[cfg(unix)]
pub fn process_group_alive(pgid: i32) -> bool {
    unsafe { libc::killpg(pgid, 0) == 0 }
}

#[cfg(not(unix))]
pub fn process_group_alive(_pgid: i32) -> bool {
    false
}

/// 读取 `/proc/<pid>/stat` 的第 5 字段（pgrp），用于验证 `setsid` 后 `pgid == pid` 的假设。
/// `comm` 可能含空格与括号，故从最后一个 `)` 之后开始切分。
///
/// 仅用于测试取证（B12），非 test 编译不产出，避免 dead_code。
#[cfg(test)]
pub fn process_group_id_of(pid: u32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = stat.rsplit_once(')')?.1;
    // rest 形如 " S <ppid> <pgrp> <session> ..."：index 0=state, 1=ppid, 2=pgrp
    rest.split_whitespace().nth(2)?.parse().ok()
}

// ----------------------------- 线程：drain / supervisor -----------------------------

/// 只读丢弃管道内容（§2.2 路线 B）。
///
/// 目的**不是**实现输出功能，而是防止「piped 而无人读」导致子进程阻塞在 `write`
/// （Linux 默认管道容量 64 KiB，实测父进程不读时子进程无法退出）。
/// **不保留字节、不置 truncated、不节流、不 emit 事件**——那些归 M2-4.d。
fn spawn_drain<R: Read + Send + 'static>(mut pipe: R) {
    thread::spawn(move || {
        let mut sink = std::io::sink();
        // 错误只可能是 EPIPE 之类，无碍；丢弃流本来就不关心结果。
        let _ = std::io::copy(&mut pipe, &mut sink);
    });
}

/// 终止整个进程组：SIGTERM → 轮询等待 → 超时未死则 SIGKILL。
/// `pgid == None`（非 unix 或 setsid 假设不成立）时无进程组可杀，只登记能力缺口。
fn terminate_group(pgid: Option<i32>, grace_secs: u32) {
    let Some(pgid) = pgid else {
        return;
    };
    kill_group(pgid, libc::SIGTERM);
    let deadline = Instant::now() + Duration::from_secs(grace_secs as u64);
    while Instant::now() < deadline {
        if !process_group_alive(pgid) {
            return;
        }
        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
    kill_group(pgid, libc::SIGKILL);
}

/// supervisor：轮询 wait + 超时/取消 + 收尾状态。
/// **`Child` 的所有权在此线程**，进程表不持 `Child`。
fn supervise(
    mut child: Child,
    pgid: Option<i32>,
    timeout_secs: u32,
    cancel: Arc<AtomicBool>,
    state: Arc<Mutex<RunSnapshot>>,
) {
    let started = Instant::now();

    let finish = |status: RunStatus, exit_code: Option<i32>, error: Option<String>| {
        let mut st = state.lock().unwrap_or_else(|p| p.into_inner());
        // 终态不可互转（a 卡 §2）：已有终态则不再改写。
        if st.status.is_terminal() {
            return;
        }
        st.status = status;
        st.exit_code = exit_code;
        st.error = error;
        st.finished_at = Some(Utc::now());
    };

    loop {
        match child.try_wait() {
            Ok(Some(exit)) => {
                let code = exit.code();
                // 退出码 0 视为成功（a 卡 §7：`Succeeded` 语义）。
                let status = if code == Some(0) {
                    RunStatus::Succeeded
                } else {
                    RunStatus::Failed
                };
                finish(status, code, None);
                return;
            }
            Ok(None) => {
                if cancel.load(Ordering::SeqCst) {
                    terminate_group(pgid, CANCEL_GRACE_SECS);
                    let code = child.wait().ok().and_then(|s| s.code());
                    finish(RunStatus::Cancelled, code, None);
                    return;
                }
                if started.elapsed().as_secs() >= timeout_secs as u64 {
                    terminate_group(pgid, HARD_GRACE_SECS);
                    let code = child.wait().ok().and_then(|s| s.code());
                    finish(RunStatus::Timeout, code, None);
                    return;
                }
                thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            }
            Err(e) => {
                finish(
                    RunStatus::Failed,
                    None,
                    Some(format!("等待子进程失败: {e}")),
                );
                return;
            }
        }
    }
}

// ----------------------------- 入口 -----------------------------

/// 启动一次脚本运行，返回 `run_id`。
///
/// 并发判定与注册在同一把锁内完成（避免 TOCTOU）：先占位为 `Running`，
/// spawn 失败再移除占位。
///
/// 消费者 = **M2-4.c**（`run_script` 命令）。b 卡只提供内核、不接命令层
/// （a 卡 §9 边界），故标注 `#[allow(dead_code)]`（同 M2-3 `join_image_path` 口径）。
#[allow(dead_code)]
pub fn start_run(
    table: &Arc<ScriptProcessTable>,
    meta: &ScriptMeta,
    script_abs_path: &Path,
    values: &HashMap<String, String>,
    roots: &[PathBuf],
    home_dir: &Path,
) -> Result<String, RunError> {
    let timeout = effective_timeout(meta.timeout_secs)?;

    // --- 1) 并发检查 + 占位注册（同一锁内，原子） ---
    let run_id = format!("run-{}", uuid::Uuid::new_v4());
    let cancel = Arc::new(AtomicBool::new(false));
    let state = Arc::new(Mutex::new(RunSnapshot {
        run_id: run_id.clone(),
        script_id: meta.id.clone(),
        status: RunStatus::Running,
        started_at: Utc::now(),
        finished_at: None,
        exit_code: None,
        error: None,
    }));

    {
        let mut runs = table.lock_runs();
        if runs
            .values()
            .any(|e| e.script_id == meta.id && !is_terminal_entry(e))
        {
            return Err(RunError::AlreadyRunning {
                script_id: meta.id.clone(),
            });
        }
        let running = runs.values().filter(|e| !is_terminal_entry(e)).count();
        if running >= MAX_CONCURRENT_RUNS {
            return Err(RunError::TooManyRuns {
                limit: MAX_CONCURRENT_RUNS,
            });
        }
        runs.insert(
            run_id.clone(),
            RunEntry {
                script_id: meta.id.clone(),
                pgid: None,
                cancel: Arc::clone(&cancel),
                state: Arc::clone(&state),
            },
        );
    }
    table.evict_if_needed();

    // --- 2) 构造 argv 与命令（P0：argv 数组，无 shell 拼接） ---
    let argv = match build_argv(
        meta.interpreter,
        script_abs_path,
        &meta.params,
        values,
        roots,
    ) {
        Ok(a) => a,
        Err(e) => {
            table.lock_runs().remove(&run_id);
            return Err(e);
        }
    };
    let (program, rest) = argv.split_first().ok_or_else(|| {
        table.lock_runs().remove(&run_id);
        RunError::ScriptPathRejected("argv 为空".to_string())
    })?;

    // cwd 锁定为脚本所在目录，且必须在允许根目录内（a 卡 §8.2，用户不可控）。
    let cwd = match script_abs_path.parent() {
        Some(p) if check_path_within_roots(&p.to_string_lossy(), roots).is_ok() => p.to_path_buf(),
        _ => {
            table.lock_runs().remove(&run_id);
            return Err(RunError::CwdRejected(
                "脚本所在目录不在允许根目录内".to_string(),
            ));
        }
    };

    let mut cmd = Command::new(program);
    cmd.args(rest)
        .current_dir(&cwd)
        .env_clear()
        .env("PATH", MINIMAL_ENV_PATH)
        .env("HOME", home_dir)
        .env("LANG", "C.UTF-8")
        .env("TERM", "dumb")
        .stdin(Stdio::null())
        // §2.2 路线 B：piped + 只读丢弃 drain（防管道写满阻塞）。
        // TODO(M2-4.d): 把 drain sink 换成环形缓冲 + 事件流（背压/尾存/落盘归 d 卡）。
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // --- 3) spawn（新进程组） ---
    let mut child = match spawn_in_new_group(&mut cmd) {
        Ok(c) => c,
        Err(e) => {
            table.lock_runs().remove(&run_id);
            return if e.kind() == std::io::ErrorKind::Unsupported {
                Err(RunError::UnsupportedPlatform)
            } else {
                Err(RunError::SpawnFailed(e.to_string()))
            };
        }
    };

    let pgid = child.id() as i32;

    // --- 4) drain 线程（必须在 supervisor 之前，否则管道无人读） ---
    if let Some(out) = child.stdout.take() {
        spawn_drain(out);
    }
    if let Some(err) = child.stderr.take() {
        spawn_drain(err);
    }

    // --- 5) 回填 pgid 并交给 supervisor ---
    if let Some(entry) = table.lock_runs().get_mut(&run_id) {
        entry.pgid = Some(pgid);
    }
    let st = Arc::clone(&state);
    thread::spawn(move || {
        supervise(child, Some(pgid), timeout, cancel, st);
    });

    Ok(run_id)
}

/// 退出收口：**遍历 Running → 置取消位 → kill 进程组 → 标记 Cancelled**。
///
/// 消费者 = **M2-4.d**（`kill-running-scripts` shutdown 任务，注册在 `shutdown-grid` 之后，
/// 见 a 卡 §8.6）。b 卡阶段无调用者，故标注 `#[allow(dead_code)]`（同 M2-3 `join_image_path` 口径）。
#[allow(dead_code)]
pub fn kill_all_running(table: &ScriptProcessTable) -> usize {
    // 先收集，再 kill：避免持 state 锁做 IO（killpg 可能慢）。
    let targets: Vec<(Option<i32>, Arc<AtomicBool>, Arc<Mutex<RunSnapshot>>)> = {
        let runs = table.lock_runs();
        runs.iter()
            .filter(|(_, e)| !is_terminal_entry(e))
            .map(|(_, e)| (e.pgid, Arc::clone(&e.cancel), Arc::clone(&e.state)))
            .collect()
    };

    let mut killed = 0;
    for (pgid, cancel, state) in targets {
        cancel.store(true, Ordering::SeqCst);
        terminate_group(pgid, CANCEL_GRACE_SECS);
        let mut st = state.lock().unwrap_or_else(|p| p.into_inner());
        if !st.status.is_terminal() {
            st.status = RunStatus::Cancelled;
            st.finished_at = Some(Utc::now());
            killed += 1;
        }
    }
    killed
}

fn is_terminal_entry(entry: &RunEntry) -> bool {
    entry
        .state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .status
        .is_terminal()
}

// ----------------------------- 测试（B1~B14） -----------------------------

#[cfg(test)]
mod script_runner_tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("m2_4b_{tag}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&d).expect("建临时目录");
        d
    }

    fn write_script(dir: &Path, body: &str) -> PathBuf {
        let p = dir.join(format!("{}.sh", uuid::Uuid::new_v4()));
        fs::write(&p, body).expect("写脚本");
        p
    }

    fn param(name: &str, t: ParamType, required: bool) -> ScriptParam {
        ScriptParam {
            name: name.to_string(),
            label: name.to_string(),
            param_type: t,
            required,
            default: None,
            options: vec![],
            raw: false,
            secret: false,
        }
    }

    fn meta_with(interpreter: ScriptInterpreter, params: Vec<ScriptParam>) -> ScriptMeta {
        ScriptMeta {
            id: uuid::Uuid::new_v4().to_string(),
            name: "t".to_string(),
            category: "t".to_string(),
            path: "t.sh".to_string(),
            interpreter,
            params,
            description: String::new(),
            builtin: false,
            enabled: true,
            timeout_secs: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// 轮询等待进入终态（避免测试 flaky）。
    fn wait_terminal(table: &ScriptProcessTable, run_id: &str, millis: u64) -> RunSnapshot {
        let deadline = Instant::now() + Duration::from_millis(millis);
        loop {
            let snap = table.snapshot(run_id).expect("run 应存在");
            if snap.status.is_terminal() {
                return snap;
            }
            assert!(
                Instant::now() < deadline,
                "等待终态超时（仍为 {:?}）",
                snap.status
            );
            thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        }
    }

    // ---------- B1：解释器映射与参数顺序 ----------
    #[test]
    fn b1_build_argv_interpreter_mapping_and_order() {
        let dir = temp_dir("b1");
        let script = write_script(&dir, "echo hi\n");
        let roots = vec![dir.clone()];
        let values = HashMap::new();

        let cases = [
            (ScriptInterpreter::Bash, Some("bash")),
            (ScriptInterpreter::Sh, Some("sh")),
            (ScriptInterpreter::Python3, Some("python3")),
            (ScriptInterpreter::Node, Some("node")),
            (ScriptInterpreter::Shebang, None),
        ];
        for (interp, bin) in cases {
            let argv = build_argv(interp, &script, &[], &values, &roots).expect("构造成功");
            match bin {
                Some(b) => {
                    assert_eq!(argv[0], b, "{interp:?} 应映射到 {b}");
                    assert_eq!(argv[1], script.to_string_lossy(), "脚本路径紧随解释器");
                    assert_eq!(argv.len(), 2);
                }
                None => {
                    assert_eq!(argv[0], script.to_string_lossy(), "Shebang 无解释器位");
                    assert_eq!(argv.len(), 1);
                }
            }
        }

        // 参数顺序严格按声明顺序，键名不进命令行
        let params = vec![
            param("second", ParamType::String, false),
            param("first", ParamType::String, false),
        ];
        let mut values = HashMap::new();
        values.insert("second".to_string(), "2".to_string());
        values.insert("first".to_string(), "1".to_string());
        let argv = build_argv(ScriptInterpreter::Bash, &script, &params, &values, &roots).unwrap();
        assert_eq!(
            argv,
            vec!["bash", &script.to_string_lossy().to_string(), "2", "1"],
            "顺序必须按声明顺序（second 先于 first）"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B2：危险值拒绝 ----------
    #[test]
    fn b2_build_argv_rejects_dangerous_values() {
        let dir = temp_dir("b2");
        let script = write_script(&dir, "echo hi\n");
        let roots = vec![dir.clone()];
        let params = vec![param("v", ParamType::String, false)];

        for bad in ["; rm -rf /tmp/x", "$(whoami)", "`id`", "a && b", "a | b"] {
            let mut values = HashMap::new();
            values.insert("v".to_string(), bad.to_string());
            let err = build_argv(ScriptInterpreter::Bash, &script, &params, &values, &roots)
                .expect_err("危险值必须被拒");
            assert!(
                matches!(err, RunError::InvalidParam(_)),
                "{bad} 应触发 InvalidParam，实际 {err:?}"
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B3：Path 逃逸 ----------
    #[test]
    fn b3_build_argv_rejects_path_escape() {
        let dir = temp_dir("b3");
        let script = write_script(&dir, "echo hi\n");
        let roots = vec![dir.clone()];
        let params = vec![param("p", ParamType::Path, false)];

        let mut values = HashMap::new();
        values.insert("p".to_string(), "../../etc/passwd".to_string());
        let err = build_argv(ScriptInterpreter::Bash, &script, &params, &values, &roots)
            .expect_err("路径逃逸必须被拒");
        assert_eq!(err.code(), "PATH_ESCAPE");
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B4：必填与枚举 ----------
    #[test]
    fn b4_build_argv_required_and_enum() {
        let dir = temp_dir("b4");
        let script = write_script(&dir, "echo hi\n");
        let roots = vec![dir.clone()];

        // 缺必填
        let params = vec![param("need", ParamType::String, true)];
        let err = build_argv(
            ScriptInterpreter::Bash,
            &script,
            &params,
            &HashMap::new(),
            &roots,
        )
        .expect_err("必填缺失必须被拒");
        assert_eq!(err.code(), "PARAM_REQUIRED");

        // Enum 不在 options 内
        let mut ep = param("mode", ParamType::Enum, false);
        ep.options = vec!["a".to_string(), "b".to_string()];
        let mut values = HashMap::new();
        values.insert("mode".to_string(), "c".to_string());
        let err = build_argv(ScriptInterpreter::Bash, &script, &[ep], &values, &roots)
            .expect_err("越界枚举值必须被拒");
        assert_eq!(err.code(), "PARAM_NOT_IN_OPTIONS");
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B5：一律不加引号（裁定书 §3.1 回归锚点） ----------
    #[test]
    fn b5_build_argv_never_quotes() {
        let dir = temp_dir("b5");
        let script = write_script(&dir, "echo hi\n");
        let roots = vec![dir.clone()];
        let params = vec![
            param("s", ParamType::String, false),
            param("sp", ParamType::String, false),
        ];
        let mut values = HashMap::new();
        values.insert("s".to_string(), "it's fine".to_string());
        values.insert("sp".to_string(), "a b 中文".to_string());

        let argv = build_argv(ScriptInterpreter::Bash, &script, &params, &values, &roots).unwrap();
        assert_eq!(argv[2], "it's fine", "含单引号的值必须原样，不得额外加引号");
        assert_eq!(argv[3], "a b 中文", "含空格/中文的值必须原样");
        assert!(
            !argv[2].starts_with('\'') && !argv[2].ends_with('\''),
            "argv 模式下不得出现包裹引号"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B6：超时解析 ----------
    #[test]
    fn b6_effective_timeout() {
        assert_eq!(effective_timeout(0).unwrap(), DEFAULT_TIMEOUT_SECS);
        assert_eq!(DEFAULT_TIMEOUT_SECS, 60, "全局默认必须为 60（a 卡 §2）");
        assert_eq!(effective_timeout(120).unwrap(), 120);
        assert_eq!(
            effective_timeout(601).unwrap_err().code(),
            "TIMEOUT_TOO_LARGE"
        );
        assert_eq!(effective_timeout(MAX_TIMEOUT_SECS).unwrap(), 600);
    }

    // ---------- B7：取消 → 进程组消失（R7） ----------
    #[test]
    fn b7_cancel_kills_process_group() {
        let dir = temp_dir("b7");
        let script = write_script(&dir, "sleep 300\n");
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];
        let meta = meta_with(ScriptInterpreter::Bash, vec![]);

        let run_id =
            start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir).expect("启动成功");
        let pgid = table
            .lock_runs()
            .get(&run_id)
            .expect("entry")
            .pgid
            .expect("pgid");
        assert!(process_group_alive(pgid), "启动后进程组应存活");

        table.cancel(&run_id).expect("取消请求");
        let snap = wait_terminal(&table, &run_id, 8_000);
        assert_eq!(snap.status, RunStatus::Cancelled);
        assert!(
            !process_group_alive(pgid),
            "取消后进程组必须整组消失（不是只杀直接子进程）"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B8：超时（R8） ----------
    #[test]
    fn b8_timeout_kills_process_group() {
        let dir = temp_dir("b8");
        let script = write_script(&dir, "sleep 300\n");
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];
        let mut meta = meta_with(ScriptInterpreter::Bash, vec![]);
        meta.timeout_secs = 2; // 缩短以便测试

        let started = Instant::now();
        let run_id =
            start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir).expect("启动成功");
        let pgid = table
            .lock_runs()
            .get(&run_id)
            .expect("entry")
            .pgid
            .expect("pgid");

        let snap = wait_terminal(&table, &run_id, 15_000);
        assert_eq!(snap.status, RunStatus::Timeout, "2 秒软超时后应为 Timeout");
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "不应长时间挂起（实际 {:?}）",
            started.elapsed()
        );
        assert!(!process_group_alive(pgid), "超时后进程组必须消失");
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B9：子进程树整组回收（R10） ----------
    #[test]
    fn b9_cancel_kills_child_tree() {
        let dir = temp_dir("b9");
        // 脚本自己派生子进程：只有 killpg（进程组）才能整组收掉
        let script = write_script(&dir, "sleep 300 &\nCHILD=$!\nwait \"$CHILD\"\n");
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];
        let meta = meta_with(ScriptInterpreter::Bash, vec![]);

        let run_id =
            start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir).expect("启动成功");
        let pgid = table
            .lock_runs()
            .get(&run_id)
            .expect("entry")
            .pgid
            .expect("pgid");

        table.cancel(&run_id).expect("取消请求");
        let snap = wait_terminal(&table, &run_id, 8_000);
        assert_eq!(snap.status, RunStatus::Cancelled);
        assert!(
            !process_group_alive(pgid),
            "子进程树必须整组回收（child.kill() 做不到这点）"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B10：终态取消幂等（R13） ----------
    #[test]
    fn b10_cancel_terminal_is_idempotent() {
        let dir = temp_dir("b10");
        let script = write_script(&dir, "exit 0\n");
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];
        let meta = meta_with(ScriptInterpreter::Bash, vec![]);

        let run_id =
            start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir).expect("启动成功");
        let snap = wait_terminal(&table, &run_id, 8_000);
        assert_eq!(snap.status, RunStatus::Succeeded);

        // 已终态：取消应幂等返回 Ok，且状态不被改写
        assert!(table.cancel(&run_id).is_ok(), "R13：终态取消必须 Ok(())");
        let after = table.snapshot(&run_id).expect("仍在");
        assert_eq!(
            after.status,
            RunStatus::Succeeded,
            "终态不可被改写成 Cancelled"
        );

        // 未知 run_id 报错
        assert!(matches!(
            table.cancel("run-not-exist"),
            Err(RunError::UnknownRun(_))
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B11：并发上限 + 同 id 禁并发（R14） ----------
    #[test]
    fn b11_concurrency_limits() {
        let dir = temp_dir("b11");
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];

        let mut run_ids = Vec::new();
        for i in 0..MAX_CONCURRENT_RUNS {
            let script = write_script(&dir, "sleep 30\n");
            let meta = meta_with(ScriptInterpreter::Bash, vec![]);
            let id = start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir)
                .unwrap_or_else(|e| panic!("第 {i} 个应能启动：{e:?}"));
            run_ids.push(id);
        }
        assert_eq!(table.running_count(), MAX_CONCURRENT_RUNS);

        // 第 9 个不同脚本：拒绝
        let script9 = write_script(&dir, "sleep 30\n");
        let meta9 = meta_with(ScriptInterpreter::Bash, vec![]);
        let err = start_run(&table, &meta9, &script9, &HashMap::new(), &roots, &dir)
            .expect_err("第 9 个必须被拒");
        assert_eq!(err.code(), "TOO_MANY_RUNS");

        // 同 script_id 第二次：拒绝（即使未达并发上限）
        let table2 = Arc::new(ScriptProcessTable::new());
        let s = write_script(&dir, "sleep 30\n");
        let meta = meta_with(ScriptInterpreter::Bash, vec![]);
        start_run(&table2, &meta, &s, &HashMap::new(), &roots, &dir).expect("首次应成功");
        let err = start_run(&table2, &meta, &s, &HashMap::new(), &roots, &dir)
            .expect_err("同 script_id 二次启动必须被拒");
        assert_eq!(err.code(), "SCRIPT_ALREADY_RUNNING");

        // 清理：全部取消，避免留下 sleep 孤儿
        for id in &run_ids {
            let _ = table.cancel(id);
        }
        let _ = table2.cancel(&table2_snapshot_first(&table2));
        let _ = fs::remove_dir_all(&dir);
    }

    /// 取 table2 中第一个 run_id（仅供 B11 清理用）。
    fn table2_snapshot_first(table: &ScriptProcessTable) -> String {
        table
            .lock_runs()
            .keys()
            .next()
            .cloned()
            .expect("至少一个 run")
    }

    // ---------- B12：setsid 生效实测（§3.5 假设） ----------
    #[cfg(target_os = "linux")]
    #[test]
    fn b12_setsid_makes_new_process_group() {
        let dir = temp_dir("b12");
        let script = write_script(&dir, "sleep 300\n");
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];
        let meta = meta_with(ScriptInterpreter::Bash, vec![]);

        let run_id =
            start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir).expect("启动成功");
        let entry = table.lock_runs();
        let e = entry.get(&run_id).expect("entry");
        let pgid = e.pgid.expect("pgid");
        drop(entry);

        // /proc/<pid>/stat 的 pgrp 字段应等于 pid（setsid 后成为新会话组长）
        let observed = process_group_id_of(pgid as u32);
        assert_eq!(
            observed,
            Some(pgid),
            "setsid 后 pgrp 应等于 pid（实测 {:?} vs 期望 {pgid}）",
            observed
        );

        table.cancel(&run_id).expect("取消");
        wait_terminal(&table, &run_id, 8_000);
        let _ = fs::remove_dir_all(&dir);
    }

    // ---------- B13：进程表容量只淘汰终态 ----------
    #[test]
    fn b13_table_capacity_evicts_terminal() {
        let table = ScriptProcessTable::new();
        let mk_entry = |id: &str, finished: bool| {
            let state = Arc::new(Mutex::new(RunSnapshot {
                run_id: id.to_string(),
                script_id: "s".to_string(),
                status: if finished {
                    RunStatus::Succeeded
                } else {
                    RunStatus::Running
                },
                started_at: Utc::now(),
                finished_at: if finished { Some(Utc::now()) } else { None },
                exit_code: None,
                error: None,
            }));
            RunEntry {
                script_id: "s".to_string(),
                pgid: None,
                cancel: Arc::new(AtomicBool::new(false)),
                state,
            }
        };

        {
            let mut runs = table.lock_runs();
            // 1 个 Running（永不淘汰）+ 250 个终态
            runs.insert("running".to_string(), mk_entry("running", false));
            for i in 0..250 {
                let id = format!("t{i:03}");
                runs.insert(id.clone(), mk_entry(&id, true));
            }
        }
        table.evict_if_needed();

        let runs = table.lock_runs();
        assert_eq!(runs.len(), MAX_TABLE_ENTRIES, "应淘汰到容量上限");
        assert!(
            runs.contains_key("running"),
            "Running entry 永不淘汰（B13 核心语义）"
        );
    }

    // ---------- B14：持续输出不阻塞（§2.2 路线 B 回归） ----------
    #[test]
    fn b14_large_output_does_not_block() {
        let dir = temp_dir("b14");
        // 输出 ~2.4 MB：远超 64 KiB 管道容量。若无人 drain，子进程会卡死在 write。
        let script = write_script(
            &dir,
            "i=0\nwhile [ $i -lt 30000 ]; do echo '0123456789012345678901234567890123456789012345678901234567890123456789'; i=$((i+1)); done\nexit 0\n",
        );
        let table = Arc::new(ScriptProcessTable::new());
        let roots = vec![dir.clone()];
        let meta = meta_with(ScriptInterpreter::Bash, vec![]);

        let started = Instant::now();
        let run_id =
            start_run(&table, &meta, &script, &HashMap::new(), &roots, &dir).expect("启动成功");
        let snap = wait_terminal(&table, &run_id, 30_000);

        assert_eq!(
            snap.status,
            RunStatus::Succeeded,
            "2.4MB 输出必须能正常跑完（piped 未 drain 会卡死）"
        );
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "不应被管道阻塞（实际 {:?}）",
            started.elapsed()
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
