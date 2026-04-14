use synaptic_graph::activation::ActivationEngine;
use synaptic_graph::db::Database;
use synaptic_graph::models::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Database>,
}

fn default_db_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("memory-graph");
    std::fs::create_dir_all(&path).ok();
    path.push("memory.db");
    path
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let db_path = std::env::var("MEMORY_GRAPH_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_db_path());

        let db = Database::open(db_path.to_str().unwrap_or("memory.db"))
            .map_err(|e| format!("Failed to open DB: {}", e))?;

        Ok(Self { db: Mutex::new(db) })
    }
}

#[derive(Serialize, Clone)]
pub struct UiImpulse {
    pub id: String,
    pub content: String,
    pub impulse_type: String,
    pub weight: f64,
    pub initial_weight: f64,
    pub emotional_valence: String,
    pub engagement_level: String,
    pub source_type: String,
    pub source_ref: String,
    pub status: String,
    pub created_at: String,
    pub last_accessed_at: String,
}

impl From<Impulse> for UiImpulse {
    fn from(i: Impulse) -> Self {
        Self {
            id: i.id,
            content: i.content,
            impulse_type: i.impulse_type.as_str().to_string(),
            weight: i.weight,
            initial_weight: i.initial_weight,
            emotional_valence: i.emotional_valence.as_str().to_string(),
            engagement_level: i.engagement_level.as_str().to_string(),
            source_type: i.source_type.as_str().to_string(),
            source_ref: i.source_ref,
            status: i.status.as_str().to_string(),
            created_at: i.created_at.to_rfc3339(),
            last_accessed_at: i.last_accessed_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct UiConnection {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub weight: f64,
    pub relationship: String,
    pub traversal_count: i64,
}

impl From<Connection> for UiConnection {
    fn from(c: Connection) -> Self {
        Self {
            id: c.id,
            source_id: c.source_id,
            target_id: c.target_id,
            weight: c.weight,
            relationship: c.relationship,
            traversal_count: c.traversal_count,
        }
    }
}

#[tauri::command]
pub fn get_all_impulses(state: State<AppState>) -> Result<Vec<UiImpulse>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let impulses = db.list_impulses(Some(ImpulseStatus::Confirmed))
        .map_err(|e| format!("DB error: {}", e))?;
    Ok(impulses.into_iter().map(UiImpulse::from).collect())
}

#[tauri::command]
pub fn get_all_connections(state: State<AppState>) -> Result<Vec<UiConnection>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let impulses = db.list_impulses(Some(ImpulseStatus::Confirmed))
        .map_err(|e| format!("DB error: {}", e))?;

    let mut all_conns = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    for impulse in &impulses {
        let conns = db.get_connections_for_node(&impulse.id)
            .map_err(|e| format!("DB error: {}", e))?;
        for conn in conns {
            if seen_ids.insert(conn.id.clone()) {
                all_conns.push(UiConnection::from(conn));
            }
        }
    }

    Ok(all_conns)
}

#[tauri::command]
pub fn get_memory_stats(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let stats = db.memory_stats().map_err(|e| format!("DB error: {}", e))?;
    Ok(serde_json::json!({
        "total_impulses": stats.total_impulses,
        "confirmed_impulses": stats.confirmed_impulses,
        "candidate_impulses": stats.candidate_impulses,
        "total_connections": stats.total_connections,
    }))
}

#[tauri::command]
pub fn search_memories(
    state: State<AppState>,
    query: String,
    max_results: Option<usize>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let engine = ActivationEngine::new(&db);

    let request = RetrievalRequest {
        query,
        max_results: max_results.unwrap_or(20),
        max_hops: 3,
    };

    let result = engine.retrieve(&request)?;

    let memories: Vec<serde_json::Value> = result.memories.iter().map(|m| {
        serde_json::json!({
            "id": m.impulse.id,
            "content": m.impulse.content,
            "activation_score": m.activation_score,
            "activation_path": m.activation_path,
        })
    }).collect();

    let ghost_activations: Vec<serde_json::Value> = result.ghost_activations.iter().map(|g| {
        serde_json::json!({
            "ghost_node_id": g.ghost_node.id,
            "title": g.ghost_node.title,
            "source_graph": g.source_graph,
            "activation_score": g.activation_score,
        })
    }).collect();

    Ok(serde_json::json!({
        "memories": memories,
        "ghost_activations": ghost_activations,
        "total_activated": result.total_nodes_activated,
    }))
}

#[tauri::command]
pub fn get_impulse_detail(
    state: State<AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let impulse = db.get_impulse(&id).map_err(|e| format!("Not found: {}", e))?;
    let connections = db.get_connections_for_node(&id)
        .map_err(|e| format!("DB error: {}", e))?;

    let conn_details: Vec<serde_json::Value> = connections.iter().map(|c| {
        let other_id = if c.source_id == id { &c.target_id } else { &c.source_id };
        let other_content = db.get_impulse(other_id)
            .map(|i| i.content)
            .unwrap_or_else(|_| "unknown".to_string());

        serde_json::json!({
            "id": c.id,
            "other_id": other_id,
            "other_content": other_content,
            "relationship": c.relationship,
            "weight": c.weight,
            "traversal_count": c.traversal_count,
        })
    }).collect();

    Ok(serde_json::json!({
        "impulse": UiImpulse::from(impulse),
        "connections": conn_details,
    }))
}

#[tauri::command]
pub fn get_ghost_sources(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let sources = db.list_ghost_sources().map_err(|e| format!("DB error: {}", e))?;

    Ok(sources.iter().map(|s| {
        serde_json::json!({
            "name": s.name,
            "root_path": s.root_path,
            "source_type": s.source_type,
            "node_count": s.node_count,
            "last_scanned_at": s.last_scanned_at.map(|d| d.to_rfc3339()),
        })
    }).collect())
}

#[tauri::command]
pub fn get_ghost_nodes(
    state: State<AppState>,
    source_name: String,
) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let nodes = db.list_ghost_nodes_by_source(&source_name)
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(nodes.iter().map(|n| {
        serde_json::json!({
            "id": n.id,
            "title": n.title,
            "external_ref": n.external_ref,
            "weight": n.weight,
            "metadata": n.metadata,
        })
    }).collect())
}
