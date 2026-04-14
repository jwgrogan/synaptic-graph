// Tests for ghost node pull-through

mod common;

mod pull_tests {
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    use synaptic_graph::db::Database;
    use synaptic_graph::ghost::pull::{pull_ghost_content, PullMode};
    use synaptic_graph::models::*;

    fn setup_ghost_vault() -> (Database, TempDir) {
        let db = Database::open_in_memory().expect("Failed to create in-memory database");
        let dir = TempDir::new().expect("Failed to create temp dir");

        // Write test markdown files
        let design_path = dir.path().join("design.md");
        let mut f = fs::File::create(&design_path).unwrap();
        writeln!(f, "# Design Overview").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "This document describes the architecture of the system in detail.").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "# Components").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "The system has several components that work together for processing.").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "# Deployment").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "Deployment uses containers and orchestration tooling for reliability.").unwrap();

        let arch_path = dir.path().join("architecture.md");
        let mut f = fs::File::create(&arch_path).unwrap();
        writeln!(f, "# Architecture Notes").unwrap();
        writeln!(f, "").unwrap();
        writeln!(f, "The architecture follows a layered pattern with clear boundaries.").unwrap();

        // Register ghost source
        let root = dir.path().to_str().unwrap();
        db.register_ghost_source("test-vault", root, "obsidian")
            .expect("Failed to register ghost source");

        // Insert ghost nodes
        let _design_node = db
            .insert_ghost_node(&NewGhostNode {
                source_graph: "test-vault".to_string(),
                external_ref: "design.md".to_string(),
                title: "Design Overview".to_string(),
                metadata: serde_json::json!({}),
                initial_weight: 0.3,
            })
            .expect("Failed to insert design ghost node");

        let _arch_node = db
            .insert_ghost_node(&NewGhostNode {
                source_graph: "test-vault".to_string(),
                external_ref: "architecture.md".to_string(),
                title: "Architecture Notes".to_string(),
                metadata: serde_json::json!({}),
                initial_weight: 0.3,
            })
            .expect("Failed to insert architecture ghost node");

        (db, dir)
    }

    #[test]
    fn test_pull_through_reads_file_content() {
        let (db, dir) = setup_ghost_vault();
        let node = db
            .get_ghost_node_by_ref("test-vault", "design.md")
            .unwrap();
        let root = dir.path().to_str().unwrap();

        let content = pull_ghost_content(&db, &node, root, PullMode::SessionOnly).unwrap();
        assert!(content.contains("Design"), "Content should contain 'Design'");
        assert!(
            content.contains("architecture"),
            "Content should contain 'architecture'"
        );
    }

    #[test]
    fn test_pull_through_permanent_creates_impulses() {
        let (db, _dir) = setup_ghost_vault();
        let node = db
            .get_ghost_node_by_ref("test-vault", "design.md")
            .unwrap();
        let sources = db.list_ghost_sources().unwrap();
        let root = &sources[0].root_path;

        let before_count = db.impulse_count().unwrap();

        let content = pull_ghost_content(&db, &node, root, PullMode::Permanent).unwrap();
        assert!(!content.is_empty());

        let after_count = db.impulse_count().unwrap();
        assert!(
            after_count > before_count,
            "Permanent pull should extract impulses from content"
        );

        // Impulses should be extracted (not raw file copy) and confirmed
        let impulses = db
            .list_impulses(Some(ImpulseStatus::Confirmed))
            .unwrap();
        let pulled: Vec<_> = impulses
            .iter()
            .filter(|i| i.source_type == SourceType::PullThrough)
            .collect();
        assert!(!pulled.is_empty());

        // Each extracted impulse should be shorter than the full file
        for imp in &pulled {
            assert!(
                imp.content.len() < content.len() || pulled.len() == 1,
                "Extracted impulses should be sections, not full file copies"
            );
        }
    }

    #[test]
    fn test_pull_through_session_only_no_impulse() {
        let (db, _dir) = setup_ghost_vault();
        let node = db
            .get_ghost_node_by_ref("test-vault", "architecture.md")
            .unwrap();
        let sources = db.list_ghost_sources().unwrap();
        let root = &sources[0].root_path;

        let before_count = db.impulse_count().unwrap();

        pull_ghost_content(&db, &node, root, PullMode::SessionOnly).unwrap();

        let after_count = db.impulse_count().unwrap();
        assert_eq!(before_count, after_count);
    }

    #[test]
    fn test_pull_through_updates_ghost_weight() {
        let (db, _dir) = setup_ghost_vault();
        let node = db
            .get_ghost_node_by_ref("test-vault", "design.md")
            .unwrap();
        let weight_before = node.weight;
        let sources = db.list_ghost_sources().unwrap();
        let root = &sources[0].root_path;

        pull_ghost_content(&db, &node, root, PullMode::SessionOnly).unwrap();

        let node_after = db.get_ghost_node(&node.id).unwrap();
        assert!(node_after.weight > weight_before);
    }
}
