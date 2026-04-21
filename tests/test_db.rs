mod common;

use serde_json::json;
use synaptic_graph::db::Database;
use synaptic_graph::graph::{GraphNodeKind, CURRENT_SCHEMA_VERSION};
use synaptic_graph::models::*;
use tempfile::TempDir;

#[test]
fn test_database_creates_tables() {
    let db = common::test_db();
    // Should be able to query the impulses table without error
    let count = db.impulse_count().unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_database_creates_fts_index() {
    let db = common::test_db();
    // FTS5 table should exist
    let count = db.fts_impulse_count().unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_insert_and_get_impulse() {
    let db = common::test_db();
    let input = NewImpulse {
        content: "Auth middleware silently drops tokens under concurrent writes".to_string(),
        impulse_type: ImpulseType::Heuristic,
        initial_weight: WEIGHT_EXPLICIT_SAVE,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::High,
        source_signals: vec!["long-form response".to_string()],
        source_type: SourceType::ExplicitSave,
        source_ref: "session-001".to_string(),
        source_provider: "unknown".to_string(),
        source_account: String::new(),
    };

    let impulse = db.insert_impulse(&input).unwrap();
    assert_eq!(impulse.content, input.content);
    assert_eq!(impulse.weight, WEIGHT_EXPLICIT_SAVE);
    assert_eq!(impulse.status, ImpulseStatus::Candidate);

    // Confirm it
    db.confirm_impulse(&impulse.id).unwrap();
    let confirmed = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(confirmed.status, ImpulseStatus::Confirmed);

    let retrieved = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(retrieved.id, impulse.id);
    assert_eq!(retrieved.content, impulse.content);
}

#[test]
fn test_insert_and_get_connection() {
    let db = common::test_db();
    let a = db
        .insert_impulse(&NewImpulse {
            content: "Impulse A".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let b = db
        .insert_impulse(&NewImpulse {
            content: "Impulse B".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let conn_input = NewConnection {
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        weight: 0.8,
        relationship: "relates_to".to_string(),
    };

    let conn = db.insert_connection(&conn_input).unwrap();
    assert_eq!(conn.source_id, a.id);
    assert_eq!(conn.target_id, b.id);
    assert_eq!(conn.weight, 0.8);
    assert_eq!(conn.traversal_count, 0);

    let conns = db.get_connections_for_node(&a.id).unwrap();
    assert_eq!(conns.len(), 1);
}

#[test]
fn test_update_impulse_creates_supersession() {
    let db = common::test_db();
    let original = db
        .insert_impulse(&NewImpulse {
            content: "Original content".to_string(),
            impulse_type: ImpulseType::Decision,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let new_id = db
        .update_impulse_content(&original.id, "Updated content")
        .unwrap();

    // Old impulse should be superseded
    let old = db.get_impulse(&original.id).unwrap();
    assert_eq!(old.status, ImpulseStatus::Superseded);

    // New impulse should exist with updated content (created as candidate via insert_impulse)
    let new = db.get_impulse(&new_id).unwrap();
    assert_eq!(new.content, "Updated content");
    assert_eq!(new.status, ImpulseStatus::Candidate);

    // Supersession connection should exist
    let conns = db.get_connections_for_node(&new_id).unwrap();
    assert_eq!(conns.len(), 1);
    assert_eq!(conns[0].relationship, "supersedes");
}

#[test]
fn test_soft_delete() {
    let db = common::test_db();
    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "To be deleted".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Low,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    db.update_impulse_status(&impulse.id, ImpulseStatus::Deleted)
        .unwrap();

    let deleted = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(deleted.status, ImpulseStatus::Deleted);

    // Should still be retrievable (soft delete)
    assert_eq!(deleted.content, "To be deleted");
}

#[test]
fn test_fts_search() {
    let db = common::test_db();
    let rust_impulse = db
        .insert_impulse(&NewImpulse {
            content: "Rust is great for systems programming".to_string(),
            impulse_type: ImpulseType::Preference,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let python_impulse = db
        .insert_impulse(&NewImpulse {
            content: "Python is slow but good for prototyping".to_string(),
            impulse_type: ImpulseType::Preference,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    // Confirm both impulses so FTS search can find them
    db.confirm_impulse(&rust_impulse.id).unwrap();
    db.confirm_impulse(&python_impulse.id).unwrap();

    let results = db.search_impulses_fts("rust systems").unwrap();
    assert_eq!(results.len(), 1);

    let results = db.search_impulses_fts("programming").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_fts_search_excludes_non_confirmed() {
    let db = common::test_db();
    // Insert an impulse but don't confirm it
    let _impulse = db
        .insert_impulse(&NewImpulse {
            content: "Unconfirmed candidate impulse about Rust".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    // FTS search should return nothing since impulse is still a candidate
    let results = db.search_impulses_fts("Rust").unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_memory_stats() {
    let db = common::test_db();
    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.total_impulses, 0);
    assert_eq!(stats.total_memory_nodes, 0);
    assert_eq!(stats.total_connections, 0);
    assert_eq!(stats.total_graph_edges, 0);

    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "Test".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.total_impulses, 1);
    assert_eq!(stats.total_memory_nodes, 1);
    assert_eq!(stats.candidate_impulses, 1);
    assert_eq!(stats.confirmed_impulses, 0);

    // Confirm it and check stats again
    db.confirm_impulse(&impulse.id).unwrap();
    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.confirmed_impulses, 1);
    assert_eq!(stats.candidate_impulses, 0);
    assert_eq!(stats.total_skill_nodes, 0);
    assert_eq!(stats.total_ghost_nodes, 0);
}

#[test]
fn test_schema_version_and_canonical_graph_are_initialized() {
    let db = common::test_db();

    let schema = db.schema_info().unwrap();
    assert_eq!(schema.version, CURRENT_SCHEMA_VERSION);
    assert!(schema
        .feature_flags
        .iter()
        .any(|flag| flag == "canonical_graph"));

    assert_eq!(db.canonical_node_count().unwrap(), 0);
    assert_eq!(db.canonical_edge_count().unwrap(), 0);
}

#[test]
fn test_canonical_graph_tracks_impulse_and_connection_changes() {
    let db = common::test_db();
    let a = db
        .insert_impulse(&NewImpulse {
            content: "A typed graph kernel needs one canonical node model".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::High,
            source_signals: vec!["design review".to_string()],
            source_type: SourceType::ExplicitSave,
            source_ref: "schema-test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();
    db.confirm_impulse(&a.id).unwrap();

    let b = db
        .insert_impulse(&NewImpulse {
            content: "Migration first, reflection later".to_string(),
            impulse_type: ImpulseType::Pattern,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec!["planning".to_string()],
            source_type: SourceType::ExplicitSave,
            source_ref: "schema-test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let conn = db
        .insert_connection(&NewConnection {
            source_id: a.id.clone(),
            target_id: b.id.clone(),
            weight: 0.8,
            relationship: "supports".to_string(),
        })
        .unwrap();

    let canonical_a = db.get_canonical_node(&a.id).unwrap();
    assert_eq!(canonical_a.kind, GraphNodeKind::Memory);
    assert_eq!(canonical_a.status, "confirmed");

    let payload_a = db.get_canonical_memory_payload(&a.id).unwrap();
    assert_eq!(
        payload_a.content,
        "A typed graph kernel needs one canonical node model"
    );
    assert_eq!(payload_a.source_signals, vec!["design review".to_string()]);

    let canonical_conn = db.get_canonical_edge(&conn.id).unwrap();
    assert_eq!(canonical_conn.relationship, "supports");
    assert_eq!(canonical_conn.weight, 0.8);

    db.update_impulse_weight(&a.id, 0.9).unwrap();
    db.touch_connection(&conn.id).unwrap();

    let canonical_a = db.get_canonical_node(&a.id).unwrap();
    assert_eq!(canonical_a.weight, 0.9);
    let canonical_conn = db.get_canonical_edge(&conn.id).unwrap();
    assert_eq!(canonical_conn.traversal_count, 1);
}

#[test]
fn test_list_candidates() {
    let db = common::test_db();
    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "A candidate".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let candidates = db.list_candidates().unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, impulse.id);

    // Confirm it and check candidates is empty
    db.confirm_impulse(&impulse.id).unwrap();
    let candidates = db.list_candidates().unwrap();
    assert_eq!(candidates.len(), 0);
}

#[test]
fn test_dismiss_impulse() {
    let db = common::test_db();
    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "To be dismissed".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    db.dismiss_impulse(&impulse.id).unwrap();
    let dismissed = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(dismissed.status, ImpulseStatus::Deleted);
}

#[test]
fn test_touch_impulse() {
    let db = common::test_db();
    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "Touch test".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let original_accessed = impulse.last_accessed_at;
    // Small delay to ensure timestamp changes
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.touch_impulse(&impulse.id).unwrap();
    let touched = db.get_impulse(&impulse.id).unwrap();
    assert!(touched.last_accessed_at >= original_accessed);
}

#[test]
fn test_touch_connection() {
    let db = common::test_db();
    let a = db
        .insert_impulse(&NewImpulse {
            content: "A".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let b = db
        .insert_impulse(&NewImpulse {
            content: "B".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let conn = db
        .insert_connection(&NewConnection {
            source_id: a.id.clone(),
            target_id: b.id.clone(),
            weight: 0.5,
            relationship: "relates_to".to_string(),
        })
        .unwrap();

    assert_eq!(conn.traversal_count, 0);
    db.touch_connection(&conn.id).unwrap();
    let touched = db.get_connection(&conn.id).unwrap();
    assert_eq!(touched.traversal_count, 1);
}

// === Ghost Node Tests ===

#[test]
fn test_insert_and_get_ghost_node() {
    let db = common::test_db();
    let input = NewGhostNode {
        source_graph: "obsidian-vault".to_string(),
        external_ref: "notes/rust-patterns.md".to_string(),
        title: "Rust Design Patterns".to_string(),
        metadata: json!({"tags": ["rust", "patterns"]}),
        initial_weight: 0.4,
    };

    let node = db.insert_ghost_node(&input).unwrap();
    assert_eq!(node.source_graph, "obsidian-vault");
    assert_eq!(node.external_ref, "notes/rust-patterns.md");
    assert_eq!(node.title, "Rust Design Patterns");
    assert_eq!(node.weight, 0.4);

    // Retrieve by id
    let fetched = db.get_ghost_node(&node.id).unwrap();
    assert_eq!(fetched.id, node.id);
    assert_eq!(fetched.title, "Rust Design Patterns");

    // Retrieve by ref
    let by_ref = db
        .get_ghost_node_by_ref("obsidian-vault", "notes/rust-patterns.md")
        .unwrap();
    assert_eq!(by_ref.id, node.id);

    // Touch and verify last_accessed_at updates
    std::thread::sleep(std::time::Duration::from_millis(10));
    db.touch_ghost_node(&node.id).unwrap();
    let touched = db.get_ghost_node(&node.id).unwrap();
    assert!(touched.last_accessed_at >= node.last_accessed_at);

    // Update weight
    db.update_ghost_node_weight(&node.id, 0.8).unwrap();
    let updated = db.get_ghost_node(&node.id).unwrap();
    assert_eq!(updated.weight, 0.8);
}

#[test]
fn test_list_ghost_nodes_by_source() {
    let db = common::test_db();

    // Insert nodes from two different sources
    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault-a".to_string(),
        external_ref: "note1.md".to_string(),
        title: "Note One".to_string(),
        metadata: json!({}),
        initial_weight: 0.3,
    })
    .unwrap();

    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault-a".to_string(),
        external_ref: "note2.md".to_string(),
        title: "Note Two".to_string(),
        metadata: json!({}),
        initial_weight: 0.5,
    })
    .unwrap();

    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault-b".to_string(),
        external_ref: "other.md".to_string(),
        title: "Other Note".to_string(),
        metadata: json!({}),
        initial_weight: 0.4,
    })
    .unwrap();

    let vault_a_nodes = db.list_ghost_nodes_by_source("vault-a").unwrap();
    assert_eq!(vault_a_nodes.len(), 2);

    let vault_b_nodes = db.list_ghost_nodes_by_source("vault-b").unwrap();
    assert_eq!(vault_b_nodes.len(), 1);
    assert_eq!(vault_b_nodes[0].title, "Other Note");

    // Delete by source and verify
    let deleted = db.delete_ghost_nodes_by_source("vault-a").unwrap();
    assert_eq!(deleted, 2);

    let vault_a_after = db.list_ghost_nodes_by_source("vault-a").unwrap();
    assert_eq!(vault_a_after.len(), 0);

    // vault-b should be unaffected
    let vault_b_after = db.list_ghost_nodes_by_source("vault-b").unwrap();
    assert_eq!(vault_b_after.len(), 1);
}

#[test]
fn test_ghost_node_connections() {
    let db = common::test_db();

    let node_a = db
        .insert_ghost_node(&NewGhostNode {
            source_graph: "vault".to_string(),
            external_ref: "a.md".to_string(),
            title: "Ghost A".to_string(),
            metadata: json!({}),
            initial_weight: 0.5,
        })
        .unwrap();

    let node_b = db
        .insert_ghost_node(&NewGhostNode {
            source_graph: "vault".to_string(),
            external_ref: "b.md".to_string(),
            title: "Ghost B".to_string(),
            metadata: json!({}),
            initial_weight: 0.5,
        })
        .unwrap();

    let conn = db
        .insert_ghost_connection(&NewGhostConnection {
            source_id: node_a.id.clone(),
            target_id: node_b.id.clone(),
            weight: 0.7,
            relationship: "links_to".to_string(),
        })
        .unwrap();

    assert_eq!(conn.source_id, node_a.id);
    assert_eq!(conn.target_id, node_b.id);
    assert_eq!(conn.weight, 0.7);
    assert_eq!(conn.relationship, "links_to");
    assert_eq!(conn.traversal_count, 0);

    // Get connections for node_a
    let conns = db.get_ghost_connections_for_node(&node_a.id).unwrap();
    assert_eq!(conns.len(), 1);
    assert_eq!(conns[0].id, conn.id);

    // Get connections for node_b (bidirectional query)
    let conns_b = db.get_ghost_connections_for_node(&node_b.id).unwrap();
    assert_eq!(conns_b.len(), 1);
}

#[test]
fn test_ghost_source_registry() {
    let db = common::test_db();

    // Register a source
    let source = db
        .register_ghost_source("my-vault", "/home/user/vault", "obsidian")
        .unwrap();
    assert_eq!(source.name, "my-vault");
    assert_eq!(source.root_path, "/home/user/vault");
    assert_eq!(source.source_type, "obsidian");
    assert!(source.last_scanned_at.is_none());
    assert_eq!(source.node_count, 0);

    // List sources
    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].name, "my-vault");

    // Add a ghost node and check node_count
    db.insert_ghost_node(&NewGhostNode {
        source_graph: "my-vault".to_string(),
        external_ref: "test.md".to_string(),
        title: "Test Note".to_string(),
        metadata: json!({}),
        initial_weight: 0.3,
    })
    .unwrap();

    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources[0].node_count, 1);

    // Update scanned timestamp
    db.update_ghost_source_scanned("my-vault").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    assert!(sources[0].last_scanned_at.is_some());

    // FTS search on ghost nodes
    let results = db.search_ghost_nodes_fts("Test Note").unwrap();
    assert_eq!(results.len(), 1);
}

// === Tag Tests ===

#[test]
fn test_create_and_list_tags() {
    use synaptic_graph::models::NewTag;

    let db = common::test_db();

    // Create tags
    let tag1 = db
        .create_tag(&NewTag {
            name: "rust".to_string(),
            color: "#FF5733".to_string(),
        })
        .unwrap();
    assert_eq!(tag1.name, "rust");
    assert_eq!(tag1.color, "#FF5733");

    let tag2 = db
        .create_tag(&NewTag {
            name: "architecture".to_string(),
            color: "#3498DB".to_string(),
        })
        .unwrap();
    assert_eq!(tag2.name, "architecture");

    // List tags (sorted by name)
    let tags = db.list_tags().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "architecture");
    assert_eq!(tags[1].name, "rust");

    // Get specific tag
    let fetched = db.get_tag("rust").unwrap();
    assert_eq!(fetched.color, "#FF5733");

    // Delete tag
    db.delete_tag("rust").unwrap();
    let tags = db.list_tags().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "architecture");
}

#[test]
fn test_tag_and_untag_impulse() {
    use synaptic_graph::models::NewTag;

    let db = common::test_db();

    // Create an impulse
    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "Rust ownership model".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    // Create a tag
    db.create_tag(&NewTag {
        name: "rust".to_string(),
        color: "#FF5733".to_string(),
    })
    .unwrap();

    // Tag the impulse
    db.tag_impulse(&impulse.id, "rust").unwrap();

    // Verify tag is associated
    let tags = db.get_tags_for_impulse(&impulse.id).unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "rust");

    // Get impulses for tag
    let impulses = db.get_impulses_for_tag("rust").unwrap();
    assert_eq!(impulses.len(), 1);
    assert_eq!(impulses[0].id, impulse.id);

    // Untag
    db.untag_impulse(&impulse.id, "rust").unwrap();
    let tags = db.get_tags_for_impulse(&impulse.id).unwrap();
    assert_eq!(tags.len(), 0);
}

#[test]
fn test_get_tags_for_impulse() {
    use synaptic_graph::models::NewTag;

    let db = common::test_db();

    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "Multi-tag test".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    db.create_tag(&NewTag {
        name: "alpha".to_string(),
        color: "#AAA".to_string(),
    })
    .unwrap();
    db.create_tag(&NewTag {
        name: "beta".to_string(),
        color: "#BBB".to_string(),
    })
    .unwrap();
    db.create_tag(&NewTag {
        name: "gamma".to_string(),
        color: "#CCC".to_string(),
    })
    .unwrap();

    db.tag_impulse(&impulse.id, "alpha").unwrap();
    db.tag_impulse(&impulse.id, "beta").unwrap();
    db.tag_impulse(&impulse.id, "gamma").unwrap();

    let tags = db.get_tags_for_impulse(&impulse.id).unwrap();
    assert_eq!(tags.len(), 3);

    // Tags should be sorted by name
    assert_eq!(tags[0].name, "alpha");
    assert_eq!(tags[1].name, "beta");
    assert_eq!(tags[2].name, "gamma");

    // Tagging same impulse twice should be idempotent (INSERT OR IGNORE)
    db.tag_impulse(&impulse.id, "alpha").unwrap();
    let tags = db.get_tags_for_impulse(&impulse.id).unwrap();
    assert_eq!(tags.len(), 3);
}

#[test]
fn test_source_provider_stored() {
    let db = common::test_db();

    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "Provider test".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "claude".to_string(),
            source_account: "user@example.com".to_string(),
        })
        .unwrap();

    let fetched = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(fetched.source_provider, "claude");
    assert_eq!(fetched.source_account, "user@example.com");

    // Default values should be 'unknown' and ''
    let default_impulse = db
        .insert_impulse(&NewImpulse {
            content: "Default provider test".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
            source_provider: "unknown".to_string(),
            source_account: String::new(),
        })
        .unwrap();

    let fetched_default = db.get_impulse(&default_impulse.id).unwrap();
    assert_eq!(fetched_default.source_provider, "unknown");
    assert_eq!(fetched_default.source_account, "");
}

#[test]
fn test_reopen_preserves_canonical_feedback_and_skill_state() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("memory.db");
    let db_path = db_path.to_str().unwrap().to_string();

    let memory_id;
    let skill_id;

    {
        let db = Database::open(&db_path).unwrap();

        let impulse = db
            .insert_impulse(&NewImpulse {
                content: "Canonical restart safety".to_string(),
                impulse_type: ImpulseType::Heuristic,
                initial_weight: 0.7,
                emotional_valence: EmotionalValence::Neutral,
                engagement_level: EngagementLevel::Medium,
                source_signals: vec![],
                source_type: SourceType::ExplicitSave,
                source_ref: "test".to_string(),
                source_provider: "unknown".to_string(),
                source_account: String::new(),
            })
            .unwrap();
        db.confirm_impulse(&impulse.id).unwrap();
        db.apply_feedback_to_node(&impulse.id, FeedbackKind::Helpful)
            .unwrap();

        let evidence = db
            .create_evidence_set(
                "restart safety",
                "stable-hash",
                std::slice::from_ref(&impulse.id),
                &[],
                Some(24),
            )
            .unwrap();

        let skill = db
            .create_skill(
                "Restart-safe skill",
                "A skill that should survive restarts without backfill reset",
                "When checking canonical restart behavior",
                &["Verify reopen behavior".to_string()],
                &[],
                &evidence.id,
                std::slice::from_ref(&impulse.id),
                "unknown",
                "",
            )
            .unwrap();

        let canonical = db.get_canonical_node(&impulse.id).unwrap();
        assert_eq!(canonical.helpful_count, 1);

        memory_id = impulse.id;
        skill_id = skill.node_id;
    }

    let reopened = Database::open(&db_path).unwrap();
    let canonical = reopened.get_canonical_node(&memory_id).unwrap();
    assert_eq!(canonical.helpful_count, 1);
    assert_eq!(canonical.unhelpful_count, 0);

    let skill = reopened.get_skill(&skill_id).unwrap();
    assert_eq!(skill.name, "Restart-safe skill");
    assert_eq!(
        reopened.get_skill_evidence_node_ids(&skill_id).unwrap(),
        vec![memory_id]
    );
}

#[test]
fn test_purge_expired_evidence_sets_removes_only_stale_rows() {
    let db = common::test_db();

    let fresh = db
        .create_evidence_set("fresh", "fresh-hash", &[], &[], Some(24))
        .unwrap();
    let expired = db
        .create_evidence_set("expired", "old-hash", &[], &[], Some(-1))
        .unwrap();

    let purged = db.purge_expired_evidence_sets().unwrap();
    assert_eq!(purged, 1);
    assert!(db.get_evidence_set(&expired.id).is_err());
    assert!(db.get_evidence_set(&fresh.id).is_ok());
}
