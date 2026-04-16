use synaptic_graph::activation::ActivationEngine;
use synaptic_graph::db::Database;
use synaptic_graph::models::*;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Database>,
    pub db_path: String,
}

fn default_db_path() -> PathBuf {
    // Check common locations in priority order:
    // 1. ~/.local/share/synaptic-graph/memory.db (Linux convention, also used by MCP configs)
    // 2. ~/Library/Application Support/synaptic-graph/memory.db (macOS dirs crate default)
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let linux_style = home.join(".local/share/synaptic-graph/memory.db");
    if linux_style.exists() {
        return linux_style;
    }

    let mac_style = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("synaptic-graph")
        .join("memory.db");
    if mac_style.exists() {
        return mac_style;
    }

    // Neither exists — create at the linux-style path (matches MCP config convention)
    let parent = home.join(".local/share/synaptic-graph");
    std::fs::create_dir_all(&parent).ok();
    parent.join("memory.db")
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let db_path = std::env::var("MEMORY_GRAPH_DB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| default_db_path());

        let db_path_str = db_path.to_str().unwrap_or("memory.db").to_string();
        let db = Database::open(&db_path_str)
            .map_err(|e| format!("Failed to open DB: {}", e))?;

        Ok(Self { db: Mutex::new(db), db_path: db_path_str })
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
    pub source_provider: String,
    pub source_account: String,
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
            source_provider: i.source_provider,
            source_account: i.source_account,
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

#[derive(serde::Deserialize)]
pub struct QuickSaveParams {
    pub content: String,
    pub impulse_type: String,
    pub emotional_valence: Option<String>,
    pub engagement_level: Option<String>,
    pub source_ref: Option<String>,
}

#[tauri::command]
pub fn quick_save(
    state: State<AppState>,
    params: QuickSaveParams,
) -> Result<serde_json::Value, String> {
    use synaptic_graph::ingestion;
    use synaptic_graph::models::*;

    let itype = ImpulseType::from_str(&params.impulse_type)
        .ok_or_else(|| format!("Invalid impulse type: {}", params.impulse_type))?;
    let valence = params.emotional_valence.as_deref()
        .map(EmotionalValence::from_str)
        .unwrap_or(Some(EmotionalValence::Neutral))
        .ok_or("Invalid valence")?;
    let engagement = params.engagement_level.as_deref()
        .map(EngagementLevel::from_str)
        .unwrap_or(Some(EngagementLevel::Medium))
        .ok_or("Invalid engagement")?;

    let db = state.db.lock().map_err(|e| format!("Lock error: {}", e))?;
    let impulse = ingestion::save_and_confirm(
        &db,
        &params.content,
        itype,
        valence,
        engagement,
        vec![],
        params.source_ref.as_deref().unwrap_or("import"),
    )?;

    Ok(serde_json::json!({
        "id": impulse.id,
        "content": impulse.content,
        "status": impulse.status.as_str(),
    }))
}

#[tauri::command]
pub fn get_all_tags(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let tags = db.list_tags().map_err(|e| format!("DB: {}", e))?;
    Ok(tags.iter().map(|t| serde_json::json!({"name": t.name, "color": t.color})).collect())
}

#[tauri::command]
pub fn get_impulse_tags(state: State<AppState>, impulse_id: String) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let tags = db.get_tags_for_impulse(&impulse_id).map_err(|e| format!("DB: {}", e))?;
    Ok(tags.iter().map(|t| serde_json::json!({"name": t.name, "color": t.color})).collect())
}

#[tauri::command]
pub async fn export_to_obsidian(
    state: State<'_, AppState>,
    output_dir: String,
) -> Result<serde_json::Value, String> {
    let db_path = state.db_path.clone();
    let dir = output_dir.clone();

    let result = tokio::task::spawn_blocking(move || {
        let db = Database::open(&db_path).map_err(|e| format!("DB: {}", e))?;
        synaptic_graph::markdown::export_to_markdown(&db, &dir)
    })
    .await
    .map_err(|e| format!("Task: {}", e))??;

    Ok(serde_json::json!({
        "files_written": result.files_written,
        "output_dir": result.output_dir,
    }))
}

#[tauri::command]
pub async fn register_external_graph(
    state: State<'_, AppState>,
    name: String,
    root_path: String,
    source_type: Option<String>,
) -> Result<serde_json::Value, String> {
    use synaptic_graph::ghost;
    use synaptic_graph::ghost::scanner::ScanConfig;

    let stype = source_type.unwrap_or_else(|| "directory".to_string());
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![".trash".to_string(), ".obsidian".to_string()],
    };

    // Clone values needed inside the blocking task
    let name_clone = name.clone();
    let root_path_clone = root_path.clone();
    let db_path = state.db_path.clone();

    let count = tokio::task::spawn_blocking(move || {
        let db = Database::open(&db_path)
            .map_err(|e| format!("Failed to open DB in worker: {}", e))?;
        ghost::register_and_scan(&db, &name_clone, &root_path_clone, &stype, &config)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    ?;

    Ok(serde_json::json!({
        "name": name,
        "root_path": root_path,
        "nodes_scanned": count,
    }))
}

#[tauri::command]
pub fn ui_create_tag(state: State<AppState>, name: String, color: String) -> Result<serde_json::Value, String> {
    use synaptic_graph::models::NewTag;
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let tag = db.create_tag(&NewTag { name: name.clone(), color: color.clone() })
        .map_err(|e| format!("DB: {}", e))?;
    Ok(serde_json::json!({"name": tag.name, "color": tag.color}))
}

#[tauri::command]
pub fn ui_delete_tag(state: State<AppState>, name: String) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    db.delete_tag(&name).map_err(|e| format!("DB: {}", e))?;
    Ok(serde_json::json!({"deleted": name}))
}

#[tauri::command]
pub fn ui_tag_impulse(state: State<AppState>, impulse_id: String, tag_name: String) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    db.tag_impulse(&impulse_id, &tag_name).map_err(|e| format!("DB: {}", e))?;
    Ok(serde_json::json!({"tagged": true}))
}

#[tauri::command]
pub fn ui_untag_impulse(state: State<AppState>, impulse_id: String, tag_name: String) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    db.untag_impulse(&impulse_id, &tag_name).map_err(|e| format!("DB: {}", e))?;
    Ok(serde_json::json!({"untagged": true}))
}

#[tauri::command]
pub fn ui_link_memories(state: State<AppState>, source_id: String, target_id: String, relationship: Option<String>) -> Result<serde_json::Value, String> {
    use synaptic_graph::ingestion;
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let rel = relationship.unwrap_or_else(|| "relates_to".to_string());
    let conn = ingestion::manual_link(&db, &source_id, &target_id, &rel, 0.5)?;
    Ok(serde_json::json!({"id": conn.id, "source_id": conn.source_id, "target_id": conn.target_id}))
}

#[tauri::command]
pub fn ui_unlink_memories(state: State<AppState>, connection_id: String) -> Result<serde_json::Value, String> {
    use synaptic_graph::ingestion;
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    ingestion::unlink(&db, &connection_id)?;
    Ok(serde_json::json!({"unlinked": connection_id}))
}

#[tauri::command]
pub fn quick_save_import(
    state: State<AppState>,
    content: String,
    impulse_type: String,
    source_provider: Option<String>,
) -> Result<serde_json::Value, String> {
    use synaptic_graph::ingestion;
    use synaptic_graph::models::*;

    let itype = ImpulseType::from_str(&impulse_type).unwrap_or(ImpulseType::Observation);
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;

    // Use lower weight for imports — they're scaffolding, not understanding
    let input = NewImpulse {
        content: content.clone(),
        impulse_type: itype,
        initial_weight: 0.3, // Low weight — fades faster unless reinforced
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Low,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "import".to_string(),
        source_provider: source_provider.unwrap_or_else(|| "import".to_string()),
        source_account: String::new(),
    };

    let impulse = db.insert_impulse(&input).map_err(|e| format!("DB: {}", e))?;
    db.confirm_impulse(&impulse.id).map_err(|e| format!("Confirm: {}", e))?;

    // Auto-link to existing memories
    let _ = ingestion::auto_link(&db, &impulse.id);

    // Auto-tag as "imported"
    let _ = db.create_tag(&NewTag {
        name: "imported".to_string(),
        color: "#8E99A4".to_string(),
    });
    let _ = db.tag_impulse(&impulse.id, "imported");

    Ok(serde_json::json!({
        "id": impulse.id,
        "content": impulse.content,
        "weight": 0.3,
        "status": "confirmed",
    }))
}

#[tauri::command]
pub fn analyze_memory_profile(state: State<AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let impulses = db.list_impulses(Some(ImpulseStatus::Confirmed))
        .map_err(|e| format!("DB: {}", e))?;

    let total = impulses.len();
    let mut by_type: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut by_source: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut imported = 0;
    let mut high_weight = 0;
    let mut patterns = 0;
    let mut heuristics = 0;

    for imp in &impulses {
        *by_type.entry(imp.impulse_type.as_str().to_string()).or_insert(0) += 1;
        *by_source.entry(imp.source_provider.clone()).or_insert(0) += 1;
        if imp.source_ref == "import" { imported += 1; }
        if imp.weight >= 0.6 { high_weight += 1; }
        if imp.impulse_type == ImpulseType::Pattern { patterns += 1; }
        if imp.impulse_type == ImpulseType::Heuristic { heuristics += 1; }
    }

    let connections = db.connection_count().map_err(|e| format!("DB: {}", e))?;

    // Build gap analysis
    let mut gaps: Vec<String> = vec![];

    if patterns == 0 {
        gaps.push("No behavioral patterns recorded yet. These build over time as the AI observes how you think and work.".to_string());
    }
    if heuristics == 0 {
        gaps.push("No deep insights yet. These are friend-level observations — contradictions, emotional patterns, growth edges.".to_string());
    }
    if total > 0 && imported > 0 && imported as f64 / total as f64 > 0.8 {
        gaps.push(format!("{}% of your memories are imported. These are shallow facts that fade quickly. Deeper understanding builds from real conversations.", (imported as f64 / total as f64 * 100.0) as i32));
    }
    if connections < 5 {
        gaps.push("Few connections between memories. As you use the system more, memories will link together into a richer understanding.".to_string());
    }

    // Build depth score (0-100)
    let pattern_score = (patterns as f64 * 15.0).min(30.0);
    let heuristic_score = (heuristics as f64 * 20.0).min(40.0);
    let connection_score = (connections as f64 * 2.0).min(20.0);
    let diversity_score = (by_type.len() as f64 * 2.0).min(10.0);
    let depth_score = (pattern_score + heuristic_score + connection_score + diversity_score).min(100.0) as i32;

    let depth_label = match depth_score {
        0..=20 => "Surface — mostly facts, not much understanding yet",
        21..=50 => "Building — some patterns emerging, keep going",
        51..=75 => "Developing — real understanding forming",
        _ => "Deep — rich relational model with patterns and insights",
    };

    Ok(serde_json::json!({
        "total_memories": total,
        "imported_count": imported,
        "by_type": by_type,
        "by_source": by_source,
        "high_weight_count": high_weight,
        "connections": connections,
        "depth_score": depth_score,
        "depth_label": depth_label,
        "gaps": gaps,
    }))
}
