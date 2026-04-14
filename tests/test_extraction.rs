mod common;

use synaptic_graph::extraction::*;

#[test]
fn test_low_engagement_session() {
    let signals = EngagementSignals {
        total_turns: 2,
        avg_user_message_length: 20.0,
        avg_assistant_message_length: 50.0,
        session_duration_minutes: 1.0,
        explicit_save_count: 0,
        topic_count: 1,
        decision_keywords_found: 0,
        emotional_keywords_found: 0,
    };
    let depth = assess_engagement(&signals);
    assert_eq!(depth, ExtractionDepth::Minimal);
    assert_eq!(depth.max_proposals(), 1);
}

#[test]
fn test_high_engagement_session() {
    let signals = EngagementSignals {
        total_turns: 40,
        avg_user_message_length: 300.0,
        avg_assistant_message_length: 500.0,
        session_duration_minutes: 90.0,
        explicit_save_count: 5,
        topic_count: 8,
        decision_keywords_found: 7,
        emotional_keywords_found: 4,
    };
    let depth = assess_engagement(&signals);
    assert_eq!(depth, ExtractionDepth::Deep);
    assert_eq!(depth.max_proposals(), 15);
}

#[test]
fn test_medium_engagement_session() {
    let signals = EngagementSignals {
        total_turns: 10,
        avg_user_message_length: 80.0,
        avg_assistant_message_length: 150.0,
        session_duration_minutes: 15.0,
        explicit_save_count: 1,
        topic_count: 2,
        decision_keywords_found: 2,
        emotional_keywords_found: 1,
    };
    let depth = assess_engagement(&signals);
    assert_eq!(depth, ExtractionDepth::Standard);
    assert_eq!(depth.max_proposals(), 5);
}

#[test]
fn test_extraction_depth_max_proposals() {
    assert_eq!(ExtractionDepth::Minimal.max_proposals(), 1);
    assert_eq!(ExtractionDepth::Standard.max_proposals(), 5);
    assert_eq!(ExtractionDepth::Deep.max_proposals(), 15);
}

#[test]
fn test_engagement_score_calculation() {
    // All zeros should give 0.0
    let zero_signals = EngagementSignals {
        total_turns: 0,
        avg_user_message_length: 0.0,
        avg_assistant_message_length: 0.0,
        session_duration_minutes: 0.0,
        explicit_save_count: 0,
        topic_count: 0,
        decision_keywords_found: 0,
        emotional_keywords_found: 0,
    };
    assert!((zero_signals.engagement_score() - 0.0).abs() < 0.001);

    // All maxed out should give 1.0
    let max_signals = EngagementSignals {
        total_turns: 60,
        avg_user_message_length: 400.0,
        avg_assistant_message_length: 600.0,
        session_duration_minutes: 120.0,
        explicit_save_count: 10,
        topic_count: 10,
        decision_keywords_found: 10,
        emotional_keywords_found: 10,
    };
    assert!((max_signals.engagement_score() - 1.0).abs() < 0.001);

    // Verify partial score: only turns at 15/30 = 0.5, weight 1.0 => 0.5/6.0
    let partial = EngagementSignals {
        total_turns: 15,
        avg_user_message_length: 0.0,
        avg_assistant_message_length: 0.0,
        session_duration_minutes: 0.0,
        explicit_save_count: 0,
        topic_count: 0,
        decision_keywords_found: 0,
        emotional_keywords_found: 0,
    };
    let expected = 0.5 / 6.0;
    assert!((partial.engagement_score() - expected).abs() < 0.001);

    // Test count_keywords utility
    let text = "I decided to go and I'm excited about the decision";
    let decision_count = count_keywords(text, &DECISION_KEYWORDS);
    assert_eq!(decision_count, 2); // "decided" and "decision"

    let emotional_count = count_keywords(text, &EMOTIONAL_KEYWORDS);
    assert_eq!(emotional_count, 1); // "excited"

    // Case insensitive
    let upper_text = "DECIDED and EXCITED";
    assert_eq!(count_keywords(upper_text, &DECISION_KEYWORDS), 1);
    assert_eq!(count_keywords(upper_text, &EMOTIONAL_KEYWORDS), 1);
}
