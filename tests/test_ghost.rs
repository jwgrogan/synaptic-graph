mod common;

use synaptic_graph::ghost::scanner::{ScanConfig, scan_directory};
use synaptic_graph::ghost::{register_and_scan, refresh};
use std::fs;
use tempfile::TempDir;

fn create_test_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create some markdown files with links
    fs::write(root.join("index.md"), "# Index\n\nSee [[design]] and [[architecture]].\n").unwrap();
    fs::write(root.join("design.md"), "# Design\n\nRelates to [[architecture]].\nTagged: #philosophy #design\n").unwrap();
    fs::write(root.join("architecture.md"), "# Architecture\n\nSQLite-based system.\n").unwrap();

    // Create a subdirectory
    fs::create_dir(root.join("notes")).unwrap();
    fs::write(root.join("notes/daily.md"), "# Daily Note\n\nNothing important.\n").unwrap();

    // Create a non-markdown file (should be ignored by default)
    fs::write(root.join("image.png"), "fake png data").unwrap();

    dir
}

#[test]
fn test_scan_discovers_markdown_files() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    assert_eq!(result.nodes.len(), 4); // index, design, architecture, daily
}

#[test]
fn test_scan_extracts_titles() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    let titles: Vec<&str> = result.nodes.iter().map(|n| n.title.as_str()).collect();
    assert!(titles.contains(&"Index"));
    assert!(titles.contains(&"Design"));
    assert!(titles.contains(&"Architecture"));
}

#[test]
fn test_scan_extracts_wikilinks() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    // index.md links to design and architecture
    assert!(result.links.len() >= 3);
}

#[test]
fn test_scan_ignores_non_matching_extensions() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    let refs: Vec<&str> = result.nodes.iter().map(|n| n.external_ref.as_str()).collect();
    assert!(!refs.iter().any(|r| r.contains("image.png")));
}

#[test]
fn test_scan_respects_ignore_patterns() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec!["notes/".to_string()],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    assert_eq!(result.nodes.len(), 3); // daily.md excluded
}

#[test]
fn test_register_and_scan_ghost_graph() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let count = register_and_scan(
        &db,
        "test-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    )
    .unwrap();

    assert_eq!(count, 4); // index, design, architecture, daily

    // Verify ghost nodes were stored in the database
    let nodes = db.list_ghost_nodes_by_source("test-vault").unwrap();
    assert_eq!(nodes.len(), 4);

    // Verify the source was registered
    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].name, "test-vault");
    assert_eq!(sources[0].source_type, "obsidian");
    assert!(sources[0].last_scanned_at.is_some());
}

#[test]
fn test_refresh_updates_ghost_graph() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    // Initial scan
    let count = register_and_scan(
        &db,
        "test-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    )
    .unwrap();
    assert_eq!(count, 4);

    // Add a new file to the vault
    fs::write(
        vault.path().join("new_note.md"),
        "# New Note\n\nSomething fresh.\n",
    )
    .unwrap();

    // Refresh
    let new_count = refresh(&db, "test-vault", &config).unwrap();
    assert_eq!(new_count, 5);

    // Verify old nodes were replaced
    let nodes = db.list_ghost_nodes_by_source("test-vault").unwrap();
    assert_eq!(nodes.len(), 5);

    let titles: Vec<&str> = nodes.iter().map(|n| n.title.as_str()).collect();
    assert!(titles.contains(&"New Note"));
}

#[test]
fn test_scan_creates_ghost_connections_from_wikilinks() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    register_and_scan(
        &db,
        "test-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    )
    .unwrap();

    // index.md links to design.md and architecture.md
    let index_node = db.get_ghost_node_by_ref("test-vault", "index.md").unwrap();
    let connections = db.get_ghost_connections_for_node(&index_node.id).unwrap();
    assert!(
        connections.len() >= 2,
        "Expected at least 2 connections from index.md, got {}",
        connections.len()
    );

    // design.md links to architecture.md
    let design_node = db.get_ghost_node_by_ref("test-vault", "design.md").unwrap();
    let design_connections = db.get_ghost_connections_for_node(&design_node.id).unwrap();
    // design has outgoing link to architecture + incoming link from index
    assert!(
        !design_connections.is_empty(),
        "Expected connections for design.md"
    );

    // Verify the relationship type is wikilink
    for conn in &connections {
        assert_eq!(conn.relationship, "wikilink");
    }
}
