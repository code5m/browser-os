//! Browser tab lifecycle, resource stats and hibernation runtime
//!
//! Physical module boundary extracted from the legacy bridge facade.
//! Public commands are re-exported by `bridge.rs`; Tauri registration and
//! frontend invoke names remain unchanged.

use super::*;

// The plugin protects each native call. This guard additionally protects the
// host metadata transaction spanning several such calls. Never hold the set's
// mutex while dispatching to GTK; competing commands fail fast instead.
static TAB_TRANSITIONS: std::sync::LazyLock<Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));

pub(super) struct TabTransition(String);

pub(super) fn reserve_tab_transition(id: &str) -> Result<TabTransition, String> {
    let mut transitions = TAB_TRANSITIONS.lock().unwrap();
    if !transitions.insert(id.to_string()) {
        return Err("TAB_LIFECYCLE_BUSY".into());
    }
    Ok(TabTransition(id.to_string()))
}

impl Drop for TabTransition {
    fn drop(&mut self) {
        TAB_TRANSITIONS.lock().unwrap().remove(&self.0);
    }
}

#[cfg(test)]
mod transition_tests {
    use super::*;

    #[test]
    fn transition_excludes_same_identity_and_releases_on_failure() {
        let first = reserve_tab_transition("transition-fixture-a").unwrap();
        assert!(
            matches!(reserve_tab_transition("transition-fixture-a"), Err(e) if e == "TAB_LIFECYCLE_BUSY")
        );
        let other = reserve_tab_transition("transition-fixture-b").unwrap();
        drop(first);
        assert!(reserve_tab_transition("transition-fixture-a").is_ok());
        drop(other);
    }
}

pub(crate) fn tab_new_native(app: AppHandle, url: String) -> Result<TabInfo, String> {
    let _owner = super::tab_mutation::reserve_internal()?;
    if super::isolated_tabs::mode_enabled() {
        return super::isolated_tabs::create(&app, &url);
    }
    let tab = create_tab(app.clone(), &url)?;
    Ok(tab)
}
#[tauri::command]
pub async fn tab_new(
    webview: tauri::Webview,
    app: AppHandle,
    url: String,
) -> Result<TabInfo, String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::New { url }).await
}

pub(crate) fn tab_new_impl(
    webview: tauri::Webview,
    app: AppHandle,
    url: String,
) -> Result<TabInfo, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_new")?;
    crate::shared::native_legacy_policy::bound_text("tab_new", "url", &url)?;
    if super::isolated_tabs::mode_enabled() {
        return super::isolated_tabs::create(&app, &url);
    }
    create_tab_receipted(app, &url)
}

/// 关闭指定页签。
pub(crate) fn tab_close_native(app: AppHandle, id: String) -> Result<(), String> {
    let _owner = super::tab_mutation::reserve_internal()?;
    close_tab(&app, &id)
}
#[tauri::command]
pub async fn tab_close(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::Close { id }).await
}

pub(crate) fn tab_close_impl(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_close")?;
    crate::shared::native_legacy_policy::bound_text("tab_close", "id", &id)?;
    close_tab(&app, &id)
}

/// 在指定页签中打开网址（导航）。
#[tauri::command]
pub async fn tab_open(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    url: String,
) -> Result<(), String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::Open { id, url }).await
}

pub(crate) fn tab_open_impl(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    url: String,
) -> Result<(), String> {
    tab_open_admitted(webview, app, id, url, None)
}

pub(super) fn tab_open_admitted(
    webview: tauri::Webview, app: AppHandle, id: String, url: String,
    binding: Option<crate::grid_ipc::RendererBinding>,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_open")?;
    crate::shared::native_legacy_policy::bound_text("tab_open", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("tab_open", "url", &url)?;
    if app
        .state::<AppState>()
        .shutdown_requested
        .load(Ordering::Acquire)
    {
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }

    let target = normalize_url(&url);
    let _ = Url::parse(&target).map_err(|e| format!("无效网址: {e}"))?;
    if let Some(index) = super::isolated_tabs::route(&id) {
        if let Some(binding) = binding {
            if binding.index != index || !app.state::<AppState>().tabs.lock().unwrap().contains_key(&id) {
                return Err("TAB_MUTATION_INSTANCE_STALE".into());
            }
            return app.state::<AppState>().grid_manager.navigate_tab_renderer_bound(binding, &target);
        }
        return navigate_tab_webview(&app, &id, &target);
    }
    if binding.is_some() { return Err("TAB_MUTATION_INSTANCE_STALE".into()); }
    app.state::<AppState>().hibernated_scroll.lock().unwrap().remove(&id);
    if let Err(e) = navigate_tab_webview(&app, &id, &target) {
        recover_tab_webview(&app, &id, "navigate", &e)?;
        navigate_tab_webview(&app, &id, &target)
            .map_err(|retry| format!("{e}; 恢复后重试导航失败: {retry}"))?;
    }
    // Publish navigation only before the exit snapshot is atomically accepted.
    let state = app.state::<AppState>();
    let mut tabs = state.tabs.lock().unwrap();
    if state.shutdown_requested.load(Ordering::Acquire) {
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    if let Some(t) = tabs.get_mut(&id) {
        if t.url != target { t.title = target.clone(); }
        t.url = target.clone();
    }
    Ok(())
}

/// Prepared embedded navigation uses the admitted document and never
/// destroys/recreates a page on a stale or failed navigation dispatch.
pub(super) fn tab_open_document(
    app: &AppHandle, id: &str, url: &str, document: &tauri_plugin_browser_tabs::CaptureDocument,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.shutdown_requested.load(Ordering::Acquire) || super::isolated_tabs::route(id).is_some() {
        return Err("TAB_MUTATION_INSTANCE_STALE".into());
    }
    let target = normalize_url(url);
    Url::parse(&target).map_err(|_| "TAB_URL_INVALID")?;
    let previous = state.tabs.lock().unwrap().get(id).map(|tab| tab.url.clone())
        .ok_or("TAB_NOT_FOUND")?;
    app.state::<tauri_plugin_browser_tabs::TabManagerState>()
        .navigate_document(&id.to_owned(), document, &target)
        .map_err(|error| error.to_string())?;
    state.hibernated_scroll.lock().unwrap().remove(id);
    let mut tabs = state.tabs.lock().unwrap();
    if state.shutdown_requested.load(Ordering::Acquire) {
        return Err("GRID_REQUEST_OUTCOME_UNKNOWN: navigation dispatched during shutdown".into());
    }
    let tab = tabs.get_mut(id).ok_or("GRID_REQUEST_OUTCOME_UNKNOWN: navigation owner removed")?;
    // Do not overwrite a more recent native URL observation.
    if tab.url == previous {
        if tab.url != target { tab.title = target.clone(); }
        tab.url = target;
    }
    Ok(())
}

/// 将指定页签定位到主窗口内容区给定矩形。
/// 前端传 CSS 像素（相对主窗口内容区左上角），换算成屏幕物理坐标后 set_position/set_size。
#[tauri::command]
pub async fn tab_position(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // Validate and bind before consuming an isolated-renderer worker permit.
    crate::shared::native_legacy_policy::require_main(&webview, "tab_position")?;
    if super::isolated_tabs::route(&id).is_some() {
        crate::shared::native_legacy_policy::bound_text("tab_position", "id", &id)?;
        crate::shared::native_legacy_policy::bound_dimension("tab_position", "x", x)?;
        crate::shared::native_legacy_policy::bound_dimension("tab_position", "y", y)?;
        crate::shared::native_legacy_policy::bound_dimension("tab_position", "width", width)?;
        crate::shared::native_legacy_policy::bound_dimension("tab_position", "height", height)?;
        let admitted = super::isolated_tabs::capture_layout(&app, &id)?;
        return super::isolated_tabs::on_worker(move || {
            super::isolated_tabs::position_admitted(&app, &id, (x, y, width, height), admitted)
        }).await;
    }
    tab_position_sync(webview, app, id, x, y, width, height)
}

pub(crate) fn tab_position_sync(
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

    if super::isolated_tabs::route(&id).is_some() {
        return super::isolated_tabs::position(&app, &id, (x, y, width, height));
    }
    if let Err(e) = apply_bounds(&app, &id, x, y, width, height) {
        let _owner = super::tab_mutation::reserve_internal()?;
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
        "[grid] load failure grid-{} snippet_present={}",
        index,
        snippet.is_some()
    );
    let _ = app.emit(
        "grid-load-failed",
        serde_json::json!({ "index": index, "url": url }),
    );
    Ok(())
}

// One worker per supported route, including its delay and native request.
 // Only the authenticated native reader can schedule a retry. A frontend
 // event bearing copied process identifiers is not native admission.
struct GridRetrySlot {
    window_started: std::time::Instant,
    attempts: u8,
    last: Option<crate::grid_ipc::RendererBinding>,
    pending: bool,
}
static GRID_RETRY_SLOTS: std::sync::LazyLock<Mutex<[GridRetrySlot; MAX_GRID]>> =
    std::sync::LazyLock::new(|| Mutex::new(std::array::from_fn(|_| GridRetrySlot {
        window_started: std::time::Instant::now(), attempts: 0, last: None, pending: false,
    })));

struct GridRetryLease(usize);
impl Drop for GridRetryLease {
    fn drop(&mut self) {
        GRID_RETRY_SLOTS.lock().unwrap_or_else(|e| e.into_inner())[self.0].pending = false;
    }
}

pub(crate) fn schedule_grid_load_retry(app: &AppHandle, payload: &serde_json::Value) {
    let Some(index) = payload["index"].as_str().and_then(|s| s.parse::<u32>().ok())
        .filter(|i| (*i as usize) < MAX_GRID) else { return; };
    let Some(generation) = payload["renderer_generation"].as_str()
        .and_then(|value| value.parse::<u64>().ok()).filter(|value| *value != 0)
        else { return; };
    let Some(pid) = payload["renderer_pid"].as_u64().and_then(|value| u32::try_from(value).ok())
        .filter(|pid| *pid != 0) else { return; };
    let Some(epoch) = payload["renderer_epoch"].as_str().and_then(|value| value.parse::<u64>().ok())
        .filter(|epoch| *epoch != 0) else { return; };
    let Some(revision) = payload["renderer_navigation_revision"].as_str()
        .and_then(|value| value.parse::<u64>().ok()) else { return; };
    let binding = crate::grid_ipc::RendererBinding {
        index, owner: (pid, epoch), generation, navigation_revision: revision,
    };
    let url = payload["url"].as_str().unwrap_or("");
    if !crate::grid_ipc::restorable_address(url)
        || !Url::parse(url).is_ok_and(|value| matches!(value.scheme(), "http" | "https")) {
        return;
    }
    let state = app.state::<AppState>();
    if state.shutdown_requested.load(Ordering::Acquire) { return; }
    // Repeated error-page JS and native loadFailed reports for one document
    // consume only one admission. The identity remains remembered across
    // the rate-window boundary. A different report while a worker owns the
    // slot is dropped, not queued for a later document or replacement.
    let lease = {
        let Ok(mut slots) = GRID_RETRY_SLOTS.try_lock() else { return; };
        let slot = &mut slots[index as usize];
        if slot.pending || slot.last == Some(binding) { return; }
        let now = std::time::Instant::now();
        if now.duration_since(slot.window_started) >= std::time::Duration::from_secs(60) {
            slot.window_started = now;
            slot.attempts = 0;
        }
        if slot.attempts >= 2 { return; }
        slot.attempts += 1;
        slot.last = Some(binding);
        slot.pending = true;
        GridRetryLease(index as usize)
    };
    let url = url.to_owned();
    let app = app.clone();
    let started = std::thread::Builder::new().name(format!("grid-load-retry-{index}")).spawn(move || {
        let _lease = lease;
        std::thread::sleep(std::time::Duration::from_secs(3));
        let state = app.state::<AppState>();
        if state.shutdown_requested.load(Ordering::Acquire) { return; }
        // Native retry rechecks the exact original process, connection,
        // document revision and URL at execution and writer admission.
        if let Err(error) = state.grid_manager.retry_failed_navigation(binding, &url) {
            eprintln!("[grid] grid-{index} retry unconfirmed: {error}");
        }
    });
    if started.is_err() {
        // Failed spawn drops the captured lease. Do not silently replay.
        eprintln!("[grid] grid-{index} retry worker unavailable");
    }
}

// ====== 资源监控（宫格设置行"资源"按钮） =====

#[derive(Clone, serde::Serialize)]
pub struct ProcStat {
    pub pid: u32,
    pub name: String,
    /// 进程树合计 RSS（MB，含 WebKit 子进程）
    pub rss_mb: f64,
    /// False if the root disappeared or could not be attributed during sampling.
    pub rss_known: bool,
}

#[derive(Clone, serde::Serialize)]
pub struct ResourceStats {
    pub mem_total_mb: u64,
    pub mem_available_mb: u64,
    /// Observed application process tree RSS, including all live renderers.
    pub app_total_mb: f64,
    /// Remaining parent tree, including unattributed/retiring descendants.
    pub main: ProcStat,
    /// 每宫格子进程树（index 即 grid-N 的 N）
    pub grids: Vec<ProcStat>,
    /// Live ordinary-tab renderer subtrees, excluded from main.rss_mb.
    pub isolated_tabs: Vec<ProcStat>,
    pub isolated_tab_start_budget: usize,
    pub isolated_tab_memory_limit_mb: usize,
    pub isolated_tab_mode_enabled: bool,
    pub memory_sample_known: bool,
    /// 页签休眠开关状态
    pub hibernation_enabled: bool,
    /// 当前已休眠页签数
    pub hibernated_count: usize,
    /// 当前已休眠 Grid 子进程数
    pub hibernated_grid_count: usize,
    /// Fresh native Grid topology, independent of the cached RSS sample.
    pub grid_routes: Option<Vec<crate::grid_ipc::GridRouteStatus>>,
    /// 内存预算守卫：当前可用内存最多支撑几格（创建宫格时的降级上限）
    pub grid_budget: usize,
}

/// Linux: /proc process inventory. Windows: Toolhelp + process working
/// set. Both produce (ppid, name, resident_kb) for identical UI grouping.
#[cfg(target_os = "linux")]
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

#[cfg(target_os = "windows")]
fn read_proc_table() -> HashMap<u32, (u32, String, u64)> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };

    let mut procs = HashMap::new();
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return procs;
    }
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) } != 0;
    while has_entry {
        let pid = entry.th32ProcessID;
        let parent = entry.th32ParentProcessID;
        let len = entry
            .szExeFile
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
        let mut rss_kb = 0u64;
        let handle =
            unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid) };
        if !handle.is_null() {
            let mut mem: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
            mem.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            if unsafe {
                GetProcessMemoryInfo(
                    handle,
                    &mut mem,
                    std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                )
            } != 0
            {
                rss_kb = mem.WorkingSetSize as u64 / 1024;
            }
            unsafe { CloseHandle(handle) };
        }
        if pid != 0 {
            procs.insert(pid, (parent, name, rss_kb));
        }
        has_entry = unsafe { Process32NextW(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    procs
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn read_proc_table() -> HashMap<u32, (u32, String, u64)> {
    HashMap::new()
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
    use super::{process_tree_rss_kb, renderer_rss};
    use crate::grid_ipc::{tab_renderer_index, RendererProcess};

    #[test]
    fn isolated_tab_rss_is_not_charged_to_grid_or_main_twice() {
        let procs = HashMap::from([
            (1, (0, "main".into(), 100)),
            (2, (1, "grid".into(), 40)),
            (3, (2, "engine".into(), 30)),
            (4, (1, "tab".into(), 20)),
            (5, (4, "engine".into(), 60)),
            (9, (0, "foreign".into(), 900)),
        ]);
        let owners = [
            RendererProcess { index: 0, pid: 2, spawn_epoch: 1, ready: true, startup_reserved: false },
            RendererProcess { index: tab_renderer_index("tab-1").unwrap(), pid: 4, spawn_epoch: 1, ready: true, startup_reserved: false },
            RendererProcess { index: tab_renderer_index("tab-2").unwrap(), pid: 9, spawn_epoch: 1, ready: false, startup_reserved: false },
        ];
        let tree = process_tree_rss_kb(&procs);
        let (grids, tabs, attributed) = renderer_rss(&owners, &procs, &tree, 1);
        assert_eq!(grids.len(), 1);
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0].name, "tab-1");
        assert!(tabs[0].rss_known);
        assert!(!tabs[1].rss_known);
        assert_eq!(attributed, 150);
        assert_eq!(tree[&1] - attributed, 100);
    }
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
    Mutex<Option<(std::time::Instant, Vec<crate::grid_ipc::RendererProcess>, ResourceStats)>>,
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
        let owners = app.state::<AppState>().grid_manager.renderer_processes();
        if let Some((sampled_at, previous, stats)) = stored.as_ref() {
            if *previous == owners && sampled_at.elapsed() < std::time::Duration::from_millis(750) {
                let mut refreshed = stats.clone();
                let state = app.state::<AppState>();
                refreshed.hibernation_enabled = state.hibernation_enabled.load(Ordering::Relaxed);
                refreshed.hibernated_count = state.hibernated_tabs.lock().unwrap().len();
                refreshed.hibernated_grid_count = state.grid_manager.hibernated_count();
                refreshed.grid_routes = state.grid_manager.grid_route_statuses();
                return refreshed;
            }
        }
        let stats = resource_stats_for_owners(&app, &owners);
        // A spawn/retirement during the OS inventory cannot populate the
        // cache under the identity of a different native owner.
        if app.state::<AppState>().grid_manager.renderer_processes() == owners {
            *stored = Some((std::time::Instant::now(), owners, stats.clone()));
        } else {
            *stored = None;
        }
        stats
    })
    .await
    .map_err(|_| "RESOURCE_STATS_WORKER_FAILED".to_string())
}

pub fn resource_stats_blocking(app: AppHandle) -> ResourceStats {
    let owners = app.state::<AppState>().grid_manager.renderer_processes();
    resource_stats_for_owners(&app, &owners)
}

fn renderer_rss(
    owners: &[crate::grid_ipc::RendererProcess],
    procs: &HashMap<u32, (u32, String, u64)>,
    tree_rss: &HashMap<u32, u64>,
    main_pid: u32,
) -> (Vec<ProcStat>, Vec<ProcStat>, u64) {
    let mut grids = Vec::new();
    let mut tabs = Vec::new();
    let mut attributed = 0u64;
    let mut seen = std::collections::HashSet::new();
    for owner in owners {
        // Each supervisor root is a direct child. A vanished or reused PID
        // must not subtract a foreign process from the application total.
        let observed = procs.get(&owner.pid).is_some_and(|(parent, _, _)| *parent == main_pid)
            && tree_rss.contains_key(&owner.pid) && seen.insert(owner.pid);
        let rss = if observed { tree_rss[&owner.pid] } else { 0 };
        attributed = attributed.saturating_add(rss);
        let stat = ProcStat {
            pid: owner.pid,
            name: crate::grid_ipc::renderer_label(owner.index),
            rss_mb: rss as f64 / 1024.0,
            rss_known: observed,
        };
        if crate::grid_ipc::is_tab_renderer(owner.index) {
            tabs.push(stat);
        } else {
            grids.push(stat);
        }
    }
    (grids, tabs, attributed)
}

fn resource_stats_for_owners(app: &AppHandle, owners: &[crate::grid_ipc::RendererProcess]) -> ResourceStats {
    let procs = read_proc_table();
    let tree_rss = process_tree_rss_kb(&procs);
    let main_pid = std::process::id();
    let (grids, isolated_tabs, attributed_kb) = renderer_rss(owners, &procs, &tree_rss, main_pid);
    let app_total_kb = tree_rss.get(&main_pid).copied().unwrap_or(0);
    let main_only_kb = app_total_kb.saturating_sub(attributed_kb);
    let main_name = procs.get(&main_pid).map(|(_, name, _)| name.clone()).unwrap_or_else(|| "main".into());
    let available = mem_available_mb();
    // Use the existing platform adapter, including GlobalMemoryStatusEx on
    // Windows, rather than a second Linux-only /proc/meminfo implementation.
    let total = super::grid_runtime::mem_total_mb();
    let state = app.state::<AppState>();
    let grid_indices: Vec<u32> = owners.iter().filter(|owner| !crate::grid_ipc::is_tab_renderer(owner.index))
        .map(|owner| owner.index).collect();
    let starting = owners.iter().filter(|owner| owner.startup_reserved).count();
    let isolated_tab_start_budget = renderer_start_budget(available, starting)
        .min(crate::grid_ipc::MAX_ISOLATED_TAB_RENDERERS.saturating_sub(isolated_tabs.len()));
    let hibernated_count = state.hibernated_tabs.lock().unwrap().len();
    ResourceStats {
        mem_total_mb: total.unwrap_or(0),
        mem_available_mb: available.unwrap_or(0),
        app_total_mb: app_total_kb as f64 / 1024.0,
        main: ProcStat {
            pid: main_pid,
            name: main_name,
            rss_mb: main_only_kb as f64 / 1024.0,
            rss_known: tree_rss.contains_key(&main_pid),
        },
        grids,
        isolated_tabs,
        isolated_tab_start_budget,
        isolated_tab_memory_limit_mb: crate::grid_ipc::TAB_RENDERER_MEMORY_LIMIT / (1024 * 1024),
        isolated_tab_mode_enabled: super::isolated_tabs::mode_enabled(),
        memory_sample_known: total.is_some() && available.is_some(),
        hibernation_enabled: state.hibernation_enabled.load(Ordering::Relaxed),
        hibernated_count,
        hibernated_grid_count: state.grid_manager.hibernated_count(),
        grid_routes: state.grid_manager.grid_route_statuses(),
        grid_budget: affordable_grid_count(available.unwrap_or(0), &grid_indices, MAX_GRID),
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
    crate::grid_ipc::valid_viewport(x,y).then_some((x, y))
}

// Policy generations prevent a disable/re-enable cycle from authorizing
// a probe collected under the earlier setting. The final admission guard
// serializes acknowledged policy changes with destructive reclaim.
static HIBERNATION_POLICY_REVISION: std::sync::Mutex<u64> = std::sync::Mutex::new(1);

pub(crate) fn hibernation_policy_revision(app: &AppHandle) -> Option<u64> {
    let revision = HIBERNATION_POLICY_REVISION.try_lock().ok()?;
    let state = app.state::<AppState>();
    (state.hibernation_enabled.load(Ordering::Acquire)
        && !state.shutdown_requested.load(Ordering::Acquire)).then_some(*revision)
}

pub(crate) fn claim_hibernation_policy(app: &AppHandle, expected: u64)
    -> Option<std::sync::MutexGuard<'static, u64>>
{
    let revision = HIBERNATION_POLICY_REVISION.try_lock().ok()?;
    let state = app.state::<AppState>();
    if *revision != expected || !state.hibernation_enabled.load(Ordering::Acquire)
        || state.shutdown_requested.load(Ordering::Acquire) {
        return None;
    }
    Some(revision)
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

    let mut revision = HIBERNATION_POLICY_REVISION.try_lock()
        .map_err(|_| "TAB_HIBERNATION_POLICY_BUSY")?;
    let state = app.state::<AppState>();
    if state.hibernation_enabled.load(Ordering::Acquire) != enabled {
        *revision = revision.checked_add(1).ok_or("TAB_HIBERNATION_POLICY_EXHAUSTED")?;
        state.hibernation_enabled.store(enabled, Ordering::Release);
    }
    eprintln!("[hibernation] 自适应资源休眠 = {}", enabled);
    Ok(())
}

fn tab_hibernation_snapshot(app: &AppHandle, id: &str,
    document: &tauri_plugin_browser_tabs::CaptureDocument, expected_url: &str,
) -> Result<(f64, f64), String> {
    use tauri_plugin_browser_tabs::TabManagerState;
    let manager = app.state::<TabManagerState>();
    if !manager.download_history_clear(&id.to_string()) {
        return Err("TAB_HIBERNATION_DOWNLOAD_HISTORY".into());
    }
    let script = include_str!("../../injected/tab-viewport-checkpoint.js");
    let (tx, rx) = std::sync::mpsc::channel();
    manager
        .eval_result_document(&id.to_string(), document, script.to_string(), move |result| {
            let _ = tx.send(result);
        })
        .map_err(|_| "TAB_HIBERNATION_SNAPSHOT_FAILED")?;
    let encoded = rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "TAB_HIBERNATION_SNAPSHOT_TIMEOUT")??;
    if encoded.len() > 32 * 1024 { return Err("TAB_HIBERNATION_SNAPSHOT_INVALID".into()); }
    let snapshot: serde_json::Value =
        serde_json::from_str(&encoded).map_err(|_| "TAB_HIBERNATION_SNAPSHOT_INVALID")?;
    if snapshot.get("address").and_then(serde_json::Value::as_str) != Some(expected_url)
        || !manager.is_current_capture_document(&id.to_string(), document) {
        return Err("TAB_HIBERNATION_DOCUMENT_STALE".into());
    }
    if !manager.download_history_clear(&id.to_string()) {
        return Err("TAB_HIBERNATION_DOWNLOAD_HISTORY".into());
    }
    accepted_hibernation_scroll(&snapshot)
        .ok_or_else(|| "TAB_HIBERNATION_PROTECTED_OR_INVALID".to_string())
}

/// Destroy an inactive tab only after a native snapshot confirms that it is
/// not playing media, holding edited form/contenteditable data, focused in an
/// editor, or requesting beforeunload. Only scroll coordinates are retained.
fn hibernate_tab(app: &AppHandle, id: &str, idle_window: std::time::Duration) -> bool {
    // The independent backend has its own strictly blank-only policy;
    // never run the embedded scroll-only destroy path against it.
    if super::isolated_tabs::route(id).is_some() {
        return super::isolated_tabs::hibernate_blank(app, id, idle_window);
    }
    let Some(policy_revision) = hibernation_policy_revision(app) else { return false; };
    let state = app.state::<AppState>();
    let expected_url = match state.tabs.lock().unwrap().get(id) {
        Some(tab) => tab.url.clone(),
        None => return false,
    };
    if !crate::grid_ipc::restorable_address(&expected_url) { return false; }
    let still_idle = || {
        // The user may disable the policy or navigate/activate the tab while a
        // two-second WebKit snapshot is in flight. Cancel on any such change.
        if !state.hibernation_enabled.load(Ordering::Acquire)
            || state.shutdown_requested.load(Ordering::Acquire)
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
    let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
    let Ok(document) = manager.capture_document(&id.to_string()) else { return false; };
    let scroll = match tab_hibernation_snapshot(app, id, &document, &expected_url) {
        Ok(scroll) => scroll,
        Err(error) => {
            eprintln!("[hibernation] skip tab={} reason={}", id, error);
            return false;
        }
    };
    // Snapshotting runs on the WebView event loop and may take up to two
    // seconds. Recheck activation/idle state before destroying the native view.
    let Ok(_owner) = super::tab_mutation::reserve_internal() else {
        return false;
    };
    let Ok(_transition) = reserve_tab_transition(id) else {
        return false;
    };
    let Some(_policy) = claim_hibernation_policy(app, policy_revision) else { return false; };
    if !still_idle() || !manager.is_current_capture_document(&id.to_string(), &document) {
        return false;
    }
    if manager.close_tab_document(&id.to_string(), &document).is_ok() {
        state.child_layouts.lock().unwrap().remove(id);
        state.layout_revisions.lock().unwrap().remove(id);
        state.last_position_at.lock().unwrap().remove(id);
        state.tab_idle_since.lock().unwrap().remove(id);
        state.hibernated_tabs.lock().unwrap().insert(id.to_string());
        state
            .hibernated_scroll
            .lock()
            .unwrap()
            .insert(id.to_string(), (expected_url, scroll));
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
/// Navigation notifications are hints, not authority for their URL payload.
/// Read the live plugin instance's cached native observation instead.
pub(crate) fn refresh_embedded_navigation(app: &AppHandle, id: &str) {
    if !is_tab_label(id) || super::isolated_tabs::route(id).is_some() { return; }
    let state = app.state::<AppState>();
    let Ok(mut tabs) = state.tabs.try_lock() else { return; };
    if state.shutdown_requested.load(Ordering::Acquire) { return; }
    let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
    let Ok(Some((_, _, Some(address)))) = manager.observed_navigation(&id.to_owned()) else { return; };
    if address.is_empty() || address.len() > 8192 { return; }
    if let Some(tab) = tabs.get_mut(id) {
        if tab.url != address {
            tab.title = address.clone();
            tab.url = address;
        }
    }
}

#[tauri::command]
pub fn tab_list(webview: tauri::Webview, app: AppHandle) -> Result<Vec<TabInfo>, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_list")?;
    let state = app.state::<AppState>();
    let tabs = state.tabs.try_lock().map_err(|_| "TAB_METADATA_BUSY")?;
    let manager = app.state::<tauri_plugin_browser_tabs::TabManagerState>();
    let mut result = Vec::with_capacity(tabs.len());
    for logical in tabs.values() {
        let mut tab = logical.clone();
        if super::isolated_tabs::route(&tab.id).is_none() {
            if let Some((_, _, Some(address))) = manager.observed_navigation(&tab.id)
                .map_err(|_| "TAB_METADATA_BUSY")? {
                if !address.is_empty() && address.len() <= 8192 && tab.url != address {
                    tab.title = address.clone();
                    tab.url = address;
                }
            }
        }
        result.push(tab);
    }
    Ok(result)
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

    let state = app.state::<AppState>();
    let mut tabs = state.tabs.lock().unwrap();
    if state.shutdown_requested.load(Ordering::Acquire) {
        return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
    }
    if let Some(t) = tabs.get_mut(&id) {
        t.title = title;
    }
    Ok(())
}

/// 在当前激活页签的 webview 内执行 history.back()（页面内后退）。
/// 历史栈为空时由浏览器内核自动忽略，无副作用。
#[tauri::command]
pub async fn tab_go_back(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::Back { id }).await
}

pub(crate) fn tab_go_back_impl(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_go_back")?;
    crate::shared::native_legacy_policy::bound_text("tab_go_back", "id", &id)?;
    app.state::<AppState>().hibernated_scroll.lock().unwrap().remove(&id);

    plugin_eval(&app, &id, "if (history.length > 1) { history.back(); }")
}

/// 在当前激活页签的 webview 内执行 history.forward()（页面内前进）。
#[tauri::command]
pub async fn tab_go_forward(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::Forward { id }).await
}

pub(crate) fn tab_go_forward_impl(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_go_forward")?;
    crate::shared::native_legacy_policy::bound_text("tab_go_forward", "id", &id)?;
    app.state::<AppState>().hibernated_scroll.lock().unwrap().remove(&id);

    plugin_eval(&app, &id, "history.forward();")
}

/// 刷新当前激活页签的 webview（location.reload）。
#[tauri::command]
pub async fn tab_reload(webview: tauri::Webview, app: AppHandle, id: String) -> Result<(), String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::Reload { id }).await
}

pub(crate) fn tab_reload_impl(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_reload")?;
    crate::shared::native_legacy_policy::bound_text("tab_reload", "id", &id)?;
    app.state::<AppState>().hibernated_scroll.lock().unwrap().remove(&id);

    plugin_eval(&app, &id, "location.reload();")
}

/// 在指定子 webview 中执行 JavaScript（用于 AI 模式向宫格注入问题）。
/// 宫格 grid-N 经 UDS 转发到子进程，页签 tab-N 仍走本地插件。
static TAB_EVAL_WORKERS: crate::shared::blocking::BlockingLane =
    crate::shared::blocking::BlockingLane::new(16, "TAB_EVAL_BUSY", "TAB_EVAL_WORKER_FAILED");

#[tauri::command]
pub async fn eval_in_tab(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    js: String,
) -> Result<String, String> {
    // Reject untrusted/oversized calls before occupying a native worker.
    crate::shared::native_legacy_policy::require_main(&webview, "eval_in_tab")?;
    crate::shared::native_legacy_policy::bound_text("eval_in_tab", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("eval_in_tab", "js", &js)?;
    let binding = capture_eval_binding(&app, &id)?;
    TAB_EVAL_WORKERS
        .run(move || eval_in_tab_blocking(app, id, js, binding))
        .await
}

/// Native-only callers already executing on their own worker retain the exact
/// authorization and plugin/IPC route; public IPC never waits on Grid here.
pub(crate) fn eval_in_tab_sync(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    js: String,
) -> Result<String, String> {
    crate::shared::native_legacy_policy::require_main(&webview, "eval_in_tab")?;
    crate::shared::native_legacy_policy::bound_text("eval_in_tab", "id", &id)?;
    crate::shared::native_legacy_policy::bound_text("eval_in_tab", "js", &js)?;

    let binding = capture_eval_binding(&app, &id)?;
    eval_in_tab_blocking(app, id, js, binding)
}

enum EvalBinding {
    Grid(crate::grid_ipc::RendererBinding),
    Isolated(crate::grid_ipc::RendererBinding),
    Embedded(tauri_plugin_browser_tabs::CaptureDocument),
}

fn capture_eval_binding(app: &AppHandle, id: &str) -> Result<EvalBinding, String> {
    let state = app.state::<AppState>();
    if let Some(index) = grid_index_of(id) {
        return state.grid_manager.grid_eval_binding(index).map(EvalBinding::Grid);
    }
    if state.hibernated_tabs.lock().unwrap().contains(id) {
        return Err("TAB_RENDERER_HIBERNATED_ACTIVATE_FIRST".into());
    }
    if let Some(index) = super::isolated_tabs::route(id) {
        return state.grid_manager.renderer_binding(index).map(EvalBinding::Isolated);
    }
    app.state::<tauri_plugin_browser_tabs::TabManagerState>()
        .capture_document(&id.to_owned())
        .map(EvalBinding::Embedded)
        .map_err(|_| "TAB_MUTATION_INSTANCE_UNAVAILABLE".into())
}

fn eval_in_tab_blocking(app: AppHandle, id: String, js: String,
    binding: EvalBinding) -> Result<String, String> {
    match binding {
        EvalBinding::Grid(binding) => {
            if grid_index_of(&id) != Some(binding.index) {
                return Err("GRID_EVAL_TARGET_STALE".into());
            }
            app.state::<AppState>().grid_manager.grid_eval_bound(binding, js)?;
        }
        EvalBinding::Isolated(binding) => {
            let _owner = super::tab_mutation::reserve_internal()?;
            let _transition = reserve_tab_transition(&id)?;
            super::isolated_tabs::eval_bound(&app, &id, &js, binding)?;
        }
        EvalBinding::Embedded(document) => {
            // Verify both native instance and observed document generation
            // after the blocking-lane wait.
            app.state::<tauri_plugin_browser_tabs::TabManagerState>()
                .eval_document(&id, &document, &js)
                .map_err(|error| format!("webview {id} JS dispatch failed: {error}"))?;
        }
    }
    Ok("ok".to_string())
}

/// 设置激活页签：只更新 active_tab，并把其它页签隐藏。
/// 目标页签的精确位置由前端随后调用 tab_position 给出，避免后端与前端争夺坐标。
#[tauri::command]
pub async fn tab_activate(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    super::tab_mutation::legacy_action(webview, app, super::tab_mutation::Action::Activate { id }).await
}

pub(crate) fn tab_activate_impl(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    tab_activate_admitted(webview, app, id, None)
}

pub(super) fn tab_activate_admitted(
    webview: tauri::Webview,
    app: AppHandle,
    id: String,
    admitted: Option<Option<(u32, u64)>>,
) -> Result<(), String> {
    crate::shared::native_legacy_policy::require_main(&webview, "tab_activate")?;
    crate::shared::native_legacy_policy::bound_text("tab_activate", "id", &id)?;
    use crate::shared::dispatch_receipt::{DispatchReceipt, WaitError};
    let receipt = std::sync::Arc::new(DispatchReceipt::new()?);
    let _transition = reserve_tab_transition(&id)?;
    if super::isolated_tabs::route(&id).is_some() {
        return super::isolated_tabs::activate_admitted(&app, &id, admitted);
    }
    if admitted.is_some() { return Err("TAB_RENDERER_ACTIVATION_STALE".into()); }

    {
        let state = app.state::<AppState>();
        if state.shutdown_requested.load(Ordering::Acquire) {
            return Err("TAB_APPLICATION_SHUTTING_DOWN".into());
        }
        if !state.tabs.lock().unwrap().contains_key(&id) {
            return Err("TAB_NOT_FOUND".into());
        }
    }

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
        if !crate::grid_ipc::restorable_address(&url) {
            return Err("TAB_RESTORE_URL_UNSUPPORTED".into());
        }
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
        eprintln!("[hibernation] tab {} restored", id);
    }
    use tauri_plugin_browser_tabs::TabManagerState;
    let active_instance = app
        .state::<TabManagerState>()
        .instance(&id)
        .map_err(|error| error.to_string())?;
    // A restore belongs only to the newly recreated instance. An ordinary
    // later activation must not reschedule an old checkpoint after an
    // ambiguous script dispatch. Preserve the map entry until final admission
    // so explicit navigation can still invalidate it.
    if was_hibernated {
        let scroll = app.state::<AppState>().hibernated_scroll.lock().unwrap().get(&id).cloned();
        if let Some((expected_url, (x,y))) = scroll {
            let instance = active_instance.clone();
            let restore_app = app.clone();
            let restore_id = id.clone();
            let spawned = std::thread::Builder::new().name("tab-viewport-restore".into()).spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(900));
                let manager = restore_app.state::<TabManagerState>();
                let state = restore_app.state::<AppState>();
                let pending = || state.hibernated_scroll.lock().unwrap().get(&restore_id)
                    .is_some_and(|saved| saved.0 == expected_url && saved.1 == (x,y));
                let current = || !state.shutdown_requested.load(Ordering::Acquire)
                    && manager.is_current_instance(&restore_id,&instance)
                    && state.active_tab.lock().unwrap().as_deref() == Some(restore_id.as_str())
                    && state.tabs.lock().unwrap().get(&restore_id).is_some_and(|tab| tab.url == expected_url);
                let report = || {
                    if manager.is_current_instance(&restore_id,&instance) {
                        let _ = restore_app.emit("tab-viewport-restore-unconfirmed", serde_json::json!({"id":restore_id}));
                    }
                };
                let consume = || {
                    let mut saved = state.hibernated_scroll.lock().unwrap();
                    if saved.get(&restore_id).is_some_and(|v| v.0 == expected_url && v.1 == (x,y)) {
                        saved.remove(&restore_id);
                    }
                };
                // Never touch an unrelated replacement's checkpoint.
                if !manager.is_current_instance(&restore_id,&instance) || !pending() { return; }
                if !current() { consume(); report(); return; }
                let Ok(_owner) = super::tab_mutation::reserve_internal() else { consume(); report(); return; };
                let Ok(_transition) = reserve_tab_transition(&restore_id) else { consume(); report(); return; };
                if !current() || !pending() { return; }
                // Consume before the first possible dispatch. No subsequent
                // activation can replay an ambiguous or partially applied write.
                consume();
                let Ok(js) = crate::grid_ipc::viewport_restore_script(&expected_url,x,y) else { report(); return; };
                let Ok(document) = manager.capture_document(&restore_id) else { report(); return; };
                if !current() { report(); return; }
                let (tx,rx) = std::sync::mpsc::channel();
                if manager.eval_result_document(&restore_id,&document,js,move |result| { let _=tx.send(result); }).is_err() {
                    report(); return;
                }
                let confirmed = matches!(rx.recv_timeout(std::time::Duration::from_secs(2)),
                    Ok(Ok(value)) if value=="applied" || value=="skip_nonzero");
                if !confirmed || !current() || !manager.is_current_capture_document(&restore_id,&document) {
                    report();
                }
            });
            if spawned.is_err() {
                app.state::<AppState>().hibernated_scroll.lock().unwrap().remove(&id);
                let _ = app.emit("tab-viewport-restore-unconfirmed", serde_json::json!({"id":id}));
            }
        }
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
    let callback = receipt.clone();
    let dispatch = app.run_on_main_thread(move || {
        // The blocking owner keeps the existing transition reservation
        // until completion or proof this callback can never enter.
        let Some(entered) = callback.enter() else {
            return;
        };
        let result: Result<(), String> = (|| {
            if !app_act
                .state::<TabManagerState>()
                .is_current_instance(&active_id, &active_instance)
            {
                return Err("TAB_ACTIVATION_SUPERSEDED".into());
            }
            let st = app_act.state::<AppState>();
            if st.shutdown_requested.load(Ordering::Acquire)
                || st.active_tab.lock().unwrap().as_deref() != Some(active_id.as_str())
            {
                return Err("TAB_ACTIVATION_SUPERSEDED".into());
            }
            let ids: Vec<String> = st.tabs.lock().unwrap().keys().cloned().collect();
            eprintln!(
                "[tab_activate] active={} others={:?}",
                active_id,
                ids.iter().filter(|k| **k != active_id).collect::<Vec<_>>()
            );
            for k in ids {
                if st.active_tab.lock().unwrap().as_deref() != Some(active_id.as_str()) {
                    return Err("TAB_ACTIVATION_SUPERSEDED".into());
                }
                if k != active_id {
                    hide_bounds(&app_act, &k);
                }
            }
            if st.active_tab.lock().unwrap().as_deref() != Some(active_id.as_str()) {
                return Err("TAB_ACTIVATION_SUPERSEDED".into());
            }
            let layout = st.child_layouts.lock().unwrap().get(&active_id).copied();
            drop(st);
            // 仅当记忆的布局是"显示态"时才立即恢复；隐藏态(-30000)时跳过，
            // 等前端随后的 tab_position 给出精确坐标，避免把激活页签恢复到屏幕外。
            if let Some(r) = layout {
                if r.0 > -1000.0 {
                    // 用无去重的 apply_bounds_inner：切页签是低频用户操作，
                    // 去重可能把这次"恢复显示"整个丢掉（实测切回前页签仍显示后一个）。
                    apply_bounds_inner(&app_act, &active_id, r.0, r.1, r.2, r.3)?;
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

            Ok(())
        })();
        entered.complete(result);
    });
    if dispatch.is_err() {
        receipt.revoke_pending();
    }
    receipt
        .wait(std::time::Duration::from_secs(20))
        .map_err(|error| match error {
            WaitError::Revoked => "TAB_ACTIVATION_LAYOUT_REVOKED",
            WaitError::Lost => "TAB_ACTIVATION_CALLBACK_LOST",
        })??;
    Ok(())
}

// ====== 真实 PTY 终端（与系统终端一致：tab 补全 / 历史 / 提示符） ======
//
// M3.a：输出管道与生命周期内核收敛到 `terminal.rs`（mpsc+pump+退避+进程组回收），
// 本文件只保留命令壳：sink 选择（F4）、ACL、state 登记。
