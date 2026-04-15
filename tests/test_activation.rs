mod common;

use synaptic_graph::activation::ActivationEngine;
use synaptic_graph::ghost;
use synaptic_graph::ghost::ScanConfig;
use synaptic_graph::ingestion;
use synaptic_graph::models::*;

fn seed_graph(db: &synaptic_graph::db::Database) -> (String, String, String) {
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
            source_provider: "unknown".to_string(),
            source_account: String::new(),
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
            source_provider: "unknown".to_string(),
            source_account: String::new(),
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
            source_provider: "unknown".to_string(),
            source_account: String::new(),
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
            source_provider: "unknown".to_string(),
            source_account: String::new(),
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
            source_provider: "unknown".to_string(),
            source_account: String::new(),
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
            source_provider: "unknown".to_string(),
            source_account: String::new(),
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

#[test]
fn test_activation_includes_ghost_nodes() {
    use std::fs;
    use tempfile::TempDir;

    let db = common::test_db();

    // Add a regular confirmed impulse so we have both ghost and normal results
    let imp = db
        .insert_impulse(&NewImpulse {
            content: "Rust concurrency patterns for parallel systems".to_string(),
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
    db.confirm_impulse(&imp.id).unwrap();

    // Create a temporary vault with markdown files that match the same query
    let vault_dir = TempDir::new().unwrap();
    let vault_path = vault_dir.path();

    fs::write(
        vault_path.join("rust-patterns.md"),
        "# Rust Design Patterns\n\nNotes on Rust concurrency and parallel programming.\n",
    )
    .unwrap();

    fs::write(
        vault_path.join("other-note.md"),
        "# Cooking Recipes\n\nThis note is about cooking and has nothing to do with programming.\n",
    )
    .unwrap();

    // Register and scan the ghost graph
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };
    let node_count = ghost::register_and_scan(
        &db,
        "test-vault",
        vault_path.to_str().unwrap(),
        "obsidian",
        &config,
    )
    .unwrap();
    assert!(node_count >= 2);

    // Query something that should match both the impulse and the ghost node
    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust patterns".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();

    // Regular impulse should be in memories
    assert!(!result.memories.is_empty());

    // Ghost activations should be non-empty (the "Rust Design Patterns" ghost node should match)
    assert!(
        !result.ghost_activations.is_empty(),
        "Expected non-empty ghost_activations but got none"
    );

    // Verify the ghost activation has correct source_graph
    assert_eq!(result.ghost_activations[0].source_graph, "test-vault");

    // Verify activation score is above threshold
    assert!(result.ghost_activations[0].activation_score >= ACTIVATION_THRESHOLD);
}

// === Edge case and stress tests (Phase 4, Task 4) ===

#[test]
fn test_activation_with_empty_graph() {
    let db = common::test_db();
    let engine = ActivationEngine::new(&db);
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "anything".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();
    assert!(result.memories.is_empty());
}

#[test]
fn test_activation_with_single_node() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db,
        "Single isolated node",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "single isolated".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();
    assert_eq!(result.memories.len(), 1);
}

#[test]
fn test_activation_with_disconnected_clusters() {
    let db = common::test_db();

    // Cluster 1: A -> B
    let a = ingestion::save_and_confirm(
        &db,
        "Rust ownership patterns",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    )
    .unwrap();
    ingestion::save_and_confirm_with_connections(
        &db,
        "Borrow checker prevents data races",
        ImpulseType::Pattern,
        EmotionalValence::Positive,
        EngagementLevel::Medium,
        vec![],
        "test",
        &[(a.id.clone(), "relates_to".to_string(), 0.8)],
    )
    .unwrap();

    // Cluster 2: C -> D (disconnected from cluster 1)
    let c = ingestion::save_and_confirm(
        &db,
        "PostgreSQL connection pooling",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();
    ingestion::save_and_confirm_with_connections(
        &db,
        "Database connections are expensive to create",
        ImpulseType::Pattern,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "test",
        &[(c.id.clone(), "relates_to".to_string(), 0.7)],
    )
    .unwrap();

    let engine = ActivationEngine::new(&db);

    // Query about Rust should NOT activate PostgreSQL cluster
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "Rust ownership borrow".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();

    let has_postgres = result
        .memories
        .iter()
        .any(|m| m.impulse.content.contains("PostgreSQL"));
    assert!(
        !has_postgres,
        "Disconnected cluster should not activate"
    );
}

#[test]
fn test_activation_at_scale_100_nodes() {
    let db = common::test_db();
    let mut ids = Vec::new();

    // Create 100 confirmed impulses
    for i in 0..100 {
        let impulse = db
            .insert_impulse(&NewImpulse {
                content: format!("Memory node {} about topic {}", i, i % 10),
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
        db.confirm_impulse(&impulse.id).unwrap();
        ids.push(impulse.id);
    }

    // Create connections: each node to the next (chain)
    for i in 0..99 {
        db.insert_connection(&NewConnection {
            source_id: ids[i].clone(),
            target_id: ids[i + 1].clone(),
            weight: 0.5,
            relationship: "next".to_string(),
        })
        .unwrap();
    }

    let engine = ActivationEngine::new(&db);

    // Time the retrieval
    let start = std::time::Instant::now();
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "Memory node 0 topic".to_string(),
            max_results: 10,
            max_hops: 5,
        })
        .unwrap();
    let duration = start.elapsed();

    assert!(!result.memories.is_empty());
    assert!(
        duration.as_secs() < 15,
        "Retrieval took {}ms at 100 nodes",
        duration.as_millis()
    );
}

#[test]
fn test_activation_at_scale_1000_nodes() {
    let db = common::test_db();
    let mut ids = Vec::new();

    for i in 0..1000 {
        let impulse = db
            .insert_impulse(&NewImpulse {
                content: format!("Scale test node {} category {}", i, i % 50),
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
        db.confirm_impulse(&impulse.id).unwrap();
        ids.push(impulse.id);
    }

    // Create a more realistic graph: each node connected to 3 others
    for i in 0..1000 {
        for offset in [1, 7, 23] {
            let target = (i + offset) % 1000;
            let _ = db.insert_connection(&NewConnection {
                source_id: ids[i].clone(),
                target_id: ids[target].clone(),
                weight: 0.4,
                relationship: "relates_to".to_string(),
            });
        }
    }

    let engine = ActivationEngine::new(&db);

    let start = std::time::Instant::now();
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "Scale test node category".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();
    let duration = start.elapsed();

    assert!(!result.memories.is_empty());
    // TRD says target is under 200ms for 10K nodes in release; debug builds are much slower
    assert!(
        duration.as_secs() < 15,
        "Retrieval took {}ms at 1000 nodes",
        duration.as_millis()
    );
}

#[test]
fn test_activation_with_cycle() {
    let db = common::test_db();

    // A -> B -> C -> A (cycle)
    let a = ingestion::save_and_confirm(
        &db,
        "Cycle node A",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();
    let b = ingestion::save_and_confirm(
        &db,
        "Cycle node B",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();
    let c = ingestion::save_and_confirm(
        &db,
        "Cycle node C",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        weight: 0.8,
        relationship: "next".to_string(),
    })
    .unwrap();
    db.insert_connection(&NewConnection {
        source_id: b.id.clone(),
        target_id: c.id.clone(),
        weight: 0.8,
        relationship: "next".to_string(),
    })
    .unwrap();
    db.insert_connection(&NewConnection {
        source_id: c.id.clone(),
        target_id: a.id.clone(),
        weight: 0.8,
        relationship: "next".to_string(),
    })
    .unwrap();

    let engine = ActivationEngine::new(&db);

    // Should not infinite loop
    let start = std::time::Instant::now();
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "Cycle node".to_string(),
            max_results: 10,
            max_hops: 10,
        })
        .unwrap();
    let duration = start.elapsed();

    assert!(!result.memories.is_empty());
    assert!(
        duration.as_millis() < 1000,
        "Cycle detection failed — took {}ms",
        duration.as_millis()
    );
}

#[test]
fn test_activation_max_results_zero() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db,
        "Some memory",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "Some memory".to_string(),
            max_results: 0,
            max_hops: 3,
        })
        .unwrap();

    assert!(result.memories.is_empty());
}

#[test]
fn test_activation_empty_query() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db,
        "Memory",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "".to_string(),
        max_results: 10,
        max_hops: 3,
    });

    // Empty query should either return empty results or an error (FTS rejects empty strings)
    match result {
        Ok(r) => assert!(r.memories.is_empty()),
        Err(_) => {} // FTS syntax error on empty query is acceptable
    }
}
