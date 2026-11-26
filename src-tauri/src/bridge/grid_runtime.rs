//! Grid process orchestration, app discovery and grid commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

pub(crate) const MAX_GRID: usize = 12;

static GRID_ZOOM_WORKERS: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(24, "GRID_ZOOM_BUSY", "GRID_ZOOM_WORKER_FAILED");
// Only one send per cell may be in progress. Different cells remain independent.
static GRID_ZOOM_WRITERS: [Mutex<()>; MAX_GRID] = [const { Mutex::new(()) }; MAX_GRID];

/// 每宫格子进程（Tauri + WebKitWebProcess + AI 站点）的实测内存估算（MB）。
/// 实测 WebKitWebProcess 约 330~365MB/格，加子进程本体取 450。
const GRID_MEM_MB: u64 = 450;
/// Two cells are a supported UI mode. Never silently require four native children.
/// Keep the 1.8 GiB core reserve even when memory is low.
const MIN_GRID: usize = 2;
/// Keep enough memory for Browser, file manager and desktop shell.
const CORE_MEMORY_RESERVE_MB: u64 = 1800;

#[cfg(target_os = "linux")]
fn linux_meminfo_mb(key: &str) -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    text.lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .map(|kb| kb / 1024)
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn mem_available_mb() -> Option<u64> {
    linux_meminfo_mb("MemAvailable:")
}
#[cfg(target_os = "linux")]
pub(crate) fn mem_total_mb() -> Option<u64> {
    linux_meminfo_mb("MemTotal:")
}

#[cfg(target_os = "windows")]
fn windows_meminfo_mb() -> Option<(u64, u64)> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut memory: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    memory.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    if unsafe { GlobalMemoryStatusEx(&mut memory) } == 0 {
        return None;
    }
    Some((
        memory.ullTotalPhys / (1024 * 1024),
        memory.ullAvailPhys / (1024 * 1024),
    ))
}
#[cfg(target_os = "windows")]
pub(crate) fn mem_available_mb() -> Option<u64> {
    windows_meminfo_mb().map(|(_, available)| available)
}
#[cfg(target_os = "windows")]
pub(crate) fn mem_total_mb() -> Option<u64> {
    windows_meminfo_mb().map(|(total, _)| total)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn mem_available_mb() -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(crate) fn mem_total_mb() -> Option<u64> {
    None
}

fn require_grid_platform() -> Result<(), String> {
    if cfg!(any(target_os = "linux", target_os = "windows")) {
        Ok(())
    } else {
        Err(format!(
            "GRID_PLATFORM_UNSUPPORTED:{}",
            std::env::consts::OS
        ))
    }
}

/// Estimated new renderer starts using the existing Grid estimate and
/// desktop reserve. Charge recently admitted starts conservatively because
/// the OS memory sample may not yet contain their eventual working sets.
pub(crate) fn renderer_start_budget(available_mb: Option<u64>, starting: usize) -> usize {
    let Some(available) = available_mb else { return 0; };
    let committed = (starting as u64).saturating_mul(GRID_MEM_MB);
    let remaining = available.saturating_sub(CORE_MEMORY_RESERVE_MB).saturating_sub(committed);
    usize::try_from(remaining / GRID_MEM_MB).unwrap_or(usize::MAX)
}

/// Reuse existing Grid children without charging their already-allocated RSS
/// against free MemAvailable again. Only missing indices consume new budget.
/// Use the SAME reserve for UI capacity and actual Grid creation.
pub(crate) fn affordable_grid_count(
    available_mb: u64,
    existing: &[u32],
    requested: usize,
) -> usize {
    let mut new_cells = available_mb.saturating_sub(CORE_MEMORY_RESERVE_MB) / GRID_MEM_MB;
    let mut capacity = 0;
    for index in 0..requested.min(MAX_GRID) {
        if existing.contains(&(index as u32)) {
            capacity += 1;
        } else if new_cells > 0 {
            new_cells -= 1;
            capacity += 1;
        } else {
            break;
        }
    }
    capacity
}

#[cfg(test)]
mod grid_memory_budget_tests {
    use super::{affordable_grid_count, renderer_start_budget, CORE_MEMORY_RESERVE_MB, GRID_MEM_MB};

    #[test]
    fn renderer_start_budget_accounts_for_unmaterialized_starts_and_unknown_samples() {
        assert_eq!(renderer_start_budget(None, 0), 0);
        assert_eq!(renderer_start_budget(Some(CORE_MEMORY_RESERVE_MB - 1), 0), 0);
        assert_eq!(renderer_start_budget(Some(CORE_MEMORY_RESERVE_MB + GRID_MEM_MB), 0), 1);
        assert_eq!(renderer_start_budget(Some(CORE_MEMORY_RESERVE_MB + GRID_MEM_MB), 1), 0);
        assert_eq!(renderer_start_budget(Some(CORE_MEMORY_RESERVE_MB + 3 * GRID_MEM_MB), 2), 1);
        assert_eq!(renderer_start_budget(Some(1), usize::MAX), 0);
    }

    #[test]
    fn only_new_children_charge_available_memory() {
        assert_eq!(affordable_grid_count(CORE_MEMORY_RESERVE_MB, &[], 4), 0);
        assert_eq!(
            affordable_grid_count(CORE_MEMORY_RESERVE_MB + 900, &[], 4),
            2
        );
        assert_eq!(
            affordable_grid_count(CORE_MEMORY_RESERVE_MB, &[0, 1, 2, 3], 4),
            4
        );
        assert_eq!(affordable_grid_count(500, &[0, 1], 4), 2);
        assert_eq!(
            affordable_grid_count(CORE_MEMORY_RESERVE_MB + 450, &[0, 1], 4),
            3
        );
        assert_eq!(affordable_grid_count(500, &[0, 2], 4), 1);
    }
}

/// 创建宫格（2..=12）：每格 spawn 一个子进程并经 UDS 建 webview，默认打开引导页，
/// 位置由前端 grid_position 告知（子进程窗口初始隐藏，定位后 show）。
/// 返回实际创建的格数：内存预算守卫会在可用内存不足时自动降级（保底 2 格），
/// 前端据此调整 gridCount 并提示用户。
#[tauri::command]
pub async fn create_grid(app: AppHandle, n: usize, urls: Vec<String>) -> Result<usize, String> {
    require_grid_platform()?;
    let targets = capture_all_grid_routes(&app)?;
    tauri::async_runtime::spawn_blocking(move || create_grid_admitted(app, n, urls, targets))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

pub(crate) fn requested_grid_url(
    requested: Option<&str>,
    existing: Option<&str>,
) -> Option<String> {
    requested
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(initial_grid_url)
        .or_else(|| existing.map(str::to_string))
}

pub(crate) fn grid_url_needs_navigation(existing: Option<&str>, target: Option<&str>) -> bool {
    target.is_some_and(|target| existing != Some(target))
}

pub fn create_grid_blocking(app: AppHandle, n: usize, urls: Vec<String>) -> Result<usize, String> {
    require_grid_platform()?;
    let targets = capture_all_grid_routes(&app)?;
    create_grid_admitted(app, n, urls, targets)
}

fn create_grid_admitted(app: AppHandle, n: usize, urls: Vec<String>,
    targets: Vec<crate::grid_ipc::GridRouteStamp>,
) -> Result<usize, String> {
    if targets.len() != MAX_GRID { return Err("GRID_BUILD_TOPOLOGY_INVALID".into()); }
    let requested = n.clamp(2, MAX_GRID);
    let state = app.state::<AppState>();
    let manager = &state.grid_manager;
    let existing: Vec<u32> = targets.iter().filter(|stamp| stamp.pid.is_some()).map(|stamp| stamp.index).collect();
    let target = match mem_available_mb() {
        Some(avail) => {
            let affordable = affordable_grid_count(avail, &existing, requested);
            if affordable < MIN_GRID {
                return Err(format!(
                    "GRID_MEMORY_GUARD: 可用内存 {}MB，至少需要 {}MB 预留空间加新建宫格内存；现有可复用宫格 {:?}",
                    avail, CORE_MEMORY_RESERVE_MB, existing
                ));
            }
            if affordable < requested {
                eprintln!(
                    "[create_grid] 内存预算守卫: 可用 {}MB，请求 {} 格 → 降级 {} 格（已复用 {:?}）",
                    avail, requested, affordable, existing
                );
            }
            affordable
        }
        None if (0..requested).all(|index| existing.contains(&(index as u32))) => requested,
        None => return Err("GRID_MEMORY_UNKNOWN: 无法测量可用内存，拒绝新建宫格".into()),
    };
    let mut prepared_stamps = Vec::with_capacity(target);
    // Keep confirmed cells on partial failure. Navigating them backward does
    // not restore the old DOM, can discard new user input, and is not safe
    // compensation. Only an unconfirmed newly spawned cell is rolled back.
    for i in 0..target {
        let prepared = manager.prepare_grid_slot(targets[i])
            .map_err(|error| format!("GRID_BUILD_PARTIAL:{i}:{error}"))?;
        let previous_url = prepared.previous_url.as_deref();
        let requested_url = urls.get(i).map(String::as_str);
        let target_url = requested_grid_url(requested_url, previous_url)
            .unwrap_or_else(|| "about:blank".to_string());
        let operation = (|| -> Result<(), String> {
            let binding = manager.grid_binding_for_stamp(prepared.stamp)?;
            if prepared.created {
                manager.grid_create_bound(binding, &target_url)?;
            } else if grid_url_needs_navigation(previous_url, requested_grid_url(requested_url, None).as_deref()) {
                manager.grid_navigate_bound(binding, &target_url)?;
            }
            Ok(())
        })();
        if let Err(error) = operation {
            let compensation = if prepared.created {
                manager.rollback_grid_creation(&prepared)
                    .map(|_| "original new cell retired".to_string())
                    .unwrap_or_else(|e| format!("rollback unconfirmed:{e}"))
            } else { "retained cell not replayed".to_string() };
            return Err(format!("GRID_BUILD_PARTIAL:{i}:{error};{compensation}"));
        }
        prepared_stamps.push(prepared.stamp);
        if prepared.created {
            schedule_grid_zoom(&app, &format!("grid-{i}"));
            if i + 1 < target {
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }

    // Validate the prepared cells before closing surplus routes. No
    // re-enumeration of surplus owners after a long-running rebuild.
    let indices: Vec<_> = (0..target as u32).collect();
    if manager.grid_route_stamps(&indices)? != prepared_stamps {
        return Err("GRID_BUILD_PARTIAL:prepared topology changed".into());
    }
    manager.close_grid_targets(&targets[target..])
        .map_err(|error| format!("GRID_BUILD_PARTIAL:surplus close:{error}"))?;
    Ok(target)
}

/// Close the exact Grid topology observed at submission. No ordinary tab
/// belongs to this snapshot and no later replacement is implicitly closed.
#[tauri::command]
pub async fn close_grid(app: AppHandle) -> Result<(), String> {
    let targets = capture_all_grid_routes(&app)?;
    tauri::async_runtime::spawn_blocking(move || close_grid_admitted(app, targets))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

fn capture_all_grid_routes(app: &AppHandle) -> Result<Vec<crate::grid_ipc::GridRouteStamp>, String> {
    let indices: Vec<_> = (0..MAX_GRID as u32).collect();
    app.state::<AppState>().grid_manager.grid_route_stamps(&indices)
}

fn close_grid_admitted(app: AppHandle, targets: Vec<crate::grid_ipc::GridRouteStamp>)
    -> Result<(), String>
{
    app.state::<AppState>().grid_manager.close_grid_targets(&targets)
}

pub fn close_grid_blocking(app: AppHandle) -> Result<(), String> {
    let targets = capture_all_grid_routes(&app)?;
    close_grid_admitted(app, targets)
}

/// 在指定宫格(index)中打开网址（经 UDS Navigate 到子进程）。
/// 导航后延迟应用存储的缩放（给页面加载留时间），确保新页面加载完成后自动缩放到位。
#[tauri::command]
pub async fn grid_open(app: AppHandle, index: usize, url: String) -> Result<(), String> {
    require_grid_platform()?;
    let binding = capture_grid_live_binding(&app, index)?;
    tauri::async_runtime::spawn_blocking(move || grid_open_admitted(app, index, url, binding))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

/// Snapshot live owner before entering a worker. A disconnected live
/// renderer is an error, not permission to resolve a different host.
fn capture_grid_live_binding(app: &AppHandle, index: usize)
    -> Result<Option<crate::grid_ipc::RendererBinding>, String>
{
    if index >= MAX_GRID { return Err("GRID_INDEX_INVALID".into()); }
    let state = app.state::<AppState>();
    state.grid_manager.grid_live_binding(index as u32)
}

pub fn grid_open_blocking(app: AppHandle, index: usize, url: String) -> Result<(), String> {
    require_grid_platform()?;
    let binding = capture_grid_live_binding(&app, index)?;
    grid_open_admitted(app, index, url, binding)
}

fn grid_open_admitted(app: AppHandle, index: usize, url: String,
    binding: Option<crate::grid_ipc::RendererBinding>,
) -> Result<(), String> {
    let label = format!("grid-{index}");
    let target = initial_grid_url(&url);
    if target != "about:blank" {
        let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    }
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    let binding = match binding {
        Some(binding) => binding,
        None => {
            // Explicit open retains the existing cold-start path. Do not
            // navigate a host that became live while the request was queued.
            if mgr.pid_of(index as u32).is_some() {
                return Err("GRID_OPEN_TARGET_CHANGED".into());
            }
            ensure_grid_child(&app, index)?;
            mgr.grid_eval_binding(index as u32)?
        }
    };
    mgr.grid_navigate_bound(binding, &target)?;
    // 页面加载完成后重新应用缩放（CSS zoom 会被新页面重置，需重新注入）
    schedule_grid_zoom(&app, &label);
    Ok(())
}

/// Ensure a Grid child process and its WebView exist. This is also the
/// resume path for memory-hibernated cells; the process manager restores the
/// last URL/rect metadata, and this command recreates the WebView exactly once.
fn ensure_grid_child(app: &AppHandle, index: usize) -> Result<(), String> {
    if index >= MAX_GRID {
        return Err("GRID_INDEX_INVALID".into());
    }
    let state = app.state::<AppState>();
    let manager = &state.grid_manager;
    let was_hibernated = manager.is_hibernated(index as u32);
    if manager.get_or_spawn(index as u32)? {
        let label = format!("grid-{index}");
        let url = manager
            .last_url(index as u32)
            .unwrap_or_else(|| "about:blank".to_string());
        if let Err(error) = manager.request(
            index as u32,
            GridCmd::CreateTab {
                id: label.clone(),
                url: url.clone(),
            },
            20000,
        ) {
            // Do not leave a spawned-but-empty child that makes subsequent
            // get_or_spawn calls believe the hibernated cell is already ready.
            if was_hibernated {
                manager.hibernate_child(index as u32);
            } else {
                manager.kill_child(index as u32);
            }
            return Err(error);
        }
        manager.record_url(index as u32, &url);
        schedule_grid_zoom(app, &label);
    } else {
        // Cold admission may not adopt a concurrent new live owner.
        return Err("GRID_OPEN_TARGET_CHANGED".into());
    }
    Ok(())
}

/// 真正给宫格 webview 应用缩放（WebKitGTK 原生 zoom_level，经 UDS 到子进程）。
/// 仅在页面加载完成后调用，避免加载中设置被重置导致卡顿。
fn schedule_grid_zoom(app: &AppHandle, label: &str) {
    let Some(index) = grid_index_of(label).filter(|index| (*index as usize) < MAX_GRID) else { return; };
    let state = app.state::<AppState>();
    if !state.grid_zooms.lock().unwrap().contains_key(label) { return; }
    let Ok(binding) = state.grid_manager.grid_eval_binding(index) else { return; };
    let Ok(permit) = GRID_ZOOM_WORKERS.reserve() else { return; };
    let app = app.clone();
    let label = label.to_owned();
    // Delay is advisory and bounded; it never resolves a replacement host.
    tauri::async_runtime::spawn(async move {
        let _ = permit.run(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            apply_grid_zoom(&app, &label, binding)
        }).await;
    });
}

fn apply_grid_zoom(app: &AppHandle, label: &str, binding: crate::grid_ipc::RendererBinding) -> Result<(), String> {
    let index = grid_index_of(label).filter(|index| (*index as usize) < MAX_GRID)
        .ok_or("GRID_INDEX_INVALID")?;
    if index != binding.index { return Err("GRID_REQUEST_TARGET_REJECTED".into()); }
    let _send = GRID_ZOOM_WRITERS[index as usize]
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    // A delayed page-load replay and an older blocking worker must both read
    // the current owner intent after acquiring the cell's send lock. Never
    // replay the value captured before a more recent user zoom.
    let state = app.state::<AppState>();
    let zoom = state.grid_zooms.lock().unwrap().get(label).copied();
    let zoom = zoom.ok_or("GRID_ZOOM_INTENT_MISSING")?;
    // 1.0 is a real reset to 100%, not a reason to skip the native command.
    state.grid_manager.grid_zoom_bound(binding, zoom)
}

/// Store the desired zoom before dispatch, then serialize only this cell's I/O.
/// A saturated lane rejects before changing intent; no unbounded task queue.
#[tauri::command]
pub async fn grid_set_zoom(app: AppHandle, index: usize, zoom: f64) -> Result<(), String> {
    require_grid_platform()?;
    if index >= MAX_GRID || !zoom.is_finite() || zoom <= 0.1 {
        return Err("GRID_ZOOM_INVALID".into());
    }
    let permit = GRID_ZOOM_WORKERS.reserve()?;
    let label = format!("grid-{index}");
    let binding = {
        let state = app.state::<AppState>();
        // A sleeping cell retains only a desired preference, not a native
        // success receipt. The next explicit restore applies that preference.
        let binding = match state.grid_manager.grid_eval_binding(index as u32) {
            Ok(binding) => Some(binding),
            Err(_) if state.grid_manager.is_hibernated(index as u32) => None,
            Err(error) => return Err(error),
        };
        let mut zooms = state.grid_zooms.lock().unwrap();
        zooms.insert(label.clone(), zoom);
        binding
    };
    let Some(binding) = binding else { return Ok(()); };
    permit.run(move || apply_grid_zoom(&app, &label, binding)).await
}

/// 批量定位所有宫格（前端按网格计算好每个格子的 x/y/w/h 后调用）。
/// 前端传相对主窗内容区的 CSS 坐标；子进程窗口是独立顶层窗口，这里换算成
/// 绝对屏幕物理坐标（主窗 inner_position + scale_factor）后经 UDS 下发，
/// 子进程 set_position/set_size + show。
/// 注意：本命令只负责定位，不再处理缩放 —— 缩放由 grid_set_zoom 单独管理。
#[tauri::command]
pub async fn grid_position(
    app: AppHandle,
    index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    require_grid_platform()?;
    let binding = capture_grid_live_binding(&app, index)?;
    let sleeping = binding.is_none() && app.state::<AppState>().grid_manager.is_hibernated(index as u32);
    tauri::async_runtime::spawn_blocking(move || {
        grid_position_admitted(app, index, (x, y, width, height), binding, sleeping)
    })
    .await
    .map_err(|_| "GRID_POSITION_WORKER_FAILED".to_string())?
}

pub fn grid_position_blocking(
    app: AppHandle,
    index: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    require_grid_platform()?;
    let binding = capture_grid_live_binding(&app, index)?;
    let sleeping = binding.is_none() && app.state::<AppState>().grid_manager.is_hibernated(index as u32);
    grid_position_admitted(app, index, (x, y, width, height), binding, sleeping)
}

fn grid_position_admitted(app: AppHandle, index: usize, css: (f64, f64, f64, f64),
    binding: Option<crate::grid_ipc::RendererBinding>, sleeping: bool,
) -> Result<(), String> {
    let (x, y, width, height) = css;
    if index >= MAX_GRID || ![x, y, width, height].iter().all(|value| value.is_finite())
        || width < 0.0 || height < 0.0
    {
        return Err("GRID_NATIVE_RECT_INVALID".into());
    }
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    let binding = match binding {
        Some(binding) => binding,
        None if x <= -20000.0 => return Ok(()), // no live window to hide
        None if sleeping && mgr.is_hibernated(index as u32) && mgr.pid_of(index as u32).is_none() => {
            // Only a previously observed sleeping cell may resume through
            // layout. An absent/closed cell requires explicit Grid creation.
            ensure_grid_child(&app, index)?;
            mgr.grid_eval_binding(index as u32)?
        }
        None => return Err("GRID_POSITION_TARGET_UNAVAILABLE".into()),
    };
    let rect = mgr.abs_rect(css)
        .ok_or_else(|| "主窗不可用，无法换算屏幕坐标".to_string())?;
    mgr.grid_position_bound(binding, css, rect)
}

/// Close one original process domain. Disconnected hosts may still be
/// retired, but a replacement identity must never be selected by index.
#[tauri::command]
pub async fn grid_close_one(app: AppHandle, index: usize) -> Result<(), String> {
    let targets = capture_one_grid_route(&app, index)?;
    tauri::async_runtime::spawn_blocking(move || close_grid_admitted(app, targets))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

fn capture_one_grid_route(app: &AppHandle, index: usize)
    -> Result<Vec<crate::grid_ipc::GridRouteStamp>, String>
{
    require_grid_platform()?;
    if index >= MAX_GRID { return Err("GRID_INDEX_INVALID".into()); }
    app.state::<AppState>().grid_manager.grid_route_stamps(&[index as u32])
}

pub fn grid_close_one_blocking(app: AppHandle, index: usize) -> Result<(), String> {
    let targets = capture_one_grid_route(&app, index)?;
    close_grid_admitted(app, targets)
}

// ====== 系统应用启动器（Linux .desktop） ======

#[derive(serde::Serialize)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub icon_path: Option<String>,
}

#[cfg(target_os = "linux")]
fn parse_desktop_file(path: &std::path::Path) -> Option<AppEntry> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut no_display = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_entry = t == "[Desktop Entry]";
            continue;
        }
        if !in_entry || !t.contains('=') {
            continue;
        }
        let (k, v) = t.split_once('=')?;
        match k.trim() {
            "Name" => name = v.to_string(),
            "Exec" => exec = v.to_string(),
            "Icon" => icon = v.to_string(),
            "NoDisplay" => no_display = v.trim() == "true",
            _ => {}
        }
    }
    if name.is_empty() || exec.is_empty() || no_display {
        return None;
    }
    // 去掉 Exec 中的字段码 (%U/%F 等)
    let exec_clean = exec
        .split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");
    let icon_path = resolve_icon(&icon);
    Some(AppEntry {
        name,
        exec: exec_clean,
        icon,
        icon_path,
    })
}

/// 解析 .desktop 的 Icon 字段为真实图标文件路径。
/// Icon 可能是：绝对路径 / 主题图标名（如 firefox）/ 空。
#[cfg(target_os = "linux")]
fn resolve_icon(icon: &str) -> Option<String> {
    if icon.is_empty() {
        return None;
    }
    // 1) 已是绝对路径：文件必须真实存在才返回，否则前端会尝试加载不存在的 file:// 而破图
    if icon.starts_with('/') {
        if std::path::Path::new(icon).exists() {
            return Some(icon.to_string());
        }
        return None;
    }

    // 2) 按 Freedesktop 图标主题规范解析（自动处理 hicolor、当前 GTK 主题、尺寸回退）
    let theme = freedesktop_icons::default_theme_gtk().unwrap_or_else(|| "hicolor".to_string());
    if let Some(path) = freedesktop_icons::lookup(icon)
        .with_theme(&theme)
        .with_size(48)
        .find()
    {
        return Some(path.to_string_lossy().to_string());
    }

    // 3) 兜底：常见主题目录与 pixmaps 递归查找
    let home = std::env::var("HOME").unwrap_or_default();
    let exts = ["png", "svg", "xpm"];
    let roots = [
        "/usr/share/pixmaps",
        "/usr/local/share/icons",
        &format!("{}/.local/share/icons", home),
        &format!("{}/.icons", home),
    ];
    for root in roots {
        for ext in exts {
            let p = format!("{}/{}.{}", root, icon, ext);
            if std::path::Path::new(&p).exists() {
                return Some(p);
            }
        }
        if let Some(found) = find_icon_recursive(std::path::Path::new(root), icon, &exts, 0) {
            return Some(found);
        }
    }
    None
}

fn find_icon_recursive(
    dir: &std::path::Path,
    icon: &str,
    exts: &[&str],
    depth: u32,
) -> Option<String> {
    if depth > 4 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            if let Some(f) = find_icon_recursive(&path, icon, exts, depth + 1) {
                return Some(f);
            }
        } else if path
            .extension()
            .map(|x| x == "png" || x == "svg" || x == "xpm")
            .unwrap_or(false)
        {
            if path.file_stem().map(|s| s == icon).unwrap_or(false) {
                return Some(path.to_string_lossy().to_string());
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn resolve_icon(_icon: &str) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub fn list_apps() -> Vec<AppEntry> {
    let mut apps = vec![];
    let mut seen = std::collections::HashSet::new();
    let mut scan = |dir: &std::path::Path| {
        if !dir.exists() {
            return;
        }
        for e in std::fs::read_dir(dir).ok().into_iter().flatten() {
            let e = e.ok();
            let path = e.as_ref().map(|e| e.path());
            let Some(path) = path else { continue };
            if !path.to_string_lossy().ends_with(".desktop") {
                continue;
            }
            if let Some(app) = parse_desktop_file(&path) {
                if seen.insert(app.name.clone()) {
                    apps.push(app);
                }
            }
        }
    };
    scan(std::path::Path::new("/usr/share/applications"));
    scan(std::path::Path::new("/usr/local/share/applications"));
    if let Ok(home) = std::env::var("HOME") {
        scan(
            std::path::Path::new(&home)
                .join(".local/share/applications")
                .as_path(),
        );
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// List user-visible Windows Start Menu applications, rather than scanning
/// Linux .desktop paths on Windows. Shell links are inspected, never executed,
/// and every discovered .exe is revalidated by the same native launch policy
/// used when the user explicitly clicks an application in the UI.
#[cfg(target_os = "windows")]
fn list_windows_apps_blocking() -> Vec<AppEntry> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    fn add_target(
        apps: &mut Vec<AppEntry>,
        seen: &mut std::collections::HashSet<String>,
        name: &str,
        executable: &str,
    ) {
        if apps.len() >= 256 || name.is_empty() || name.len() > 128 || name.contains(['\r', '\n']) {
            return;
        }
        let path = std::path::Path::new(executable);
        if !path.is_absolute()
            || executable.starts_with(r"\\")
            || !path
                .extension()
                .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("exe"))
        {
            return;
        }
        let proposed = format!("\"{}\"", executable);
        let Ok((real, arguments)) = crate::security_policy::check_launch_target(&proposed) else {
            return;
        };
        if !arguments.is_empty() {
            return;
        }
        if seen.insert(real.to_lowercase()) {
            apps.push(AppEntry {
                name: name.to_owned(),
                exec: format!("\"{}\"", real),
                icon: "🪟".into(),
                icon_path: None, // EXE bytes are not image files.
            });
        }
    }

    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut system = [0u16; 512];
    let len = unsafe { GetSystemDirectoryW(system.as_mut_ptr(), system.len() as u32) };
    if len == 0 || len as usize >= system.len() {
        return apps;
    }
    let system_dir =
        std::path::PathBuf::from(std::ffi::OsString::from_wide(&system[..len as usize]));
    // A stuck COM shortcut reader must not block the workbench forever:
    // share the 8s/192KiB bounded PowerShell runner with AUMID discovery.
    const SCRIPT: &str = r#"
$ErrorActionPreference='SilentlyContinue'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$shell=New-Object -ComObject WScript.Shell
$apps=New-Object 'System.Collections.Generic.List[object]'
foreach($kind in @('StartMenu','CommonStartMenu')){
  $root=[Environment]::GetFolderPath($kind)
  if([string]::IsNullOrWhiteSpace($root)){continue}
  foreach($link in (Get-ChildItem -LiteralPath (Join-Path $root 'Programs') -Filter '*.lnk' -File -Recurse -ErrorAction SilentlyContinue)){
    if($apps.Count -ge 256){break}
    try{
      $target=$shell.CreateShortcut($link.FullName).TargetPath
      if($target -and $target.EndsWith('.exe',[StringComparison]::OrdinalIgnoreCase)){
        $apps.Add([pscustomobject]@{name=$link.BaseName;exec=$target})
      }
    }catch{}
  }
}
ConvertTo-Json -InputObject @($apps) -Depth 3 -Compress
"#;
    if let Some(bytes) = crate::windows_apps::run_bounded_builtin_script(SCRIPT) {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
        if let Ok(items) = serde_json::from_slice::<serde_json::Value>(bytes) {
            if let Some(values) = items.as_array() {
                for item in values.iter().take(256) {
                    if let (Some(name), Some(exe)) = (item["name"].as_str(), item["exec"].as_str())
                    {
                        add_target(&mut apps, &mut seen, name, exe);
                    }
                }
            }
        }
    }
    // MSIX/UWP applications are addressed by an AppUserModelID instead of a
    // launchable EXE. The "appsfolder:" marker is never parsed as a shell
    // command: the native launcher revalidates the installed AUMID before
    // opening its trusted shell namespace target.
    for (name, id) in crate::windows_apps::installed_packaged_apps() {
        if apps.len() >= 512 {
            break;
        }
        let key = format!("appsfolder:{}", id.to_ascii_lowercase());
        if seen.insert(key) {
            apps.push(AppEntry {
                name,
                exec: format!("appsfolder:{id}"),
                icon: "🪟".into(),
                icon_path: None,
            });
        }
    }
    // System applications are legitimate Windows launch targets even on
    // machines where Start Menu link enumeration is disabled by policy.
    for (name, exe) in [("Notepad", "notepad.exe"), ("Paint", "mspaint.exe")] {
        let candidate = system_dir.join(exe);
        if candidate.is_file() {
            add_target(&mut apps, &mut seen, name, &candidate.to_string_lossy());
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn list_apps() -> Vec<AppEntry> {
    tauri::async_runtime::spawn_blocking(list_windows_apps_blocking)
        .await
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[tauri::command]
pub fn list_apps() -> Result<Vec<AppEntry>, String> {
    Err(format!(
        "APP_DISCOVERY_PLATFORM_UNSUPPORTED:{}",
        std::env::consts::OS
    ))
}

// ====== 浏览器页签命令（供前端页签栏调用） ======
