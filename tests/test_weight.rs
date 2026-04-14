mod common;

use synaptic_graph::models::*;
use synaptic_graph::weight;

#[test]
fn test_effective_weight_no_decay() {
    // Just created, no time passed
    let w = weight::effective_weight(0.7, 0.0, DECAY_SEMANTIC);
    assert!((w - 0.7).abs() < 0.001);
}

#[test]
fn test_effective_weight_decays_over_time() {
    // After 100 hours with semantic decay rate
    let w = weight::effective_weight(0.7, 100.0, DECAY_SEMANTIC);
    // e^(-0.0005 * 100) = e^(-0.05) ≈ 0.951
    // 0.7 * 0.951 ≈ 0.666
    assert!(w < 0.7);
    assert!(w > 0.6);
}

#[test]
fn test_effective_weight_episodic_decays_faster() {
    let semantic = weight::effective_weight(0.7, 200.0, DECAY_SEMANTIC);
    let episodic = weight::effective_weight(0.7, 200.0, DECAY_EPISODIC);
    assert!(episodic < semantic);
}

#[test]
fn test_effective_weight_never_below_floor() {
    // After a very long time
    let w = weight::effective_weight(0.1, 100_000.0, DECAY_EPISODIC);
    assert!(w >= WEIGHT_FLOOR);
}

#[test]
fn test_reinforce_adds_bump() {
    let new_weight = weight::reinforce(0.5);
    assert!((new_weight - 0.55).abs() < 0.001);
}

#[test]
fn test_reinforce_caps_at_one() {
    let new_weight = weight::reinforce(0.98);
    assert!((new_weight - 1.0).abs() < 0.001);
}

#[test]
fn test_reinforce_from_floor() {
    let new_weight = weight::reinforce(WEIGHT_FLOOR);
    assert!((new_weight - (WEIGHT_FLOOR + REINFORCEMENT_BUMP)).abs() < 0.001);
}

#[test]
fn test_decay_rate_for_impulse_type() {
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Heuristic), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Preference), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Decision), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Pattern), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Observation), DECAY_EPISODIC);
}
