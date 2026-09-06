//! M5-7 / M5-8 知识图谱 core 切片（Lane A7, M5-W5）。
//!
//! 纯逻辑、无副作用：DTO 在 `domain.rs`，此处提供校验 / 容量 / 脱敏 / bounded store /
//! bounded query 助手。本模块**不**引入 `tauri` / `AppHandle` / `crate::bridge` / 网络 /
//! 命令 / 第二执行路径；持久化（SQLite 单连接）留待 M5-8 接 `database.rs`（A3/A4 稳定后）。
//! 与 A4/A5 的契约边界（承 W4 A7 delta）：Skill/Agent 节点**按 id 引用** A5 的
//! `SkillDef.id`/`AgentDef.id`，图谱不存 `agent_kv` 值（仅 `Memorizes` 关系边）。

// M5-W5：本切片暂无消费方（图谱 UI / agent 消费 / 后台重建 worker 均留待 M5-8），
// 故整模块允许 dead_code；待 M5-8 接入 store/query/UI 后自然消解。
#![allow(dead_code)]

use crate::domain::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// 凭据/正文敏感字段名黑名单（与 A4 agent_kv 隐私双扫同源）。
const SENSITIVE_KEY_NAMES: &[&str] = &["token", "password", "secret", "api_key"];
/// 凭据/正文敏感值模式（前缀/关键字匹配，避免误杀）。
const SENSITIVE_VALUE_PATTERNS: &[&str] = &["sk-", "AKIA", "Bearer ", "eyJ", "-----BEGIN"];

/// 单节点 props 条目数上限（防 props 膨胀）。
const GRAPH_PROPS_MAX_ENTRIES: usize = 64;

/// 图谱校验/容量错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    EmptyId,
    IdTooLong,
    RefIdNotHex,
    LabelTooLong,
    PropKeyTooLong(String),
    PropValueTooLong(String),
    PropCountExceeded,
    SecretInProps,
    NodeCapacityExceeded,
    EdgeCapacityExceeded,
    DuplicateNode(String),
    DuplicateEdge(String),
    Serde(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::EmptyId => write!(f, "node/edge id 为空"),
            GraphError::IdTooLong => write!(f, "node/edge id 超长"),
            GraphError::RefIdNotHex => {
                write!(
                    f,
                    "Skill/Agent 节点 id 须为 {GRAPH_NODE_ID_HEX_LEN} 位十六进制（AGRAPH-10）"
                )
            }
            GraphError::LabelTooLong => write!(f, "label 超 GRAPH_LABEL_MAX_BYTES"),
            GraphError::PropKeyTooLong(k) => write!(f, "prop key 超长: {k}"),
            GraphError::PropValueTooLong(k) => {
                write!(f, "prop value 超 GRAPH_PROPS_MAX_BYTES: {k}")
            }
            GraphError::PropCountExceeded => {
                write!(f, "单节点 props 条目超过 {GRAPH_PROPS_MAX_ENTRIES}")
            }
            GraphError::SecretInProps => write!(f, "props 含凭据/正文敏感字段（AGRAPH-9）"),
            GraphError::NodeCapacityExceeded => {
                write!(f, "节点数超过 GRAPH_MAX_NODES={GRAPH_MAX_NODES}")
            }
            GraphError::EdgeCapacityExceeded => {
                write!(f, "边数超过 GRAPH_MAX_EDGES={GRAPH_MAX_EDGES}")
            }
            GraphError::DuplicateNode(id) => write!(f, "重复节点 id: {id}"),
            GraphError::DuplicateEdge(k) => write!(f, "重复边: {k}"),
            GraphError::Serde(m) => write!(f, "序列化失败: {m}"),
        }
    }
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// 递归扫描一个字符串叶子是否命中敏感值模式（与 A4 隐私双扫同源）。
fn scan_graph_value(s: &str) -> bool {
    SENSITIVE_VALUE_PATTERNS.iter().any(|p| s.contains(p))
}

/// props 双重扫描：字段名黑名单 + 字符串值模式，任一命中即视为凭据/正文泄漏（AGRAPH-9）。
pub fn graph_props_contain_secret(props: &GraphProps) -> bool {
    for key in props.keys() {
        let k = key.to_ascii_lowercase();
        if SENSITIVE_KEY_NAMES.iter().any(|n| k.contains(n)) {
            return true;
        }
    }
    for val in props.values() {
        if scan_graph_value(val) {
            return true;
        }
    }
    false
}

fn validate_id(id: &str, kind: GraphNodeKind) -> Result<(), GraphError> {
    if id.is_empty() {
        return Err(GraphError::EmptyId);
    }
    match kind {
        GraphNodeKind::Skill | GraphNodeKind::Agent => {
            if !is_hex(id, GRAPH_NODE_ID_HEX_LEN) {
                return Err(GraphError::RefIdNotHex);
            }
        }
        _ => {
            if id.len() > 512 {
                return Err(GraphError::IdTooLong);
            }
        }
    }
    Ok(())
}

fn validate_props(props: &GraphProps) -> Result<(), GraphError> {
    if props.len() > GRAPH_PROPS_MAX_ENTRIES {
        return Err(GraphError::PropCountExceeded);
    }
    for (k, v) in props.iter() {
        if k.len() > GRAPH_LABEL_MAX_BYTES {
            return Err(GraphError::PropKeyTooLong(k.clone()));
        }
        if v.len() > GRAPH_PROPS_MAX_BYTES {
            return Err(GraphError::PropValueTooLong(k.clone()));
        }
    }
    if graph_props_contain_secret(props) {
        return Err(GraphError::SecretInProps);
    }
    Ok(())
}

/// 校验单个节点（容量 / 脱敏 / Skill·Agent id 完整性）。
pub fn validate_graph_node(node: &GraphNode) -> Result<(), GraphError> {
    validate_id(&node.id, node.kind)?;
    if node.label.as_bytes().len() > GRAPH_LABEL_MAX_BYTES {
        return Err(GraphError::LabelTooLong);
    }
    validate_props(&node.props)
}

/// 校验单条边（容量 / 脱敏；from/to 非空）。
pub fn validate_graph_edge(edge: &GraphEdge) -> Result<(), GraphError> {
    if edge.from.is_empty() || edge.to.is_empty() {
        return Err(GraphError::EmptyId);
    }
    if edge.from.len() > 512 || edge.to.len() > 512 {
        return Err(GraphError::IdTooLong);
    }
    validate_props(&edge.props)
}

/// 内存 bounded 图谱存储。所有插入受 `GRAPH_MAX_NODES` / `GRAPH_MAX_EDGES` 约束。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStore {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn find_node(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// 插入节点；容量/去重/校验任一失败即拒。
    pub fn insert_node(&mut self, node: GraphNode) -> Result<(), GraphError> {
        validate_graph_node(&node)?;
        if self.find_node(&node.id).is_some() {
            return Err(GraphError::DuplicateNode(node.id.clone()));
        }
        if self.nodes.len() >= GRAPH_MAX_NODES {
            return Err(GraphError::NodeCapacityExceeded);
        }
        self.nodes.push(node);
        Ok(())
    }

    /// 插入边；容量/去重/校验任一失败即拒。
    pub fn insert_edge(&mut self, edge: GraphEdge) -> Result<(), GraphError> {
        validate_graph_edge(&edge)?;
        let key = format!("{}|{:?}|{}", edge.from, edge.kind, edge.to);
        if self
            .edges
            .iter()
            .any(|e| format!("{}|{:?}|{}", e.from, e.kind, e.to) == key)
        {
            return Err(GraphError::DuplicateEdge(key));
        }
        if self.edges.len() >= GRAPH_MAX_EDGES {
            return Err(GraphError::EdgeCapacityExceeded);
        }
        self.edges.push(edge);
        Ok(())
    }

    /// 从 `start_id` 出发的 bounded 邻居查询：深度 ≤ `GRAPH_MAX_DEPTH`，返回节点数 ≤ `limit`。
    /// 双向遍历（边无向用于发现），结果按 BFS 层级稳定排序。
    pub fn bounded_neighbors(&self, start_id: &str, depth: usize, limit: usize) -> Vec<GraphNode> {
        let depth = depth.min(GRAPH_MAX_DEPTH);
        let limit = limit.min(GRAPH_QUERY_LIMIT);
        let mut visited: BTreeMap<String, usize> = BTreeMap::new();
        let mut out: Vec<GraphNode> = Vec::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        if let Some(n) = self.nodes.iter().find(|n| n.id == start_id) {
            visited.insert(n.id.clone(), 0);
            out.push(n.clone());
            queue.push_back((n.id.clone(), 0));
        }
        while let Some((id, d)) = queue.pop_front() {
            if d >= depth || out.len() >= limit {
                break;
            }
            for e in self.edges.iter() {
                let next = if e.from == id {
                    Some(&e.to)
                } else if e.to == id {
                    Some(&e.from)
                } else {
                    None
                };
                if let Some(nb) = next {
                    if visited.contains_key(nb) {
                        continue;
                    }
                    if let Some(node) = self.nodes.iter().find(|n| &n.id == nb) {
                        visited.insert(node.id.clone(), d + 1);
                        out.push(node.clone());
                        if out.len() >= limit {
                            return out;
                        }
                        queue.push_back((node.id.clone(), d + 1));
                    }
                }
            }
        }
        out
    }

    /// 从 `start_id` 出发、深度 ≤ `depth` 的 bounded 子图（节点 + 其间边），用于导出/查询。
    pub fn bounded_subgraph(&self, start_id: &str, depth: usize) -> GraphStore {
        let nodes = self.bounded_neighbors(start_id, depth, GRAPH_QUERY_LIMIT);
        let ids: std::collections::BTreeSet<String> = nodes.iter().map(|n| n.id.clone()).collect();
        let edges: Vec<GraphEdge> = self
            .edges
            .iter()
            .filter(|e| ids.contains(&e.from) && ids.contains(&e.to))
            .cloned()
            .collect();
        GraphStore { nodes, edges }
    }

    /// 序列化为 JSON（受 bounded 约束，整体不会超 `GRAPH_MAX_NODES/EDGES`）。
    pub fn to_json(&self) -> Result<String, GraphError> {
        serde_json::to_string(self).map_err(|e| GraphError::Serde(e.to_string()))
    }

    /// 从 JSON 反序列化并逐条校验（容量/脱敏/完整性）。
    pub fn from_json(text: &str) -> Result<Self, GraphError> {
        let store: GraphStore =
            serde_json::from_str(text).map_err(|e| GraphError::Serde(e.to_string()))?;
        for n in &store.nodes {
            validate_graph_node(n)?;
        }
        for e in &store.edges {
            validate_graph_edge(e)?;
        }
        if store.nodes.len() > GRAPH_MAX_NODES {
            return Err(GraphError::NodeCapacityExceeded);
        }
        if store.edges.len() > GRAPH_MAX_EDGES {
            return Err(GraphError::EdgeCapacityExceeded);
        }
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_id() -> String {
        // 64 位十六进制占位（sha256 派生形态；"skill:"/"agent:" 仅为派生前辍，不进存储 id）。
        let mut s = String::with_capacity(64);
        for _ in 0..64 {
            s.push('a');
        }
        s
    }

    #[test]
    fn graph_constants_eq() {
        // GRAPH_PROPS_MAX_BYTES 必须等于 MAX_TEXT_FIELD_BYTES（单一不变量）。
        assert_eq!(GRAPH_PROPS_MAX_BYTES, MAX_TEXT_FIELD_BYTES);
    }

    #[test]
    fn validate_rejects_oversized_label() {
        let node = GraphNode {
            id: "file:/a".into(),
            kind: GraphNodeKind::File,
            label: "x".repeat(GRAPH_LABEL_MAX_BYTES + 1),
            props: GraphProps::new(),
        };
        assert!(matches!(
            validate_graph_node(&node),
            Err(GraphError::LabelTooLong)
        ));
    }

    #[test]
    fn validate_rejects_secret_in_props() {
        let mut props = GraphProps::new();
        props.insert("note".into(), "Bearer abcd1234".into());
        let node = GraphNode {
            id: "file:/a".into(),
            kind: GraphNodeKind::File,
            label: "a".into(),
            props,
        };
        assert!(matches!(
            validate_graph_node(&node),
            Err(GraphError::SecretInProps)
        ));
        // 字段名黑名单同样拦截
        let mut props2 = GraphProps::new();
        props2.insert("api_key".into(), "ok".into());
        let node2 = GraphNode {
            id: "file:/b".into(),
            kind: GraphNodeKind::File,
            label: "b".into(),
            props: props2,
        };
        assert!(matches!(
            validate_graph_node(&node2),
            Err(GraphError::SecretInProps)
        ));
    }

    #[test]
    fn validate_requires_hex_ref_id_for_skill_agent() {
        let bad = GraphNode {
            id: "not-hex".into(),
            kind: GraphNodeKind::Skill,
            label: "s".into(),
            props: GraphProps::new(),
        };
        assert!(matches!(
            validate_graph_node(&bad),
            Err(GraphError::RefIdNotHex)
        ));
        let good = GraphNode {
            id: ref_id(),
            kind: GraphNodeKind::Skill,
            label: "s".into(),
            props: GraphProps::new(),
        };
        assert!(validate_graph_node(&good).is_ok());
    }

    #[test]
    fn store_enforces_node_capacity() {
        let mut store = GraphStore::new();
        for i in 0..GRAPH_MAX_NODES {
            let node = GraphNode {
                id: format!("file:/{i}"),
                kind: GraphNodeKind::File,
                label: "x".into(),
                props: GraphProps::new(),
            };
            store.insert_node(node).unwrap();
        }
        let overflow = GraphNode {
            id: "file:/overflow".into(),
            kind: GraphNodeKind::File,
            label: "x".into(),
            props: GraphProps::new(),
        };
        assert!(matches!(
            store.insert_node(overflow),
            Err(GraphError::NodeCapacityExceeded)
        ));
    }

    #[test]
    fn store_rejects_duplicate_node() {
        let mut store = GraphStore::new();
        let node = GraphNode {
            id: "file:/dup".into(),
            kind: GraphNodeKind::File,
            label: "x".into(),
            props: GraphProps::new(),
        };
        store.insert_node(node.clone()).unwrap();
        assert!(matches!(
            store.insert_node(node),
            Err(GraphError::DuplicateNode(_))
        ));
    }

    #[test]
    fn bounded_neighbors_respects_depth_and_limit() {
        let mut store = GraphStore::new();
        // 链 a->b->c->d->e->f（每节一个节点 + 一条 References 边）
        let ids = ["a", "b", "c", "d", "e", "f"];
        for id in ids.iter() {
            store
                .insert_node(GraphNode {
                    id: id.to_string(),
                    kind: GraphNodeKind::File,
                    label: id.to_string(),
                    props: GraphProps::new(),
                })
                .unwrap();
        }
        for w in ids.windows(2) {
            store
                .insert_edge(GraphEdge {
                    from: w[0].to_string(),
                    to: w[1].to_string(),
                    kind: GraphEdgeKind::References,
                    weight: 1,
                    props: GraphProps::new(),
                })
                .unwrap();
        }
        // 深度 1：只有 a 和 b
        let lvl1 = store.bounded_neighbors("a", 1, GRAPH_QUERY_LIMIT);
        assert_eq!(lvl1.len(), 2);
        // 深度 2：a,b,c
        let lvl2 = store.bounded_neighbors("a", 2, GRAPH_QUERY_LIMIT);
        assert_eq!(lvl2.len(), 3);
        // 限制为 1：只有 a
        let lim = store.bounded_neighbors("a", GRAPH_MAX_DEPTH, 1);
        assert_eq!(lim.len(), 1);
    }

    #[test]
    fn json_roundtrip_preserves_graph() {
        let mut store = GraphStore::new();
        store
            .insert_node(GraphNode {
                id: "file:/x".into(),
                kind: GraphNodeKind::File,
                label: "x".into(),
                props: GraphProps::new(),
            })
            .unwrap();
        store
            .insert_edge(GraphEdge {
                from: "file:/x".into(),
                to: ref_id(),
                kind: GraphEdgeKind::Uses,
                weight: 1,
                props: GraphProps::new(),
            })
            .unwrap();
        let json = store.to_json().unwrap();
        let back = GraphStore::from_json(&json).unwrap();
        assert_eq!(back.nodes.len(), 1);
        assert_eq!(back.edges.len(), 1);
        assert_eq!(back.edges[0].kind, GraphEdgeKind::Uses);
    }

    #[test]
    fn from_json_rejects_secret() {
        let json = r#"{"nodes":[{"id":"file:/x","kind":"file","label":"x","props":{"api_key":"leak"}}],"edges":[]}"#;
        assert!(matches!(
            GraphStore::from_json(json),
            Err(GraphError::SecretInProps)
        ));
    }
}
