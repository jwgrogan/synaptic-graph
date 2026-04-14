mod common;

use memory_graph::activation::ActivationEngine;
use memory_graph::models::*;

fn seed_graph(db: &memory_graph::db::Database) -> (String, String, String) {
    // Create three connected impulses: A -> B -> C
    // All confirmed so they appear in FTS search
    let a = db
        .insert_impulse(&NewImpulse {
            content: "Rust is great for building memory systems".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();
    db.confirm_impulse(&a.id).unwrap();

    let b = db
        .insert_impulse(&NewImpulse {
            content: "SQLite works well for local-first graph storage".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.6,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();
    db.confirm_impulse(&b.id).unwrap();

    let c = db
        .insert_impulse(&NewImpulse {
            content: "Spreading activation mimics human memory recall".to_string(),
            impulse_type: ImpulseType::Pattern,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();
    db.confirm_impulse(&c.id).unwrap();

    // A relates_to B, B relates_to C
    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        weight: 0.8,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: b.id.clone(),
        target_id: c.id.clone(),
        weight: 0.6,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    (a.id, b.id, c.id)
}

#[test]
fn test_direct_match_returns_result() {
    let db = common::test_db();
    let (a_id, _, _) = seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    assert!(!result.memories.is_empty());
    // Node A should be in results (direct FTS match on "Rust" and "memory")
    assert!(result.memories.iter().any(|m| m.impulse.id == a_id));
}

#[test]
fn test_activation_spreads_to_connected_nodes() {
    let db = common::test_db();
    let (a_id, b_id, _) = seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    // Node B should also appear (connected to A which matched directly)
    assert!(result.memories.iter().any(|m| m.impulse.id == b_id));
}

#[test]
fn test_activation_decays_with_hops() {
    let db = common::test_db();
    let (a_id, b_id, c_id) = seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();

    let score_a = result
        .memories
        .iter()
        .find(|m| m.impulse.id == a_id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    let score_b = result
        .memories
        .iter()
        .find(|m| m.impulse.id == b_id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    let score_c = result
        .memories
        .iter()
        .find(|m| m.impulse.id == c_id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    // Direct match should have highest score
    assert!(score_a > score_b);
    // 1-hop should be higher than 2-hop
    assert!(score_b > score_c);
}

#[test]
fn test_max_results_limits_output() {
    let db = common::test_db();
    seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 1,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    assert!(result.memories.len() <= 1);
}

#[test]
fn test_no_match_returns_empty() {
    let db = common::test_db();
    seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "quantum physics".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    assert!(result.memories.is_empty());
}

#[test]
fn test_deleted_impulses_excluded() {
    let db = common::test_db();
    let (a_id, _, _) = seed_graph(&db);

    db.update_impulse_status(&a_id, ImpulseStatus::Deleted).unwrap();

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    // Deleted node should not appear in results
    assert!(!result.memories.iter().any(|m| m.impulse.id == a_id));
}

#[test]
fn test_high_engagement_amplifies_propagation() {
    let db = common::test_db();

    // Create two paths: A -> B_high (high engagement) and A -> B_low (low engagement)
    let a = db
        .insert_impulse(&NewImpulse {
            content: "Memory design patterns".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();
    db.confirm_impulse(&a.id).unwrap();

    let b_high = db
        .insert_impulse(&NewImpulse {
            content: "High engagement insight about graphs".to_string(),
            impulse_type: ImpulseType::Pattern,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();
    db.confirm_impulse(&b_high.id).unwrap();

    let b_low = db
        .insert_impulse(&NewImpulse {
            content: "Low engagement note about graphs".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Low,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();
    db.confirm_impulse(&b_low.id).unwrap();

    // Same connection weight for both
    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b_high.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b_low.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "memory design".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();

    let score_high = result
        .memories
        .iter()
        .find(|m| m.impulse.id == b_high.id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    let score_low = result
        .memories
        .iter()
        .find(|m| m.impulse.id == b_low.id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    // High engagement node should receive more activation
    assert!(score_high > score_low);
}

#[test]
fn test_retrieval_reinforces_traversed_connections() {
    let db = common::test_db();
    let (a_id, b_id, _) = seed_graph(&db);

    let conns_before = db.get_connections_for_node(&a_id).unwrap();
    let weight_before = conns_before[0].weight;
    let count_before = conns_before[0].traversal_count;

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    engine.retrieve(&request).unwrap();

    let conns_after = db.get_connections_for_node(&a_id).unwrap();
    let ab_conn = conns_after
        .iter()
        .find(|c| {
            (c.source_id == a_id && c.target_id == b_id)
                || (c.source_id == b_id && c.target_id == a_id)
        })
        .unwrap();

    // Connection should have been reinforced
    assert!(ab_conn.weight >= weight_before);
    assert!(ab_conn.traversal_count > count_before);
}
