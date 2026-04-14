// Adaptive end-of-session extraction for ghost graph content parsing

/// Keywords that signal decisions or commitments in conversation.
pub const DECISION_KEYWORDS: &[&str] = &[
    "decided",
    "decision",
    "chose",
    "choosing",
    "commit",
    "committed",
    "agreed",
    "agreement",
    "settled",
    "concluded",
    "resolved",
    "finalized",
    "approved",
    "confirmed",
    "determined",
];

/// Keywords that signal emotional significance in conversation.
pub const EMOTIONAL_KEYWORDS: &[&str] = &[
    "love",
    "hate",
    "frustrated",
    "excited",
    "worried",
    "grateful",
    "afraid",
    "happy",
    "angry",
    "disappointed",
    "thrilled",
    "anxious",
    "passionate",
    "overwhelmed",
    "relieved",
];

/// Signals derived from a conversation session used to assess engagement depth.
#[derive(Debug, Clone)]
pub struct EngagementSignals {
    pub total_turns: usize,
    pub avg_user_message_length: f64,
    pub avg_assistant_message_length: f64,
    pub session_duration_minutes: f64,
    pub explicit_save_count: usize,
    pub topic_count: usize,
    pub decision_keywords_found: usize,
    pub emotional_keywords_found: usize,
}

impl EngagementSignals {
    /// Compute a normalized engagement score between 0.0 and 1.0.
    ///
    /// Each signal is normalized to 0.0-1.0, then multiplied by its weight.
    /// The final score is the weighted sum divided by the maximum possible score.
    pub fn engagement_score(&self) -> f64 {
        let turn_score = (self.total_turns as f64 / 30.0).min(1.0);
        let msg_score = (self.avg_user_message_length / 200.0).min(1.0);
        let duration_score = (self.session_duration_minutes / 60.0).min(1.0);
        let save_score = (self.explicit_save_count as f64 / 3.0).min(1.0);
        let topic_score = (self.topic_count as f64 / 5.0).min(1.0);
        let decision_score = (self.decision_keywords_found as f64 / 5.0).min(1.0);
        let emotional_score = (self.emotional_keywords_found as f64 / 3.0).min(1.0);

        // Weights for each signal
        let weighted_sum = turn_score * 1.0
            + msg_score * 1.0
            + duration_score * 1.0
            + save_score * 1.0
            + topic_score * 0.5
            + decision_score * 1.0
            + emotional_score * 0.5;

        let max_score = 1.0 + 1.0 + 1.0 + 1.0 + 0.5 + 1.0 + 0.5; // 6.0

        (weighted_sum / max_score).min(1.0)
    }
}

/// How deeply to extract information from a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionDepth {
    Minimal,
    Standard,
    Deep,
}

impl ExtractionDepth {
    /// Maximum number of proposals to generate at this extraction depth.
    pub fn max_proposals(&self) -> usize {
        match self {
            ExtractionDepth::Minimal => 1,
            ExtractionDepth::Standard => 5,
            ExtractionDepth::Deep => 15,
        }
    }
}

/// Determine extraction depth based on engagement signals.
pub fn assess_engagement(signals: &EngagementSignals) -> ExtractionDepth {
    let score = signals.engagement_score();
    if score >= 0.6 {
        ExtractionDepth::Deep
    } else if score >= 0.3 {
        ExtractionDepth::Standard
    } else {
        ExtractionDepth::Minimal
    }
}

/// Count how many of the given keywords appear in the text (case-insensitive).
///
/// Each keyword is counted at most once per occurrence in the text.
pub fn count_keywords(text: &str, keywords: &[&str]) -> usize {
    let lower = text.to_lowercase();
    keywords
        .iter()
        .filter(|kw| lower.contains(&kw.to_lowercase()))
        .count()
}
