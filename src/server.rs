// MCP tool handlers

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use crate::activation::ActivationEngine;
use crate::assessments;
use crate::backup;
use crate::db::Database;
use crate::evidence;
use crate::ghost;
use crate::ingestion;
use crate::models::*;
use crate::reflection;
use crate::session::Session;
use crate::sync;

use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{schemars, tool, ServerHandler};

fn retrieve_skill_matches(
    db: &Database,
    query: &str,
    max_results: usize,
) -> Result<Vec<RetrievedSkill>, String> {
    let matches = db
        .search_skills_fts(query)
        .map_err(|e| format!("Skill FTS search failed: {}", e))?;

    let mut results = Vec::new();
    for (skill_id, rank) in matches.into_iter().take(max_results) {
        let node = match db.get_canonical_node(&skill_id) {
            Ok(node) => node,
            Err(_) => continue,
        };
        let skill = match db.get_skill(&skill_id) {
            Ok(skill) => skill,
            Err(_) => continue,
        };
        let effective_confidence =
            crate::confidence::effective_confidence(node.helpful_count, node.unhelpful_count);
        let base_score = (-rank).clamp(0.1, 1.0);
        let ranking_score =
            base_score * crate::confidence::ranking_multiplier(effective_confidence);
        let evidence_node_ids = db
            .get_skill_evidence_node_ids(&skill_id)
            .unwrap_or_default();

        results.push(RetrievedSkill {
            skill,
            weight: node.weight,
            confidence: node.confidence,
            effective_confidence,
            ranking_score,
            evidence_node_ids,
        });
    }

    results.sort_by(|a, b| {
        b.ranking_score
            .partial_cmp(&a.ranking_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(results)
}

#[derive(Debug, Default)]
struct CompressionSanitization {
    sanitized_content: String,
    suppressed_evidence_set_ids: Vec<String>,
    suppressed_hashes: Vec<String>,
    stripped_line_count: usize,
    stripped_char_count: usize,
}

fn sanitize_for_pre_compression(
    db: &Database,
    session_content: &str,
    recent_evidence_set_ids: &[String],
) -> CompressionSanitization {
    let mut suppressed_hashes = BTreeSet::new();

    for evidence_set_id in recent_evidence_set_ids {
        let evidence_set = match db.get_evidence_set(evidence_set_id) {
            Ok(evidence_set) => evidence_set,
            Err(_) => continue,
        };
        if !evidence_set.response_hash.is_empty() {
            suppressed_hashes.insert(evidence_set.response_hash.clone());
        }
    }

    let mut kept_lines = Vec::new();
    let mut suppressed_ids = BTreeSet::new();
    let mut stripped_line_count = 0usize;
    let mut stripped_char_count = 0usize;
    let mut inside_graph_block = false;
    let mut graph_block_closer = "";
    let mut inside_recall_block = false;

    for line in session_content.lines() {
        let trimmed = line.trim();

        if inside_graph_block {
            stripped_line_count += 1;
            stripped_char_count += line.len();
            if trimmed == graph_block_closer {
                inside_graph_block = false;
                graph_block_closer = "";
            }
            suppressed_ids.extend(recent_evidence_set_ids.iter().cloned());
            continue;
        }

        if trimmed == "<!-- synaptic-graph:start -->" {
            inside_graph_block = true;
            graph_block_closer = "<!-- synaptic-graph:end -->";
            stripped_line_count += 1;
            stripped_char_count += line.len();
            suppressed_ids.extend(recent_evidence_set_ids.iter().cloned());
            continue;
        }

        if trimmed.starts_with("```synaptic-graph") || trimmed.starts_with("```memory-graph") {
            inside_graph_block = true;
            graph_block_closer = "```";
            stripped_line_count += 1;
            stripped_char_count += line.len();
            suppressed_ids.extend(recent_evidence_set_ids.iter().cloned());
            continue;
        }

        if trimmed.starts_with("[Recalled memories") {
            inside_recall_block = true;
            stripped_line_count += 1;
            stripped_char_count += line.len();
            suppressed_ids.extend(recent_evidence_set_ids.iter().cloned());
            continue;
        }

        if inside_recall_block {
            if trimmed.is_empty() || trimmed.starts_with("- ") {
                stripped_line_count += 1;
                stripped_char_count += line.len();
                suppressed_ids.extend(recent_evidence_set_ids.iter().cloned());
                if trimmed.is_empty() {
                    inside_recall_block = false;
                }
                continue;
            }
            inside_recall_block = false;
        }

        kept_lines.push(line.to_string());
    }

    CompressionSanitization {
        sanitized_content: collapse_blank_lines(&kept_lines.join("\n")),
        suppressed_evidence_set_ids: suppressed_ids.into_iter().collect(),
        suppressed_hashes: suppressed_hashes.into_iter().collect(),
        stripped_line_count,
        stripped_char_count,
    }
}

fn collapse_blank_lines(content: &str) -> String {
    let mut lines = Vec::new();
    let mut last_blank = false;

    for line in content.lines() {
        let is_blank = line.trim().is_empty();
        if is_blank && last_blank {
            continue;
        }
        lines.push(line);
        last_blank = is_blank;
    }

    lines.join("\n").trim().to_string()
}

pub struct MemoryGraphServer {
    db: Mutex<Database>,
    session: Mutex<Session>,
}

impl MemoryGraphServer {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let db = Database::open(db_path).map_err(|e| format!("DB open failed: {}", e))?;
        Ok(Self {
            db: Mutex::new(db),
            session: Mutex::new(Session::new(&uuid::Uuid::new_v4().to_string())),
        })
    }

    pub fn new_with_db(db: Database) -> Self {
        Self {
            db: Mutex::new(db),
            session: Mutex::new(Session::new(&uuid::Uuid::new_v4().to_string())),
        }
    }

    pub fn is_incognito(&self) -> bool {
        self.session.lock().unwrap().is_incognito()
    }

    pub fn set_incognito(&self, incognito: bool) {
        self.session.lock().unwrap().set_incognito(incognito);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_save_memory(
        &self,
        content: String,
        impulse_type: String,
        emotional_valence: Option<String>,
        engagement_level: Option<String>,
        source_ref: Option<String>,
        source_provider: Option<String>,
        source_account: Option<String>,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot save memory in incognito mode".to_string());
        }

        let itype = ImpulseType::from_str(&impulse_type)
            .ok_or_else(|| format!("Invalid impulse type: {}", impulse_type))?;

        let valence = emotional_valence
            .as_deref()
            .map(EmotionalValence::from_str)
            .unwrap_or(Some(EmotionalValence::Neutral))
            .ok_or("Invalid emotional valence")?;

        let engagement = engagement_level
            .as_deref()
            .map(EngagementLevel::from_str)
            .unwrap_or(Some(EngagementLevel::Medium))
            .ok_or("Invalid engagement level")?;

        let sref = source_ref.unwrap_or_default();
        let provider = source_provider.unwrap_or_else(|| "unknown".to_string());
        let account = source_account.unwrap_or_default();

        let db = self.db.lock().unwrap();
        let impulse = ingestion::explicit_save_with_provider(
            &db,
            &content,
            itype,
            valence,
            engagement,
            vec![],
            &sref,
            &provider,
            &account,
        )?;

        serde_json::to_string_pretty(&impulse).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_retrieve_context(
        &self,
        query: String,
        max_results: Option<usize>,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let engine = ActivationEngine::new(&db);
        let request = RetrievalRequest {
            query,
            max_results: max_results.unwrap_or(10),
            max_hops: 3,
        };

        let mut result = if self.is_incognito() {
            engine.retrieve_read_only(&request)?
        } else {
            engine.retrieve(&request)?
        };
        result.skills = retrieve_skill_matches(&db, &request.query, request.max_results)?;

        if !self.is_incognito() {
            let evidence_set = evidence::persist_retrieval_evidence(&db, &request, &result)?;
            result.evidence_set = Some(evidence_set);
        }

        serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_delete_memory(&self, id: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot modify memory in incognito mode".to_string());
        }

        let db = self.db.lock().unwrap();
        db.update_impulse_status(&id, ImpulseStatus::Deleted)
            .map_err(|e| format!("Delete failed: {}", e))?;

        Ok(format!("{{\"deleted\": \"{}\"}}", id))
    }

    pub fn handle_update_memory(&self, id: String, new_content: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot modify memory in incognito mode".to_string());
        }

        let db = self.db.lock().unwrap();
        let new_id = db
            .update_impulse_content(&id, &new_content)
            .map_err(|e| format!("Update failed: {}", e))?;

        let new_impulse = db
            .get_canonical_memory_impulse(&new_id)
            .map_err(|e| format!("Fetch failed: {}", e))?;

        serde_json::to_string_pretty(&new_impulse)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_inspect_memory(&self, id: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let impulse = db
            .get_canonical_memory_impulse(&id)
            .map_err(|e| format!("Not found: {}", e))?;
        let canonical = db
            .get_canonical_node(&id)
            .map_err(|e| format!("Canonical node lookup failed: {}", e))?;

        let connections = db
            .get_canonical_edges_for_node(&id)
            .map_err(|e| format!("Connection lookup failed: {}", e))?;

        let tags = db.get_tags_for_impulse(&id).unwrap_or_default();

        let response = serde_json::json!({
            "id": impulse.id,
            "content": impulse.content,
            "impulse_type": impulse.impulse_type,
            "weight": impulse.weight,
            "initial_weight": impulse.initial_weight,
            "emotional_valence": impulse.emotional_valence,
            "engagement_level": impulse.engagement_level,
            "source_signals": impulse.source_signals,
            "created_at": impulse.created_at.to_rfc3339(),
            "last_accessed_at": impulse.last_accessed_at.to_rfc3339(),
            "source_type": impulse.source_type,
            "source_ref": impulse.source_ref,
            "source_provider": impulse.source_provider,
            "source_account": impulse.source_account,
            "status": impulse.status,
            "node_kind": canonical.kind.as_str(),
            "confidence": canonical.confidence,
            "effective_confidence": crate::confidence::effective_confidence(
                canonical.helpful_count,
                canonical.unhelpful_count
            ),
            "helpful_count": canonical.helpful_count,
            "unhelpful_count": canonical.unhelpful_count,
            "tags": tags.iter().map(|t| serde_json::json!({"name": t.name, "color": t.color})).collect::<Vec<_>>(),
            "connections": connections.iter().map(|c| serde_json::json!({
                "id": c.id,
                "source_id": c.source_id,
                "target_id": c.target_id,
                "weight": c.weight,
                "confidence": c.confidence,
                "relationship": c.relationship,
                "traversal_count": c.traversal_count,
            })).collect::<Vec<_>>(),
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_memory_status(&self) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let stats = db
            .memory_stats()
            .map_err(|e| format!("Stats failed: {}", e))?;

        let incognito = self.is_incognito();
        let response = serde_json::json!({
            "total_impulses": stats.total_impulses,
            "confirmed_impulses": stats.confirmed_impulses,
            "candidate_impulses": stats.candidate_impulses,
            "total_connections": stats.total_connections,
            "total_memory_nodes": stats.total_memory_nodes,
            "total_skill_nodes": stats.total_skill_nodes,
            "total_ghost_nodes": stats.total_ghost_nodes,
            "total_graph_edges": stats.total_graph_edges,
            "total_assessments": stats.total_assessments,
            "total_evidence_sets": stats.total_evidence_sets,
            "incognito": incognito,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_set_incognito(&self, enabled: bool) -> Result<String, String> {
        self.set_incognito(enabled);
        Ok(format!("{{\"incognito\": {}}}", enabled))
    }

    pub fn handle_explain_recall(
        &self,
        query: String,
        memory_id: String,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let engine = ActivationEngine::new(&db);

        let request = RetrievalRequest {
            query,
            max_results: 100,
            max_hops: 5,
        };

        let result = engine.retrieve_read_only(&request)?;

        let explanation = result
            .memories
            .iter()
            .find(|m| m.impulse.id == memory_id)
            .map(|m| {
                serde_json::json!({
                    "memory_id": m.impulse.id,
                    "activation_score": m.activation_score,
                    "confidence_score": m.confidence_score,
                    "ranking_score": m.ranking_score,
                    "activation_path": m.activation_path,
                    "content": m.impulse.content,
                })
            })
            .unwrap_or_else(|| {
                serde_json::json!({
                    "memory_id": memory_id,
                    "error": "Memory was not activated by this query"
                })
            });

        serde_json::to_string_pretty(&explanation)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_confirm_proposal(&self, id: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot confirm memory in incognito mode".to_string());
        }

        let db = self.db.lock().unwrap();
        db.confirm_impulse(&id)
            .map_err(|e| format!("Confirm failed: {}", e))?;

        let impulse = db
            .get_canonical_memory_impulse(&id)
            .map_err(|e| format!("Fetch failed: {}", e))?;

        serde_json::to_string_pretty(&impulse).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_dismiss_proposal(&self, id: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot dismiss memory in incognito mode".to_string());
        }

        let db = self.db.lock().unwrap();
        db.dismiss_impulse(&id)
            .map_err(|e| format!("Dismiss failed: {}", e))?;

        Ok(format!("{{\"dismissed\": \"{}\"}}", id))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_quick_save(
        &self,
        content: String,
        impulse_type: String,
        emotional_valence: Option<String>,
        engagement_level: Option<String>,
        source_ref: Option<String>,
        source_provider: Option<String>,
        source_account: Option<String>,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot save memory in incognito mode".to_string());
        }

        let itype = ImpulseType::from_str(&impulse_type)
            .ok_or_else(|| format!("Invalid impulse type: {}", impulse_type))?;

        let valence = emotional_valence
            .as_deref()
            .map(EmotionalValence::from_str)
            .unwrap_or(Some(EmotionalValence::Neutral))
            .ok_or("Invalid emotional valence")?;

        let engagement = engagement_level
            .as_deref()
            .map(EngagementLevel::from_str)
            .unwrap_or(Some(EngagementLevel::Medium))
            .ok_or("Invalid engagement level")?;

        let sref = source_ref.unwrap_or_default();
        let provider = source_provider.unwrap_or_else(|| "unknown".to_string());
        let account = source_account.unwrap_or_default();

        let db = self.db.lock().unwrap();
        let impulse = ingestion::save_and_confirm_with_provider(
            &db,
            &content,
            itype,
            valence,
            engagement,
            vec![],
            &sref,
            &provider,
            &account,
        )?;

        serde_json::to_string_pretty(&impulse).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_list_candidates(&self) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let candidates = db
            .list_candidates()
            .map_err(|e| format!("List failed: {}", e))?;

        serde_json::to_string_pretty(&candidates).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_link_memories(
        &self,
        source_id: String,
        target_id: String,
        relationship: Option<String>,
        weight: Option<f64>,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot link memories in incognito mode".to_string());
        }
        let db = self.db.lock().unwrap();
        let rel = relationship.unwrap_or_else(|| "relates_to".to_string());
        let w = weight.unwrap_or(0.5);
        let conn = ingestion::manual_link(&db, &source_id, &target_id, &rel, w)?;
        serde_json::to_string_pretty(&conn).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_unlink_memories(&self, connection_id: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot unlink memories in incognito mode".to_string());
        }
        let db = self.db.lock().unwrap();
        ingestion::unlink(&db, &connection_id)?;
        Ok(format!("{{\"unlinked\": \"{}\"}}", connection_id))
    }

    pub fn handle_register_ghost_graph(
        &self,
        name: String,
        root_path: String,
        source_type: Option<String>,
        ignore_patterns: Option<Vec<String>>,
    ) -> Result<String, String> {
        let stype = source_type.unwrap_or_else(|| "directory".to_string());
        let config = ghost::scanner::ScanConfig {
            extensions: vec!["md".to_string()],
            ignore_patterns: ignore_patterns.unwrap_or_default(),
        };

        let db = self.db.lock().unwrap();
        let count = ghost::register_and_scan(&db, &name, &root_path, &stype, &config)?;

        let response = serde_json::json!({
            "name": name,
            "root_path": root_path,
            "source_type": stype,
            "nodes_scanned": count,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_refresh_ghost_graph(&self, name: String) -> Result<String, String> {
        let config = ghost::scanner::ScanConfig {
            extensions: vec!["md".to_string()],
            ignore_patterns: vec![],
        };

        let db = self.db.lock().unwrap();
        let count = ghost::refresh(&db, &name, &config)?;

        let response = serde_json::json!({
            "name": name,
            "nodes_refreshed": count,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_pull_through(
        &self,
        source_graph: String,
        external_ref: String,
        mode: Option<String>,
    ) -> Result<String, String> {
        let pull_mode = match mode.as_deref() {
            Some("permanent") => ghost::PullMode::Permanent,
            _ => ghost::PullMode::SessionOnly,
        };

        let db = self.db.lock().unwrap();
        let ghost_node = db
            .get_ghost_node_by_ref(&source_graph, &external_ref)
            .map_err(|e| format!("Ghost node not found: {}", e))?;

        let sources = db
            .list_ghost_sources()
            .map_err(|e| format!("Failed to list sources: {}", e))?;

        let source = sources
            .iter()
            .find(|s| s.name == source_graph)
            .ok_or_else(|| format!("Source '{}' not found", source_graph))?;

        let content =
            ghost::pull::pull_ghost_content(&db, &ghost_node, &source.root_path, pull_mode)?;

        let response = serde_json::json!({
            "ghost_node_id": ghost_node.id,
            "source_graph": source_graph,
            "external_ref": external_ref,
            "mode": format!("{:?}", pull_mode),
            "content": content,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_create_backup(&self, backup_path: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let result = backup::create_backup(&db, &backup_path)?;

        let response = serde_json::json!({
            "path": result.path,
            "checksum": result.checksum,
            "impulse_count": result.impulse_count,
            "connection_count": result.connection_count,
            "size_bytes": result.size_bytes,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_sync_export(
        &self,
        sync_dir: String,
        device_id: String,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let result = sync::export_snapshot(&db, &sync_dir, &device_id)?;

        let response = serde_json::json!({
            "snapshot_path": result.snapshot_path,
            "checksum": result.checksum,
            "schema_version": result.schema_version,
            "feature_flags": result.feature_flags,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_create_tag(&self, name: String, color: Option<String>) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let new_tag = crate::models::NewTag {
            name,
            color: color.unwrap_or_else(|| "#8E99A4".to_string()),
        };
        let tag = db
            .create_tag(&new_tag)
            .map_err(|e| format!("Failed to create tag: {}", e))?;
        serde_json::to_string_pretty(&tag).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_list_tags(&self) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let tags = db
            .list_tags()
            .map_err(|e| format!("Failed to list tags: {}", e))?;
        serde_json::to_string_pretty(&tags).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_tag_memory(
        &self,
        impulse_id: String,
        tag_name: String,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot tag memory in incognito mode".to_string());
        }
        let db = self.db.lock().unwrap();
        // Verify impulse exists
        db.get_canonical_memory_impulse(&impulse_id)
            .map_err(|e| format!("Impulse not found: {}", e))?;
        // Verify tag exists
        db.get_tag(&tag_name)
            .map_err(|e| format!("Tag not found: {}", e))?;
        db.tag_impulse(&impulse_id, &tag_name)
            .map_err(|e| format!("Failed to tag impulse: {}", e))?;
        Ok(format!(
            "{{\"tagged\": true, \"impulse_id\": \"{}\", \"tag\": \"{}\"}}",
            impulse_id, tag_name
        ))
    }

    pub fn handle_untag_memory(
        &self,
        impulse_id: String,
        tag_name: String,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot untag memory in incognito mode".to_string());
        }
        let db = self.db.lock().unwrap();
        db.untag_impulse(&impulse_id, &tag_name)
            .map_err(|e| format!("Failed to untag impulse: {}", e))?;
        Ok(format!(
            "{{\"untagged\": true, \"impulse_id\": \"{}\", \"tag\": \"{}\"}}",
            impulse_id, tag_name
        ))
    }

    pub fn handle_export_to_obsidian(&self, output_dir: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let result = crate::markdown::export_to_markdown(&db, &output_dir)?;
        let response = serde_json::json!({
            "files_written": result.files_written,
            "output_dir": result.output_dir,
        });
        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_recall_narrative(&self, topic: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let engine = ActivationEngine::new(&db);
        let result = engine.retrieve(&RetrievalRequest {
            query: topic.clone(),
            max_results: 20,
            max_hops: 5,
        })?;

        let mut narrative_parts = Vec::new();
        for mem in &result.memories {
            let tags = db.get_tags_for_impulse(&mem.impulse.id).unwrap_or_default();
            let tag_str = tags
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
                .join(", ");
            let conns = db
                .get_canonical_edges_for_node(&mem.impulse.id)
                .unwrap_or_default()
                .into_iter()
                .filter(|conn| {
                    let other_id = if conn.source_id == mem.impulse.id {
                        &conn.target_id
                    } else {
                        &conn.source_id
                    };
                    db.get_canonical_node(other_id)
                        .map(|node| node.kind == crate::graph::GraphNodeKind::Memory)
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();

            narrative_parts.push(serde_json::json!({
                "content": mem.impulse.content,
                "type": mem.impulse.impulse_type.as_str(),
                "weight": mem.impulse.weight,
                "activation_score": mem.activation_score,
                "tags": tag_str,
                "connections": conns.len(),
                "engagement": mem.impulse.engagement_level.as_str(),
                "source_provider": mem.impulse.source_provider,
            }));
        }

        let response = serde_json::json!({
            "topic": topic,
            "impulse_count": result.memories.len(),
            "narrative_context": narrative_parts,
            "instruction": "Reconstruct a coherent narrative from these connected impulses. Tell the story of what was learned, decided, and understood about this topic. Use the activation scores to prioritize more relevant pieces."
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization: {}", e))
    }

    pub fn handle_propose_memories(
        &self,
        session_content: String,
        session_duration_minutes: Option<f64>,
    ) -> Result<String, String> {
        let word_count = session_content.split_whitespace().count();
        let decision_count = crate::extraction::count_keywords(
            &session_content,
            crate::extraction::DECISION_KEYWORDS,
        );
        let emotional_count = crate::extraction::count_keywords(
            &session_content,
            crate::extraction::EMOTIONAL_KEYWORDS,
        );
        let turn_estimate = (word_count / 50).max(1);

        let signals = crate::extraction::EngagementSignals {
            total_turns: turn_estimate,
            avg_user_message_length: (word_count / turn_estimate) as f64,
            avg_assistant_message_length: 0.0,
            session_duration_minutes: session_duration_minutes.unwrap_or(30.0),
            explicit_save_count: 0,
            topic_count: 1,
            decision_keywords_found: decision_count,
            emotional_keywords_found: emotional_count,
        };

        let depth = crate::extraction::assess_engagement(&signals);

        let response = serde_json::json!({
            "engagement_score": signals.engagement_score(),
            "depth": format!("{:?}", depth),
            "max_proposals": depth.max_proposals(),
            "instruction": format!(
                concat!(
                    "Reflect on this session and extract up to {} memories. Prioritize depth over breadth:\n",
                    "1. PATTERNS (type 'pattern'): How does the user think? What reasoning style did they use? ",
                    "What made them engage deeply or disengage? Did they approach problems top-down or bottom-up?\n",
                    "2. INSIGHTS (type 'heuristic'): What would a close friend notice? Any contradictions between ",
                    "what they said and what they did? Emotional patterns? Growth edges?\n",
                    "3. DECISIONS (type 'decision'): Major choices made, including the WHY not just the WHAT.\n",
                    "4. FACTS (type 'observation'): Personal facts shared, but only if genuinely new.\n",
                    "Do NOT save surface-level summaries of what was discussed. Save what you LEARNED about the person.\n",
                    "For each, call quick_save with appropriate type and engagement_level."
                ),
                depth.max_proposals()
            ),
            "session_stats": {
                "word_count": word_count,
                "estimated_turns": turn_estimate,
                "decision_keywords": decision_count,
                "emotional_keywords": emotional_count,
            }
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization: {}", e))
    }

    pub fn handle_prepare_compression(
        &self,
        session_content: String,
        recent_evidence_set_ids: Option<Vec<String>>,
        session_duration_minutes: Option<f64>,
        reason: Option<String>,
    ) -> Result<String, String> {
        let recent_evidence_set_ids = recent_evidence_set_ids.unwrap_or_default();
        let sanitization = {
            let db = self.db.lock().unwrap();
            sanitize_for_pre_compression(&db, &session_content, &recent_evidence_set_ids)
        };
        let reason = reason.unwrap_or_else(|| "pre_compress".to_string());

        let proposal = self.handle_propose_memories(
            sanitization.sanitized_content.clone(),
            session_duration_minutes,
        )?;
        let proposal: serde_json::Value = serde_json::from_str(&proposal)
            .map_err(|e| format!("Proposal serialization: {}", e))?;

        let checkpoint = {
            let mut session = self.session.lock().unwrap();
            session.record_pre_compression(
                &reason,
                sanitization.suppressed_evidence_set_ids.clone(),
                sanitization.stripped_char_count,
            );
            session.compression_checkpoint()
        };

        let response = serde_json::json!({
            "reason": reason,
            "sanitized_session_content": sanitization.sanitized_content,
            "suppressed_evidence_set_ids": sanitization.suppressed_evidence_set_ids,
            "suppressed_hashes": sanitization.suppressed_hashes,
            "stripped_line_count": sanitization.stripped_line_count,
            "stripped_char_count": sanitization.stripped_char_count,
            "memory_proposal": proposal,
            "compression_checkpoint": checkpoint,
            "instruction": "Review this bounded proposal result before compressing context. If durable procedures emerged from a live evidence set, call propose_skills separately before compression."
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization: {}", e))
    }

    pub fn handle_compression_status(&self) -> Result<String, String> {
        let session = self.session.lock().unwrap();
        let response = serde_json::json!({
            "session_id": session.id(),
            "compression_checkpoint": session.compression_checkpoint(),
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization: {}", e))
    }

    pub fn handle_feedback_recall(
        &self,
        evidence_set_id: String,
        feedback_kind: String,
        node_ids: Option<Vec<String>>,
        idempotency_key: Option<String>,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot record recall feedback in incognito mode".to_string());
        }

        let feedback_kind = FeedbackKind::from_str(&feedback_kind)
            .ok_or_else(|| format!("Invalid feedback kind: {}", feedback_kind))?;

        let db = self.db.lock().unwrap();
        let evidence_set = db
            .get_evidence_set(&evidence_set_id)
            .map_err(|e| format!("Evidence set not found: {}", e))?;

        if evidence_set
            .expires_at
            .is_some_and(|expires_at| expires_at < chrono::Utc::now())
        {
            return Err(format!("Evidence set {} has expired", evidence_set_id));
        }

        let target_node_ids = match node_ids {
            Some(ids) if !ids.is_empty() => ids,
            _ => evidence_set.node_ids.clone(),
        };

        if target_node_ids.is_empty() {
            return Err("No node_ids supplied and evidence set has no node targets".to_string());
        }

        let allowed_ids: std::collections::HashSet<String> =
            evidence_set.node_ids.iter().cloned().collect();
        let idempotency_base = idempotency_key.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut applied = Vec::new();
        let mut skipped = Vec::new();

        for node_id in target_node_ids {
            if !allowed_ids.contains(&node_id) {
                return Err(format!(
                    "Node {} is not present in evidence set {}",
                    node_id, evidence_set_id
                ));
            }

            let event_key = format!("{}:node:{}", idempotency_base, node_id);
            let record = db
                .create_feedback_record(
                    &evidence_set_id,
                    Some(&node_id),
                    None,
                    feedback_kind,
                    &event_key,
                )
                .map_err(|e| format!("Failed to record feedback event: {}", e))?;

            if record.is_some() {
                let node = db
                    .apply_feedback_to_node(&node_id, feedback_kind)
                    .map_err(|e| format!("Failed to update node confidence: {}", e))?;
                applied.push(serde_json::json!({
                    "node_id": node.id,
                    "confidence": node.confidence,
                    "effective_confidence": crate::confidence::effective_confidence(
                        node.helpful_count,
                        node.unhelpful_count
                    ),
                    "helpful_count": node.helpful_count,
                    "unhelpful_count": node.unhelpful_count,
                }));
            } else {
                skipped.push(node_id);
            }
        }

        let response = serde_json::json!({
            "evidence_set_id": evidence_set_id,
            "feedback_kind": feedback_kind.as_str(),
            "applied": applied,
            "skipped": skipped,
            "idempotency_key": idempotency_base,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization: {}", e))
    }

    pub fn handle_sync_status(
        &self,
        sync_dir: String,
        device_id: String,
    ) -> Result<String, String> {
        let result = sync::check_sync_status(&sync_dir, &device_id)?;

        let response = serde_json::json!({
            "has_remote_updates": result.has_remote_updates,
            "remote_devices": result.remote_devices,
            "latest_remote_device": result.latest_remote_device,
            "latest_remote_time": result.latest_remote_time,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_reflect_context(
        &self,
        evidence_set_id: String,
        max_memories: Option<usize>,
        max_relationships: Option<usize>,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let evidence_set = db
            .get_evidence_set(&evidence_set_id)
            .map_err(|e| format!("Evidence set not found: {}", e))?;

        if evidence_set
            .expires_at
            .is_some_and(|expires_at| expires_at < chrono::Utc::now())
        {
            return Err(format!("Evidence set {} has expired", evidence_set_id));
        }

        let packet = reflection::build_reflection_packet(
            &db,
            &evidence_set,
            max_memories.unwrap_or(10),
            max_relationships.unwrap_or(20),
        )?;

        serde_json::to_string_pretty(&packet).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_propose_skills(
        &self,
        evidence_set_id: String,
        max_candidates: Option<usize>,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let evidence_set = db
            .get_evidence_set(&evidence_set_id)
            .map_err(|e| format!("Evidence set not found: {}", e))?;

        if evidence_set
            .expires_at
            .is_some_and(|expires_at| expires_at < chrono::Utc::now())
        {
            return Err(format!("Evidence set {} has expired", evidence_set_id));
        }

        let reflection_packet = reflection::build_reflection_packet(&db, &evidence_set, 12, 24)?;
        let max_candidates = max_candidates.unwrap_or(3);

        let response = serde_json::json!({
            "evidence_set_id": evidence_set_id,
            "max_candidates": max_candidates,
            "reflection_packet": reflection_packet,
            "instruction": format!(
                concat!(
                    "Synthesize up to {} graph-native procedural skills from this reflection packet.\n",
                    "Each candidate should include:\n",
                    "- name\n",
                    "- description\n",
                    "- trigger\n",
                    "- ordered steps\n",
                    "- constraints\n",
                    "- evidence_node_ids drawn only from reflection_packet.memory_items\n",
                    "Only propose durable procedures, not one-off facts. Save approved skills with save_skill."
                ),
                max_candidates
            ),
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_save_skill(
        &self,
        name: String,
        description: String,
        trigger: String,
        steps: Vec<String>,
        constraints: Option<Vec<String>>,
        evidence_set_id: String,
        evidence_node_ids: Vec<String>,
        source_provider: Option<String>,
        source_account: Option<String>,
    ) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot save skill in incognito mode".to_string());
        }

        if steps.is_empty() {
            return Err("Skill must include at least one step".to_string());
        }
        if evidence_node_ids.is_empty() {
            return Err("Skill must cite at least one evidence node".to_string());
        }

        let db = self.db.lock().unwrap();
        let evidence_set = db
            .get_evidence_set(&evidence_set_id)
            .map_err(|e| format!("Evidence set not found: {}", e))?;

        if evidence_set
            .expires_at
            .is_some_and(|expires_at| expires_at < chrono::Utc::now())
        {
            return Err(format!("Evidence set {} has expired", evidence_set_id));
        }

        let allowed_ids: std::collections::HashSet<String> =
            evidence_set.node_ids.iter().cloned().collect();
        for node_id in &evidence_node_ids {
            if !allowed_ids.contains(node_id) {
                return Err(format!(
                    "Node {} is not present in evidence set {}",
                    node_id, evidence_set_id
                ));
            }
        }
        let provider = source_provider.unwrap_or_else(|| "unknown".to_string());
        let account = source_account.unwrap_or_default();
        let constraints = constraints.unwrap_or_default();

        let skill = db
            .create_skill(
                &name,
                &description,
                &trigger,
                &steps,
                &constraints,
                &evidence_set_id,
                &evidence_node_ids,
                &provider,
                &account,
            )
            .map_err(|e| format!("Failed to create skill: {}", e))?;

        let evidence_edges = db
            .get_skill_evidence_node_ids(&skill.node_id)
            .map_err(|e| format!("Failed to load skill evidence: {}", e))?;

        let response = serde_json::json!({
            "skill": skill,
            "evidence_node_ids": evidence_edges,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_inspect_skill(&self, node_id: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let node = db
            .get_canonical_node(&node_id)
            .map_err(|e| format!("Skill not found: {}", e))?;
        if node.kind != crate::graph::GraphNodeKind::Skill {
            return Err(format!("Node {} is not a skill", node_id));
        }

        let skill = db
            .get_skill(&node_id)
            .map_err(|e| format!("Failed to load skill payload: {}", e))?;
        let evidence_node_ids = db
            .get_skill_evidence_node_ids(&node_id)
            .map_err(|e| format!("Failed to load skill evidence: {}", e))?;
        let relationships = db
            .get_canonical_edges_for_node(&node_id)
            .map_err(|e| format!("Failed to load skill relationships: {}", e))?;

        let response = serde_json::json!({
            "skill": skill,
            "weight": node.weight,
            "confidence": node.confidence,
            "effective_confidence": crate::confidence::effective_confidence(
                node.helpful_count,
                node.unhelpful_count
            ),
            "helpful_count": node.helpful_count,
            "unhelpful_count": node.unhelpful_count,
            "evidence_node_ids": evidence_node_ids,
            "relationships": relationships,
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_retrieve_skills(
        &self,
        query: String,
        max_results: Option<usize>,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let results = retrieve_skill_matches(&db, &query, max_results.unwrap_or(10))?;

        serde_json::to_string_pretty(&results).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_detect_contradictions(
        &self,
        evidence_set_id: String,
        max_results: Option<usize>,
    ) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let evidence_set = db
            .get_evidence_set(&evidence_set_id)
            .map_err(|e| format!("Evidence set not found: {}", e))?;

        if evidence_set
            .expires_at
            .is_some_and(|expires_at| expires_at < chrono::Utc::now())
        {
            return Err(format!("Evidence set {} has expired", evidence_set_id));
        }

        let assessments =
            assessments::detect_contradictions(&db, &evidence_set, max_results.unwrap_or(6))?;

        let response = serde_json::json!({
            "evidence_set_id": evidence_set_id,
            "assessments": assessments,
            "instruction": "Review candidate contradictions. Use confirm_assessment for true conflicts and dismiss_assessment for false positives.",
        });

        serde_json::to_string_pretty(&response).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_confirm_assessment(&self, id: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot confirm assessments in incognito mode".to_string());
        }

        let db = self.db.lock().unwrap();
        let assessment = db
            .set_assessment_status(&id, AssessmentStatus::Confirmed)
            .map_err(|e| format!("Failed to confirm assessment: {}", e))?;

        serde_json::to_string_pretty(&assessment).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_dismiss_assessment(&self, id: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot dismiss assessments in incognito mode".to_string());
        }

        let db = self.db.lock().unwrap();
        let assessment = db
            .set_assessment_status(&id, AssessmentStatus::Dismissed)
            .map_err(|e| format!("Failed to dismiss assessment: {}", e))?;

        serde_json::to_string_pretty(&assessment).map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_list_assessments(
        &self,
        assessment_type: Option<String>,
        status: Option<String>,
        node_id: Option<String>,
    ) -> Result<String, String> {
        let assessment_type = match assessment_type {
            Some(value) => Some(
                AssessmentType::from_str(&value)
                    .ok_or_else(|| format!("Invalid assessment type: {}", value))?,
            ),
            None => None,
        };
        let status = match status {
            Some(value) => Some(
                AssessmentStatus::from_str(&value)
                    .ok_or_else(|| format!("Invalid assessment status: {}", value))?,
            ),
            None => None,
        };

        let db = self.db.lock().unwrap();
        let assessments = db
            .list_assessments(assessment_type, status, node_id.as_deref())
            .map_err(|e| format!("Failed to list assessments: {}", e))?;

        serde_json::to_string_pretty(&assessments)
            .map_err(|e| format!("Serialization error: {}", e))
    }
}

// ---------------------------------------------------------------------------
// MCP transport wrapper
// ---------------------------------------------------------------------------

/// Wraps MemoryGraphServer in an Arc so it satisfies the Clone + Send + Sync
/// bounds required by rmcp's ServerHandler trait.
#[derive(Clone)]
pub struct McpHandler {
    inner: Arc<MemoryGraphServer>,
}

impl McpHandler {
    pub fn new(server: MemoryGraphServer) -> Self {
        Self {
            inner: Arc::new(server),
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SaveMemoryParams {
    /// The content/text of the memory to save.
    pub content: String,
    /// The type of impulse: "explicit", "inferred", "environmental", or "episodic".
    pub impulse_type: String,
    /// Optional emotional valence: "positive", "negative", or "neutral".
    #[schemars(default)]
    pub emotional_valence: Option<String>,
    /// Optional engagement level: "high", "medium", or "low".
    #[schemars(default)]
    pub engagement_level: Option<String>,
    /// Optional source reference string.
    #[schemars(default)]
    pub source_ref: Option<String>,
    /// Optional source provider name (e.g., 'claude', 'chatgpt', 'cursor'). Defaults to 'unknown'.
    #[schemars(default)]
    pub source_provider: Option<String>,
    /// Optional source account identifier. Defaults to empty string.
    #[schemars(default)]
    pub source_account: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RetrieveContextParams {
    /// The query string to search memories.
    pub query: String,
    /// Maximum number of results to return (default 10).
    #[schemars(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteMemoryParams {
    /// The ID of the memory to delete.
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateMemoryParams {
    /// The ID of the memory to update.
    pub id: String,
    /// The new content for the memory.
    pub new_content: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectMemoryParams {
    /// The ID of the memory to inspect.
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SetIncognitoParams {
    /// Whether to enable or disable incognito mode.
    pub enabled: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExplainRecallParams {
    /// The query that triggered recall.
    pub query: String,
    /// The memory ID to explain.
    pub memory_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConfirmProposalParams {
    /// The ID of the candidate memory to confirm.
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DismissProposalParams {
    /// The ID of the candidate memory to dismiss.
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LinkMemoriesParams {
    /// ID of the source memory.
    pub source_id: String,
    /// ID of the target memory.
    pub target_id: String,
    /// Relationship label (e.g., 'relates_to', 'derived_from'). Defaults to 'relates_to'.
    #[schemars(default)]
    pub relationship: Option<String>,
    /// Connection weight (0.0 to 1.0). Defaults to 0.5.
    #[schemars(default)]
    pub weight: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UnlinkMemoriesParams {
    /// The ID of the connection to remove.
    pub connection_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RegisterGhostGraphParams {
    /// Name for this ghost graph (e.g., 'obsidian-vault').
    pub name: String,
    /// Root path to the knowledge base directory.
    pub root_path: String,
    /// Source type (e.g., 'obsidian', 'directory'). Defaults to 'directory'.
    #[schemars(default)]
    pub source_type: Option<String>,
    /// Path patterns to ignore during scan.
    #[schemars(default)]
    pub ignore_patterns: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RefreshGhostGraphParams {
    /// Name of the ghost graph to refresh.
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PullThroughParams {
    /// Name of the ghost graph source.
    pub source_graph: String,
    /// External reference path of the ghost node.
    pub external_ref: String,
    /// Pull mode: 'session_only' (default) or 'permanent'.
    #[schemars(default)]
    pub mode: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateBackupParams {
    /// The file path where the backup should be written.
    pub backup_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SyncExportParams {
    /// The directory to export the sync snapshot into.
    pub sync_dir: String,
    /// A unique identifier for this device.
    pub device_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SyncStatusParams {
    /// The sync directory to check for remote updates.
    pub sync_dir: String,
    /// The local device identifier.
    pub device_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExportToObsidianParams {
    /// The output directory where markdown files will be written. This becomes an Obsidian vault.
    pub output_dir: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTagParams {
    /// The name of the tag.
    pub name: String,
    /// Hex color for the tag (e.g., '#FF5733'). Defaults to '#8E99A4'.
    #[schemars(default)]
    pub color: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TagMemoryParams {
    /// The ID of the memory to tag.
    pub impulse_id: String,
    /// The name of the tag to apply.
    pub tag_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UntagMemoryParams {
    /// The ID of the memory to untag.
    pub impulse_id: String,
    /// The name of the tag to remove.
    pub tag_name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecallNarrativeParams {
    /// The topic to recall a narrative about.
    pub topic: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProposeMemoriesParams {
    /// The session content (conversation text) to analyze for memory extraction.
    pub session_content: String,
    /// Optional session duration in minutes. Defaults to 30.
    #[schemars(default)]
    pub session_duration_minutes: Option<f64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PrepareCompressionParams {
    /// The current session content before the client compresses or truncates it.
    pub session_content: String,
    /// Optional recent evidence sets whose recalled content should be suppressed from proposal input.
    #[schemars(default)]
    pub recent_evidence_set_ids: Option<Vec<String>>,
    /// Optional session duration in minutes. Defaults to 30.
    #[schemars(default)]
    pub session_duration_minutes: Option<f64>,
    /// Optional checkpoint reason, such as pre_compress, end_session, or explicit_review.
    #[schemars(default)]
    pub reason: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FeedbackRecallParams {
    /// The evidence set returned by retrieve_context.
    pub evidence_set_id: String,
    /// helpful or unhelpful.
    pub feedback_kind: String,
    /// Optional subset of evidence-set node ids to apply feedback to. Defaults to all node ids in the evidence set.
    #[schemars(default)]
    pub node_ids: Option<Vec<String>>,
    /// Optional idempotency key for replay-safe feedback submission.
    #[schemars(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReflectContextParams {
    /// The evidence set returned by retrieve_context.
    pub evidence_set_id: String,
    /// Maximum number of memory items to include.
    #[schemars(default)]
    pub max_memories: Option<usize>,
    /// Maximum number of relationships to include.
    #[schemars(default)]
    pub max_relationships: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProposeSkillsParams {
    /// The evidence set returned by retrieve_context.
    pub evidence_set_id: String,
    /// Maximum number of candidates to request from the client model.
    #[schemars(default)]
    pub max_candidates: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SaveSkillParams {
    /// Short skill name.
    pub name: String,
    /// Brief description of what the procedure accomplishes.
    pub description: String,
    /// Natural-language trigger describing when to use the skill.
    pub trigger: String,
    /// Ordered procedure steps.
    pub steps: Vec<String>,
    /// Optional constraints or caveats.
    #[schemars(default)]
    pub constraints: Option<Vec<String>>,
    /// The evidence set grounding this skill.
    pub evidence_set_id: String,
    /// Evidence node ids from that evidence set supporting the procedure.
    pub evidence_node_ids: Vec<String>,
    /// Optional source provider tag for auditability.
    #[schemars(default)]
    pub source_provider: Option<String>,
    /// Optional source account tag for auditability.
    #[schemars(default)]
    pub source_account: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InspectSkillParams {
    /// Skill node id.
    pub node_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RetrieveSkillsParams {
    /// Query string for skill retrieval.
    pub query: String,
    /// Maximum number of skills to return.
    #[schemars(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DetectContradictionsParams {
    /// The evidence set returned by retrieve_context.
    pub evidence_set_id: String,
    /// Maximum number of contradiction candidates to persist.
    #[schemars(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AssessmentStatusParams {
    /// Assessment id.
    pub id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListAssessmentsParams {
    /// Optional assessment type, currently 'contradiction'.
    #[schemars(default)]
    pub assessment_type: Option<String>,
    /// Optional status filter: candidate, confirmed, or dismissed.
    #[schemars(default)]
    pub status: Option<String>,
    /// Optional node id filter.
    #[schemars(default)]
    pub node_id: Option<String>,
}

#[tool(tool_box)]
impl McpHandler {
    #[tool(description = "Save a new memory to the graph")]
    fn save_memory(&self, #[tool(aggr)] params: SaveMemoryParams) -> Result<String, String> {
        self.inner.handle_save_memory(
            params.content,
            params.impulse_type,
            params.emotional_valence,
            params.engagement_level,
            params.source_ref,
            params.source_provider,
            params.source_account,
        )
    }

    #[tool(description = "Retrieve context-relevant memories for a query")]
    fn retrieve_context(
        &self,
        #[tool(aggr)] params: RetrieveContextParams,
    ) -> Result<String, String> {
        self.inner
            .handle_retrieve_context(params.query, params.max_results)
    }

    #[tool(description = "Soft-delete a memory by ID")]
    fn delete_memory(&self, #[tool(aggr)] params: DeleteMemoryParams) -> Result<String, String> {
        self.inner.handle_delete_memory(params.id)
    }

    #[tool(description = "Update the content of an existing memory")]
    fn update_memory(&self, #[tool(aggr)] params: UpdateMemoryParams) -> Result<String, String> {
        self.inner
            .handle_update_memory(params.id, params.new_content)
    }

    #[tool(description = "Inspect a memory and its connections")]
    fn inspect_memory(&self, #[tool(aggr)] params: InspectMemoryParams) -> Result<String, String> {
        self.inner.handle_inspect_memory(params.id)
    }

    #[tool(description = "Get memory graph status and statistics")]
    fn memory_status(&self) -> Result<String, String> {
        self.inner.handle_memory_status()
    }

    #[tool(description = "Enable or disable incognito mode")]
    fn set_incognito(&self, #[tool(aggr)] params: SetIncognitoParams) -> Result<String, String> {
        self.inner.handle_set_incognito(params.enabled)
    }

    #[tool(description = "Explain why a memory was recalled for a given query")]
    fn explain_recall(&self, #[tool(aggr)] params: ExplainRecallParams) -> Result<String, String> {
        self.inner
            .handle_explain_recall(params.query, params.memory_id)
    }

    #[tool(description = "Confirm a candidate memory proposal")]
    fn confirm_proposal(
        &self,
        #[tool(aggr)] params: ConfirmProposalParams,
    ) -> Result<String, String> {
        self.inner.handle_confirm_proposal(params.id)
    }

    #[tool(description = "Dismiss a candidate memory proposal")]
    fn dismiss_proposal(
        &self,
        #[tool(aggr)] params: DismissProposalParams,
    ) -> Result<String, String> {
        self.inner.handle_dismiss_proposal(params.id)
    }

    #[tool(description = "List all candidate memory proposals")]
    fn list_candidates(&self) -> Result<String, String> {
        self.inner.handle_list_candidates()
    }

    #[tool(
        description = "Save and immediately confirm a memory in one step. Use this for proactive saves during conversations — skips the candidate review step. Types: heuristic, preference, decision, pattern, observation."
    )]
    fn quick_save(&self, #[tool(aggr)] params: SaveMemoryParams) -> Result<String, String> {
        self.inner.handle_quick_save(
            params.content,
            params.impulse_type,
            params.emotional_valence,
            params.engagement_level,
            params.source_ref,
            params.source_provider,
            params.source_account,
        )
    }

    #[tool(
        description = "Create a connection between two memories. Use for manually linking related impulses."
    )]
    fn link_memories(&self, #[tool(aggr)] params: LinkMemoriesParams) -> Result<String, String> {
        self.inner.handle_link_memories(
            params.source_id,
            params.target_id,
            params.relationship,
            params.weight,
        )
    }

    #[tool(description = "Remove a connection between two memories by connection ID.")]
    fn unlink_memories(
        &self,
        #[tool(aggr)] params: UnlinkMemoriesParams,
    ) -> Result<String, String> {
        self.inner.handle_unlink_memories(params.connection_id)
    }

    #[tool(
        description = "Register an external knowledge base as a ghost graph. Scans the directory for markdown files and maps their structure without ingesting content."
    )]
    fn register_ghost_graph(
        &self,
        #[tool(aggr)] params: RegisterGhostGraphParams,
    ) -> Result<String, String> {
        self.inner.handle_register_ghost_graph(
            params.name,
            params.root_path,
            params.source_type,
            params.ignore_patterns,
        )
    }

    #[tool(
        description = "Refresh a ghost graph by re-scanning the external knowledge base for changes"
    )]
    fn refresh_ghost_graph(
        &self,
        #[tool(aggr)] params: RefreshGhostGraphParams,
    ) -> Result<String, String> {
        self.inner.handle_refresh_ghost_graph(params.name)
    }

    #[tool(
        description = "Pull content from a ghost node. 'session_only' releases after session. 'permanent' creates a memory node."
    )]
    fn pull_through(&self, #[tool(aggr)] params: PullThroughParams) -> Result<String, String> {
        self.inner
            .handle_pull_through(params.source_graph, params.external_ref, params.mode)
    }

    #[tool(description = "Create a backup of the memory graph database")]
    fn create_backup(&self, #[tool(aggr)] params: CreateBackupParams) -> Result<String, String> {
        self.inner.handle_create_backup(params.backup_path)
    }

    #[tool(description = "Export a sync snapshot of the database for cross-device synchronization")]
    fn sync_export(&self, #[tool(aggr)] params: SyncExportParams) -> Result<String, String> {
        self.inner
            .handle_sync_export(params.sync_dir, params.device_id)
    }

    #[tool(description = "Check sync directory for remote device updates")]
    fn sync_status(&self, #[tool(aggr)] params: SyncStatusParams) -> Result<String, String> {
        self.inner
            .handle_sync_status(params.sync_dir, params.device_id)
    }

    #[tool(
        description = "Export the memory graph as linked markdown files suitable for an Obsidian vault. Creates one .md file per confirmed memory with YAML frontmatter, wikilinks for connections, and tag index files."
    )]
    fn export_to_obsidian(
        &self,
        #[tool(aggr)] params: ExportToObsidianParams,
    ) -> Result<String, String> {
        self.inner.handle_export_to_obsidian(params.output_dir)
    }

    #[tool(description = "Create a tag with a name and hex color for organizing memories")]
    fn create_tag(&self, #[tool(aggr)] params: CreateTagParams) -> Result<String, String> {
        self.inner.handle_create_tag(params.name, params.color)
    }

    #[tool(description = "List all tags")]
    fn list_tags(&self) -> Result<String, String> {
        self.inner.handle_list_tags()
    }

    #[tool(description = "Add a tag to a memory")]
    fn tag_memory(&self, #[tool(aggr)] params: TagMemoryParams) -> Result<String, String> {
        self.inner
            .handle_tag_memory(params.impulse_id, params.tag_name)
    }

    #[tool(description = "Remove a tag from a memory")]
    fn untag_memory(&self, #[tool(aggr)] params: UntagMemoryParams) -> Result<String, String> {
        self.inner
            .handle_untag_memory(params.impulse_id, params.tag_name)
    }

    #[tool(
        description = "Recall a narrative about a topic — retrieves connected impulses with context for the LLM to reconstruct a coherent story"
    )]
    fn recall_narrative(
        &self,
        #[tool(aggr)] params: RecallNarrativeParams,
    ) -> Result<String, String> {
        self.inner.handle_recall_narrative(params.topic)
    }

    #[tool(
        description = "Analyze a session and propose memories to extract. Returns engagement assessment and extraction instructions for the LLM."
    )]
    fn propose_memories(
        &self,
        #[tool(aggr)] params: ProposeMemoriesParams,
    ) -> Result<String, String> {
        self.inner
            .handle_propose_memories(params.session_content, params.session_duration_minutes)
    }

    #[tool(
        description = "Sanitize recalled graph content out of session text, run propose_memories on the cleaned text, and record a pre-compression checkpoint"
    )]
    fn prepare_compression(
        &self,
        #[tool(aggr)] params: PrepareCompressionParams,
    ) -> Result<String, String> {
        self.inner.handle_prepare_compression(
            params.session_content,
            params.recent_evidence_set_ids,
            params.session_duration_minutes,
            params.reason,
        )
    }

    #[tool(
        description = "Report whether the current session has used the pre-compression checkpoint hook"
    )]
    fn compression_status(&self) -> Result<String, String> {
        self.inner.handle_compression_status()
    }

    #[tool(
        description = "Record helpful or unhelpful feedback for memories returned in a retrieve_context evidence set"
    )]
    fn feedback_recall(
        &self,
        #[tool(aggr)] params: FeedbackRecallParams,
    ) -> Result<String, String> {
        self.inner.handle_feedback_recall(
            params.evidence_set_id,
            params.feedback_kind,
            params.node_ids,
            params.idempotency_key,
        )
    }

    #[tool(
        description = "Build a bounded, typed reflection packet over a retrieve_context evidence set"
    )]
    fn reflect_context(
        &self,
        #[tool(aggr)] params: ReflectContextParams,
    ) -> Result<String, String> {
        self.inner.handle_reflect_context(
            params.evidence_set_id,
            params.max_memories,
            params.max_relationships,
        )
    }

    #[tool(
        description = "Assemble a grounded proposal packet for procedural skills from an evidence set"
    )]
    fn propose_skills(&self, #[tool(aggr)] params: ProposeSkillsParams) -> Result<String, String> {
        self.inner
            .handle_propose_skills(params.evidence_set_id, params.max_candidates)
    }

    #[tool(
        description = "Save a graph-native procedural skill grounded in a retrieve_context evidence set"
    )]
    fn save_skill(&self, #[tool(aggr)] params: SaveSkillParams) -> Result<String, String> {
        self.inner.handle_save_skill(
            params.name,
            params.description,
            params.trigger,
            params.steps,
            params.constraints,
            params.evidence_set_id,
            params.evidence_node_ids,
            params.source_provider,
            params.source_account,
        )
    }

    #[tool(description = "Inspect a saved graph-native procedural skill")]
    fn inspect_skill(&self, #[tool(aggr)] params: InspectSkillParams) -> Result<String, String> {
        self.inner.handle_inspect_skill(params.node_id)
    }

    #[tool(description = "Retrieve saved procedural skills relevant to a query")]
    fn retrieve_skills(
        &self,
        #[tool(aggr)] params: RetrieveSkillsParams,
    ) -> Result<String, String> {
        self.inner
            .handle_retrieve_skills(params.query, params.max_results)
    }

    #[tool(
        description = "Detect and persist contradiction assessments from a retrieve_context evidence set"
    )]
    fn detect_contradictions(
        &self,
        #[tool(aggr)] params: DetectContradictionsParams,
    ) -> Result<String, String> {
        self.inner
            .handle_detect_contradictions(params.evidence_set_id, params.max_results)
    }

    #[tool(description = "Confirm an assessment, such as a contradiction candidate")]
    fn confirm_assessment(
        &self,
        #[tool(aggr)] params: AssessmentStatusParams,
    ) -> Result<String, String> {
        self.inner.handle_confirm_assessment(params.id)
    }

    #[tool(description = "Dismiss an assessment so it stops resurfacing until evidence changes")]
    fn dismiss_assessment(
        &self,
        #[tool(aggr)] params: AssessmentStatusParams,
    ) -> Result<String, String> {
        self.inner.handle_dismiss_assessment(params.id)
    }

    #[tool(description = "List saved assessments with optional type, status, or node filters")]
    fn list_assessments(
        &self,
        #[tool(aggr)] params: ListAssessmentsParams,
    ) -> Result<String, String> {
        self.inner
            .handle_list_assessments(params.assessment_type, params.status, params.node_id)
    }
}

#[tool(tool_box)]
impl ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                concat!(
                    "synaptic-graph: deep relational memory — build a true understanding of the user, not a fact sheet.\n\n",

                    "PHILOSOPHY: Think like a close friend, not an executive assistant. A friend remembers how you think, ",
                    "what you care about, your patterns of behavior, and your blind spots — not just your preferences.\n\n",

                    "THREE LAYERS OF MEMORY:\n",
                    "Layer 1 (Facts): Personal facts, major decisions, stated preferences. Save when explicitly shared.\n",
                    "Layer 2 (Patterns): HOW the user thinks and behaves. Observe over time:\n",
                    "  - Reasoning style (philosophical first? practical first? needs options or wants a recommendation?)\n",
                    "  - Work patterns (when they're most engaged, how they approach problems, what makes them disengage)\n",
                    "  - Decision patterns (what factors actually drive their choices, not just what they say drives them)\n",
                    "  Save with type 'pattern' when you notice a recurring behavior across interactions.\n",
                    "Layer 3 (Insights): What a close friend would notice:\n",
                    "  - Contradictions between what they say and what they do (held gently, not judgmentally)\n",
                    "  - Emotional patterns (topics that consistently energize or drain them)\n",
                    "  - Growth edges (where they've improved, where they still struggle)\n",
                    "  - Self-deception (when they say they'll do something they historically don't)\n",
                    "  Save with type 'heuristic' — these are the most valuable memories.\n\n",

                    "WHEN TO SAVE:\n",
                    "- Explicit requests ('remember this', 'save that') — always\n",
                    "- Personal facts shared naturally — yes\n",
                    "- When you notice a PATTERN forming (not on first occurrence, on second or third) — yes, with type 'pattern'\n",
                    "- When engagement is notably high or low — note what caused it, with type 'observation'\n",
                    "- End of substantial sessions — reflect on what you learned about HOW the user thinks, not just WHAT was discussed\n",
                    "DO NOT save routine messages, every opinion, surface-level observations, or things derivable from code/git.\n",
                    "1-3 saves per session is right. 0 is fine for short sessions. 10 is too many.\n\n",

                    "AMBIENT RECALL:\n",
                    "Relevant memories are automatically retrieved and injected into your context on each message (via a hook). ",
                    "You do NOT need to call retrieve_context manually for ambient recall — it happens automatically.\n",
                    "Only call retrieve_context manually when:\n",
                    "- The user explicitly asks 'what do you know about X'\n",
                    "- You need a broader/deeper search than what was auto-retrieved\n",
                    "- You want to search for a specific topic not in the current message\n\n",

                    "HOW TO USE RECALLED MEMORIES (critical):\n",
                    "Memories should SHAPE your response, not DECORATE it. Never announce that you're using a memory.\n",
                    "BAD: 'As someone who prefers lightweight tools like Rust...' (awkwardly proving you remember)\n",
                    "BAD: 'Based on your memory profile, I think...' (robotic, breaks immersion)\n",
                    "BAD: 'Since you live in Paris and work in VC...' (restating facts to seem personal)\n",
                    "GOOD: 'You'd probably want to start with the philosophy here before touching code.' (shaped by pattern, not announced)\n",
                    "GOOD: 'This tool would slot right into your deal flow.' (used knowledge to filter, didn't cite it)\n",
                    "GOOD: 'Heads up — this is the kind of thing you tend to overcommit on and then wish you'd scoped smaller.' (friend advice)\n",
                    "The test: would a close friend cite their knowledge source? No. They just know you and it shows in how they talk.\n",
                    "Only explicitly reference memories when the user ASKS what you know or when the memory IS the point of the conversation.\n\n",

                    "ENGAGEMENT SIGNALS:\n",
                    "- Long, thoughtful messages = high engagement\n",
                    "- The user arguing with you or pushing back = very high engagement (they care)\n",
                    "- Short replies, topic-switching = low engagement or discomfort\n",
                    "- Late-night sessions, weekend work = passionate about the topic\n\n",

                    "Use quick_save for saves. Set engagement_level honestly. At end of long sessions, call propose_memories. Before compression or truncation boundaries, call prepare_compression.\n",
                )
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
