use std::cmp::Ordering;

use crate::db::Database;
use crate::models::{Assessment, AssessmentStatus, AssessmentType, EmotionalValence, EvidenceSet};

const MAX_MEMORY_CANDIDATES: usize = 24;
const MIN_SHARED_TERMS: usize = 2;
const MIN_OVERLAP_RATIO: f64 = 0.34;
const MIN_CONTRADICTION_CONFIDENCE: f64 = 0.58;

const NEGATION_TERMS: &[&str] = &[
    "avoid", "cant", "cannot", "didnt", "doesnt", "dont", "isnt", "never", "no", "not", "shouldnt",
    "wont",
];

const POSITIVE_CUES: &[&str] = &[
    "always",
    "best",
    "choose",
    "good",
    "great",
    "keep",
    "like",
    "love",
    "prefer",
    "recommend",
    "should",
    "use",
];

const NEGATIVE_CUES: &[&str] = &[
    "avoid", "bad", "dislike", "dont", "hate", "never", "not", "shouldnt", "stop", "worse", "worst",
];

const TOPIC_STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "been", "being", "but", "by", "for", "from", "had",
    "has", "have", "how", "if", "in", "into", "is", "it", "its", "of", "on", "or", "that", "the",
    "their", "them", "then", "there", "these", "they", "this", "those", "to", "too", "very", "was",
    "we", "were", "what", "when", "where", "which", "with", "would",
];

#[derive(Debug, Clone)]
struct MemoryEvidence {
    node_id: String,
    content: String,
    emotional_valence: EmotionalValence,
    updated_at: chrono::DateTime<chrono::Utc>,
    ranking_hint: f64,
}

#[derive(Debug, Clone)]
struct DetectedContradiction {
    subject_node_id: String,
    object_node_id: String,
    confidence: f64,
    rationale: String,
}

pub fn detect_contradictions(
    db: &Database,
    evidence_set: &EvidenceSet,
    max_results: usize,
) -> Result<Vec<Assessment>, String> {
    let memories = load_memory_evidence(db, evidence_set)?;
    if memories.len() < 2 || max_results == 0 {
        return Ok(Vec::new());
    }

    let mut detected = Vec::new();
    for (index, left) in memories.iter().enumerate() {
        for right in memories.iter().skip(index + 1) {
            if let Some(candidate) = detect_pair(left, right) {
                detected.push(candidate);
            }
        }
    }

    detected.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(Ordering::Equal)
    });

    let mut persisted = Vec::new();
    for candidate in detected.into_iter().take(max_results) {
        let existing = db
            .find_assessment_for_pair(
                AssessmentType::Contradiction,
                &candidate.subject_node_id,
                &candidate.object_node_id,
            )
            .map_err(|e| format!("Failed to read contradiction state: {}", e))?;

        if is_dismissal_still_authoritative(existing.as_ref(), &memories, &candidate) {
            continue;
        }

        let assessment = db
            .upsert_pair_assessment(
                AssessmentType::Contradiction,
                &candidate.subject_node_id,
                &candidate.object_node_id,
                candidate.confidence,
                &candidate.rationale,
            )
            .map_err(|e| format!("Failed to persist contradiction assessment: {}", e))?;
        persisted.push(assessment);
    }

    Ok(persisted)
}

fn load_memory_evidence(
    db: &Database,
    evidence_set: &EvidenceSet,
) -> Result<Vec<MemoryEvidence>, String> {
    let mut memories = Vec::new();

    for node_id in &evidence_set.node_ids {
        let node = match db.get_canonical_node(node_id) {
            Ok(node) => node,
            Err(_) => continue,
        };

        if node.kind != crate::graph::GraphNodeKind::Memory {
            continue;
        }

        if matches!(node.status.as_str(), "deleted" | "superseded") {
            continue;
        }

        let payload = db
            .get_canonical_memory_payload(node_id)
            .map_err(|e| format!("Failed to load memory payload for {}: {}", node_id, e))?;

        let valence = EmotionalValence::from_str(&payload.emotional_valence)
            .unwrap_or(EmotionalValence::Neutral);
        let ranking_hint = node.weight
            * crate::confidence::ranking_multiplier(crate::confidence::effective_confidence(
                node.helpful_count,
                node.unhelpful_count,
            ));

        memories.push(MemoryEvidence {
            node_id: node.id,
            content: payload.content,
            emotional_valence: valence,
            updated_at: node.updated_at,
            ranking_hint,
        });
    }

    memories.sort_by(|a, b| {
        b.ranking_hint
            .partial_cmp(&a.ranking_hint)
            .unwrap_or(Ordering::Equal)
    });
    memories.truncate(MAX_MEMORY_CANDIDATES);

    Ok(memories)
}

fn detect_pair(left: &MemoryEvidence, right: &MemoryEvidence) -> Option<DetectedContradiction> {
    let left_tokens = normalized_tokens(&left.content);
    let right_tokens = normalized_tokens(&right.content);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return None;
    }

    let left_topics = topic_terms(&left_tokens);
    let right_topics = topic_terms(&right_tokens);
    if left_topics.is_empty() || right_topics.is_empty() {
        return None;
    }

    let mut overlap = left_topics
        .iter()
        .filter(|term| right_topics.contains(*term))
        .cloned()
        .collect::<Vec<_>>();
    overlap.sort();
    overlap.dedup();

    let overlap_ratio = overlap.len() as f64 / left_topics.len().min(right_topics.len()) as f64;
    if overlap.len() < MIN_SHARED_TERMS && overlap_ratio < MIN_OVERLAP_RATIO {
        return None;
    }

    let left_stance = stance_score(&left_tokens, left.emotional_valence);
    let right_stance = stance_score(&right_tokens, right.emotional_valence);
    let opposite_stance = left_stance * right_stance < 0;
    let negation_conflict = has_negation(&left_tokens) ^ has_negation(&right_tokens);

    if !opposite_stance && !negation_conflict {
        return None;
    }

    let confidence = (0.34
        + (overlap_ratio * 0.32)
        + if opposite_stance { 0.22 } else { 0.0 }
        + if negation_conflict { 0.12 } else { 0.0 })
    .clamp(0.0, 0.95);

    if confidence < MIN_CONTRADICTION_CONFIDENCE {
        return None;
    }

    let rationale = build_rationale(
        &overlap,
        opposite_stance,
        negation_conflict,
        &left.content,
        &right.content,
    );
    let (subject_node_id, object_node_id) = if left.node_id <= right.node_id {
        (left.node_id.clone(), right.node_id.clone())
    } else {
        (right.node_id.clone(), left.node_id.clone())
    };

    Some(DetectedContradiction {
        subject_node_id,
        object_node_id,
        confidence,
        rationale,
    })
}

fn is_dismissal_still_authoritative(
    existing: Option<&Assessment>,
    memories: &[MemoryEvidence],
    candidate: &DetectedContradiction,
) -> bool {
    let Some(existing) = existing else {
        return false;
    };

    if existing.status != AssessmentStatus::Dismissed {
        return false;
    }

    let Some(dismissed_at) = existing.dismissed_at else {
        return false;
    };

    let subject_updated = memories
        .iter()
        .find(|memory| memory.node_id == candidate.subject_node_id)
        .map(|memory| memory.updated_at)
        .unwrap_or(dismissed_at);
    let object_updated = memories
        .iter()
        .find(|memory| memory.node_id == candidate.object_node_id)
        .map(|memory| memory.updated_at)
        .unwrap_or(dismissed_at);

    subject_updated <= dismissed_at && object_updated <= dismissed_at
}

fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .replace("don't", "dont")
        .replace("doesn't", "doesnt")
        .replace("didn't", "didnt")
        .replace("can't", "cant")
        .replace("cannot", "cant")
        .replace("won't", "wont")
        .replace("shouldn't", "shouldnt")
        .replace("isn't", "isnt")
}

fn normalized_tokens(text: &str) -> Vec<String> {
    normalize_text(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect()
}

fn topic_terms(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| token.len() >= 3)
        .filter(|token| !TOPIC_STOP_WORDS.contains(&token.as_str()))
        .filter(|token| !POSITIVE_CUES.contains(&token.as_str()))
        .filter(|token| !NEGATIVE_CUES.contains(&token.as_str()))
        .filter(|token| !NEGATION_TERMS.contains(&token.as_str()))
        .cloned()
        .collect()
}

fn stance_score(tokens: &[String], emotional_valence: EmotionalValence) -> i32 {
    let positive = tokens
        .iter()
        .filter(|token| POSITIVE_CUES.contains(&token.as_str()))
        .count() as i32;
    let negative = tokens
        .iter()
        .filter(|token| {
            NEGATIVE_CUES.contains(&token.as_str()) || NEGATION_TERMS.contains(&token.as_str())
        })
        .count() as i32;

    let valence_adjustment = match emotional_valence {
        EmotionalValence::Positive => 1,
        EmotionalValence::Negative => -1,
        EmotionalValence::Neutral => 0,
    };

    positive - negative + valence_adjustment
}

fn has_negation(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| NEGATION_TERMS.contains(&token.as_str()))
}

fn build_rationale(
    overlap: &[String],
    opposite_stance: bool,
    negation_conflict: bool,
    left_content: &str,
    right_content: &str,
) -> String {
    let shared_terms = overlap
        .iter()
        .take(4)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");

    let mut reasons = Vec::new();
    if opposite_stance {
        reasons.push("opposing stance");
    }
    if negation_conflict {
        reasons.push("asymmetric negation");
    }

    format!(
        "Possible contradiction around [{}] due to {}. Compare '{}' with '{}'.",
        shared_terms,
        reasons.join(" and "),
        left_content,
        right_content
    )
}
