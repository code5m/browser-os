//! Knowledge Graph 只读 live-query 命令（M5-W12，Lane A7）。
//!
//! 命令体仅做来源校验 + 调 `crate::graph` 纯内核；不反向依赖 `crate::bridge`
//! （graph.rs 仍是纯逻辑，守 GRAPH_NO_SECOND_PATH），无副作用 / 无 IO / 无写。
//! 出参经 View DTO 删除 `props`；错误仅稳定 `GRAPH_*` 码，绝不 echo 凭据/路径/body。
//!
//! 从 `bridge.rs` 迁入本 capability-owned 命令模块（Native Physical Boundary 分解）。
//! 来源校验复用共享 helper `crate::shared::invocation::check_invocation_source`，不再经 bridge。

use tauri::{AppHandle, Manager};

use crate::domain::{GraphEdge, GraphEdgeKind, GraphNode, GraphNodeKind, GraphNodeView, GraphQueryRequest, GraphQueryResult, GraphStats};
use crate::graph::{
    graph_node_get_impl, graph_query_impl, graph_stats_impl, validate_id_public, GraphState, GraphStore,
};
use crate::shared::invocation::check_invocation_source;


// Graph has one bounded in-memory store and one atomic snapshot. Imports and
// on-demand file indexing converge on that same store; never expose node props
// or full absolute paths in command results.
const MAX_IMPORT_BYTES: usize = 4 * 1024 * 1024;
const MAX_INDEX_NODES: usize = 1500;

#[derive(serde::Serialize)]
pub struct GraphImportSummary {
    node_count: usize,
    edge_count: usize,
    first_id: Option<String>,
}

fn summary(store: &GraphStore) -> GraphImportSummary {
    GraphImportSummary {
        node_count: store.nodes.len(),
        edge_count: store.edges.len(),
        first_id: store.nodes.first().map(|n| n.id.clone()),
    }
}

/// Import only a validated, referentially consistent graph. No unchecked
/// serde persistence, no shell, no network, no props in the result.
fn parse_import(text: &str) -> Result<GraphStore, String> {
    if text.len() > MAX_IMPORT_BYTES { return Err("GRAPH_IMPORT_TOO_LARGE".into()); }
    let candidate = GraphStore::from_json(text).map_err(|e| e.code().to_string())?;
    let mut clean = GraphStore::default();
    for node in candidate.nodes {
        clean.insert_node(node).map_err(|e| e.code().to_string())?;
    }
    for edge in candidate.edges {
        if !clean.nodes.iter().any(|n| n.id == edge.from)
            || !clean.nodes.iter().any(|n| n.id == edge.to) {
            return Err("GRAPH_EDGE_MISSING_ENDPOINT".into());
        }
        clean.insert_edge(edge).map_err(|e| e.code().to_string())?;
    }
    Ok(clean)
}

fn persist_graph(app: &AppHandle, store: GraphStore) -> Result<GraphImportSummary, String> {
    let encoded = store.to_json().map_err(|e| e.code().to_string())?;
    if encoded.len() > MAX_IMPORT_BYTES { return Err("GRAPH_IMPORT_TOO_LARGE".into()); }
    let state = app.state::<GraphState>();
    let mut guard = state.store.write().map_err(|_| "GRAPH_STORE_LOCK".to_string())?;
    // Existing startup read path; atomic rename so a crash cannot create
    // a partial JSON snapshot. Update live state only after disk success.
    let path = crate::workspace::data_dir(app).join("graph.json");
    crate::session::atomic_write(&path, &encoded).map_err(|_| "GRAPH_PERSIST_FAILED".to_string())?;
    let result = summary(&store);
    *guard = store;
    crate::workspace::log_audit(app, "graph.snapshot.update",
        format!("nodes={} edges={}", result.node_count, result.edge_count));
    Ok(result)
}

#[tauri::command]
pub fn graph_import_json(
    app: AppHandle,
    webview: tauri::Webview,
    text: String,
) -> Result<GraphImportSummary, String> {
    check_invocation_source(&webview, "graph_import_json", None, &app)?;
    let store = parse_import(&text)?;
    persist_graph(&app, store)
}

fn hashed_path(kind: &str, path: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{kind}:{}", path.display()).as_bytes());
    format!("{digest:x}")
}

fn collect_documents(roots: &[std::path::PathBuf]) -> GraphStore {
    use std::collections::VecDeque;
    let mut out = GraphStore::default();
    let mut queue: VecDeque<(std::path::PathBuf, Option<String>, u8)> =
        roots.iter().map(|p| (p.clone(), None, 0)).collect();
    while let Some((path, parent_id, depth)) = queue.pop_front() {
        if out.nodes.len() >= MAX_INDEX_NODES { break; }
        let Ok(meta) = std::fs::symlink_metadata(&path) else { continue };
        if meta.file_type().is_symlink() { continue; }
        let is_dir = meta.is_dir();
        let is_note = path.extension().and_then(|x| x.to_str())
            .map(|x| matches!(x.to_ascii_lowercase().as_str(), "md" | "markdown" | "txt" | "pdf" | "docx"))
            .unwrap_or(false);
        if !is_dir && (!meta.is_file() || !is_note) { continue; }
        let kind = if is_dir { GraphNodeKind::Dir } else { GraphNodeKind::File };
        let id = hashed_path(if is_dir { "dir" } else { "file" }, &path);
        let label = path.file_name().and_then(|x| x.to_str()).unwrap_or("知识目录")
            .chars().take(90).collect::<String>();
        if out.insert_node(GraphNode { id: id.clone(), kind, label, props: Default::default() }).is_err() {
            continue;
        }
        if let Some(parent) = parent_id {
            let _ = out.insert_edge(GraphEdge {
                from: id.clone(), to: parent, kind: GraphEdgeKind::InDir,
                weight: 1, props: Default::default(),
            });
        }
        if is_dir && depth < 6 {
            if let Ok(entries) = std::fs::read_dir(&path) {
                let mut children: Vec<_> = entries.flatten().map(|e| e.path()).collect();
                children.sort();
                for child in children.into_iter().take(800) {
                    queue.push_back((child, Some(id.clone()), depth + 1));
                }
            }
        }
    }
    out
}

/// On-demand bounded metadata index: only BrowserOS workspace and notes.
/// Never read document bodies or follow symlinks. The user triggers the scan.
#[tauri::command]
pub fn graph_rebuild_documents(
    app: AppHandle,
    webview: tauri::Webview,
) -> Result<GraphImportSummary, String> {
    check_invocation_source(&webview, "graph_rebuild_documents", None, &app)?;
    let roots = [
        crate::workspace::workspace_dir(&app),
        crate::workspace::notes_dir(&app),
    ];
    persist_graph(&app, collect_documents(&roots))
}

#[cfg(test)]
mod import_tests {
    use super::*;
    #[test]
    fn import_requires_valid_endpoints_and_unique_nodes() {
        let bad = r#"{"nodes":[{"id":"a","kind":"file","label":"a"}],"edges":[{"from":"a","to":"missing","kind":"references"}]}"#;
        assert_eq!(parse_import(bad).err().as_deref(), Some("GRAPH_EDGE_MISSING_ENDPOINT"));
        let duplicate = r#"{"nodes":[{"id":"a","kind":"file","label":"a"},{"id":"a","kind":"file","label":"b"}],"edges":[]}"#;
        assert_eq!(parse_import(duplicate).err().as_deref(), Some("GRAPH_DUPLICATE_NODE"));
        assert_eq!(parse_import(&"x".repeat(MAX_IMPORT_BYTES+1)).err().as_deref(), Some("GRAPH_IMPORT_TOO_LARGE"));
    }
    #[test]
    fn scan_does_not_follow_symlinks_or_read_body() {
        let base = std::env::temp_dir().join(format!("browseros-graph-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("a.md"), "SECRET_DO_NOT_READ").unwrap();
        #[cfg(unix)] std::os::unix::fs::symlink("/etc", base.join("escape")).unwrap();
        let result = collect_documents(&[base.clone()]);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);
        assert!(!result.to_json().unwrap().contains("SECRET_DO_NOT_READ"));
        let _ = std::fs::remove_dir_all(base);
    }
}

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
