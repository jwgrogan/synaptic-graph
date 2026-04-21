mod common;

mod test_ingestion {
    use crate::common;
    use synaptic_graph::ingestion::*;
    use synaptic_graph::models::*;

    #[test]
    fn test_explicit_save_creates_impulse() {
        let db = common::test_db();

        let result = explicit_save(
            &db,
            "User prefers dark mode in all editors",
            ImpulseType::Preference,
            EmotionalValence::Positive,
            EngagementLevel::High,
            vec!["user_stated".to_string()],
            "session-001",
        );

        assert!(result.is_ok(), "explicit_save should succeed");
        let impulse = result.unwrap();
        assert_eq!(impulse.status, ImpulseStatus::Candidate);
        assert_eq!(impulse.content, "User prefers dark mode in all editors");
        assert_eq!(impulse.impulse_type, ImpulseType::Preference);
        assert_eq!(impulse.initial_weight, WEIGHT_EXPLICIT_SAVE);
        assert_eq!(impulse.weight, WEIGHT_EXPLICIT_SAVE);
        assert_eq!(impulse.source_type, SourceType::ExplicitSave);
        assert_eq!(impulse.source_ref, "session-001");
    }

    #[test]
    fn test_save_and_confirm_creates_confirmed_impulse() {
        let db = common::test_db();

        let result = save_and_confirm(
            &db,
            "Always use Rust for systems programming",
            ImpulseType::Heuristic,
            EmotionalValence::Positive,
            EngagementLevel::High,
            vec!["repeated_mention".to_string()],
            "session-002",
        );

        assert!(result.is_ok(), "save_and_confirm should succeed");
        let impulse = result.unwrap();
        assert_eq!(impulse.status, ImpulseStatus::Confirmed);
        assert_eq!(impulse.content, "Always use Rust for systems programming");
    }

    #[test]
    fn test_explicit_save_redacts_secrets() {
        let db = common::test_db();

        let content_with_secret = "My API key is api_key=sk-abc123def456ghi789jkl012mno345pqr678";

        let result = explicit_save(
            &db,
            content_with_secret,
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Low,
            vec![],
            "session-003",
        );

        assert!(
            result.is_ok(),
            "explicit_save should succeed even with secrets"
        );
        let impulse = result.unwrap();
        assert!(
            !impulse
                .content
                .contains("sk-abc123def456ghi789jkl012mno345pqr678"),
            "Secret should be redacted from content"
        );
        assert!(
            impulse.content.contains("[REDACTED]"),
            "Content should contain redaction markers"
        );
    }

    #[test]
    fn test_explicit_save_with_connections() {
        let db = common::test_db();

        // Create a target impulse first
        let target = explicit_save(
            &db,
            "Prefers vim keybindings",
            ImpulseType::Preference,
            EmotionalValence::Positive,
            EngagementLevel::Medium,
            vec![],
            "session-004",
        )
        .expect("target impulse should be created");

        let connections = vec![(target.id.clone(), "relates_to".to_string(), 0.8)];

        let result = explicit_save_with_connections(
            &db,
            "Uses neovim as primary editor",
            ImpulseType::Observation,
            EmotionalValence::Positive,
            EngagementLevel::Medium,
            vec![],
            "session-004",
            &connections,
        );

        assert!(
            result.is_ok(),
            "explicit_save_with_connections should succeed"
        );
        let impulse = result.unwrap();
        assert_eq!(impulse.status, ImpulseStatus::Candidate);

        // Verify connection was created
        let conns = db
            .get_connections_for_node(&impulse.id)
            .expect("should get connections");
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].source_id, impulse.id);
        assert_eq!(conns[0].target_id, target.id);
        assert_eq!(conns[0].relationship, "relates_to");
        assert!((conns[0].weight - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_save_empty_content_fails() {
        let db = common::test_db();

        let result = explicit_save(
            &db,
            "",
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Low,
            vec![],
            "session-005",
        );

        assert!(result.is_err(), "empty content should fail");
        assert!(
            result.unwrap_err().contains("Content must not be empty"),
            "error should mention empty content"
        );

        // Also test whitespace-only content
        let result_whitespace = explicit_save(
            &db,
            "   \t\n  ",
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Low,
            vec![],
            "session-005",
        );

        assert!(
            result_whitespace.is_err(),
            "whitespace-only content should fail"
        );
    }

    #[test]
    fn test_save_only_secrets_fails() {
        let db = common::test_db();

        // Content that is entirely a secret -- after redaction it becomes "[REDACTED]"
        // which is non-empty, so it should still save successfully
        let result = explicit_save(
            &db,
            "sk-abcdefghijklmnopqrstuvwxyz1234567890",
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Low,
            vec![],
            "session-006",
        );

        assert!(
            result.is_ok(),
            "content that is only secrets should still save (redacted but non-empty)"
        );
        let impulse = result.unwrap();
        assert_eq!(impulse.content, "[REDACTED]");
        assert_eq!(impulse.status, ImpulseStatus::Candidate);
    }
} // mod test_ingestion
