use crate::confidence::{effective_confidence, ranking_multiplier};
use crate::db::Database;
use crate::models::{
    AssessmentStatus, EvidenceSet, ReflectionAssessmentItem, ReflectionGhostItem,
    ReflectionMemoryItem, ReflectionPacket, ReflectionRelationship, ReflectionSkillItem,
};

pub fn build_reflection_packet(
    db: &Database,
    evidence_set: &EvidenceSet,
    max_memories: usize,
    max_relationships: usize,
) -> Result<ReflectionPacket, String> {
    let mut memory_items = Vec::new();
    let mut skill_items = Vec::new();
    let mut ghost_items = Vec::new();
    let mut relationships = Vec::new();
    let mut assessment_items = Vec::new();

    for node_id in &evidence_set.node_ids {
        if let Ok(node) = db.get_canonical_node(node_id) {
            match node.kind {
                crate::graph::GraphNodeKind::Memory => {
                    let payload = db.get_canonical_memory_payload(node_id).map_err(|e| {
                        format!("Failed to load memory payload for {}: {}", node_id, e)
                    })?;
                    let tags = db
                        .get_tags_for_impulse(node_id)
                        .map_err(|e| format!("Failed to load tags for {}: {}", node_id, e))?
                        .into_iter()
                        .map(|tag| tag.name)
                        .collect::<Vec<_>>();

                    memory_items.push(ReflectionMemoryItem {
                        node_id: node.id,
                        content: payload.content,
                        impulse_type: payload.impulse_type,
                        status: node.status,
                        weight: node.weight,
                        confidence: node.confidence,
                        effective_confidence: effective_confidence(
                            node.helpful_count,
                            node.unhelpful_count,
                        ),
                        helpful_count: node.helpful_count,
                        unhelpful_count: node.unhelpful_count,
                        tags,
                    });
                }
                crate::graph::GraphNodeKind::Skill => {
                    let skill = db.get_skill(node_id).map_err(|e| {
                        format!("Failed to load skill payload for {}: {}", node_id, e)
                    })?;
                    let evidence_node_ids =
                        db.get_skill_evidence_node_ids(node_id).map_err(|e| {
                            format!("Failed to load skill evidence for {}: {}", node_id, e)
                        })?;
                    skill_items.push(ReflectionSkillItem {
                        node_id: node.id,
                        name: skill.name,
                        description: skill.description,
                        trigger: skill.trigger,
                        steps: skill.steps,
                        constraints: skill.constraints,
                        weight: node.weight,
                        confidence: node.confidence,
                        effective_confidence: effective_confidence(
                            node.helpful_count,
                            node.unhelpful_count,
                        ),
                        evidence_node_ids,
                    });
                }
                crate::graph::GraphNodeKind::Ghost => {
                    let ghost = db
                        .get_ghost_node(node_id)
                        .map_err(|e| format!("Failed to load ghost node for {}: {}", node_id, e))?;
                    ghost_items.push(ReflectionGhostItem {
                        node_id: node.id,
                        source_graph: ghost.source_graph,
                        external_ref: ghost.external_ref,
                        title: ghost.title,
                        weight: node.weight,
                        confidence: node.confidence,
                    });
                }
            }
        }
    }

    for edge_id in &evidence_set.edge_ids {
        let edge = db
            .get_canonical_edge(edge_id)
            .map_err(|e| format!("Failed to load canonical edge for {}: {}", edge_id, e))?;
        relationships.push(ReflectionRelationship {
            edge_id: edge.id,
            source_id: edge.source_id,
            target_id: edge.target_id,
            relationship: edge.relationship,
            weight: edge.weight,
            confidence: edge.confidence,
        });
    }

    let assessments = db
        .list_assessments_for_node_ids(&evidence_set.node_ids)
        .map_err(|e| {
            format!(
                "Failed to load assessments for evidence set {}: {}",
                evidence_set.id, e
            )
        })?;
    for assessment in assessments {
        if assessment.status == AssessmentStatus::Dismissed {
            continue;
        }

        assessment_items.push(ReflectionAssessmentItem {
            assessment_id: assessment.id,
            assessment_type: assessment.assessment_type.as_str().to_string(),
            status: assessment.status.as_str().to_string(),
            subject_node_id: assessment.subject_node_id,
            object_node_id: assessment.object_node_id,
            confidence: assessment.confidence,
            rationale: assessment.rationale,
        });
    }

    memory_items.sort_by(|a, b| {
        let a_rank = a.weight * ranking_multiplier(a.effective_confidence);
        let b_rank = b.weight * ranking_multiplier(b.effective_confidence);
        b_rank
            .partial_cmp(&a_rank)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    relationships.sort_by(|a, b| {
        b.weight
            .partial_cmp(&a.weight)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    assessment_items.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let memory_truncated = memory_items.len() > max_memories;
    let relationship_truncated = relationships.len() > max_relationships;
    let assessment_limit = max_relationships.clamp(4, 12);
    let assessment_truncated = assessment_items.len() > assessment_limit;
    memory_items.truncate(max_memories);
    relationships.truncate(max_relationships);
    assessment_items.truncate(assessment_limit);

    Ok(ReflectionPacket {
        evidence_set_id: evidence_set.id.clone(),
        query: evidence_set.query.clone(),
        created_at: evidence_set.created_at,
        expires_at: evidence_set.expires_at,
        memory_items,
        skill_items,
        ghost_items,
        relationships,
        assessment_items,
        truncated: memory_truncated || relationship_truncated || assessment_truncated,
        instruction: "Synthesize only from these evidence items. Cite node_id or edge_id in any downstream reasoning artifacts and do not introduce claims not grounded in this packet.".to_string(),
    })
}
