//! Vault -> global GraphStore, using the existing node/edge storage only.
//! A Vault sync is an explicit projection; Markdown files remain the source of truth.
use super::markdown_links::{note_links, resolve_note};
use crate::domain::{GraphEdge, GraphEdgeKind, GraphNode, GraphNodeKind};
use crate::graph::GraphStore;
use crate::workbench::VaultNote;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn hash(kind: &str, text: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{kind}:{text}").as_bytes()))
}
fn file_id(root: &Path, relative: &str) -> String {
    hash("file", &root.join(relative).display().to_string())
}
fn root_hash(root: &Path) -> String {
    hash("vault", &root.display().to_string())
}
fn add_edge(
    store: &mut GraphStore,
    from: &str,
    to: &str,
    kind: GraphEdgeKind,
) -> Result<(), String> {
    if store
        .edges
        .iter()
        .any(|e| e.from == from && e.to == to && e.kind == kind)
    {
        return Ok(());
    }
    store
        .insert_edge(GraphEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            kind,
            weight: 1,
            props: Default::default(),
        })
        .map_err(|e| e.code().to_owned())
}
fn indexed_paths(store: &GraphStore, marker: &str) -> BTreeMap<String, String> {
    store
        .nodes
        .iter()
        .filter_map(|n| {
            if n.props.get("vault_root")? == marker {
                Some((n.props.get("vault_path")?.clone(), n.id.clone()))
            } else {
                None
            }
        })
        .collect()
}
fn add_references(
    store: &mut GraphStore,
    source: &str,
    path: &str,
    body: &str,
    index: &BTreeMap<String, String>,
) -> Result<(), String> {
    let paths: Vec<String> = index.keys().cloned().collect();
    for link in note_links(body) {
        if let Some(target_path) = resolve_note(&link, path, &paths) {
            if let Some(target_id) = index.get(&target_path) {
                if source != target_id {
                    add_edge(store, source, target_id, GraphEdgeKind::References)?;
                }
            }
        }
    }
    Ok(())
}

/// Replace only the named Vault projection. Do not wipe unrelated GraphStore nodes.
pub fn full_sync(store: &mut GraphStore, root: &Path, notes: &[VaultNote]) -> Result<(), String> {
    let marker = root_hash(root);
    let old = indexed_paths(store, &marker);
    let present: BTreeSet<&str> = notes.iter().map(|n| n.path.as_str()).collect();
    // Remove only obsolete note nodes and references that belonged to this Vault.
    let obsolete: BTreeSet<String> = old
        .iter()
        .filter(|(path, _)| !present.contains(path.as_str()))
        .map(|(_, id)| id.clone())
        .collect();
    store.nodes.retain(|n| !obsolete.contains(&n.id));
    store.edges.retain(|e| {
        !obsolete.contains(&e.from)
            && !obsolete.contains(&e.to)
            && !(old.values().any(|id| id == &e.from) && e.kind == GraphEdgeKind::References)
    });
    let root_id = hash("vaultdir", &root.display().to_string());
    if !store.nodes.iter().any(|n| n.id == root_id) {
        store
            .insert_node(GraphNode {
                id: root_id.clone(),
                kind: GraphNodeKind::Dir,
                label: "Vault 笔记库".into(),
                props: Default::default(),
            })
            .map_err(|e| e.code().to_owned())?;
    }
    for note in notes {
        if note.text.len() > crate::workbench::MAX_NOTE_BYTES as usize {
            return Err("GRAPH_VAULT_NOTE_LIMIT".into());
        }
        let id = file_id(root, &note.path);
        let mut props = crate::domain::GraphProps::default();
        props.insert("vault_root".to_owned(), marker.clone());
        props.insert("vault_path".to_owned(), note.path.clone());
        if let Some(existing) = store.nodes.iter_mut().find(|n| n.id == id) {
            existing.props = props;
        } else {
            store
                .insert_node(GraphNode {
                    id: id.clone(),
                    kind: GraphNodeKind::File,
                    label: note
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or("笔记")
                        .chars()
                        .take(72)
                        .collect(),
                    props,
                })
                .map_err(|e| e.code().to_owned())?;
        }
        add_edge(store, &id, &root_id, GraphEdgeKind::InDir)?;
    }
    let index = indexed_paths(store, &marker);
    for note in notes {
        let source = index.get(&note.path).ok_or("GRAPH_VAULT_SOURCE_MISSING")?;
        add_references(store, source, &note.path, &note.text, &index)?;
    }
    Ok(())
}

/// Only the saved note's outgoing References edges change; inbound links stay intact.
pub fn update_note(
    store: &mut GraphStore,
    root: &Path,
    relative: &str,
    body: &str,
) -> Result<(), String> {
    if body.len() > crate::workbench::MAX_NOTE_BYTES as usize {
        return Err("GRAPH_VAULT_NOTE_LIMIT".into());
    }
    let index = indexed_paths(store, &root_hash(root));
    let source = index
        .get(relative)
        .ok_or("GRAPH_VAULT_NOT_INDEXED")?
        .clone();
    store
        .edges
        .retain(|e| !(e.from == source && e.kind == GraphEdgeKind::References));
    add_references(store, &source, relative, body, &index)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn note(path: &str, text: &str) -> VaultNote {
        VaultNote {
            path: path.into(),
            text: text.into(),
        }
    }
    #[test]
    fn full_sync_and_update_preserve_other_nodes() {
        let root = Path::new("/tmp/safe-vault");
        let notes = vec![note("A.md", "[[B]]"), note("B.md", "text")];
        let mut store = GraphStore::default();
        store
            .insert_node(GraphNode {
                id: "independent".into(),
                kind: GraphNodeKind::File,
                label: "Independent".into(),
                props: Default::default(),
            })
            .unwrap();
        full_sync(&mut store, root, &notes).unwrap();
        assert_eq!(
            store
                .edges
                .iter()
                .filter(|e| e.kind == GraphEdgeKind::References)
                .count(),
            1
        );
        update_note(&mut store, root, "A.md", "no links").unwrap();
        assert_eq!(
            store
                .edges
                .iter()
                .filter(|e| e.kind == GraphEdgeKind::References)
                .count(),
            0
        );
        full_sync(&mut store, root, &[note("A.md", "orphan")]).unwrap();
        assert!(store.nodes.iter().any(|n| n.id == "independent"));
        assert!(!store.nodes.iter().any(|n| n.label == "B.md"));
    }
    #[test]
    fn cross_vault_and_ambiguous_names_fail_closed() {
        let mut store = GraphStore::default();
        let root = Path::new("/tmp/another-vault");
        full_sync(
            &mut store,
            root,
            &[
                note("src.md", "[[A]]"),
                note("x/A.md", ""),
                note("y/A.md", ""),
            ],
        )
        .unwrap();
        assert!(store
            .edges
            .iter()
            .all(|e| e.kind != GraphEdgeKind::References));
        assert_eq!(
            update_note(&mut store, root, "unknown.md", "[[src]]"),
            Err("GRAPH_VAULT_NOT_INDEXED".into())
        );
    }
}
