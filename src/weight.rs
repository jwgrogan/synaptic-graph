// Decay and reinforcement weight mechanics

use crate::models::*;

/// Calculate effective weight after time-based decay.
/// weight: current stored weight (0.0 to 1.0)
/// hours_elapsed: hours since last access
/// lambda: decay rate constant
///
/// Formula: effective = max(WEIGHT_FLOOR, weight * e^(-lambda * hours))
pub fn effective_weight(weight: f64, hours_elapsed: f64, lambda: f64) -> f64 {
    let decayed = weight * (-lambda * hours_elapsed).exp();
    decayed.max(WEIGHT_FLOOR)
}

/// Apply reinforcement bump to a weight.
/// Returns new weight, capped at 1.0.
pub fn reinforce(weight: f64) -> f64 {
    (weight + REINFORCEMENT_BUMP).min(1.0)
}

/// Get the appropriate decay rate for an impulse type.
/// Semantic knowledge (heuristics, preferences, decisions, patterns) decays slowly.
/// Episodic observations decay faster.
pub fn decay_rate_for_type(impulse_type: ImpulseType) -> f64 {
    match impulse_type {
        ImpulseType::Heuristic
        | ImpulseType::Preference
        | ImpulseType::Decision
        | ImpulseType::Pattern => DECAY_SEMANTIC,
        ImpulseType::Observation => DECAY_EPISODIC,
    }
}

/// Calculate hours elapsed between two timestamps.
pub fn hours_since(from: &chrono::DateTime<chrono::Utc>, to: &chrono::DateTime<chrono::Utc>) -> f64 {
    let duration = *to - *from;
    duration.num_seconds() as f64 / 3600.0
}
