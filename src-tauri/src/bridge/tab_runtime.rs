//! Browser tab lifecycle, resource stats and hibernation runtime
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

pub(crate) fn tab_new_native(app: AppHandle, url: String) -> Result<TabInfo, String> {
    let tab = create_tab(app.clone(), &url)?;
    Ok(tab)
}
#[tauri::command]
pub fn tab_new(webview: tauri::Webview, app: AppHandle, url: String) -> Result<TabInfo, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_new")?;
    crate::shared::native_legacy_policy::bound_text("tab_new", "url", &url)?;
    tab_new_native(app, url)
}

/// 关闭指定页签。
pub(crate) fn tab_close_native(app: AppHandle, id: String) -> Result<(), String> {
    close_tab(&app, &id)
}
#[tauri::command]
pub fn tab_close(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_close")?;
    crate::shared::native_legacy_policy::bound_text("tab_close", "id", &id)?;
    tab_close_native(app, id)
}

/// 在指定页签中打开网址（导航）。
#[tauri::command]
pub fn tab_open(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    url: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_open")?;
    crate::shared::native_legacy_policy::bound_text("tab_open", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("tab_open", "url", &url)?;

    let target = normalize_url(&url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    if let Err(e) = navigate_tab_webview(&app, &id, &target) {
        recover_tab_webview(&app, &id, "navigate", &e)?;
        navigate_tab_webview(&app, &id, &target)
            .map_err(|retry| format!("{e}; 恢复后重试导航失败: {retry}"))?;
    }
    // 更新存储的 url
    if let Some(t) = app.state::<AppState>().tabs.lock().unwrap().get_mut(&id) {
        t.url = target.clone();
    }
    Ok(())
}

/// 将指定页签定位到主窗口内容区给定矩形。
/// 前端传 CSS 像素（相对主窗口内容区左上角），换算成屏幕物理坐标后 set_position/set_size。
#[tauri::command]
pub fn tab_position(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_position")?;
    crate::shared::native_legacy_policy::bound_text("tab_position", "id", &id)?;
    crate::shared::native_legacy_policy::bound_dimension("tab_position", "x", x)?;
    crate::shared::native_legacy_policy::bound_dimension("tab_position", "y", y)?;
    crate::shared::native_legacy_policy::bound_dimension("tab_position", "width", width)?;
    crate::shared::native_legacy_policy::bound_dimension("tab_position", "height", height)?;

    if let Err(e) = apply_bounds(&app, &id, x, y, width, height) {
        recover_tab_webview(&app, &id, "position", &e)?;
        apply_bounds_inner(&app, &id, x, y, width, height)
            .map_err(|retry| format!("{e}; 恢复后重试定位失败: {retry}"))?;
    }
    Ok(())
}

// ====== 宫格加载失败自动重试 =====
//
// 内存压力下宫格 webview 可能 TLS 握手失败（错误页），插件只有 navigationFinished
// 无法区分成败。collect.js 检测错误页特征 → invoke report_grid_load_failed
// （子进程内）→ emit 事件 → 子进程 UDS 转发主进程 → 主进程自动重试导航。

/// 宫格 webview 内 collect.js 上报加载失败（子进程内执行）。
#[tauri::command]
pub fn report_grid_load_failed(
    app: AppHandle,
    url: String,
    snippet: Option<String>,
) -> Result<(), String> {
    let index = std::env::var("GRID_CHILD_INDEX").unwrap_or_default();
    eprintln!(
        "[grid] 加载失败上报 grid-{} url={} snippet={:?}",
        index, url, snippet
    );
    let _ = app.emit(
        "grid-load-failed",
        serde_json::json!({ "index": index, "url": url }),
    );
    Ok(())
}

/// 主进程启动宫格加载失败自动重试：收到 grid-load-failed 事件后延迟 3s 重新导航。
/// 每格 60s 内最多重试 2 次（防循环），成功页不会触发上报故不影响正常浏览。
pub fn start_grid_load_retry(app: AppHandle) {
    use tauri::Listener;
    let retries: std::sync::Arc<Mutex<HashMap<u32, (std::time::Instant, u8)>>> =
        std::sync::Arc::new(Mutex::new(HashMap::new()));
    let app2 = app.clone();
    app.listen("grid-load-failed", move |event| {
        let payload: serde_json::Value = serde_json::from_str(event.payload()).unwrap_or_default();
        let index: u32 = match payload["index"].as_str().and_then(|s| s.parse().ok()) {
            Some(i) => i,
            None => return,
        };
        let url = payload["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return;
        }
        // 重试配额：60s 窗口内最多 2 次
        {
            let mut map = retries.lock().unwrap();
            let now = std::time::Instant::now();
            let entry = map.entry(index).or_insert((now, 0));
            if now.duration_since(entry.0).as_secs() > 60 {
                *entry = (now, 0);
            }
            if entry.1 >= 2 {
                eprintln!("[grid] grid-{index} 自动重试次数用尽，放弃");
                return;
            }
            entry.1 += 1;
        }
        eprintln!("[grid] grid-{index} 加载失败，3s 后自动重试导航 {url}");
        let app3 = app2.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let state = app3.state::<AppState>();
            if state.shutdown_requested.load(Ordering::SeqCst) {
                return;
            }
            let mgr = &state.grid_manager;
            if let Err(e) = mgr.request(
                index,
                GridCmd::Navigate {
                    id: format!("grid-{index}"),
                    url: url.clone(),
                },
                10000,
            ) {
                eprintln!("[grid] grid-{index} 自动重试导航失败: {e}");
            } else {
                mgr.record_url(index, &url);
                eprintln!("[grid] grid-{index} 自动重试导航已下发");
            }
        });
    });
}

// ====== 资源监控（宫格设置行"资源"按钮） =====

#[derive(Clone, serde::Serialize)]
pub struct ProcStat {
    pub pid: u32,
    pub name: String,
    /// 进程树合计 RSS（MB，含 WebKit 子进程）
    pub rss_mb: f64,
}

#[derive(Clone, serde::Serialize)]
pub struct ResourceStats {
    pub mem_total_mb: u64,
    pub mem_available_mb: u64,
    /// 应用全部进程合计（主进程 + 全部宫格子进程树）
    pub app_total_mb: f64,
    /// 主进程树（不含宫格子进程树）
    pub main: ProcStat,
    /// 每宫格子进程树（index 即 grid-N 的 N）
    pub grids: Vec<ProcStat>,
    /// 页签休眠开关状态
    pub hibernation_enabled: bool,
    /// 当前已休眠页签数
    pub hibernated_count: usize,
    /// 当前已休眠 Grid 子进程数
    pub hibernated_grid_count: usize,
    /// 内存预算守卫：当前可用内存最多支撑几格（创建宫格时的降级上限）
    pub grid_budget: usize,
}

/// 读取 /proc 全量进程表：pid -> (ppid, comm, rss_kb)。
fn read_proc_table() -> HashMap<u32, (u32, String, u64)> {
    let mut procs = HashMap::new();
    let Ok(rd) = std::fs::read_dir("/proc") else {
        return procs;
    };
    for e in rd.flatten() {
        let name = e.file_name();
        let Some(pid) = name.to_str().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        // comm 在括号内（可含空格），ppid 是右括号后的第 2 个字段
        let Some(lp) = stat.find('(') else { continue };
        let Some(rp) = stat.rfind(')') else { continue };
        let comm = stat[lp + 1..rp].to_string();
        let ppid: u32 = stat[rp + 2..]
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let rss_kb = std::fs::read_to_string(format!("/proc/{pid}/status"))
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("VmRSS:"))
                    .and_then(|r| r.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            })
            .unwrap_or(0);
        procs.insert(pid, (ppid, comm, rss_kb));
    }
    procs
}

/// 由叶到根汇总所有子树的 RSS：一次建立父子关系，每个 PID 至多出队一次。
/// 内存数据仍是 RSS（共享页会重复计入），不得误认为 PSS。
fn process_tree_rss_kb(procs: &HashMap<u32, (u32, String, u64)>) -> HashMap<u32, u64> {
    let mut totals: HashMap<u32, u64> = procs
        .iter()
        .map(|(&pid, (_, _, rss))| (pid, *rss))
        .collect();
    let mut remaining_children: HashMap<u32, usize> = HashMap::new();
    for (&pid, (ppid, _, _)) in procs {
        if pid != *ppid && procs.contains_key(ppid) {
            *remaining_children.entry(*ppid).or_default() += 1;
        }
    }
    let mut ready: std::collections::VecDeque<u32> = procs
        .keys()
        .filter(|pid| !remaining_children.contains_key(pid))
        .copied()
        .collect();
    while let Some(pid) = ready.pop_front() {
        let parent = procs[&pid].0;
        if parent == pid || !procs.contains_key(&parent) {
            continue;
        }
        let rss = totals[&pid];
        *totals.get_mut(&parent).expect("parent is in proc table") += rss;
        let outstanding = remaining_children
            .get_mut(&parent)
            .expect("parent must have a child count");
        *outstanding -= 1;
        if *outstanding == 0 {
            ready.push_back(parent);
        }
    }
    totals
}

#[cfg(test)]
mod resource_tree_tests {
    use super::process_tree_rss_kb;
    use std::collections::HashMap;

    #[test]
    fn nested_grid_and_independent_processes_are_counted_once() {
        let processes = HashMap::from([
            (1, (0, "main".to_string(), 100)),
            (2, (1, "grid".to_string(), 40)),
            (3, (2, "webkit".to_string(), 30)),
            (4, (1, "tab".to_string(), 20)),
            (9, (0, "other".to_string(), 300)),
        ]);
        let totals = process_tree_rss_kb(&processes);
        assert_eq!(totals.get(&1), Some(&190));
        assert_eq!(totals.get(&2), Some(&70));
        assert_eq!(totals.get(&3), Some(&30));
        assert_eq!(totals.get(&9), Some(&300));
    }
}

/// 资源统计缓存仅加速频繁的 UI 采样；必须同时检查进程 PID 以免重建宫格返回旧值。
/// 前端仍按已有 3/15 秒节奏拉取，命令工作保持在阻塞线程池。
static RESOURCE_STATS_CACHE: std::sync::OnceLock<
    Mutex<Option<(std::time::Instant, Vec<(u32, u32)>, ResourceStats)>>,
> = std::sync::OnceLock::new();

/// 资源占用统计：主进程树 + 每宫格子进程树的 RSS（含 WebKit 子进程）。
#[tauri::command]
pub async fn resource_stats(
    webview: tauri::Webview,
    app: AppHandle,
) -> Result<ResourceStats, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "resource_stats")?;

    tauri::async_runtime::spawn_blocking(move || {
        let cache = RESOURCE_STATS_CACHE.get_or_init(|| Mutex::new(None));
        let mut stored = cache.lock().unwrap_or_else(|error| error.into_inner());
        let grid_pids = app.state::<AppState>().grid_manager.pids();
        if let Some((sampled_at, pids, stats)) = stored.as_ref() {
            if *pids == grid_pids && sampled_at.elapsed() < std::time::Duration::from_millis(750) {
                let mut refreshed = stats.clone();
                let state = app.state::<AppState>();
                refreshed.hibernation_enabled = state.hibernation_enabled.load(Ordering::Relaxed);
                refreshed.hibernated_count = state.hibernated_tabs.lock().unwrap().len();
                refreshed.hibernated_grid_count = state.grid_manager.hibernated_count();
                return refreshed;
            }
        }
        let stats = resource_stats_blocking(app);
        *stored = Some((std::time::Instant::now(), grid_pids, stats.clone()));
        stats
    })
    .await
    .map_err(|_| "RESOURCE_STATS_WORKER_FAILED".to_string())
}

pub fn resource_stats_blocking(app: AppHandle) -> ResourceStats {
    let procs = read_proc_table();
    let tree_rss = process_tree_rss_kb(&procs);
    let main_pid = std::process::id();
    let grid_pids = app.state::<AppState>().grid_manager.pids();
    let grid_total_kb: u64 = grid_pids
        .iter()
        .map(|(_, pid)| tree_rss.get(pid).copied().unwrap_or(0))
        .sum();
    let app_total_kb = tree_rss.get(&main_pid).copied().unwrap_or(0); // 宫格子进程是主进程的子进程，已含
    let main_only_kb = app_total_kb.saturating_sub(grid_total_kb);
    let main_name = procs
        .get(&main_pid)
        .map(|(_, c, _)| c.clone())
        .unwrap_or_else(|| "main".to_string());
    let grids = grid_pids
        .iter()
        .map(|(index, pid)| ProcStat {
            pid: *pid,
            name: format!("grid-{index}"),
            rss_mb: tree_rss.get(pid).copied().unwrap_or(0) as f64 / 1024.0,
        })
        .collect();
    let avail = mem_available_mb().unwrap_or(0);
    let hibernation_enabled = app
        .state::<AppState>()
        .hibernation_enabled
        .load(Ordering::Relaxed);
    let hibernated_count = app
        .state::<AppState>()
        .hibernated_tabs
        .lock()
        .unwrap()
        .len();
    let grid_indices: Vec<u32> = grid_pids.iter().map(|(index, _)| *index).collect();
    ResourceStats {
        mem_total_mb: std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find_map(|l| l.strip_prefix("MemTotal:"))
                    .and_then(|r| r.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            })
            .unwrap_or(0)
            / 1024,
        mem_available_mb: avail,
        app_total_mb: app_total_kb as f64 / 1024.0,
        main: ProcStat {
            pid: main_pid,
            name: main_name,
            rss_mb: main_only_kb as f64 / 1024.0,
        },
        grids,
        hibernation_enabled,
        hibernated_count,
        hibernated_grid_count: app.state::<AppState>().grid_manager.hibernated_count(),
        // The displayed limit agrees with create_grid and includes already
        // running Grid children instead of charging their memory twice.
        grid_budget: affordable_grid_count(avail, &grid_indices, MAX_GRID),
    }
}

// ====== 页签休眠（设置面板开关，默认关） =====
//
// 背景：14G 内存机器上每个页签 WebKitWebProcess 占 300+MB，长期不用的页签
// 白吃内存。冻结（FREEZE_JS）只省 CPU 不省内存；休眠 = 销毁 webview 仅留 URL，
// 激活时按 URL 重建（页面状态如滚动位置/表单不保留，登录态由 WebKit 持久会话保留）。

/// Normal pressure keeps the historical 10-minute idle policy; warning and
/// critical pressure progressively shorten it. These thresholds are pure and
/// tested so a low-memory machine does not silently inherit desktop-class limits.
fn hibernation_idle_window(available_mb: u64, total_mb: u64) -> std::time::Duration {
    let warning = (total_mb.saturating_mul(15) / 100)
        .max(1536)
        .min(total_mb.saturating_mul(35) / 100);
    let critical = (total_mb.saturating_mul(8) / 100)
        .max(768)
        .min(total_mb.saturating_mul(20) / 100);
    if available_mb <= critical {
        std::time::Duration::from_secs(30)
    } else if available_mb <= warning {
        std::time::Duration::from_secs(120)
    } else {
        std::time::Duration::from_secs(600)
    }
}

/// A sweep has a bounded destruction budget: avoid killing dozens of WebViews
/// in one pass and immediately rebuilding them when the user returns.
fn hibernation_batch_limits(available_mb: u64, total_mb: u64) -> (usize, usize) {
    match hibernation_idle_window(available_mb, total_mb).as_secs() {
        // Critical: reclaim more, but never with an unbounded kill storm.
        30 => (4, 2),
        // Warning: preserve more page state between sweeps.
        120 => (2, 1),
        // Normal: no burst teardown.
        _ => (1, 1),
    }
}

/// Snapshot parsing must fail closed. A missing 'protected' field or non-finite
/// scroll value is not proof that the page is safe to discard.
fn accepted_hibernation_scroll(snapshot: &serde_json::Value) -> Option<(f64, f64)> {
    if snapshot
        .get("protected")
        .and_then(serde_json::Value::as_bool)
        != Some(false)
    {
        return None;
    }
    let x = snapshot.get("scrollX")?.as_f64()?;
    let y = snapshot.get("scrollY")?.as_f64()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

fn mem_total_mb() -> Option<u64> {
    std::fs::read_to_string("/proc/meminfo")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse::<u64>()
        .ok()
        .map(|kb| kb / 1024)
}

/// Settings remain explicit opt-in: adaptive hibernation never destroys a page
/// until the user enables the existing browser resource policy.
#[tauri::command]
pub fn set_tab_hibernation(
    webview: tauri::Webview,
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "set_tab_hibernation")?;

    app.state::<AppState>()
        .hibernation_enabled
        .store(enabled, Ordering::Relaxed);
    eprintln!("[hibernation] 自适应资源休眠 = {}", enabled);
    Ok(())
}

fn tab_hibernation_snapshot(app: &AppHandle, id: &str) -> Result<(f64, f64), String> {
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    let script = r#"(() => {
      const media = [...document.querySelectorAll('audio,video')].some(el => !el.paused && !el.ended);
      const controls = [...document.querySelectorAll('input,textarea,select')];
      const dirty = controls.some(el => {
        if (el instanceof HTMLInputElement) {
          const type = (el.type || '').toLowerCase();
          if (type === 'password') return el.value.length > 0;
          if (type === 'file') return !!el.files?.length;
          if (type === 'hidden') return false;
          if (type === 'checkbox' || type === 'radio') return el.checked !== el.defaultChecked;
        }
        if (el instanceof HTMLSelectElement) return [...el.options].some(o => o.selected !== o.defaultSelected);
        return 'defaultValue' in el && el.value !== el.defaultValue;
      });
      const focused = document.activeElement && document.activeElement !== document.body &&
        document.activeElement.matches('input,textarea,select,[contenteditable="true"]');
      const richEditor = [...document.querySelectorAll('[contenteditable="true"]')]
        .some(el => (el.textContent || '').trim().length > 0);
      // Embedded documents, canvas applications, framework-driven textboxes,
      // dialogs and even empty rich editors can carry non-serializable state.
      // URL + scroll is insufficient to restore them: skip rather than guess.
      const complexState = !!document.querySelector(
        'iframe,frame,canvas,embed,object,[contenteditable]:not([contenteditable="false"]),[role="textbox"],[aria-modal="true"],dialog[open]'
      );
      const protectedPage = media || dirty || focused || richEditor || complexState ||
        !!window.onbeforeunload || window.__browserosPreventHibernate === true;
      return JSON.stringify({
        protected: protectedPage,
        scrollX: Number(window.scrollX) || 0,
        scrollY: Number(window.scrollY) || 0
      });
    })()"#;
    let (tx, rx) = std::sync::mpsc::channel();
    manager
        .eval_result(&id.to_string(), script.to_string(), move |result| {
            let _ = tx.send(result);
        })
        .map_err(|_| "TAB_HIBERNATION_SNAPSHOT_FAILED")?;
    let encoded = rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "TAB_HIBERNATION_SNAPSHOT_TIMEOUT")??;
    let snapshot: serde_json::Value =
        serde_json::from_str(&encoded).map_err(|_| "TAB_HIBERNATION_SNAPSHOT_INVALID")?;
    accepted_hibernation_scroll(&snapshot)
        .ok_or_else(|| "TAB_HIBERNATION_PROTECTED_OR_INVALID".to_string())
}

/// Destroy an inactive tab only after a native snapshot confirms that it is
/// not playing media, holding edited form/contenteditable data, focused in an
/// editor, or requesting beforeunload. Only scroll coordinates are retained.
fn hibernate_tab(app: &AppHandle, id: &str, idle_window: std::time::Duration) -> bool {
    let state = app.state::<AppState>();
    let expected_url = match state.tabs.lock().unwrap().get(id) {
        Some(tab) => tab.url.clone(),
        None => return false,
    };
    let still_idle = || {
        // The user may disable the policy or navigate/activate the tab while a
        // two-second WebKit snapshot is in flight. Cancel on any such change.
        if !state.hibernation_enabled.load(Ordering::Relaxed)
            || state.active_tab.lock().unwrap().as_deref() == Some(id)
        {
            return false;
        }
        if !state
            .tab_idle_since
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|since| since.elapsed() >= idle_window)
        {
            return false;
        }
        let same_page = state
            .tabs
            .lock()
            .unwrap()
            .get(id)
            .is_some_and(|tab| tab.url == expected_url);
        same_page && !state.hibernated_tabs.lock().unwrap().contains(id)
    };
    if !still_idle() {
        return false;
    }
    let scroll = match tab_hibernation_snapshot(app, id) {
        Ok(scroll) => scroll,
        Err(error) => {
            eprintln!("[hibernation] skip tab={} reason={}", id, error);
            return false;
        }
    };
    // Snapshotting runs on the WebView event loop and may take up to two
    // seconds. Recheck activation/idle state before destroying the native view.
    if !still_idle() {
        return false;
    }
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    if manager.close_tab(&id.to_string()).is_ok() {
        state.child_layouts.lock().unwrap().remove(id);
        state.last_position_at.lock().unwrap().remove(id);
        state.tab_idle_since.lock().unwrap().remove(id);
        state.hibernated_tabs.lock().unwrap().insert(id.to_string());
        state
            .hibernated_scroll
            .lock()
            .unwrap()
            .insert(id.to_string(), scroll);
        state.tab_recovery.lock().unwrap().remove(id);
        eprintln!(
            "[hibernation] tab={} webview destroyed; URL and scroll retained",
            id
        );
        true
    } else {
        false
    }
}

/// Adaptive resource sweeper: normal/warning/critical memory pressure uses
/// 10m/2m/30s idle windows. Grid children are eligible only after an explicit
/// hide (not a main-window blur/minimize), and active/focused/dirty tabs survive.
pub fn start_hibernation_sweeper(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let state = app.state::<AppState>();
        if state.shutdown_requested.load(Ordering::SeqCst) {
            break;
        }
        if !state.hibernation_enabled.load(Ordering::Relaxed) {
            continue;
        }
        let available = mem_available_mb();
        let total = mem_total_mb();
        let (idle_window, tab_limit, grid_limit) = match (available, total) {
            (Some(available), Some(total)) => {
                let (tabs, grids) = hibernation_batch_limits(available, total);
                (hibernation_idle_window(available, total), tabs, grids)
            }
            _ => (std::time::Duration::from_secs(600), 1, 1),
        };
        let grids = state
            .grid_manager
            .hibernate_idle_hidden(idle_window, grid_limit);
        if !grids.is_empty() {
            eprintln!(
                "[hibernation] adaptive memory pressure hibernated grid children {:?}",
                grids
            );
        }

        let active = state.active_tab.lock().unwrap().clone();
        let now = std::time::Instant::now();
        let mut victims: Vec<(std::time::Instant, String)> = {
            let idle = state.tab_idle_since.lock().unwrap();
            let hibernated = state.hibernated_tabs.lock().unwrap();
            let tabs = state.tabs.lock().unwrap();
            idle.iter()
                .filter(|(id, since)| {
                    Some(*id) != active.as_ref()
                        && !hibernated.contains(*id)
                        && tabs.contains_key(*id)
                        && now.duration_since(**since) >= idle_window
                })
                .map(|(id, since)| (*since, id.clone()))
                .collect()
        };
        // Reclaim the oldest inactive tabs first; cap per sweep to avoid a
        // massive WebKit process teardown/restart spike.
        victims.sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        for (_, id) in victims.into_iter().take(tab_limit) {
            if !state.hibernation_enabled.load(Ordering::Relaxed) {
                break;
            }
            // Recheck pressure after a previous WebView was released; if it
            // recovered, do not keep applying the earlier critical threshold.
            let current_window = match (mem_available_mb(), mem_total_mb()) {
                (Some(available), Some(total)) => hibernation_idle_window(available, total),
                _ => std::time::Duration::from_secs(600),
            };
            hibernate_tab(&app, &id, current_window);
        }
    });
}

#[cfg(test)]
mod adaptive_hibernation_tests {
    use super::{accepted_hibernation_scroll, hibernation_batch_limits, hibernation_idle_window};
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn memory_pressure_shortens_idle_window_monotonically() {
        let total = 16 * 1024;
        assert_eq!(
            hibernation_idle_window(8 * 1024, total),
            Duration::from_secs(600)
        );
        assert_eq!(
            hibernation_idle_window(2 * 1024, total),
            Duration::from_secs(120)
        );
        assert_eq!(hibernation_idle_window(700, total), Duration::from_secs(30));
    }

    #[test]
    fn sweep_limits_scale_with_pressure_without_unbounded_kills() {
        let total = 16 * 1024;
        assert_eq!(hibernation_batch_limits(8 * 1024, total), (1, 1));
        assert_eq!(hibernation_batch_limits(2 * 1024, total), (2, 1));
        assert_eq!(hibernation_batch_limits(700, total), (4, 2));
    }

    #[test]
    fn unsafe_or_incomplete_snapshots_must_not_destroy_webviews() {
        assert_eq!(
            accepted_hibernation_scroll(&json!({"protected":false,"scrollX":4.0,"scrollY":8.0})),
            Some((4.0, 8.0))
        );
        for value in [
            json!(null),
            json!({}),
            json!({"scrollX":0,"scrollY":0}),
            json!({"protected":true,"scrollX":0,"scrollY":0}),
            json!({"protected":"false","scrollX":0,"scrollY":0}),
            json!({"protected":false,"scrollX":"0","scrollY":0}),
            json!({"protected":false,"scrollX":0}),
        ] {
            assert!(accepted_hibernation_scroll(&value).is_none(), "{value}");
        }
    }
}

/// 列出当前所有页签。
pub(crate) fn tab_list_native(app: AppHandle) -> Vec<TabInfo> {
    let state = app.state::<AppState>();
    let tabs = state.tabs.lock().unwrap();
    tabs.values().cloned().collect()
}
#[tauri::command]
pub fn tab_list(webview: tauri::Webview, app: AppHandle) -> Result<Vec<TabInfo>, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_list")?;
    Ok(tab_list_native(app))
}

/// 前端设置页签标题（如用户手动改名）。
#[tauri::command]
pub fn tab_set_title(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    title: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_set_title")?;
    crate::shared::native_legacy_policy::bound_text("tab_set_title", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("tab_set_title", "title", &title)?;

    if let Some(t) = app.state::<AppState>().tabs.lock().unwrap().get_mut(&id) {
        t.title = title;
    }
    Ok(())
}

/// 在当前激活页签的 webview 内执行 history.back()（页面内后退）。
/// 历史栈为空时由浏览器内核自动忽略，无副作用。
#[tauri::command]
pub fn tab_go_back(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_go_back")?;
    crate::shared::native_legacy_policy::bound_text("tab_go_back", "id", &id)?;

    plugin_eval(&app, &id, "if (history.length > 1) { history.back(); }")
}

/// 在当前激活页签的 webview 内执行 history.forward()（页面内前进）。
#[tauri::command]
pub fn tab_go_forward(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_go_forward")?;
    crate::shared::native_legacy_policy::bound_text("tab_go_forward", "id", &id)?;

    plugin_eval(&app, &id, "history.forward();")
}

/// 刷新当前激活页签的 webview（location.reload）。
#[tauri::command]
pub fn tab_reload(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_reload")?;
    crate::shared::native_legacy_policy::bound_text("tab_reload", "id", &id)?;

    plugin_eval(&app, &id, "location.reload();")
}

/// 在指定子 webview 中执行 JavaScript（用于 AI 模式向宫格注入问题）。
/// 宫格 grid-N 经 UDS 转发到子进程，页签 tab-N 仍走本地插件。
#[tauri::command]
pub fn eval_in_tab(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    js: String,
) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "eval_in_tab")?;
    crate::shared::native_legacy_policy::bound_text("eval_in_tab", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("eval_in_tab", "js", &js)?;

    if let Some(index) = grid_index_of(&id) {
        app.state::<AppState>()
            .grid_manager
            .request(index, GridCmd::Eval { id, js }, 10000)?;
        return Ok("ok".to_string());
    }
    plugin_eval(&app, &id, &js)?;
    Ok("ok".to_string())
}

/// 设置激活页签：只更新 active_tab，并把其它页签隐藏。
/// 目标页签的精确位置由前端随后调用 tab_position 给出，避免后端与前端争夺坐标。
#[tauri::command]
pub fn tab_activate(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_activate")?;
    crate::shared::native_legacy_policy::bound_text("tab_activate", "id", &id)?;

    // 休眠重建：webview 已销毁的页签先按原 URL 重建（稳定初始视口隐藏，前端 tab_position 定位）
    let was_hibernated = app
        .state::<AppState>()
        .hibernated_tabs
        .lock()
        .unwrap()
        .contains(&id);
    if was_hibernated {
        let url = app
            .state::<AppState>()
            .tabs
            .lock()
            .unwrap()
            .get(&id)
            .map(|t| t.url.clone())
            .ok_or("TAB_NOT_FOUND")?;
        // Keep the hibernated marker until the native WebView is recreated.
        spawn_child_window(
            &app,
            &id,
            &url,
            0.0,
            0.0,
            INITIAL_HIDDEN_WEBVIEW_WIDTH,
            INITIAL_HIDDEN_WEBVIEW_HEIGHT,
        )?;
        app.state::<AppState>()
            .hibernated_tabs
            .lock()
            .unwrap()
            .remove(&id);
        eprintln!("[hibernation] 页签 {} 已从休眠重建 url={}", id, url);
    }
    // Keep scroll metadata until the restored WebView accepts the command. If
    // the first load is still racing, a later activation can retry restoration.
    let scroll = app
        .state::<AppState>()
        .hibernated_scroll
        .lock()
        .unwrap()
        .get(&id)
        .copied();
    if let Some((x, y)) = scroll {
        let restore_app = app.clone();
        let restore_id = id.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(900));
            if !restore_app
                .state::<AppState>()
                .tabs
                .lock()
                .unwrap()
                .contains_key(&restore_id)
            {
                return;
            }
            let js = format!(
                "if (window.scrollX === 0 && window.scrollY === 0) window.scrollTo({:.3},{:.3});",
                x.max(0.0),
                y.max(0.0)
            );
            if plugin_eval(&restore_app, &restore_id, &js).is_ok() {
                restore_app
                    .state::<AppState>()
                    .hibernated_scroll
                    .lock()
                    .unwrap()
                    .remove(&restore_id);
            }
        });
    }
    // 暂存激活页签，前端会随后调用 tab_position 精确布局
    *app.state::<AppState>().active_tab.lock().unwrap() = Some(id.clone());
    // 休眠计时：目标页签移出 idle 表；其它页签从首次失活时刻起算（已有记录不刷新）
    {
        let state = app.state::<AppState>();
        let mut idle = state.tab_idle_since.lock().unwrap();
        idle.remove(&id);
        let now = std::time::Instant::now();
        for k in state.tabs.lock().unwrap().keys() {
            if *k != id {
                idle.entry(k.clone()).or_insert(now);
            }
        }
    }
    // 把其它页签隐藏（子窗口 hide），目标页签若有记住布局则立即重定位显示。
    // 这些 GTK 操作必须整体走 run_on_main_thread，避免 command 同步上下文死锁。
    let app_act = app.clone();
    let active_id = id.clone();
    let _ = app.run_on_main_thread(move || {
        let st = app_act.state::<AppState>();
        let ids: Vec<String> = st.tabs.lock().unwrap().keys().cloned().collect();
        eprintln!(
            "[tab_activate] active={} others={:?}",
            active_id,
            ids.iter().filter(|k| **k != active_id).collect::<Vec<_>>()
        );
        for k in ids {
            if k != active_id {
                hide_bounds(&app_act, &k);
            }
        }
        let layout = st.child_layouts.lock().unwrap().get(&active_id).copied();
        drop(st);
        // 仅当记忆的布局是"显示态"时才立即恢复；隐藏态(-30000)时跳过，
        // 等前端随后的 tab_position 给出精确坐标，避免把激活页签恢复到屏幕外。
        if let Some(r) = layout {
            if r.0 > -1000.0 {
                // 用无去重的 apply_bounds_inner：切页签是低频用户操作，
                // 去重可能把这次"恢复显示"整个丢掉（实测切回前页签仍显示后一个）。
                let _ = apply_bounds_inner(&app_act, &active_id, r.0, r.1, r.2, r.3);
            } else {
                eprintln!(
                    "[tab_activate] active={} 记忆布局为隐藏态，跳过立即恢复",
                    active_id
                );
            }
        }
        // 清除激活页签的去重时间戳：保证前端随后的 tab_position 精确坐标
        // 不会落在 apply_bounds 的 50ms 去重窗口内被丢弃。
        app_act
            .state::<AppState>()
            .last_position_at
            .lock()
            .unwrap()
            .remove(&active_id);
    });
    Ok(())
}

// ====== 真实 PTY 终端（与系统终端一致：tab 补全 / 历史 / 提示符） ======
//
// M3.a：输出管道与生命周期内核收敛到 `terminal.rs`（mpsc+pump+退避+进程组回收），
// 本文件只保留命令壳：sink 选择（F4）、ACL、state 登记。
