mod common;

use synaptic_graph::activation::ActivationEngine;
use synaptic_graph::extraction::{self, EngagementSignals, ExtractionDepth};
use synaptic_graph::ghost;
use synaptic_graph::ghost::pull::{pull_ghost_content, PullMode};
use synaptic_graph::ghost::scanner::ScanConfig;
use synaptic_graph::ingestion;
use synaptic_graph::models::*;
use std::fs;
use tempfile::TempDir;

fn create_validation_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(
        root.join("design-philosophy.md"),
        "# Design Philosophy\n\nMemory should be portable and user-owned.\nSee [[architecture]] for implementation.\n",
    ).unwrap();

    fs::write(
        root.join("architecture.md"),
        "# Architecture\n\nSQLite-based local-first service with graph model.\nSee [[design-philosophy]] for principles.\n",
    ).unwrap();

    fs::write(
        root.join("rust-patterns.md"),
        "# Rust Patterns\n\nOwnership, borrowing, and lifetimes for safe memory management.\n",
    ).unwrap();

    fs::create_dir(root.join("private")).unwrap();
    fs::write(
        root.join("private/secrets.md"),
        "# Secrets\n\nAPI key: AKIAIOSFODNN7EXAMPLE\nDo not share.\n",
    ).unwrap();

    dir
}

// ============================================================
// PRD P2 Criterion 1: Ghost graph maps topology without content
// ============================================================

#[test]
fn validation_p2_ghost_maps_topology_no_content() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(
        &db,
        "validation-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    ).unwrap();

    let nodes = db.list_ghost_nodes_by_source("validation-vault").unwrap();

    // Should have all 4 markdown files mapped
    assert_eq!(nodes.len(), 4);

    // No impulses should have been created (content not ingested)
    assert_eq!(db.impulse_count().unwrap(), 0);

    // Nodes should have titles and metadata but not full content
    let design = nodes.iter().find(|n| n.title == "Design Philosophy").unwrap();
    assert!(!design.external_ref.is_empty());
}

// ============================================================
// PRD P2 Criterion 2: Pull-through activates on relevant query
// ============================================================

#[test]
fn validation_p2_pull_through_on_query() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(
        &db,
        "v-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    ).unwrap();

    // Add a related impulse to the memory graph
    ingestion::save_and_confirm(
        &db,
        "Memory systems should be portable across providers",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "design philosophy".to_string(),
        max_results: 10,
        max_hops: 2,
    }).unwrap();

    // Should have ghost activations for the design philosophy note
    assert!(
        !result.ghost_activations.is_empty(),
        "Ghost nodes should activate on relevant query"
    );

    // Pull the activated ghost node content
    let sources = db.list_ghost_sources().unwrap();
    let root = &sources[0].root_path;
    let ghost_node = &result.ghost_activations[0].ghost_node;

    let content = pull_ghost_content(&db, ghost_node, root, PullMode::SessionOnly).unwrap();
    assert!(!content.is_empty());
    assert!(content.contains("portable") || content.contains("Philosophy"));
}

// ============================================================
// PRD P2 Criterion 3: Session-only pulls leave no persistent trace
// ============================================================

#[test]
fn validation_p2_session_only_no_trace() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let before = db.impulse_count().unwrap();

    let node = db.get_ghost_node_by_ref("v-vault", "architecture.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    pull_ghost_content(&db, &node, &sources[0].root_path, PullMode::SessionOnly).unwrap();

    let after = db.impulse_count().unwrap();
    assert_eq!(before, after, "Session-only pull should not create impulses");
}

// ============================================================
// PRD P2 Criterion 4: Permanent pulls create full memory nodes
// ============================================================

#[test]
fn validation_p2_permanent_pull_creates_node() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let before = db.impulse_count().unwrap();

    let node = db.get_ghost_node_by_ref("v-vault", "rust-patterns.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    pull_ghost_content(&db, &node, &sources[0].root_path, PullMode::Permanent).unwrap();

    let after = db.impulse_count().unwrap();
    assert!(after > before, "Permanent pull should create an impulse");

    // The created impulse should have pull_through source type
    let impulses = db.list_impulses(None).unwrap();
    let pulled = impulses.iter().find(|i| i.source_type == SourceType::PullThrough);
    assert!(pulled.is_some());
    assert!(pulled.unwrap().source_ref.contains("v-vault"));
}

// ============================================================
// PRD P2 Criterion 6: Adaptive extraction scales with engagement
// ============================================================

#[test]
fn validation_p2_adaptive_extraction_scales() {
    // Low engagement
    let low = EngagementSignals {
        total_turns: 2,
        avg_user_message_length: 15.0,
        avg_assistant_message_length: 40.0,
        session_duration_minutes: 1.0,
        explicit_save_count: 0,
        topic_count: 1,
        decision_keywords_found: 0,
        emotional_keywords_found: 0,
    };
    assert_eq!(extraction::assess_engagement(&low), ExtractionDepth::Minimal);
    assert_eq!(ExtractionDepth::Minimal.max_proposals(), 1);

    // High engagement (like this design conversation)
    let high = EngagementSignals {
        total_turns: 25,
        avg_user_message_length: 180.0,
        avg_assistant_message_length: 450.0,
        session_duration_minutes: 75.0,
        explicit_save_count: 4,
        topic_count: 7,
        decision_keywords_found: 10,
        emotional_keywords_found: 5,
    };
    assert_eq!(extraction::assess_engagement(&high), ExtractionDepth::Deep);
    assert_eq!(ExtractionDepth::Deep.max_proposals(), 15);

    // Deep should propose more than minimal
    assert!(ExtractionDepth::Deep.max_proposals() > ExtractionDepth::Minimal.max_proposals());
}

// ============================================================
// Ghost node weight learning
// ============================================================

#[test]
fn validation_p2_ghost_weight_learns_from_access() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let node = db.get_ghost_node_by_ref("v-vault", "design-philosophy.md").unwrap();
    let initial_weight = node.weight;
    let sources = db.list_ghost_sources().unwrap();

    // Pull multiple times
    for _ in 0..5 {
        let n = db.get_ghost_node_by_ref("v-vault", "design-philosophy.md").unwrap();
        pull_ghost_content(&db, &n, &sources[0].root_path, PullMode::SessionOnly).unwrap();
    }

    let node_after = db.get_ghost_node_by_ref("v-vault", "design-philosophy.md").unwrap();
    assert!(
        node_after.weight > initial_weight,
        "Ghost node weight should increase with repeated access: {} > {}",
        node_after.weight,
        initial_weight
    );
}

// ============================================================
// End-to-end: ghost + memory graph integrated retrieval
// ============================================================

#[test]
fn validation_p2_integrated_retrieval() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    // Add memory graph impulses
    ingestion::save_and_confirm(
        &db,
        "Spreading activation mimics human memory recall patterns",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    ).unwrap();

    ingestion::save_and_confirm(
        &db,
        "SQLite is excellent for portable local-first storage",
        ImpulseType::Decision,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    ).unwrap();

    // Query that matches memory graph impulses (via FTS on content)
    let engine = ActivationEngine::new(&db);
    let result_memories = engine.retrieve(&RetrievalRequest {
        query: "SQLite portable storage".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();

    // Should get results from memory graph
    assert!(!result_memories.memories.is_empty(), "Should find memory graph impulses matching SQLite");

    // Query that matches ghost node titles (via FTS on ghost titles)
    let result_ghost = engine.retrieve(&RetrievalRequest {
        query: "architecture".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();

    // Should get ghost activations from the vault
    assert!(
        !result_ghost.ghost_activations.is_empty(),
        "Should activate ghost nodes for matching vault titles"
    );
}
