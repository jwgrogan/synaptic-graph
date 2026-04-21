use rusqlite::Connection;
use synaptic_graph::db::Database;
use synaptic_graph::graph::CURRENT_SCHEMA_VERSION;
use synaptic_graph::models::*;
use synaptic_graph::sync;
use tempfile::TempDir;

fn create_test_impulse(db: &Database, content: &str) -> String {
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
    let impulse = db.insert_impulse(&input).unwrap();
    impulse.id
}

#[test]
fn test_sync_export_creates_snapshot() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("local.db");
    let sync_dir = tmp.path().join("sync");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    create_test_impulse(&db, "Test impulse for export");

    let result = sync::export_snapshot(&db, sync_dir.to_str().unwrap(), "device-a").unwrap();

    // Snapshot file should exist
    assert!(std::path::Path::new(&result.snapshot_path).exists());
    assert!(!result.checksum.is_empty());

    // Snapshot filename should follow the convention
    assert!(result.snapshot_path.contains("memory-graph-device-a.db"));

    // Manifest should exist and contain the device
    let manifest = sync::read_manifest(sync_dir.to_str().unwrap()).unwrap();
    assert!(manifest.devices.contains_key("device-a"));
    let entry = &manifest.devices["device-a"];
    assert_eq!(entry.device_id, "device-a");
    assert_eq!(entry.impulse_count, 1);
    assert_eq!(entry.checksum, result.checksum);
    assert_eq!(entry.schema_version, CURRENT_SCHEMA_VERSION);
    assert!(entry
        .feature_flags
        .iter()
        .any(|flag| flag == "canonical_graph"));
    assert_eq!(result.schema_version, CURRENT_SCHEMA_VERSION);
}

#[test]
fn test_sync_detects_newer_remote_snapshot() {
    let tmp = TempDir::new().unwrap();
    let db_path_a = tmp.path().join("local-a.db");
    let db_path_b = tmp.path().join("local-b.db");
    let sync_dir = tmp.path().join("sync");

    // Device A exports first
    let db_a = Database::open(db_path_a.to_str().unwrap()).unwrap();
    create_test_impulse(&db_a, "Impulse from device A");
    sync::export_snapshot(&db_a, sync_dir.to_str().unwrap(), "device-a").unwrap();

    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Device B exports after
    let db_b = Database::open(db_path_b.to_str().unwrap()).unwrap();
    create_test_impulse(&db_b, "Impulse from device B");
    sync::export_snapshot(&db_b, sync_dir.to_str().unwrap(), "device-b").unwrap();

    // Check from device A's perspective
    let status = sync::check_sync_status(sync_dir.to_str().unwrap(), "device-a").unwrap();

    assert!(status.has_remote_updates);
    assert_eq!(status.remote_devices.len(), 1);
    assert_eq!(status.remote_devices[0], "device-b");
    assert_eq!(status.latest_remote_device, Some("device-b".to_string()));
    assert!(status.latest_remote_time.is_some());
}

#[test]
fn test_sync_import_remote_snapshot() {
    let tmp = TempDir::new().unwrap();
    let db_path_local = tmp.path().join("local.db");
    let db_path_remote = tmp.path().join("remote.db");
    let sync_dir = tmp.path().join("sync");

    // Create local DB with one impulse
    let local_db = Database::open(db_path_local.to_str().unwrap()).unwrap();
    let local_id = create_test_impulse(&local_db, "Local impulse");

    // Create remote DB with a different impulse
    let remote_db = Database::open(db_path_remote.to_str().unwrap()).unwrap();
    let _remote_id = create_test_impulse(&remote_db, "Remote impulse");

    // Also insert the same ID into both databases (simulating shared history).
    // Insert into remote first so it has an older last_accessed_at,
    // then insert into local later so local is newer -> should be skipped on merge.
    let shared_input = NewImpulse {
        content: "Shared impulse".to_string(),
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
    let shared_id = "shared-impulse-id-123";

    // Remote gets the shared impulse first (older timestamp)
    remote_db
        .insert_impulse_with_id(shared_id, &shared_input)
        .unwrap();

    // Small delay so local timestamp is definitively newer
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Local gets the shared impulse after (newer timestamp)
    local_db
        .insert_impulse_with_id(shared_id, &shared_input)
        .unwrap();

    // Export remote snapshot
    let export_result =
        sync::export_snapshot(&remote_db, sync_dir.to_str().unwrap(), "device-remote").unwrap();

    // Import the remote snapshot into the local DB -- this is ID-based merge
    let merge_result = sync::import_snapshot(
        &export_result.snapshot_path,
        db_path_local.to_str().unwrap(),
        &export_result.checksum,
    )
    .unwrap();

    // The remote-only impulse should be inserted
    assert_eq!(
        merge_result.inserted, 1,
        "Should insert 1 new remote impulse"
    );
    // The shared impulse should be skipped (local is newer)
    assert_eq!(
        merge_result.skipped, 1,
        "Should skip 1 shared impulse where local is newer"
    );

    // Re-open local DB and verify merge result
    let local_db = Database::open(db_path_local.to_str().unwrap()).unwrap();
    let all_impulses = local_db.list_impulses(None).unwrap();

    // Should have: local_id + shared_id + the remote-only impulse = 3
    assert_eq!(
        all_impulses.len(),
        3,
        "Local DB should have 3 impulses after merge"
    );

    // The original local impulse should still be there
    let local_impulse = local_db.get_impulse(&local_id).unwrap();
    assert_eq!(local_impulse.content, "Local impulse");
}

#[test]
fn test_sync_import_preserves_canonical_skill_assessment_and_feedback_state() {
    let tmp = TempDir::new().unwrap();
    let db_path_local = tmp.path().join("local.db");
    let db_path_remote = tmp.path().join("remote.db");
    let sync_dir = tmp.path().join("sync");

    let local_db = Database::open(db_path_local.to_str().unwrap()).unwrap();
    create_test_impulse(&local_db, "Local baseline");

    let remote_db = Database::open(db_path_remote.to_str().unwrap()).unwrap();
    let primary_id = create_test_impulse(&remote_db, "Remote canonical memory");
    remote_db.confirm_impulse(&primary_id).unwrap();
    remote_db
        .apply_feedback_to_node(&primary_id, FeedbackKind::Helpful)
        .unwrap();

    let peer_id = create_test_impulse(&remote_db, "Remote contradiction peer");
    remote_db.confirm_impulse(&peer_id).unwrap();

    let evidence = remote_db
        .create_evidence_set(
            "remote canonical query",
            "remote-stable-hash",
            std::slice::from_ref(&primary_id),
            &[],
            Some(24),
        )
        .unwrap();

    let skill = remote_db
        .create_skill(
            "Remote synced skill",
            "Skill persisted through canonical sync",
            "When importing canonical snapshots",
            &["Verify skill survives import".to_string()],
            &[],
            &evidence.id,
            std::slice::from_ref(&primary_id),
            "unknown",
            "",
        )
        .unwrap();

    let assessment = remote_db
        .upsert_pair_assessment(
            AssessmentType::Contradiction,
            &primary_id,
            &peer_id,
            0.91,
            "remote contradiction",
        )
        .unwrap();
    remote_db
        .set_assessment_status(&assessment.id, AssessmentStatus::Confirmed)
        .unwrap();

    let export_result =
        sync::export_snapshot(&remote_db, sync_dir.to_str().unwrap(), "device-remote").unwrap();
    let merge_result = sync::import_snapshot(
        &export_result.snapshot_path,
        db_path_local.to_str().unwrap(),
        &export_result.checksum,
    )
    .unwrap();

    assert!(merge_result.inserted >= 2);

    let local_db = Database::open(db_path_local.to_str().unwrap()).unwrap();
    let canonical = local_db.get_canonical_node(&primary_id).unwrap();
    assert_eq!(canonical.helpful_count, 1);

    let local_skill = local_db.get_skill(&skill.node_id).unwrap();
    assert_eq!(local_skill.name, "Remote synced skill");
    assert_eq!(
        local_db
            .get_skill_evidence_node_ids(&skill.node_id)
            .unwrap(),
        vec![primary_id.clone()]
    );

    let assessments = local_db
        .list_assessments(
            Some(AssessmentType::Contradiction),
            Some(AssessmentStatus::Confirmed),
            Some(&primary_id),
        )
        .unwrap();
    assert_eq!(assessments.len(), 1);
    assert_eq!(assessments[0].rationale, "remote contradiction");
}

#[test]
fn test_sync_manifest_tracks_devices() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");

    let db_path_a = tmp.path().join("a.db");
    let db_path_b = tmp.path().join("b.db");
    let db_path_c = tmp.path().join("c.db");

    // Three devices export snapshots
    let db_a = Database::open(db_path_a.to_str().unwrap()).unwrap();
    create_test_impulse(&db_a, "A1");
    create_test_impulse(&db_a, "A2");
    sync::export_snapshot(&db_a, sync_dir.to_str().unwrap(), "laptop").unwrap();

    let db_b = Database::open(db_path_b.to_str().unwrap()).unwrap();
    create_test_impulse(&db_b, "B1");
    sync::export_snapshot(&db_b, sync_dir.to_str().unwrap(), "desktop").unwrap();

    let db_c = Database::open(db_path_c.to_str().unwrap()).unwrap();
    create_test_impulse(&db_c, "C1");
    create_test_impulse(&db_c, "C2");
    create_test_impulse(&db_c, "C3");
    sync::export_snapshot(&db_c, sync_dir.to_str().unwrap(), "tablet").unwrap();

    // Read manifest and verify all devices are tracked
    let manifest = sync::read_manifest(sync_dir.to_str().unwrap()).unwrap();

    assert_eq!(manifest.devices.len(), 3);
    assert!(manifest.devices.contains_key("laptop"));
    assert!(manifest.devices.contains_key("desktop"));
    assert!(manifest.devices.contains_key("tablet"));

    // Verify impulse counts
    assert_eq!(manifest.devices["laptop"].impulse_count, 2);
    assert_eq!(manifest.devices["desktop"].impulse_count, 1);
    assert_eq!(manifest.devices["tablet"].impulse_count, 3);

    // Verify snapshot filenames
    assert_eq!(
        manifest.devices["laptop"].snapshot_filename,
        "memory-graph-laptop.db"
    );
    assert_eq!(
        manifest.devices["desktop"].snapshot_filename,
        "memory-graph-desktop.db"
    );
    assert_eq!(
        manifest.devices["tablet"].snapshot_filename,
        "memory-graph-tablet.db"
    );
}

#[test]
fn test_external_schema_check_rejects_future_schema_version() {
    let tmp = TempDir::new().unwrap();
    let db_path_local = tmp.path().join("local.db");
    let db_path_remote = tmp.path().join("remote.db");
    let sync_dir = tmp.path().join("sync");

    let local_db = Database::open(db_path_local.to_str().unwrap()).unwrap();
    create_test_impulse(&local_db, "Local impulse");

    let remote_db = Database::open(db_path_remote.to_str().unwrap()).unwrap();
    create_test_impulse(&remote_db, "Remote impulse");

    let export_result =
        sync::export_snapshot(&remote_db, sync_dir.to_str().unwrap(), "device-remote").unwrap();

    let raw = Connection::open(&export_result.snapshot_path).unwrap();
    raw.execute("DELETE FROM schema_version", []).unwrap();
    raw.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        [CURRENT_SCHEMA_VERSION + 1],
    )
    .unwrap();
    drop(raw);

    let err =
        Database::require_compatible_external_schema(&export_result.snapshot_path).unwrap_err();

    assert!(err.contains("incompatible"));
    assert!(err.contains(&(CURRENT_SCHEMA_VERSION + 1).to_string()));
}
