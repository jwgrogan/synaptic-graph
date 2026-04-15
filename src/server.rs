// MCP tool handlers

use std::sync::{Arc, Mutex};

use crate::activation::ActivationEngine;
use crate::backup;
use crate::db::Database;
use crate::ghost;
use crate::ingestion;
use crate::models::*;
use crate::session::Session;
use crate::sync;

use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, schemars, tool};

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

        let result = engine.retrieve(&request)?;
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
            .get_impulse(&new_id)
            .map_err(|e| format!("Fetch failed: {}", e))?;

        serde_json::to_string_pretty(&new_impulse)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_inspect_memory(&self, id: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let impulse = db
            .get_impulse(&id)
            .map_err(|e| format!("Not found: {}", e))?;

        let connections = db
            .get_connections_for_node(&id)
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
            "tags": tags.iter().map(|t| serde_json::json!({"name": t.name, "color": t.color})).collect::<Vec<_>>(),
            "connections": connections.iter().map(|c| serde_json::json!({
                "id": c.id,
                "source_id": c.source_id,
                "target_id": c.target_id,
                "weight": c.weight,
                "relationship": c.relationship,
                "traversal_count": c.traversal_count,
            })).collect::<Vec<_>>(),
        });

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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
            "incognito": incognito,
        });

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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

        let result = engine.retrieve(&request)?;

        let explanation = result
            .memories
            .iter()
            .find(|m| m.impulse.id == memory_id)
            .map(|m| {
                serde_json::json!({
                    "memory_id": m.impulse.id,
                    "activation_score": m.activation_score,
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

        let impulse = db.get_impulse(&id)
            .map_err(|e| format!("Fetch failed: {}", e))?;

        serde_json::to_string_pretty(&impulse)
            .map_err(|e| format!("Serialization error: {}", e))
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
        let candidates = db.list_candidates()
            .map_err(|e| format!("List failed: {}", e))?;

        serde_json::to_string_pretty(&candidates)
            .map_err(|e| format!("Serialization error: {}", e))
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
        serde_json::to_string_pretty(&conn)
            .map_err(|e| format!("Serialization error: {}", e))
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

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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
        let ghost_node = db.get_ghost_node_by_ref(&source_graph, &external_ref)
            .map_err(|e| format!("Ghost node not found: {}", e))?;

        let sources = db.list_ghost_sources()
            .map_err(|e| format!("Failed to list sources: {}", e))?;

        let source = sources.iter().find(|s| s.name == source_graph)
            .ok_or_else(|| format!("Source '{}' not found", source_graph))?;

        let content = ghost::pull::pull_ghost_content(&db, &ghost_node, &source.root_path, pull_mode)?;

        let response = serde_json::json!({
            "ghost_node_id": ghost_node.id,
            "source_graph": source_graph,
            "external_ref": external_ref,
            "mode": format!("{:?}", pull_mode),
            "content": content,
        });

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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
        });

        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_create_tag(&self, name: String, color: Option<String>) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let new_tag = crate::models::NewTag {
            name,
            color: color.unwrap_or_else(|| "#8E99A4".to_string()),
        };
        let tag = db.create_tag(&new_tag)
            .map_err(|e| format!("Failed to create tag: {}", e))?;
        serde_json::to_string_pretty(&tag)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_list_tags(&self) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let tags = db.list_tags()
            .map_err(|e| format!("Failed to list tags: {}", e))?;
        serde_json::to_string_pretty(&tags)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    pub fn handle_tag_memory(&self, impulse_id: String, tag_name: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot tag memory in incognito mode".to_string());
        }
        let db = self.db.lock().unwrap();
        // Verify impulse exists
        db.get_impulse(&impulse_id)
            .map_err(|e| format!("Impulse not found: {}", e))?;
        // Verify tag exists
        db.get_tag(&tag_name)
            .map_err(|e| format!("Tag not found: {}", e))?;
        db.tag_impulse(&impulse_id, &tag_name)
            .map_err(|e| format!("Failed to tag impulse: {}", e))?;
        Ok(format!("{{\"tagged\": true, \"impulse_id\": \"{}\", \"tag\": \"{}\"}}", impulse_id, tag_name))
    }

    pub fn handle_untag_memory(&self, impulse_id: String, tag_name: String) -> Result<String, String> {
        if self.is_incognito() {
            return Err("Cannot untag memory in incognito mode".to_string());
        }
        let db = self.db.lock().unwrap();
        db.untag_impulse(&impulse_id, &tag_name)
            .map_err(|e| format!("Failed to untag impulse: {}", e))?;
        Ok(format!("{{\"untagged\": true, \"impulse_id\": \"{}\", \"tag\": \"{}\"}}", impulse_id, tag_name))
    }

    pub fn handle_export_to_obsidian(&self, output_dir: String) -> Result<String, String> {
        let db = self.db.lock().unwrap();
        let result = crate::markdown::export_to_markdown(&db, &output_dir)?;
        let response = serde_json::json!({
            "files_written": result.files_written,
            "output_dir": result.output_dir,
        });
        serde_json::to_string_pretty(&response)
            .map_err(|e| format!("Serialization error: {}", e))
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
            let tag_str = tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", ");
            let conns = db.get_connections_for_node(&mem.impulse.id).unwrap_or_default();

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
        let decision_count = crate::extraction::count_keywords(&session_content, crate::extraction::DECISION_KEYWORDS);
        let emotional_count = crate::extraction::count_keywords(&session_content, crate::extraction::EMOTIONAL_KEYWORDS);
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

        serde_json::to_string_pretty(&response)
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

#[tool(tool_box)]
impl McpHandler {
    #[tool(description = "Save a new memory to the graph")]
    fn save_memory(
        &self,
        #[tool(aggr)] params: SaveMemoryParams,
    ) -> Result<String, String> {
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
    fn delete_memory(
        &self,
        #[tool(aggr)] params: DeleteMemoryParams,
    ) -> Result<String, String> {
        self.inner.handle_delete_memory(params.id)
    }

    #[tool(description = "Update the content of an existing memory")]
    fn update_memory(
        &self,
        #[tool(aggr)] params: UpdateMemoryParams,
    ) -> Result<String, String> {
        self.inner
            .handle_update_memory(params.id, params.new_content)
    }

    #[tool(description = "Inspect a memory and its connections")]
    fn inspect_memory(
        &self,
        #[tool(aggr)] params: InspectMemoryParams,
    ) -> Result<String, String> {
        self.inner.handle_inspect_memory(params.id)
    }

    #[tool(description = "Get memory graph status and statistics")]
    fn memory_status(&self) -> Result<String, String> {
        self.inner.handle_memory_status()
    }

    #[tool(description = "Enable or disable incognito mode")]
    fn set_incognito(
        &self,
        #[tool(aggr)] params: SetIncognitoParams,
    ) -> Result<String, String> {
        self.inner.handle_set_incognito(params.enabled)
    }

    #[tool(description = "Explain why a memory was recalled for a given query")]
    fn explain_recall(
        &self,
        #[tool(aggr)] params: ExplainRecallParams,
    ) -> Result<String, String> {
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

    #[tool(description = "Save and immediately confirm a memory in one step. Use this for proactive saves during conversations — skips the candidate review step. Types: heuristic, preference, decision, pattern, observation.")]
    fn quick_save(
        &self,
        #[tool(aggr)] params: SaveMemoryParams,
    ) -> Result<String, String> {
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

    #[tool(description = "Create a connection between two memories. Use for manually linking related impulses.")]
    fn link_memories(
        &self,
        #[tool(aggr)] params: LinkMemoriesParams,
    ) -> Result<String, String> {
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

    #[tool(description = "Register an external knowledge base as a ghost graph. Scans the directory for markdown files and maps their structure without ingesting content.")]
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

    #[tool(description = "Refresh a ghost graph by re-scanning the external knowledge base for changes")]
    fn refresh_ghost_graph(
        &self,
        #[tool(aggr)] params: RefreshGhostGraphParams,
    ) -> Result<String, String> {
        self.inner.handle_refresh_ghost_graph(params.name)
    }

    #[tool(description = "Pull content from a ghost node. 'session_only' releases after session. 'permanent' creates a memory node.")]
    fn pull_through(
        &self,
        #[tool(aggr)] params: PullThroughParams,
    ) -> Result<String, String> {
        self.inner.handle_pull_through(
            params.source_graph,
            params.external_ref,
            params.mode,
        )
    }

    #[tool(description = "Create a backup of the memory graph database")]
    fn create_backup(
        &self,
        #[tool(aggr)] params: CreateBackupParams,
    ) -> Result<String, String> {
        self.inner.handle_create_backup(params.backup_path)
    }

    #[tool(description = "Export a sync snapshot of the database for cross-device synchronization")]
    fn sync_export(
        &self,
        #[tool(aggr)] params: SyncExportParams,
    ) -> Result<String, String> {
        self.inner
            .handle_sync_export(params.sync_dir, params.device_id)
    }

    #[tool(description = "Check sync directory for remote device updates")]
    fn sync_status(
        &self,
        #[tool(aggr)] params: SyncStatusParams,
    ) -> Result<String, String> {
        self.inner
            .handle_sync_status(params.sync_dir, params.device_id)
    }

    #[tool(description = "Export the memory graph as linked markdown files suitable for an Obsidian vault. Creates one .md file per confirmed memory with YAML frontmatter, wikilinks for connections, and tag index files.")]
    fn export_to_obsidian(
        &self,
        #[tool(aggr)] params: ExportToObsidianParams,
    ) -> Result<String, String> {
        self.inner.handle_export_to_obsidian(params.output_dir)
    }

    #[tool(description = "Create a tag with a name and hex color for organizing memories")]
    fn create_tag(
        &self,
        #[tool(aggr)] params: CreateTagParams,
    ) -> Result<String, String> {
        self.inner.handle_create_tag(params.name, params.color)
    }

    #[tool(description = "List all tags")]
    fn list_tags(&self) -> Result<String, String> {
        self.inner.handle_list_tags()
    }

    #[tool(description = "Add a tag to a memory")]
    fn tag_memory(
        &self,
        #[tool(aggr)] params: TagMemoryParams,
    ) -> Result<String, String> {
        self.inner.handle_tag_memory(params.impulse_id, params.tag_name)
    }

    #[tool(description = "Remove a tag from a memory")]
    fn untag_memory(
        &self,
        #[tool(aggr)] params: UntagMemoryParams,
    ) -> Result<String, String> {
        self.inner.handle_untag_memory(params.impulse_id, params.tag_name)
    }

    #[tool(description = "Recall a narrative about a topic — retrieves connected impulses with context for the LLM to reconstruct a coherent story")]
    fn recall_narrative(
        &self,
        #[tool(aggr)] params: RecallNarrativeParams,
    ) -> Result<String, String> {
        self.inner.handle_recall_narrative(params.topic)
    }

    #[tool(description = "Analyze a session and propose memories to extract. Returns engagement assessment and extraction instructions for the LLM.")]
    fn propose_memories(
        &self,
        #[tool(aggr)] params: ProposeMemoriesParams,
    ) -> Result<String, String> {
        self.inner.handle_propose_memories(params.session_content, params.session_duration_minutes)
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

                    "AMBIENT RECALL (always-on memory awareness):\n",
                    "Call retrieve_context SILENTLY in these situations. Never mention that you checked.\n\n",
                    "Trigger 1 — FIRST MESSAGE: Always retrieve on the very first user message of a session, even if it is just 'hey'. ",
                    "Use whatever comes back to warm-start the conversation naturally. A friend picking up from where you left off, not 'Hi, I'm Claude.'\n",
                    "Trigger 2 — SUBSTANTIAL MESSAGES: Any message with real content (more than a few words, not just 'ok' or 'sure') — ",
                    "silently call retrieve_context with the message as query. If pattern memories activate, let them shape your response.\n",
                    "Trigger 3 — CADENCE CHECK: If 4-5 back-and-forth messages have passed without a retrieval, do one based on the current ",
                    "conversation topic. The conversation may have drifted into territory where you have relevant memories.\n",
                    "Trigger 4 — EXPLICIT: When the user asks 'what do you know about X' or 'remind me.'\n\n",
                    "If nothing relevant comes back from a retrieval, just proceed normally. The cost of checking is near-zero. ",
                    "The cost of missing a relevant memory that would have made your response more personal is high.\n\n",

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

                    "Use quick_save for saves. Set engagement_level honestly. At end of long sessions, call propose_memories.\n",
                )
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
