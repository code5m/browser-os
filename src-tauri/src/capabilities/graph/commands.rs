//! Knowledge Graph 只读 live-query 命令（M5-W12，Lane A7）。
//!
//! 命令体仅做来源校验 + 调 `crate::graph` 纯内核；不反向依赖 `crate::bridge`
//! （graph.rs 仍是纯逻辑，守 GRAPH_NO_SECOND_PATH），无副作用 / 无 IO / 无写。
//! 出参经 View DTO 删除 `props`；错误仅稳定 `GRAPH_*` 码，绝不 echo 凭据/路径/body。
//!
//! 从 `bridge.rs` 迁入本 capability-owned 命令模块（Native Physical Boundary 分解）。
//! 来源校验复用共享 helper `crate::shared::invocation::check_invocation_source`，不再经 bridge。

use tauri::{AppHandle, Manager};

use crate::domain::{GraphNodeView, GraphQueryRequest, GraphQueryResult, GraphStats};
use crate::graph::{graph_node_get_impl, graph_query_impl, graph_stats_impl, validate_id_public, GraphState};
use crate::shared::invocation::check_invocation_source;

#[tauri::command]
pub async fn graph_query(
    app: AppHandle,
    webview: tauri::Webview,
    req: GraphQueryRequest,
) -> Result<GraphQueryResult, String> {
    check_invocation_source(&webview, "graph_query", None, &app)?;
    let start = req.start_id.trim();
    if let Err(e) = validate_id_public(start) {
        return Err(e.code().to_string());
    }
    // 纯内存读锁；临界区不跨 await，drop 后即无残留状态（取消 = no-op）。
    let state = app.state::<GraphState>();
    let store = state.store.read().unwrap();
    let result = graph_query_impl(&store, start, req.depth, req.limit);
    drop(store);
    Ok(result)
}

#[tauri::command]
pub async fn graph_node_get(
    app: AppHandle,
    webview: tauri::Webview,
    id: String,
) -> Result<Option<GraphNodeView>, String> {
    check_invocation_source(&webview, "graph_node_get", None, &app)?;
    if let Err(e) = validate_id_public(&id) {
        return Err(e.code().to_string());
    }
    let state = app.state::<GraphState>();
    let store = state.store.read().unwrap();
    let view = graph_node_get_impl(&store, &id);
    drop(store);
    Ok(view)
}

#[tauri::command]
pub async fn graph_stats(app: AppHandle, webview: tauri::Webview) -> Result<GraphStats, String> {
    check_invocation_source(&webview, "graph_stats", None, &app)?;
    let state = app.state::<GraphState>();
    let store = state.store.read().unwrap();
    let stats = graph_stats_impl(&store);
    drop(store);
    Ok(stats)
}
