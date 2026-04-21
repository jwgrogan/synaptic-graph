mod common;

mod test_validation {
    use crate::common;
    use synaptic_graph::activation::ActivationEngine;
    use synaptic_graph::db::Database;
    use synaptic_graph::ingestion;
    use synaptic_graph::models::*;
    use synaptic_graph::redaction;
    use synaptic_graph::server::MemoryGraphServer;
    use synaptic_graph::weight;

    // ---------------------------------------------------------------
    // 1. Store-retrieve round trip
    // ---------------------------------------------------------------
    #[test]
    fn validation_store_retrieve_round_trip() {
        let db = common::test_db();

        let topics = [
            (
                "Rust borrow checker prevents data races at compile time",
                ImpulseType::Heuristic,
            ),
            (
                "PostgreSQL JSONB columns support GIN indexes for fast lookups",
                ImpulseType::Pattern,
            ),
            (
                "Docker containers should use non-root users for security",
                ImpulseType::Decision,
            ),
            (
                "Kubernetes liveness probes prevent cascading failures",
                ImpulseType::Observation,
            ),
            (
                "GraphQL schema stitching merges multiple APIs into one",
                ImpulseType::Preference,
            ),
        ];

        for (content, itype) in &topics {
            ingestion::save_and_confirm(
                &db,
                content,
                *itype,
                EmotionalValence::Neutral,
                EngagementLevel::Medium,
                vec![],
                "validation-test",
            )
            .expect("save_and_confirm should succeed");
        }

        let engine = ActivationEngine::new(&db);

        // Query each topic keyword and verify correct impulse is returned first
        let queries = [
            ("borrow checker", "Rust borrow checker"),
            ("JSONB indexes", "PostgreSQL JSONB"),
            ("Docker containers security", "Docker containers"),
            ("liveness probes", "Kubernetes liveness"),
            ("schema stitching", "GraphQL schema"),
        ];

        for (query, expected_fragment) in &queries {
            let request = RetrievalRequest {
                query: query.to_string(),
                max_results: 5,
                max_hops: 3,
            };
            let result = engine.retrieve(&request).expect("retrieve should succeed");
            assert!(
                !result.memories.is_empty(),
                "Query '{}' should return at least one result",
                query
            );
            assert!(
                result.memories[0]
                    .impulse
                    .content
                    .contains(expected_fragment),
                "First result for '{}' should contain '{}', got: '{}'",
                query,
                expected_fragment,
                result.memories[0].impulse.content
            );
        }
    }

    // ---------------------------------------------------------------
    // 2. Spreading activation through adjacency
    // ---------------------------------------------------------------
    #[test]
    fn validation_spreading_activation_through_adjacency() {
        let db = common::test_db();

        // Create chain: auth -> security -> mitigation (connected)
        let auth = ingestion::save_and_confirm(
            &db,
            "JWT token authentication validates user identity via signed claims",
            ImpulseType::Heuristic,
            EmotionalValence::Neutral,
            EngagementLevel::High,
            vec![],
            "validation-test",
        )
        .expect("auth impulse");

        let security = ingestion::save_and_confirm_with_connections(
            &db,
            "Security best practices require rotating secrets and validating tokens",
            ImpulseType::Pattern,
            EmotionalValence::Neutral,
            EngagementLevel::High,
            vec![],
            "validation-test",
            &[(auth.id.clone(), "relates_to".to_string(), 0.9)],
        )
        .expect("security impulse");

        let _mitigation = ingestion::save_and_confirm_with_connections(
            &db,
            "Threat mitigation strategies include rate limiting and input validation",
            ImpulseType::Decision,
            EmotionalValence::Neutral,
            EngagementLevel::High,
            vec![],
            "validation-test",
            &[(security.id.clone(), "relates_to".to_string(), 0.9)],
        )
        .expect("mitigation impulse");

        let engine = ActivationEngine::new(&db);
        let request = RetrievalRequest {
            query: "JWT token".to_string(),
            max_results: 10,
            max_hops: 3,
        };
        let result = engine.retrieve(&request).expect("retrieve should succeed");

        // All three should appear via spreading activation
        assert!(
            result.memories.len() >= 3,
            "Spreading activation should reach all 3 connected impulses, got {}",
            result.memories.len()
        );

        let contents: Vec<&str> = result
            .memories
            .iter()
            .map(|m| m.impulse.content.as_str())
            .collect();
        assert!(
            contents.iter().any(|c| c.contains("JWT token")),
            "Should find auth impulse"
        );
        assert!(
            contents
                .iter()
                .any(|c| c.contains("Security best practices")),
            "Should find security impulse via adjacency"
        );
        assert!(
            contents.iter().any(|c| c.contains("Threat mitigation")),
            "Should find mitigation impulse via 2-hop adjacency"
        );
    }

    // ---------------------------------------------------------------
    // 3. Decay reduces effective weight
    // ---------------------------------------------------------------
    #[test]
    fn validation_decay_reduces_effective_weight() {
        let initial_weight = 0.7;

        // Semantic (slow decay) after 100 hours
        let semantic_100h = weight::effective_weight(initial_weight, 100.0, DECAY_SEMANTIC);
        assert!(
            semantic_100h < initial_weight,
            "Semantic weight should decay: {} < {}",
            semantic_100h,
            initial_weight
        );
        assert!(
            semantic_100h > 0.6,
            "Semantic decay should be slow after 100h: {}",
            semantic_100h
        );

        // Episodic (fast decay) after 100 hours
        let episodic_100h = weight::effective_weight(initial_weight, 100.0, DECAY_EPISODIC);
        assert!(
            episodic_100h < semantic_100h,
            "Episodic should decay faster than semantic: {} < {}",
            episodic_100h,
            semantic_100h
        );

        // Both should be above floor
        assert!(
            semantic_100h >= WEIGHT_FLOOR,
            "Semantic weight should not go below floor"
        );
        assert!(
            episodic_100h >= WEIGHT_FLOOR,
            "Episodic weight should not go below floor"
        );

        // After extreme time, should hit floor but not go below
        let extreme_hours = 1_000_000.0;
        let semantic_extreme =
            weight::effective_weight(initial_weight, extreme_hours, DECAY_SEMANTIC);
        let episodic_extreme =
            weight::effective_weight(initial_weight, extreme_hours, DECAY_EPISODIC);

        assert!(
            (semantic_extreme - WEIGHT_FLOOR).abs() < f64::EPSILON
                || semantic_extreme >= WEIGHT_FLOOR,
            "Semantic weight should be at or above floor after extreme time"
        );
        assert!(
            (episodic_extreme - WEIGHT_FLOOR).abs() < f64::EPSILON
                || episodic_extreme >= WEIGHT_FLOOR,
            "Episodic weight should be at or above floor after extreme time"
        );

        // Nothing should ever go below the floor
        assert!(
            semantic_extreme >= WEIGHT_FLOOR,
            "Semantic must never go below floor: {}",
            semantic_extreme
        );
        assert!(
            episodic_extreme >= WEIGHT_FLOOR,
            "Episodic must never go below floor: {}",
            episodic_extreme
        );
    }

    // ---------------------------------------------------------------
    // 4. Reinforcement counters decay
    // ---------------------------------------------------------------
    #[test]
    fn validation_reinforcement_counters_decay() {
        let initial_weight = WEIGHT_EXPLICIT_SAVE; // 0.7
        let lambda = DECAY_SEMANTIC;
        let hours_per_day = 24.0;

        // Simulate 10 days of daily access: decay for a day, then reinforce
        let mut current_weight = initial_weight;
        for _day in 0..10 {
            let decayed = weight::effective_weight(current_weight, hours_per_day, lambda);
            current_weight = weight::reinforce(decayed);
        }

        assert!(
            current_weight > initial_weight,
            "After 10 days of daily access + reinforcement, weight ({}) should exceed initial ({})",
            current_weight,
            initial_weight
        );

        // Now simulate 60 days of no access (pure decay)
        let hours_no_access = 60.0 * hours_per_day;
        let after_absence = weight::effective_weight(current_weight, hours_no_access, lambda);

        assert!(
            after_absence < current_weight,
            "After 60 days of no access, weight ({}) should be less than before ({})",
            after_absence,
            current_weight
        );
        assert!(
            after_absence >= WEIGHT_FLOOR,
            "Weight ({}) should stay above floor ({}) even after 60 days",
            after_absence,
            WEIGHT_FLOOR
        );
    }

    // ---------------------------------------------------------------
    // 5. Narrative reconstruction from connections
    // ---------------------------------------------------------------
    #[test]
    fn validation_narrative_reconstruction_from_connections() {
        let db = common::test_db();

        // Create a cluster: problem -> insight -> decision -> implementation
        let problem = ingestion::save_and_confirm(
            &db,
            "Memory portability is the core problem: AI context is trapped in individual sessions",
            ImpulseType::Observation,
            EmotionalValence::Negative,
            EngagementLevel::High,
            vec![],
            "validation-test",
        )
        .expect("problem impulse");

        let insight = ingestion::save_and_confirm_with_connections(
            &db,
            "Human memory uses spreading activation to link related memories portably",
            ImpulseType::Pattern,
            EmotionalValence::Positive,
            EngagementLevel::High,
            vec![],
            "validation-test",
            &[(problem.id.clone(), "insight_from".to_string(), 0.9)],
        )
        .expect("insight impulse");

        let decision = ingestion::save_and_confirm_with_connections(
            &db,
            "Decision: build a graph-based memory system with weighted decay for portability",
            ImpulseType::Decision,
            EmotionalValence::Positive,
            EngagementLevel::High,
            vec![],
            "validation-test",
            &[(insight.id.clone(), "decided_from".to_string(), 0.9)],
        )
        .expect("decision impulse");

        let _implementation = ingestion::save_and_confirm_with_connections(
            &db,
            "Implementation: SQLite graph with FTS5 for seed matching and spreading activation",
            ImpulseType::Heuristic,
            EmotionalValence::Positive,
            EngagementLevel::High,
            vec![],
            "validation-test",
            &[(decision.id.clone(), "implements".to_string(), 0.9)],
        )
        .expect("implementation impulse");

        let engine = ActivationEngine::new(&db);
        let request = RetrievalRequest {
            query: "memory portability".to_string(),
            max_results: 10,
            max_hops: 5,
        };
        let result = engine.retrieve(&request).expect("retrieve should succeed");

        // Should return multiple connected impulses that together tell the story
        assert!(
            result.memories.len() >= 3,
            "Narrative reconstruction should return at least 3 impulses, got {}",
            result.memories.len()
        );

        let contents: Vec<&str> = result
            .memories
            .iter()
            .map(|m| m.impulse.content.as_str())
            .collect();
        assert!(
            contents.iter().any(|c| c.contains("core problem")),
            "Should find the problem statement"
        );
        assert!(
            contents.iter().any(|c| c.contains("spreading activation")),
            "Should find the insight"
        );
        assert!(
            contents.iter().any(|c| c.contains("graph-based memory")),
            "Should find the decision"
        );
    }

    // ---------------------------------------------------------------
    // 6. Emotional weighting
    // ---------------------------------------------------------------
    #[test]
    fn validation_emotional_weighting() {
        let db = common::test_db();

        // Create an anchor impulse that the query will match
        let anchor = ingestion::save_and_confirm(
            &db,
            "Database optimization techniques improve query performance significantly",
            ImpulseType::Heuristic,
            EmotionalValence::Neutral,
            EngagementLevel::Medium,
            vec![],
            "validation-test",
        )
        .expect("anchor impulse");

        // High engagement impulse connected to anchor
        let high_eng = ingestion::save_and_confirm_with_connections(
            &db,
            "High engagement: indexing strategy that saved production from outage",
            ImpulseType::Decision,
            EmotionalValence::Positive,
            EngagementLevel::High,
            vec![],
            "validation-test",
            &[(anchor.id.clone(), "relates_to".to_string(), 0.8)],
        )
        .expect("high engagement impulse");

        // Low engagement impulse connected to same anchor with same connection weight
        let low_eng = ingestion::save_and_confirm_with_connections(
            &db,
            "Low engagement: minor indexing tweak on a test table",
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Low,
            vec![],
            "validation-test",
            &[(anchor.id.clone(), "relates_to".to_string(), 0.8)],
        )
        .expect("low engagement impulse");

        let engine = ActivationEngine::new(&db);
        let request = RetrievalRequest {
            query: "database optimization".to_string(),
            max_results: 10,
            max_hops: 3,
        };
        let result = engine.retrieve(&request).expect("retrieve should succeed");

        // Find the activation scores for both
        let high_score = result
            .memories
            .iter()
            .find(|m| m.impulse.id == high_eng.id)
            .map(|m| m.activation_score);
        let low_score = result
            .memories
            .iter()
            .find(|m| m.impulse.id == low_eng.id)
            .map(|m| m.activation_score);

        assert!(
            high_score.is_some(),
            "High engagement impulse should appear in results"
        );
        assert!(
            low_score.is_some(),
            "Low engagement impulse should appear in results"
        );
        assert!(
            high_score.unwrap() > low_score.unwrap(),
            "High engagement ({}) should get higher activation score than low engagement ({})",
            high_score.unwrap(),
            low_score.unwrap()
        );
    }

    // ---------------------------------------------------------------
    // 7. Security: secrets stripped
    // ---------------------------------------------------------------
    #[test]
    fn validation_security_secrets_stripped() {
        let db = common::test_db();

        let content_with_secrets = "Config: \
            AWS key AKIAIOSFODNN7EXAMPLE, \
            auth bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9abcdef, \
            database postgresql://admin:secret@prod-db:5432/myapp";

        let impulse = ingestion::explicit_save(
            &db,
            content_with_secrets,
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Low,
            vec![],
            "validation-test",
        )
        .expect("save should succeed despite secrets");

        // AWS key should be redacted
        assert!(
            !impulse.content.contains("AKIAIOSFODNN7EXAMPLE"),
            "AWS key should be redacted from persisted content"
        );
        // Bearer token should be redacted
        assert!(
            !impulse
                .content
                .contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9abcdef"),
            "Bearer token should be redacted from persisted content"
        );
        // Connection string should be redacted
        assert!(
            !impulse
                .content
                .contains("postgresql://admin:secret@prod-db:5432/myapp"),
            "Connection string should be redacted from persisted content"
        );

        // Verify [REDACTED] markers are present
        let redacted_count = impulse.content.matches("[REDACTED]").count();
        assert!(
            redacted_count >= 3,
            "Should have at least 3 [REDACTED] markers, got {}",
            redacted_count
        );

        // Also verify via the redaction module directly
        let result = redaction::redact(content_with_secrets);
        assert!(
            result.redactions.len() >= 3,
            "Redaction should detect at least 3 secrets, found {}",
            result.redactions.len()
        );
    }

    // ---------------------------------------------------------------
    // 8. Incognito zero trace
    // ---------------------------------------------------------------
    #[test]
    fn validation_incognito_zero_trace() {
        let db = Database::open_in_memory().expect("in-memory db");
        let server = MemoryGraphServer::new_with_db(db);

        // Save before incognito
        let before_result = server.handle_save_memory(
            "Pre-incognito memory about project architecture".to_string(),
            "heuristic".to_string(),
            None,
            None,
            Some("validation-test".to_string()),
            None,
            None,
        );
        assert!(
            before_result.is_ok(),
            "Save before incognito should succeed"
        );

        // Get count before incognito
        let status_before = server.handle_memory_status().expect("status should work");
        let status_before: serde_json::Value =
            serde_json::from_str(&status_before).expect("parse status");
        let count_before = status_before["total_impulses"].as_i64().unwrap();

        // Enable incognito
        server.set_incognito(true);
        assert!(server.is_incognito(), "Server should be in incognito mode");

        // Attempt save in incognito (should fail)
        let incognito_result = server.handle_save_memory(
            "This should never be saved because we are incognito".to_string(),
            "observation".to_string(),
            None,
            None,
            Some("validation-test".to_string()),
            None,
            None,
        );
        assert!(
            incognito_result.is_err(),
            "Save in incognito mode should fail"
        );
        assert!(
            incognito_result.unwrap_err().contains("incognito"),
            "Error should mention incognito"
        );

        // Disable incognito
        server.set_incognito(false);
        assert!(!server.is_incognito(), "Incognito should be disabled");

        // Verify count unchanged
        let status_after = server.handle_memory_status().expect("status should work");
        let status_after: serde_json::Value =
            serde_json::from_str(&status_after).expect("parse status");
        let count_after = status_after["total_impulses"].as_i64().unwrap();

        assert_eq!(
            count_before, count_after,
            "Impulse count should be unchanged after incognito attempt: before={}, after={}",
            count_before, count_after
        );
    }

    // ---------------------------------------------------------------
    // 9. Supersession preserves history
    // ---------------------------------------------------------------
    #[test]
    fn validation_supersession_preserves_history() {
        let db = common::test_db();

        // Save v1
        let v1 = ingestion::save_and_confirm(
            &db,
            "v1: Use REST API for all external communication",
            ImpulseType::Decision,
            EmotionalValence::Neutral,
            EngagementLevel::Medium,
            vec![],
            "validation-test",
        )
        .expect("v1 should save");
        assert_eq!(v1.status, ImpulseStatus::Confirmed);

        // Update to v2 (supersede v1)
        let v2_id = db
            .update_impulse_content(
                &v1.id,
                "v2: Use GraphQL for external APIs, REST for internal",
            )
            .expect("update should succeed");

        // v1 should now be superseded
        let v1_after = db.get_impulse(&v1.id).expect("v1 should still exist");
        assert_eq!(
            v1_after.status,
            ImpulseStatus::Superseded,
            "v1 should be superseded"
        );

        // v2 should be candidate (update_impulse_content creates candidate)
        let v2 = db.get_impulse(&v2_id).expect("v2 should exist");
        assert_eq!(
            v2.status,
            ImpulseStatus::Candidate,
            "v2 should be candidate after creation"
        );
        assert!(
            v2.content.contains("GraphQL"),
            "v2 should have updated content"
        );

        // Supersedes connection should exist
        let connections = db
            .get_connections_for_node(&v2_id)
            .expect("should get connections");
        let supersedes_conn = connections.iter().find(|c| {
            c.relationship == "supersedes" && c.source_id == v2_id && c.target_id == v1.id
        });
        assert!(
            supersedes_conn.is_some(),
            "A 'supersedes' connection should exist from v2 to v1"
        );

        // v1 content is preserved (not deleted)
        assert!(
            v1_after.content.contains("REST API"),
            "v1 content should be preserved for history"
        );
    }

    // ---------------------------------------------------------------
    // 10. Candidate gate: candidates excluded from FTS
    // ---------------------------------------------------------------
    #[test]
    fn validation_candidate_gate() {
        let db = common::test_db();

        // Create a candidate (not confirmed) via explicit_save
        let candidate = ingestion::explicit_save(
            &db,
            "Candidate impulse about quantum computing algorithms",
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Medium,
            vec![],
            "validation-test",
        )
        .expect("explicit_save should succeed");

        assert_eq!(
            candidate.status,
            ImpulseStatus::Candidate,
            "explicit_save should create candidate"
        );

        // FTS search should NOT find the candidate
        let engine = ActivationEngine::new(&db);
        let request = RetrievalRequest {
            query: "quantum computing".to_string(),
            max_results: 10,
            max_hops: 3,
        };
        let result = engine.retrieve(&request).expect("retrieve should succeed");
        let found_candidate = result.memories.iter().any(|m| m.impulse.id == candidate.id);
        assert!(
            !found_candidate,
            "Candidate should NOT appear in FTS search results before confirmation"
        );

        // Confirm the candidate
        db.confirm_impulse(&candidate.id)
            .expect("confirm should succeed");

        // Now FTS search should find it
        let result_after = engine.retrieve(&request).expect("retrieve should succeed");
        let found_after = result_after
            .memories
            .iter()
            .any(|m| m.impulse.id == candidate.id);
        assert!(
            found_after,
            "Confirmed impulse SHOULD appear in FTS search results after confirmation"
        );
    }
}
