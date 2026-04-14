mod common;

use synaptic_graph::ghost::scanner::{ScanConfig, scan_directory};
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
