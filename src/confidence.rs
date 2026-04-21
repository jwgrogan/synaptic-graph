pub const CONFIDENCE_PRIOR_HELPFUL: f64 = 1.0;
pub const CONFIDENCE_PRIOR_UNHELPFUL: f64 = 1.0;
pub const MIN_FEEDBACK_FOR_RANKING: i64 = 3;
pub const NEUTRAL_CONFIDENCE: f64 = 0.5;

pub fn posterior_confidence(helpful_count: i64, unhelpful_count: i64) -> f64 {
    let helpful = helpful_count.max(0) as f64;
    let unhelpful = unhelpful_count.max(0) as f64;
    (helpful + CONFIDENCE_PRIOR_HELPFUL)
        / (helpful + unhelpful + CONFIDENCE_PRIOR_HELPFUL + CONFIDENCE_PRIOR_UNHELPFUL)
}

pub fn effective_confidence(helpful_count: i64, unhelpful_count: i64) -> f64 {
    if helpful_count + unhelpful_count < MIN_FEEDBACK_FOR_RANKING {
        NEUTRAL_CONFIDENCE
    } else {
        posterior_confidence(helpful_count, unhelpful_count)
    }
}

pub fn ranking_multiplier(confidence: f64) -> f64 {
    0.5 + confidence.clamp(0.0, 1.0)
}
