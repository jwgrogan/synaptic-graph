use synaptic_graph::server::{McpHandler, MemoryGraphServer};
use synaptic_graph::db::Database;
use synaptic_graph::activation::ActivationEngine;
use synaptic_graph::models::*;
use rmcp::ServiceExt;
use std::path::PathBuf;

fn resolve_db_path() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("MEMORY_GRAPH_DB") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    let linux_style = home.join(".local/share/synaptic-graph/memory.db");
    if linux_style.exists() {
        return Ok(linux_style);
    }
    let mac_style = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("synaptic-graph")
        .join("memory.db");
    if mac_style.exists() {
        return Ok(mac_style);
    }
    Ok(home.join(".local/share/synaptic-graph/memory.db"))
}

/// CLI mode: retrieve context and print to stdout for use in hooks/scripts
fn cli_retrieve(query: &str, max_results: usize) -> Result<(), String> {
    let db_path = resolve_db_path()?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let db = Database::open(db_path.to_str().unwrap_or("memory.db"))
        .map_err(|e| format!("DB error: {}", e))?;

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: query.to_string(),
        max_results,
        max_hops: 3,
    };

    let result = engine.retrieve(&request)?;

    if result.memories.is_empty() {
        return Ok(()); // Silent — no output if no memories
    }

    // Output as clean text for injection into context
    println!("[Recalled memories — use these to inform your response naturally, never cite them directly]");
    for mem in &result.memories {
        let tags = db.get_tags_for_impulse(&mem.impulse.id).unwrap_or_default();
        let tag_str = if tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", "))
        };
        println!("- ({}) {}{}", mem.impulse.impulse_type.as_str(), mem.impulse.content, tag_str);
    }

    Ok(())
}

/// CLI mode: quick save a memory
fn cli_save(content: &str, impulse_type: &str) -> Result<(), String> {
    let db_path = resolve_db_path()?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let db = Database::open(db_path.to_str().unwrap_or("memory.db"))
        .map_err(|e| format!("DB error: {}", e))?;

    let itype = ImpulseType::from_str(impulse_type).unwrap_or(ImpulseType::Observation);

    let impulse = synaptic_graph::ingestion::save_and_confirm(
        &db,
        content,
        itype,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "cli",
    )?;

    eprintln!("Saved: {} ({})", impulse.id, impulse.impulse_type.as_str());
    Ok(())
}

/// CLI mode: get memory status
fn cli_status() -> Result<(), String> {
    let db_path = resolve_db_path()?;
    let db = Database::open(db_path.to_str().unwrap_or("memory.db"))
        .map_err(|e| format!("DB error: {}", e))?;

    let stats = db.memory_stats().map_err(|e| format!("DB error: {}", e))?;
    println!("{} memories ({} confirmed, {} candidates), {} connections",
        stats.total_impulses, stats.confirmed_impulses, stats.candidate_impulses, stats.total_connections);
    Ok(())
}

/// CLI mode: merge another database into the local one
fn cli_merge(other_db_path: &str) -> Result<(), String> {
    let local_path = resolve_db_path()?;
    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let local_path_str = local_path.to_str().unwrap_or("memory.db");

    // Verify the other DB exists
    if !std::path::Path::new(other_db_path).exists() {
        return Err(format!("File not found: {}", other_db_path));
    }

    eprintln!("Merging {} into {}", other_db_path, local_path_str);

    // Open both databases
    let other_db = Database::open(other_db_path)
        .map_err(|e| format!("Failed to open source DB: {}", e))?;
    let local_db = Database::open(local_path_str)
        .map_err(|e| format!("Failed to open local DB: {}", e))?;

    let other_impulses = other_db.list_impulses(None)
        .map_err(|e| format!("Failed to read source: {}", e))?;

    let mut inserted = 0;
    let mut skipped = 0;

    for imp in &other_impulses {
        match local_db.get_impulse(&imp.id) {
            Ok(_) => {
                skipped += 1; // Already exists locally
            }
            Err(_) => {
                // New record — insert with original ID
                let input = NewImpulse {
                    content: imp.content.clone(),
                    impulse_type: imp.impulse_type,
                    initial_weight: imp.initial_weight,
                    emotional_valence: imp.emotional_valence,
                    engagement_level: imp.engagement_level,
                    source_signals: imp.source_signals.clone(),
                    source_type: imp.source_type,
                    source_ref: imp.source_ref.clone(),
                    source_provider: imp.source_provider.clone(),
                    source_account: imp.source_account.clone(),
                };
                match local_db.insert_impulse_with_id(&imp.id, &input) {
                    Ok(_) => {
                        if imp.status == ImpulseStatus::Confirmed {
                            let _ = local_db.confirm_impulse(&imp.id);
                        }
                        inserted += 1;
                    }
                    Err(e) => {
                        eprintln!("  Skip {}: {}", &imp.id[..8], e);
                        skipped += 1;
                    }
                }
            }
        }
    }

    println!("Merged: {} new, {} already existed", inserted, skipped);

    // Also merge connections
    let mut conn_inserted = 0;
    for imp in &other_impulses {
        let conns = other_db.get_connections_for_node(&imp.id).unwrap_or_default();
        for conn in &conns {
            match local_db.get_connection(&conn.id) {
                Ok(_) => {} // Already exists
                Err(_) => {
                    let input = NewConnection {
                        source_id: conn.source_id.clone(),
                        target_id: conn.target_id.clone(),
                        weight: conn.weight,
                        relationship: conn.relationship.clone(),
                    };
                    if local_db.insert_connection(&input).is_ok() {
                        conn_inserted += 1;
                    }
                }
            }
        }
    }

    if conn_inserted > 0 {
        println!("Connections merged: {}", conn_inserted);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // CLI mode — direct commands without MCP
    if args.len() > 1 {
        let result = match args[1].as_str() {
            "retrieve" | "recall" => {
                let query = args.get(2).map(|s| s.as_str()).unwrap_or("");
                let max = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(10);
                cli_retrieve(query, max)
            }
            "save" => {
                let content = args.get(2).map(|s| s.as_str()).unwrap_or("");
                let itype = args.get(3).map(|s| s.as_str()).unwrap_or("observation");
                if content.is_empty() {
                    Err("Usage: synaptic-graph save <content> [type]".to_string())
                } else {
                    cli_save(content, itype)
                }
            }
            "status" => cli_status(),
            "merge" => {
                let path = args.get(2).map(|s| s.as_str()).unwrap_or("");
                if path.is_empty() {
                    Err("Usage: synaptic-graph merge <path-to-other.db>".to_string())
                } else {
                    cli_merge(path)
                }
            }
            "help" | "--help" | "-h" => {
                eprintln!("synaptic-graph — persistent memory for AI\n");
                eprintln!("Usage:");
                eprintln!("  synaptic-graph                    Start MCP server (stdio)");
                eprintln!("  synaptic-graph retrieve <query>   Retrieve relevant memories");
                eprintln!("  synaptic-graph save <content>     Save a memory");
                eprintln!("  synaptic-graph status             Show memory stats");
                eprintln!("  synaptic-graph merge <db-path>    Merge another database into local");
                Ok(())
            }
            _ => {
                eprintln!("Unknown command: {}. Use --help for usage.", args[1]);
                std::process::exit(1);
            }
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
        return Ok(());
    }

    // MCP server mode (no args)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let db_path = resolve_db_path().map_err(|e| format!("Path error: {}", e))?;
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_path_str = db_path
        .to_str()
        .ok_or("Database path contains invalid UTF-8")?;

    let server = MemoryGraphServer::new(db_path_str)
        .map_err(|e| format!("Failed to initialise MemoryGraphServer: {}", e))?;
    let handler = McpHandler::new(server);

    let service = handler.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    Ok(())
}
