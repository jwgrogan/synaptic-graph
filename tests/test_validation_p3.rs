mod common;

use std::fs;
use synaptic_graph::activation::ActivationEngine;
use synaptic_graph::backup;
use synaptic_graph::db::Database;
use synaptic_graph::ghost;
use synaptic_graph::ghost::scanner::ScanConfig;
use synaptic_graph::ingestion;
use synaptic_graph::models::*;
use synaptic_graph::sync;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a file-backed database inside the given TempDir.
fn file_db(dir: &TempDir, name: &str) -> Database {
    let path = dir.path().join(name);
    Database::open(path.to_str().unwrap()).expect("Failed to create file-backed database")
}

/// Return the on-disk path for a database created via `file_db`.
fn db_path(dir: &TempDir, name: &str) -> String {
    dir.path().join(name).to_string_lossy().to_string()
}

/// Create a simulated Obsidian vault with markdown files.
fn create_obsidian_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(
        root.join("daily-notes.md"),
        "# Daily Notes\n\nCapture fleeting thoughts throughout the day.\nSee [[project-plan]] for context.\n",
    ).unwrap();

    fs::write(
        root.join("project-plan.md"),
        "# Project Plan\n\nBuild a local-first memory graph with spreading activation.\nSee [[daily-notes]] for observations.\n",
    ).unwrap();

    fs::write(
        root.join("reading-list.md"),
        "# Reading List\n\nPapers on associative memory and graph databases.\n",
    )
    .unwrap();

    dir
}

/// Create a simulated code-docs vault with markdown files.
fn create_code_docs_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(
        root.join("api-reference.md"),
        "# API Reference\n\nREST endpoints for the memory graph service.\n",
    )
    .unwrap();

    fs::write(
        root.join("architecture-decisions.md"),
        "# Architecture Decisions\n\nSQLite chosen for portable local-first storage.\n",
    )
    .unwrap();

    dir
}

/// Default scan config for markdown files.
fn md_scan_config() -> ScanConfig {
    ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    }
}

/// Insert a confirmed impulse with a connection to an existing impulse.
fn insert_connected_impulse(db: &Database, content: &str, target_id: &str) -> Impulse {
    ingestion::save_and_confirm_with_connections(
        db,
        content,
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
        &[(target_id.to_string(), "relates_to".to_string(), 0.8)],
    )
    .unwrap()
}

// ============================================================
// Test 1: Cross-device consistency via sync
// ============================================================

#[test]
fn validation_p3_cross_device_consistency() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");

    // --- Device A: create data and export ---
    let db_a = file_db(&tmp, "device-a.db");

    let imp1 = ingestion::save_and_confirm(
        &db_a,
        "Spreading activation is inspired by neural associative memory",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "device-a",
    )
    .unwrap();

    let _imp2 = ingestion::save_and_confirm(
        &db_a,
        "SQLite FTS5 provides fast full-text search for seed matching",
        ImpulseType::Decision,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "device-a",
    )
    .unwrap();

    let _imp3 = insert_connected_impulse(
        &db_a,
        "Weighted decay models forgetting curves from cognitive science",
        &imp1.id,
    );

    let count_a = db_a.impulse_count().unwrap();
    assert_eq!(count_a, 3);

    // Export Device A snapshot to sync directory
    let export_result =
        sync::export_snapshot(&db_a, sync_dir.to_str().unwrap(), "device-a").unwrap();

    // --- Device B: fresh database, import from sync ---
    let db_b = file_db(&tmp, "device-b.db");
    let count_b_before = db_b.impulse_count().unwrap();
    assert_eq!(count_b_before, 0);

    let merge_result = sync::import_snapshot(
        &export_result.snapshot_path,
        &db_path(&tmp, "device-b.db"),
        &export_result.checksum,
    )
    .unwrap();

    assert_eq!(
        merge_result.inserted, 3,
        "All 3 impulses should be inserted into Device B"
    );
    assert_eq!(merge_result.skipped, 0);

    // Re-open Device B and verify content
    let db_b = Database::open(&db_path(&tmp, "device-b.db")).unwrap();
    let count_b_after = db_b.impulse_count().unwrap();
    assert_eq!(
        count_b_after, 3,
        "Device B should have same impulse count as Device A"
    );

    // Verify specific content survived the transfer
    let imp_b = db_b.get_impulse(&imp1.id).unwrap();
    assert_eq!(
        imp_b.content, "Spreading activation is inspired by neural associative memory",
        "Content should be identical after sync"
    );
}

// ============================================================
// Test 2: Backup and restore full integrity
// ============================================================

#[test]
fn validation_p3_backup_restore_full_integrity() {
    let tmp = TempDir::new().unwrap();

    // Create a rich graph
    let db = file_db(&tmp, "rich.db");

    // Impulses with connections
    let imp_a = ingestion::save_and_confirm(
        &db,
        "Memory portability requires a universal graph format",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    )
    .unwrap();

    let imp_b = insert_connected_impulse(
        &db,
        "Graph serialization via SQLite VACUUM INTO for consistent snapshots",
        &imp_a.id,
    );

    let _imp_c = insert_connected_impulse(
        &db,
        "Checksum verification ensures backup integrity across devices",
        &imp_b.id,
    );

    // Ghost source and nodes
    let vault = create_obsidian_vault();
    ghost::register_and_scan(
        &db,
        "obsidian-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &md_scan_config(),
    )
    .unwrap();

    // Record counts before backup
    let impulse_count_before = db.impulse_count().unwrap();
    let connection_count_before = db.connection_count().unwrap();
    let ghost_sources_before = db.list_ghost_sources().unwrap();
    let ghost_nodes_before = db.list_ghost_nodes_by_source("obsidian-vault").unwrap();

    assert_eq!(impulse_count_before, 3);
    assert!(
        connection_count_before >= 2,
        "Should have at least 2 connections"
    );
    assert_eq!(ghost_sources_before.len(), 1);
    assert_eq!(ghost_nodes_before.len(), 3, "Vault has 3 markdown files");

    // Create backup
    let backup_path = tmp.path().join("backup.db");
    let backup_result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    assert_eq!(backup_result.impulse_count, impulse_count_before);
    assert_eq!(backup_result.connection_count, connection_count_before);

    // Restore to a new location
    let restore_path = tmp.path().join("restored.db");
    backup::restore_backup(
        backup_path.to_str().unwrap(),
        restore_path.to_str().unwrap(),
        &backup_result.checksum,
    )
    .unwrap();

    // Open restored database and verify everything
    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();

    let impulse_count_after = restored_db.impulse_count().unwrap();
    let connection_count_after = restored_db.connection_count().unwrap();
    let ghost_sources_after = restored_db.list_ghost_sources().unwrap();
    let ghost_nodes_after = restored_db
        .list_ghost_nodes_by_source("obsidian-vault")
        .unwrap();

    assert_eq!(
        impulse_count_after, impulse_count_before,
        "Impulse count must match after restore"
    );
    assert_eq!(
        connection_count_after, connection_count_before,
        "Connection count must match after restore"
    );
    assert_eq!(
        ghost_sources_after.len(),
        ghost_sources_before.len(),
        "Ghost source count must match after restore"
    );
    assert_eq!(
        ghost_nodes_after.len(),
        ghost_nodes_before.len(),
        "Ghost node count must match after restore"
    );

    // Verify specific content survived
    let restored_imp = restored_db.get_impulse(&imp_a.id).unwrap();
    assert_eq!(
        restored_imp.content,
        "Memory portability requires a universal graph format"
    );

    // Verify ghost source metadata survived
    assert_eq!(ghost_sources_after[0].name, "obsidian-vault");
    assert_eq!(ghost_sources_after[0].source_type, "obsidian");

    // Verify a specific ghost node survived
    let daily_node = ghost_nodes_after
        .iter()
        .find(|n| n.title == "Daily Notes")
        .expect("Daily Notes ghost node should survive restore");
    assert_eq!(daily_node.source_graph, "obsidian-vault");
}

// ============================================================
// Test 3: Cross-source activation from multiple ghost sources
// ============================================================

#[test]
fn validation_p3_cross_source_activation() {
    let tmp = TempDir::new().unwrap();
    let db = file_db(&tmp, "cross-source.db");

    // Register TWO separate ghost graph sources
    let obsidian_vault = create_obsidian_vault();
    let code_docs_vault = create_code_docs_vault();

    ghost::register_and_scan(
        &db,
        "obsidian",
        obsidian_vault.path().to_str().unwrap(),
        "obsidian",
        &md_scan_config(),
    )
    .unwrap();

    ghost::register_and_scan(
        &db,
        "code-docs",
        code_docs_vault.path().to_str().unwrap(),
        "docs",
        &md_scan_config(),
    )
    .unwrap();

    // Verify both sources are registered
    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 2, "Should have 2 ghost sources registered");

    let obsidian_nodes = db.list_ghost_nodes_by_source("obsidian").unwrap();
    let code_docs_nodes = db.list_ghost_nodes_by_source("code-docs").unwrap();
    assert_eq!(obsidian_nodes.len(), 3, "Obsidian vault has 3 files");
    assert_eq!(code_docs_nodes.len(), 2, "Code-docs vault has 2 files");

    // Add memory graph impulses that relate to content in both vaults
    ingestion::save_and_confirm(
        &db,
        "Local-first architecture means the memory graph runs entirely on the user device",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    )
    .unwrap();

    ingestion::save_and_confirm(
        &db,
        "The project plan calls for portable SQLite-based storage with REST API",
        ImpulseType::Decision,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    // Query for memory graph impulses
    // Note: avoid hyphens in FTS5 queries as they are treated as column operators
    let engine = ActivationEngine::new(&db);
    let result_memories = engine
        .retrieve(&RetrievalRequest {
            query: "project plan portable SQLite".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();

    // Should have memory results
    assert!(
        !result_memories.memories.is_empty(),
        "Should find memory graph impulses matching the query"
    );

    // Query for ghost activations (matching ghost node titles)
    let result_ghost = engine
        .retrieve(&RetrievalRequest {
            query: "architecture decisions".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();

    // Should have ghost activations from the code-docs vault
    assert!(
        !result_ghost.ghost_activations.is_empty(),
        "Should have ghost activations from vault sources"
    );
}

// ============================================================
// Test 4: Sync preserves local-first operation
// ============================================================

#[test]
fn validation_p3_sync_preserves_local_first() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");

    // Create local database with data
    let db = file_db(&tmp, "local.db");

    let imp1 = ingestion::save_and_confirm(
        &db,
        "Local-first means the app works fully offline without any sync dependency",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "local",
    )
    .unwrap();

    let _imp2 = insert_connected_impulse(
        &db,
        "Sync is additive: it enhances but never gates core functionality",
        &imp1.id,
    );

    // Export a snapshot (simulating a sync action)
    let _export_result =
        sync::export_snapshot(&db, sync_dir.to_str().unwrap(), "local-device").unwrap();

    // After exporting, create MORE local data (local operations must not depend on sync)
    let imp3 = ingestion::save_and_confirm(
        &db,
        "New insight created after sync export, purely local",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "local",
    )
    .unwrap();

    let _imp4 = insert_connected_impulse(
        &db,
        "Another local thought: memory retrieval should never block on network",
        &imp3.id,
    );

    // Verify all local operations still work correctly
    let total_count = db.impulse_count().unwrap();
    assert_eq!(total_count, 4, "All 4 local impulses should exist");

    // Retrieval still works on all data including post-sync-export data
    let engine = ActivationEngine::new(&db);

    // Note: avoid hyphens in FTS5 queries as they are treated as column operators
    let result_old = engine
        .retrieve(&RetrievalRequest {
            query: "offline fully works".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();
    assert!(
        !result_old.memories.is_empty(),
        "Retrieval of pre-export data should still work"
    );

    let result_new = engine
        .retrieve(&RetrievalRequest {
            query: "insight created after sync".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();
    assert!(
        !result_new.memories.is_empty(),
        "Retrieval of post-export data should work without depending on sync"
    );

    // Verify the exported snapshot does NOT contain the post-export data
    // (the snapshot is a point-in-time capture)
    let manifest = sync::read_manifest(sync_dir.to_str().unwrap()).unwrap();
    let entry = &manifest.devices["local-device"];
    assert_eq!(
        entry.impulse_count, 2,
        "Snapshot should only contain the 2 impulses that existed at export time"
    );
}
