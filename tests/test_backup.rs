mod common;

use synaptic_graph::backup::{create_backup, restore_backup, verify_backup};
use synaptic_graph::db::Database;
use synaptic_graph::models::*;
use tempfile::TempDir;

/// Helper: create a file-backed database in a temp directory
fn file_db(dir: &TempDir, name: &str) -> Database {
    let path = dir.path().join(name);
    Database::open(path.to_str().unwrap()).expect("Failed to create file-backed database")
}

/// Helper: insert a sample impulse
fn insert_sample_impulse(db: &Database, content: &str) -> Impulse {
    let input = NewImpulse {
        content: content.to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: WEIGHT_EXPLICIT_SAVE,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
        source_provider: "unknown".to_string(),
        source_account: String::new(),
    };
    db.insert_impulse(&input).unwrap()
}

/// Helper: insert a connection between two impulses
fn insert_sample_connection(db: &Database, source_id: &str, target_id: &str) {
    let input = NewConnection {
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        weight: 0.8,
        relationship: "relates_to".to_string(),
    };
    db.insert_connection(&input).unwrap();
}

#[test]
fn test_backup_creates_file() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");
    insert_sample_impulse(&db, "test impulse");

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    assert!(std::path::Path::new(&result.path).exists());
    assert!(result.size_bytes > 0);
}

#[test]
fn test_backup_includes_checksum() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");
    insert_sample_impulse(&db, "checksum test");

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    // SHA-256 hex is 64 characters
    assert_eq!(result.checksum.len(), 64);
    // Should be hex
    assert!(result.checksum.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(result.impulse_count, 1);
    assert_eq!(result.connection_count, 0);
}

#[test]
fn test_verify_backup_integrity() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");
    insert_sample_impulse(&db, "verify test");

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    let valid = verify_backup(backup_path.to_str().unwrap(), &result.checksum).unwrap();
    assert!(valid);
}

#[test]
fn test_corrupted_backup_fails_verification() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");
    insert_sample_impulse(&db, "corrupt test");

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    // Corrupt the file by appending bytes
    std::fs::write(
        backup_path.to_str().unwrap(),
        b"corrupted data that is definitely not the original file",
    )
    .unwrap();

    let valid = verify_backup(backup_path.to_str().unwrap(), &result.checksum).unwrap();
    assert!(!valid);
}

#[test]
fn test_restore_from_backup() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");
    let impulse = insert_sample_impulse(&db, "restore test impulse");
    db.confirm_impulse(&impulse.id).unwrap();

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    let restore_path = dir.path().join("restored.db");
    restore_backup(
        backup_path.to_str().unwrap(),
        restore_path.to_str().unwrap(),
        &result.checksum,
    )
    .unwrap();

    // Open restored DB and verify data
    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();
    let restored_impulse = restored_db.get_impulse(&impulse.id).unwrap();
    assert_eq!(restored_impulse.content, "restore test impulse");
    assert_eq!(restored_impulse.status, ImpulseStatus::Confirmed);
}

#[test]
fn test_restore_preserves_connections() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");
    let imp1 = insert_sample_impulse(&db, "node A");
    let imp2 = insert_sample_impulse(&db, "node B");
    insert_sample_connection(&db, &imp1.id, &imp2.id);

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    assert_eq!(result.connection_count, 1);

    let restore_path = dir.path().join("restored.db");
    restore_backup(
        backup_path.to_str().unwrap(),
        restore_path.to_str().unwrap(),
        &result.checksum,
    )
    .unwrap();

    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();
    let connections = restored_db.get_connections_for_node(&imp1.id).unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].source_id, imp1.id);
    assert_eq!(connections[0].target_id, imp2.id);
}

#[test]
fn test_restore_preserves_ghost_nodes() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "source.db");

    // Insert a ghost node
    let ghost_input = NewGhostNode {
        source_graph: "external-graph".to_string(),
        external_ref: "ext-001".to_string(),
        title: "Ghost reference node".to_string(),
        metadata: serde_json::json!({"key": "value"}),
        initial_weight: 0.5,
    };
    let ghost = db.insert_ghost_node(&ghost_input).unwrap();

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    let restore_path = dir.path().join("restored.db");
    restore_backup(
        backup_path.to_str().unwrap(),
        restore_path.to_str().unwrap(),
        &result.checksum,
    )
    .unwrap();

    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();
    let restored_ghost = restored_db.get_ghost_node(&ghost.id).unwrap();
    assert_eq!(restored_ghost.title, "Ghost reference node");
    assert_eq!(restored_ghost.source_graph, "external-graph");
}

#[test]
fn test_backup_empty_database() {
    let dir = TempDir::new().unwrap();
    let db = file_db(&dir, "empty.db");

    let backup_path = dir.path().join("backup.db");
    let result = create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    assert_eq!(result.impulse_count, 0);
    assert_eq!(result.connection_count, 0);
    assert!(result.size_bytes > 0); // Even empty DB has schema
    assert!(std::path::Path::new(&result.path).exists());
}

#[test]
fn test_restore_nonexistent_backup() {
    let dir = TempDir::new().unwrap();
    let backup_path = dir.path().join("does_not_exist.db");
    let restore_path = dir.path().join("restored.db");

    let result = restore_backup(
        backup_path.to_str().unwrap(),
        restore_path.to_str().unwrap(),
        "0000000000000000000000000000000000000000000000000000000000000000",
    );

    assert!(result.is_err());
}
