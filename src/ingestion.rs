// Ingestion pipeline for explicit saves

use std::collections::HashSet;

use crate::db::Database;
use crate::models::*;
use crate::redaction;

// Common stop words to exclude from keyword matching
const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "shall", "can", "need", "dare", "ought",
    "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
    "as", "into", "through", "during", "before", "after", "above", "below",
    "between", "out", "off", "over", "under", "again", "further", "then",
    "once", "here", "there", "when", "where", "why", "how", "all", "each",
    "every", "both", "few", "more", "most", "other", "some", "such", "no",
    "nor", "not", "only", "own", "same", "so", "than", "too", "very",
    "just", "because", "but", "and", "or", "if", "while", "that", "this",
    "these", "those", "it", "its", "my", "your", "his", "her", "our",
    "their", "what", "which", "who", "whom", "i", "me", "we", "you",
    "he", "she", "they", "them", "about", "up",
];

/// Save content as a candidate impulse with redaction applied.
///
/// Validates that content is not empty, runs redaction to strip secrets/PII,
/// then inserts the impulse with WEIGHT_EXPLICIT_SAVE as initial weight.
/// The impulse is created with Candidate status.
pub fn explicit_save(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
) -> Result<Impulse, String> {
    if content.trim().is_empty() {
        return Err("Content must not be empty".to_string());
    }

    let redacted = redaction::redact(content);

    let new_impulse = NewImpulse {
        content: redacted.clean_content,
        impulse_type,
        initial_weight: WEIGHT_EXPLICIT_SAVE,
        emotional_valence,
        engagement_level,
        source_signals,
        source_type: SourceType::ExplicitSave,
        source_ref: source_ref.to_string(),
        source_provider: "unknown".to_string(),
        source_account: String::new(),
    };

    db.insert_impulse(&new_impulse)
        .map_err(|e| format!("Failed to insert impulse: {}", e))
}

/// Save content as a candidate impulse with source provider info.
#[allow(clippy::too_many_arguments)]
pub fn explicit_save_with_provider(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
    source_provider: &str,
    source_account: &str,
) -> Result<Impulse, String> {
    if content.trim().is_empty() {
        return Err("Content must not be empty".to_string());
    }

    let redacted = redaction::redact(content);

    let new_impulse = NewImpulse {
        content: redacted.clean_content,
        impulse_type,
        initial_weight: WEIGHT_EXPLICIT_SAVE,
        emotional_valence,
        engagement_level,
        source_signals,
        source_type: SourceType::ExplicitSave,
        source_ref: source_ref.to_string(),
        source_provider: source_provider.to_string(),
        source_account: source_account.to_string(),
    };

    db.insert_impulse(&new_impulse)
        .map_err(|e| format!("Failed to insert impulse: {}", e))
}

/// Save content and immediately confirm it with source provider info.
#[allow(clippy::too_many_arguments)]
pub fn save_and_confirm_with_provider(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
    source_provider: &str,
    source_account: &str,
) -> Result<Impulse, String> {
    let impulse = explicit_save_with_provider(
        db,
        content,
        impulse_type,
        emotional_valence,
        engagement_level,
        source_signals,
        source_ref,
        source_provider,
        source_account,
    )?;

    db.confirm_impulse(&impulse.id)
        .map_err(|e| format!("Failed to confirm impulse: {}", e))?;

    let _ = auto_link(db, &impulse.id);

    db.get_impulse(&impulse.id)
        .map_err(|e| format!("Failed to retrieve confirmed impulse: {}", e))
}

/// Save content as a candidate impulse and also insert connections.
///
/// Each connection is a tuple of (target_id, relationship, weight).
#[allow(clippy::too_many_arguments)]
pub fn explicit_save_with_connections(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
    connections: &[(String, String, f64)],
) -> Result<Impulse, String> {
    let impulse = explicit_save(
        db,
        content,
        impulse_type,
        emotional_valence,
        engagement_level,
        source_signals,
        source_ref,
    )?;

    for (target_id, relationship, weight) in connections {
        let new_conn = NewConnection {
            source_id: impulse.id.clone(),
            target_id: target_id.clone(),
            weight: *weight,
            relationship: relationship.clone(),
        };
        db.insert_connection(&new_conn)
            .map_err(|e| format!("Failed to insert connection: {}", e))?;
    }

    Ok(impulse)
}

/// Save content and immediately confirm it, then auto-link to related memories.
///
/// Calls explicit_save, confirms the impulse, runs auto_link, returns the confirmed impulse.
pub fn save_and_confirm(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
) -> Result<Impulse, String> {
    let impulse = explicit_save(
        db,
        content,
        impulse_type,
        emotional_valence,
        engagement_level,
        source_signals,
        source_ref,
    )?;

    db.confirm_impulse(&impulse.id)
        .map_err(|e| format!("Failed to confirm impulse: {}", e))?;

    // Auto-link to existing memories by keyword overlap
    let _ = auto_link(db, &impulse.id);

    db.get_impulse(&impulse.id)
        .map_err(|e| format!("Failed to retrieve confirmed impulse: {}", e))
}

/// Save content with connections and immediately confirm it.
#[allow(clippy::too_many_arguments)]
pub fn save_and_confirm_with_connections(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
    connections: &[(String, String, f64)],
) -> Result<Impulse, String> {
    let impulse = explicit_save_with_connections(
        db,
        content,
        impulse_type,
        emotional_valence,
        engagement_level,
        source_signals,
        source_ref,
        connections,
    )?;

    db.confirm_impulse(&impulse.id)
        .map_err(|e| format!("Failed to confirm impulse: {}", e))?;

    db.get_impulse(&impulse.id)
        .map_err(|e| format!("Failed to retrieve confirmed impulse: {}", e))
}

/// Extract significant keywords from text (lowercased, stop words removed, 3+ chars).
fn extract_keywords(text: &str) -> HashSet<String> {
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 3 && !stop.contains(w.as_str()))
        .collect()
}

/// Auto-detect and create connections between a new impulse and existing confirmed impulses.
/// Uses keyword overlap: shared significant words create connections.
/// Connection weight scales with overlap ratio. Returns number of connections created.
pub fn auto_link(db: &Database, impulse_id: &str) -> Result<usize, String> {
    let impulse = db.get_impulse(impulse_id)
        .map_err(|e| format!("Impulse not found: {}", e))?;

    let new_keywords = extract_keywords(&impulse.content);
    if new_keywords.is_empty() {
        return Ok(0);
    }

    let existing = db.list_impulses(Some(ImpulseStatus::Confirmed))
        .map_err(|e| format!("Failed to list impulses: {}", e))?;

    let existing_conns = db.get_connections_for_node(impulse_id)
        .map_err(|e| format!("Failed to get connections: {}", e))?;
    let already_connected: HashSet<String> = existing_conns.iter()
        .flat_map(|c| vec![c.source_id.clone(), c.target_id.clone()])
        .collect();

    let mut created = 0;

    for other in &existing {
        if other.id == impulse.id || already_connected.contains(&other.id) {
            continue;
        }

        let other_keywords = extract_keywords(&other.content);
        if other_keywords.is_empty() {
            continue;
        }

        let overlap: HashSet<_> = new_keywords.intersection(&other_keywords).collect();
        let overlap_count = overlap.len();
        if overlap_count == 0 {
            continue;
        }

        let min_size = new_keywords.len().min(other_keywords.len());
        let ratio = overlap_count as f64 / min_size as f64;

        // Connect if at least 1 shared keyword and ratio >= 0.15
        if ratio >= 0.15 {
            let weight = (ratio * 0.8).min(0.9);
            let conn = NewConnection {
                source_id: impulse.id.clone(),
                target_id: other.id.clone(),
                weight,
                relationship: "relates_to".to_string(),
            };
            db.insert_connection(&conn)
                .map_err(|e| format!("Failed to create connection: {}", e))?;
            created += 1;
        }
    }

    Ok(created)
}

/// Link two impulses manually with a specified relationship.
pub fn manual_link(
    db: &Database,
    source_id: &str,
    target_id: &str,
    relationship: &str,
    weight: f64,
) -> Result<Connection, String> {
    db.get_impulse(source_id).map_err(|e| format!("Source not found: {}", e))?;
    db.get_impulse(target_id).map_err(|e| format!("Target not found: {}", e))?;

    let conn = NewConnection {
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        weight: weight.clamp(0.0, 1.0),
        relationship: relationship.to_string(),
    };
    db.insert_connection(&conn)
        .map_err(|e| format!("Failed to create connection: {}", e))
}

/// Remove a connection by ID.
pub fn unlink(db: &Database, connection_id: &str) -> Result<(), String> {
    db.delete_connection(connection_id)
        .map_err(|e| format!("Failed to delete connection: {}", e))
}
