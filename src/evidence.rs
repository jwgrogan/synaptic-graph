use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::db::Database;
use crate::models::{EvidenceSet, RetrievalRequest, RetrievalResult};

pub const DEFAULT_EVIDENCE_SET_TTL_HOURS: i64 = 24 * 7;

pub fn persist_retrieval_evidence(
    db: &Database,
    request: &RetrievalRequest,
    result: &RetrievalResult,
) -> Result<EvidenceSet, String> {
    let node_ids = collect_node_ids(result);
    let edge_ids = collect_edge_ids(db, result)?;
    let response_hash = hash_retrieval_packet(request, result, &node_ids, &edge_ids)?;

    db.create_evidence_set(
        &request.query,
        &response_hash,
        &node_ids,
        &edge_ids,
        Some(DEFAULT_EVIDENCE_SET_TTL_HOURS),
    )
    .map_err(|e| format!("Failed to persist evidence set: {}", e))
}

fn collect_node_ids(result: &RetrievalResult) -> Vec<String> {
    let mut ids = BTreeSet::new();

    for memory in &result.memories {
        ids.insert(memory.impulse.id.clone());
        for node_id in &memory.activation_path {
            ids.insert(node_id.clone());
        }
    }

    for skill in &result.skills {
        ids.insert(skill.skill.node_id.clone());
        for node_id in &skill.evidence_node_ids {
            ids.insert(node_id.clone());
        }
    }

    for ghost in &result.ghost_activations {
        ids.insert(ghost.ghost_node.id.clone());
    }

    ids.into_iter().collect()
}

fn collect_edge_ids(db: &Database, result: &RetrievalResult) -> Result<Vec<String>, String> {
    let mut ids = BTreeSet::new();

    for memory in &result.memories {
        for pair in memory.activation_path.windows(2) {
            if let Some(edge) = db
                .find_canonical_edge_between(&pair[0], &pair[1])
                .map_err(|e| format!("Failed to resolve activation edge: {}", e))?
            {
                ids.insert(edge.id);
            }
        }
    }

    Ok(ids.into_iter().collect())
}

fn hash_retrieval_packet(
    request: &RetrievalRequest,
    result: &RetrievalResult,
    node_ids: &[String],
    edge_ids: &[String],
) -> Result<String, String> {
    let memory_ids = result
        .memories
        .iter()
        .map(|memory| memory.impulse.id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let skill_ids = result
        .skills
        .iter()
        .map(|skill| skill.skill.node_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let ghost_ids = result
        .ghost_activations
        .iter()
        .map(|ghost| ghost.ghost_node.id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let packet = serde_json::json!({
        "query": request.query.trim(),
        "max_results": request.max_results,
        "max_hops": request.max_hops,
        "memory_ids": memory_ids,
        "skill_ids": skill_ids,
        "ghost_ids": ghost_ids,
        "node_ids": node_ids,
        "edge_ids": edge_ids,
        "result_counts": {
            "memories": result.memories.len(),
            "skills": result.skills.len(),
            "ghosts": result.ghost_activations.len(),
        },
    });

    let bytes = serde_json::to_vec(&packet)
        .map_err(|e| format!("Failed to serialize retrieval packet: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
