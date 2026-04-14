mod common;

use memory_graph::models::*;

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
    let rust_impulse = db.insert_impulse(&NewImpulse {
        content: "Rust is great for systems programming".to_string(),
        impulse_type: ImpulseType::Preference,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Positive,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    })
    .unwrap();

    let python_impulse = db.insert_impulse(&NewImpulse {
        content: "Python is slow but good for prototyping".to_string(),
        impulse_type: ImpulseType::Preference,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
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
    let _impulse = db.insert_impulse(&NewImpulse {
        content: "Unconfirmed candidate impulse about Rust".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
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
    assert_eq!(stats.total_connections, 0);

    let impulse = db.insert_impulse(&NewImpulse {
        content: "Test".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    })
    .unwrap();

    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.total_impulses, 1);
    assert_eq!(stats.candidate_impulses, 1);
    assert_eq!(stats.confirmed_impulses, 0);

    // Confirm it and check stats again
    db.confirm_impulse(&impulse.id).unwrap();
    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.confirmed_impulses, 1);
    assert_eq!(stats.candidate_impulses, 0);
}

#[test]
fn test_list_candidates() {
    let db = common::test_db();
    let impulse = db.insert_impulse(&NewImpulse {
        content: "A candidate".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
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
    let impulse = db.insert_impulse(&NewImpulse {
        content: "To be dismissed".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    })
    .unwrap();

    db.dismiss_impulse(&impulse.id).unwrap();
    let dismissed = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(dismissed.status, ImpulseStatus::Deleted);
}

#[test]
fn test_touch_impulse() {
    let db = common::test_db();
    let impulse = db.insert_impulse(&NewImpulse {
        content: "Touch test".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
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
    let a = db.insert_impulse(&NewImpulse {
        content: "A".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    }).unwrap();

    let b = db.insert_impulse(&NewImpulse {
        content: "B".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    }).unwrap();

    let conn = db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    }).unwrap();

    assert_eq!(conn.traversal_count, 0);
    db.touch_connection(&conn.id).unwrap();
    let touched = db.get_connection(&conn.id).unwrap();
    assert_eq!(touched.traversal_count, 1);
}
