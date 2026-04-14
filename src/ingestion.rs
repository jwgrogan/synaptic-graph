// Ingestion pipeline for explicit saves

use crate::db::Database;
use crate::models::*;
use crate::redaction;

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
    };

    db.insert_impulse(&new_impulse)
        .map_err(|e| format!("Failed to insert impulse: {}", e))
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

/// Save content and immediately confirm it.
///
/// Calls explicit_save then confirms the impulse, returning the confirmed impulse.
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
