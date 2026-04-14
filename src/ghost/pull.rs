// Ghost graph pull-through — promotes ghost nodes to full impulses

use std::fs;
use std::path::Path;

use crate::db::Database;
use crate::models::*;
use crate::weight;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullMode {
    SessionOnly,
    Permanent,
}

pub fn pull_ghost_content(
    db: &Database,
    ghost_node: &GhostNode,
    root_path: &str,
    mode: PullMode,
) -> Result<String, String> {
    let file_path = Path::new(root_path).join(&ghost_node.external_ref);

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

    // Update ghost node weight (reinforcement — access pattern is remembered regardless of mode)
    let new_weight = weight::reinforce(ghost_node.weight);
    db.update_ghost_node_weight(&ghost_node.id, new_weight)
        .map_err(|e| format!("Failed to update ghost weight: {}", e))?;
    db.touch_ghost_node(&ghost_node.id)
        .map_err(|e| format!("Failed to touch ghost node: {}", e))?;

    if mode == PullMode::Permanent {
        // Extract impulses from the content rather than storing raw file.
        // For now, use a simple heuristic: split on headings/paragraphs and
        // create one impulse per meaningful section. In production this would
        // be an LLM extraction call.
        let extracted = extract_impulses_from_content(&content);

        for extracted_content in &extracted {
            // Redact before persisting
            let redacted = crate::redaction::redact(extracted_content);

            let input = NewImpulse {
                content: redacted.clean_content,
                impulse_type: ImpulseType::Observation,
                initial_weight: WEIGHT_PULL_THROUGH,
                emotional_valence: EmotionalValence::Neutral,
                engagement_level: EngagementLevel::Medium,
                source_signals: vec![format!("ghost_pull:{}", ghost_node.source_graph)],
                source_type: SourceType::PullThrough,
                source_ref: format!("{}:{}", ghost_node.source_graph, ghost_node.external_ref),
            };

            let impulse = db.insert_impulse(&input)
                .map_err(|e| format!("Failed to create impulse from pull: {}", e))?;
            db.confirm_impulse(&impulse.id)
                .map_err(|e| format!("Failed to confirm: {}", e))?;
        }
    }

    Ok(content)
}

/// Simple extraction: split content into meaningful sections by headings.
/// Each non-empty section becomes a candidate impulse.
/// In production, this would be an LLM call to extract impulses/insights.
pub fn extract_impulses_from_content(content: &str) -> Vec<String> {
    let mut sections = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        if line.starts_with('#') && !current.trim().is_empty() {
            let trimmed = current.trim().to_string();
            if trimmed.len() > 10 {
                sections.push(trimmed);
            }
            current = line.to_string() + "\n";
        } else {
            current.push_str(line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() && current.trim().len() > 10 {
        sections.push(current.trim().to_string());
    }

    if sections.is_empty() && content.trim().len() > 10 {
        sections.push(content.trim().to_string());
    }

    sections
}
