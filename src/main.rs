use synaptic_graph::server::{McpHandler, MemoryGraphServer};
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialise minimal logging (controlled via RUST_LOG env var).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    // Determine database path:
    //   1. MEMORY_GRAPH_DB env var, or
    //   2. <data_local_dir>/synaptic-graph/memory.db
    let db_path = match std::env::var("MEMORY_GRAPH_DB") {
        Ok(p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => {
            // Check ~/.local/share first (matches MCP config convention), then dirs default
            let home = dirs::home_dir().ok_or("Could not determine home directory")?;
            let linux_style = home.join(".local/share/synaptic-graph/memory.db");
            if linux_style.exists() {
                linux_style
            } else {
                let base = dirs::data_local_dir()
                    .ok_or("Could not determine data_local_dir for this platform")?;
                let mac_style = base.join("synaptic-graph").join("memory.db");
                if mac_style.exists() {
                    mac_style
                } else {
                    // Default to ~/.local/share for new installs
                    home.join(".local/share/synaptic-graph/memory.db")
                }
            }
        }
    };

    // Ensure the parent directory exists.
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_path_str = db_path
        .to_str()
        .ok_or("Database path contains invalid UTF-8")?;

    let server = MemoryGraphServer::new(db_path_str)
        .map_err(|e| format!("Failed to initialise MemoryGraphServer: {}", e))?;
    let handler = McpHandler::new(server);

    // Start the MCP server on stdio transport.
    let service = handler.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;

    Ok(())
}
