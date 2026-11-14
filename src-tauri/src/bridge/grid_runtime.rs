//! Grid process orchestration, app discovery and grid commands
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

pub(crate) const MAX_GRID: usize = 12;

/// 每宫格子进程（Tauri + WebKitWebProcess + AI 站点）的实测内存估算（MB）。
/// 实测 WebKitWebProcess 约 330~365MB/格，加子进程本体取 450。
const GRID_MEM_MB: u64 = 450;
/// Two cells are a supported UI mode. Never silently require four native children.
/// Keep the 1.8 GiB core reserve even when memory is low.
const MIN_GRID: usize = 2;
/// Keep enough memory for Browser, file manager and desktop shell.
const CORE_MEMORY_RESERVE_MB: u64 = 1800;

pub(crate) fn mem_available_mb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb: u64 = rest.trim().trim_end_matches("kB").trim().parse().ok()?;
            return Some(kb / 1024);
        }
    }
    None
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
    use super::{affordable_grid_count, CORE_MEMORY_RESERVE_MB};

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
    tauri::async_runtime::spawn_blocking(move || create_grid_blocking(app, n, urls))
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
    let requested = n.clamp(2, MAX_GRID);
    let state = app.state::<AppState>();
    let manager = &state.grid_manager;
    let existing = manager.indices();
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
    let mut newly_created: Vec<(u32, bool)> = Vec::new();
    let mut navigated_existing: Vec<(u32, String)> = Vec::new();

    // Bring the target layout to a ready state before destroying surplus cells.
    // This preserves the old Grid if a cold child fails to start or a navigation
    // request times out during a resize.
    for i in 0..target {
        let index = i as u32;
        let label = format!("grid-{i}");
        let was_hibernated = manager.is_hibernated(index);
        let requested_url = urls.get(i).map(String::as_str);
        let created = match manager.get_or_spawn(index) {
            Ok(created) => created,
            Err(error) => {
                for (rollback_index, old_url) in navigated_existing.iter().rev() {
                    let _ = manager.request(
                        *rollback_index,
                        GridCmd::Navigate {
                            id: format!("grid-{rollback_index}"),
                            url: old_url.clone(),
                        },
                        10000,
                    );
                    manager.record_url(*rollback_index, old_url);
                }
                for (created_index, was_hibernated) in newly_created.iter().rev() {
                    if *was_hibernated {
                        manager.hibernate_child(*created_index);
                    } else {
                        manager.kill_child(*created_index);
                    }
                }
                return Err(error);
            }
        };
        let previous_url = manager.last_url(index);
        let target_url = requested_grid_url(requested_url, previous_url.as_deref())
            .unwrap_or_else(|| "about:blank".to_string());
        if created {
            newly_created.push((index, was_hibernated));
        }

        let command = if created {
            Some(GridCmd::CreateTab {
                id: label.clone(),
                url: target_url.clone(),
            })
        } else if grid_url_needs_navigation(
            previous_url.as_deref(),
            requested_grid_url(requested_url, None).as_deref(),
        ) {
            Some(GridCmd::Navigate {
                id: label.clone(),
                url: target_url.clone(),
            })
        } else {
            None
        };
        if let Some(command) = command {
            let timeout = if created { 20000 } else { 10000 };
            if let Err(error) = manager.request(index, command, timeout) {
                for (rollback_index, old_url) in navigated_existing.iter().rev() {
                    let _ = manager.request(
                        *rollback_index,
                        GridCmd::Navigate {
                            id: format!("grid-{rollback_index}"),
                            url: old_url.clone(),
                        },
                        10000,
                    );
                    manager.record_url(*rollback_index, old_url);
                }
                for (created_index, was_hibernated) in newly_created.iter().rev() {
                    if *was_hibernated {
                        manager.hibernate_child(*created_index);
                    } else {
                        manager.kill_child(*created_index);
                    }
                }
                return Err(error);
            }
            if !created {
                if let Some(old_url) = previous_url {
                    navigated_existing.push((index, old_url));
                }
            }
            manager.record_url(index, &target_url);
        }

        if created {
            if let Some(zoom) = state.grid_zooms.lock().unwrap().get(&label).copied() {
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(900));
                    apply_grid_zoom(&app, &label, zoom);
                });
            }
            if i + 1 < target {
                // Stagger only cold starts; healthy retained children need no delay.
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        }
    }

    // The new target layout is healthy; only now close cells outside its range.
    for index in manager
        .indices()
        .into_iter()
        .filter(|index| *index as usize >= target)
    {
        grid_close_one_blocking(app.clone(), index as usize)?;
    }
    Ok(target)
}

/// 关闭所有宫格（销毁对应子进程）。
/// 顺序：先 HideWindow（窗口立即从屏幕消失，消除"幽灵窗口残留几秒"的视觉问题），
/// 再 CloseTab + kill（进程销毁与 X 窗口清理有延迟，藏在 hide 之后用户无感）。
#[tauri::command]
pub async fn close_grid(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || close_grid_blocking(app))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

pub fn close_grid_blocking(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    for index in mgr.indices() {
        mgr.send(
            index,
            GridCmd::HideWindow {
                id: format!("grid-{index}"),
            },
        );
    }
    // 给 hide 一个生效窗口期（UDS 是异步 send，100ms 足够子进程执行 hide）
    std::thread::sleep(std::time::Duration::from_millis(100));
    for index in mgr.indices() {
        mgr.send(
            index,
            GridCmd::CloseTab {
                id: format!("grid-{index}"),
            },
        );
    }
    mgr.shutdown_all();
    for i in 0..MAX_GRID {
        let label = format!("grid-{i}");
        state.child_layouts.lock().unwrap().remove(&label);
        state.grid_zooms.lock().unwrap().remove(&label);
        state.last_position_at.lock().unwrap().remove(&label);
    }
    Ok(())
}

/// 在指定宫格(index)中打开网址（经 UDS Navigate 到子进程）。
/// 导航后延迟应用存储的缩放（给页面加载留时间），确保新页面加载完成后自动缩放到位。
#[tauri::command]
pub async fn grid_open(app: AppHandle, index: usize, url: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || grid_open_blocking(app, index, url))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

pub fn grid_open_blocking(app: AppHandle, index: usize, url: String) -> Result<(), String> {
    let label = format!("grid-{index}");
    ensure_grid_child(&app, index)?;
    let target = initial_grid_url(&url);
    if target != "about:blank" {
        let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    }
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    mgr.request(
        index as u32,
        GridCmd::Navigate {
            id: label.clone(),
            url: target.clone(),
        },
        10000,
    )?;
    mgr.record_url(index as u32, &target);
    // 页面加载完成后重新应用缩放（CSS zoom 会被新页面重置，需重新注入）
    let stored = state.grid_zooms.lock().unwrap().get(&label).copied();
    if let Some(z) = stored {
        let app3 = app.clone();
        let label3 = label.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            apply_grid_zoom(&app3, &label3, z);
        });
    }
    Ok(())
}

/// Ensure a Grid child process and its WebView exist. This is also the
/// resume path for memory-hibernated cells; the process manager restores the
/// last URL/rect metadata, and this command recreates the WebView exactly once.
fn ensure_grid_child(app: &AppHandle, index: usize) -> Result<(), String> {
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
        if let Some(zoom) = state.grid_zooms.lock().unwrap().get(&label).copied() {
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(900));
                apply_grid_zoom(&app, &label, zoom);
            });
        }
    }
    Ok(())
}

/// 真正给宫格 webview 应用缩放（WebKitGTK 原生 zoom_level，经 UDS 到子进程）。
/// 仅在页面加载完成后调用，避免加载中设置被重置导致卡顿。
fn apply_grid_zoom(app: &AppHandle, label: &str, z: f64) {
    if z <= 0.1 || (z - 1.0).abs() < 0.01 {
        return;
    }
    if let Some(index) = grid_index_of(label) {
        app.state::<AppState>().grid_manager.send(
            index,
            GridCmd::SetZoom {
                id: label.to_string(),
                zoom: z,
            },
        );
    }
}

/// 设置宫格某格的缩放（存起来 + 若页面已加载则立即应用）。
/// 前端在每次定位/布局变化后调用；应用时机做了防抖，避免反复 zoom 卡顿。
#[tauri::command]
pub fn grid_set_zoom(app: AppHandle, index: usize, zoom: f64) -> Result<(), String> {
    let label = format!("grid-{index}");
    // 与上次相同则跳过（去重，避免重复 zoom 触发重排卡顿）
    {
        let state = app.state::<AppState>();
        let mut zooms = state.grid_zooms.lock().unwrap();
        if let Some(&prev) = zooms.get(&label) {
            if (prev - zoom).abs() < 0.02 {
                return Ok(());
            }
        }
        zooms.insert(label.clone(), zoom);
    }
    apply_grid_zoom(&app, &label, zoom);
    Ok(())
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
    tauri::async_runtime::spawn_blocking(move || {
        grid_position_blocking(app, index, x, y, width, height)
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
    // Repeated suspend/blur notifications must not wake a cell that is already
    // hibernated. A positive/visible rectangle below is the explicit resume path.
    if x <= -20000.0
        && app
            .state::<AppState>()
            .grid_manager
            .is_hibernated(index as u32)
    {
        return Ok(());
    }
    ensure_grid_child(&app, index)?;
    let label = format!("grid-{index}");
    // 后端去重：仅"rect 相同且 <50ms"才丢弃（防 IPC 洪泛），rect 变了必须应用
    {
        let state = app.state::<AppState>();
        let mut last = state.last_position_at.lock().unwrap();
        let now = std::time::Instant::now();
        if let Some((t, lx, ly, lw, lh)) = last.get(&label) {
            let same_rect = (lx - x).abs() < 0.5
                && (ly - y).abs() < 0.5
                && (lw - width).abs() < 0.5
                && (lh - height).abs() < 0.5;
            if same_rect && now.duration_since(*t).as_millis() < 50 {
                return Ok(());
            }
        }
        last.insert(label.clone(), (now, x, y, width, height));
    }
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    let rect = mgr
        .abs_rect((x, y, width, height))
        .ok_or_else(|| "主窗不可用，无法换算屏幕坐标".to_string())?;
    mgr.record_rect(index as u32, (x, y, width, height));
    // Grid suspend uses the established off-screen hide contract (x=-30000,
    // unchanged cell dimensions); record it as explicitly hidden so the adaptive
    // hibernator can reclaim the child process without calling Browser-owned APIs.
    if x <= -20000.0 {
        mgr.record_hidden(index as u32);
    }
    mgr.request(index as u32, GridCmd::UpdateRect { id: label, rect }, 3000)
}

/// 关闭单个宫格（按 index）：销毁对应子进程，其余宫格保留。
/// 同样先 HideWindow 再 kill（消除幽灵窗口）。
#[tauri::command]
pub async fn grid_close_one(app: AppHandle, index: usize) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || grid_close_one_blocking(app, index))
        .await
        .map_err(|_| "GRID_WORKER_FAILED".to_string())?
}

pub fn grid_close_one_blocking(app: AppHandle, index: usize) -> Result<(), String> {
    let label = format!("grid-{index}");
    let state = app.state::<AppState>();
    let mgr = &state.grid_manager;
    mgr.send(index as u32, GridCmd::HideWindow { id: label.clone() });
    std::thread::sleep(std::time::Duration::from_millis(100));
    mgr.send(index as u32, GridCmd::CloseTab { id: label.clone() });
    mgr.kill_child(index as u32);
    state.child_layouts.lock().unwrap().remove(&label);
    state.grid_zooms.lock().unwrap().remove(&label);
    state.last_position_at.lock().unwrap().remove(&label);
    Ok(())
}

// ====== 系统应用启动器（Linux .desktop） ======

#[derive(serde::Serialize)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub icon: String,
    pub icon_path: Option<String>,
}

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
fn resolve_icon(_icon: &str) -> Option<String> { None }

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

// ====== 浏览器页签命令（供前端页签栏调用） ======
