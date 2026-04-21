pub mod pull;
pub mod scanner;

pub use pull::PullMode;
pub use scanner::ScanConfig;

use std::path::Path;

use crate::db::Database;
use crate::models::*;

/// Register a new ghost source and immediately scan its directory.
/// Returns the number of ghost nodes created.
pub fn register_and_scan(
    db: &Database,
    name: &str,
    root_path: &str,
    source_type: &str,
    config: &ScanConfig,
) -> Result<usize, String> {
    db.register_ghost_source(name, root_path, source_type)
        .map_err(|e| format!("Failed to register ghost source: {}", e))?;

    scan_and_store(db, name, root_path, config)
}

/// Refresh an existing ghost source by deleting all its ghost nodes and
/// re-scanning the directory. Returns the new node count.
pub fn refresh(db: &Database, name: &str, config: &ScanConfig) -> Result<usize, String> {
    // Look up the source by name
    let sources = db
        .list_ghost_sources()
        .map_err(|e| format!("Failed to list ghost sources: {}", e))?;

    let source = sources
        .into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("Ghost source '{}' not found", name))?;

    // Delete existing ghost nodes for this source
    db.delete_ghost_nodes_by_source(name)
        .map_err(|e| format!("Failed to delete ghost nodes: {}", e))?;

    // Re-scan and store
    scan_and_store(db, name, &source.root_path, config)
}

/// Scan a directory and store the resulting ghost nodes and connections.
/// Returns the number of ghost nodes created.
fn scan_and_store(
    db: &Database,
    name: &str,
    root_path: &str,
    config: &ScanConfig,
) -> Result<usize, String> {
    let result = scanner::scan_directory(Path::new(root_path), config)?;

    let node_count = result.nodes.len();

    // Insert ghost nodes
    for node in &result.nodes {
        let input = NewGhostNode {
            source_graph: name.to_string(),
            external_ref: node.external_ref.clone(),
            title: node.title.clone(),
            metadata: node.metadata.clone(),
            initial_weight: WEIGHT_EXPLICIT_SAVE,
        };
        db.insert_ghost_node(&input)
            .map_err(|e| format!("Failed to insert ghost node '{}': {}", node.external_ref, e))?;
    }

    // Insert ghost connections by resolving from_ref/to_ref to ghost node IDs
    for link in &result.links {
        let from_node = db.get_ghost_node_by_ref(name, &link.from_ref).ok();
        let to_node = db.get_ghost_node_by_ref(name, &link.to_ref).ok();

        if let (Some(from), Some(to)) = (from_node, to_node) {
            let input = NewGhostConnection {
                source_id: from.id,
                target_id: to.id,
                weight: 0.5,
                relationship: link.link_type.clone(),
            };
            db.insert_ghost_connection(&input)
                .map_err(|e| format!("Failed to insert ghost connection: {}", e))?;
        }
    }

    // Update scan timestamp
    db.update_ghost_source_scanned(name)
        .map_err(|e| format!("Failed to update scan timestamp: {}", e))?;

    Ok(node_count)
}
