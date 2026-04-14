# Phase 5: Tauri UI — Memory Galaxy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri v2 desktop app that visualizes the memory graph as an interactive dark-cosmos galaxy with orbital zoom, ghost graph overlays, search with activation path highlighting, and a node detail panel.

**Architecture:** Tauri v2 app with Svelte frontend. Pixi.js on WebGL Canvas for rendering (handles 10K+ nodes). d3-force for layout calculation. The memory-graph Rust crate is a direct library dependency — Tauri commands call into it without MCP/HTTP. The existing crate needs a lib target added alongside its bin target.

**Tech Stack:** Tauri v2, Svelte 5, TypeScript, Pixi.js v8, d3-force, memory-graph crate (path dependency)

**Depends on:** Phase 1 complete (core memory graph). Phase 2 recommended (ghost graphs). The UI will work with Phase 1 only but ghost graph features require Phase 2.

---

## File Structure

```
memory-graph/
  Cargo.toml              # MODIFY: add [lib] target, keep [bin] target
  src/
    lib.rs                # CREATE: re-exports public API for use as library
    main.rs               # EXISTING: MCP binary entry point (unchanged)
    ...                   # all existing modules
  ui/
    src-tauri/
      Cargo.toml          # Tauri app, depends on memory-graph = { path = "../.." }
      tauri.conf.json     # Window config, title, size
      src/
        main.rs           # Tauri entry, registers commands
        commands.rs        # Tauri command handlers wrapping memory-graph API
    src/
      App.svelte          # Root component: sidebar + main content area
      app.css             # Global dark cosmos styles
      lib/
        api.ts            # Tauri invoke wrappers (typed)
        types.ts          # TypeScript types mirroring Rust models
        stores.ts         # Svelte stores for graph data, selection, zoom state
        Galaxy.svelte     # Main galaxy canvas component (hosts Pixi.js)
        DetailPanel.svelte # Right-side node detail panel
        Sidebar.svelte    # Left navigation sidebar
        SearchPalette.svelte # Cmd+K search overlay
        StatsView.svelte  # Memory statistics view
        GhostList.svelte  # Ghost graph management view
        renderer/
          engine.ts       # Pixi.js Application setup, render loop
          nodes.ts        # Node sprite rendering (stars, glow, sizing)
          connections.ts  # Connection line rendering (light trails)
          nebula.ts       # Cluster nebula glow effects (radial gradients)
          camera.ts       # Pan/zoom camera with orbital zoom levels
          clusters.ts     # Cluster detection and color assignment
          layout.ts       # d3-force simulation wrapper
          ghost.ts        # Ghost node ethereal rendering
          search.ts       # Activation path highlighting
    package.json
    vite.config.ts
    svelte.config.js
    tsconfig.json
```

---

### Task 1: Add Library Target to Existing Crate

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`

The existing crate is a binary. We need to add a library target so the Tauri app can import it as a dependency.

- [ ] **Step 1: Create src/lib.rs that re-exports the public API**

`src/lib.rs`:
```rust
pub mod activation;
pub mod db;
pub mod ingestion;
pub mod models;
pub mod redaction;
pub mod server;
pub mod session;
pub mod weight;

#[cfg(feature = "ghost")]
pub mod ghost;

#[cfg(feature = "ghost")]
pub mod extraction;

#[cfg(feature = "backup")]
pub mod backup;

#[cfg(feature = "backup")]
pub mod sync;
```

- [ ] **Step 2: Update Cargo.toml to have both lib and bin targets**

Add to `Cargo.toml`:
```toml
[lib]
name = "memory_graph"
path = "src/lib.rs"

[[bin]]
name = "memory-graph"
path = "src/main.rs"

[features]
default = ["ghost", "backup"]
ghost = ["walkdir", "pulldown-cmark"]
backup = ["sha2"]
```

Move `walkdir`, `pulldown-cmark`, and `sha2` from `[dependencies]` to optional:
```toml
walkdir = { version = "2", optional = true }
pulldown-cmark = { version = "0.10", optional = true }
sha2 = { version = "0.10", optional = true }
```

- [ ] **Step 3: Update src/main.rs to import from lib instead of declaring modules**

`src/main.rs`:
```rust
use memory_graph::db::Database;
use memory_graph::server::{McpHandler, MemoryGraphServer};

// ... rest of main.rs unchanged, just replace mod declarations with use imports
```

- [ ] **Step 4: Verify everything still compiles and tests pass**

Run: `cargo build && cargo test 2>&1`
Expected: All existing tests pass, both lib and bin targets compile

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/main.rs
git commit -m "feat: add library target to memory-graph crate for Tauri integration"
```

---

### Task 2: Scaffold Tauri v2 App with Svelte

**Files:**
- Create: `ui/` directory with full Tauri v2 + Svelte scaffold

- [ ] **Step 1: Create the Tauri app**

Run from repo root:
```bash
cd /Users/jwgrogan/GitHub/memory-graph && npm create tauri-app@latest ui -- --template svelte-ts --manager npm
```

If the interactive prompt doesn't work in the agent context, create the structure manually:

- [ ] **Step 2: Create package.json**

`ui/package.json`:
```json
{
  "name": "memory-graph-ui",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-shell": "^2",
    "pixi.js": "^8",
    "d3-force": "^3"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4",
    "@tauri-apps/cli": "^2",
    "@types/d3-force": "^3",
    "svelte": "^5",
    "svelte-check": "^4",
    "typescript": "^5",
    "vite": "^6"
  }
}
```

- [ ] **Step 3: Create vite.config.ts**

`ui/vite.config.ts`:
```typescript
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
```

- [ ] **Step 4: Create svelte.config.js**

`ui/svelte.config.js`:
```javascript
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```

- [ ] **Step 5: Create tsconfig.json**

`ui/tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "types": ["svelte"]
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"]
}
```

- [ ] **Step 6: Create index.html**

`ui/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Memory Galaxy</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 7: Create src/main.ts**

`ui/src/main.ts`:
```typescript
import App from "./App.svelte";
import "./app.css";
import { mount } from "svelte";

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
```

- [ ] **Step 8: Create app.css with dark cosmos base styles**

`ui/src/app.css`:
```css
:root {
  --bg-deep: #06060f;
  --bg-surface: #0d0d1a;
  --bg-panel: #111127;
  --border-subtle: rgba(99, 102, 241, 0.15);
  --text-primary: #e2e8f0;
  --text-secondary: #94a3b8;
  --text-muted: #64748b;
  --accent-indigo: #818cf8;
  --accent-violet: #a78bfa;
  --accent-pink: #f472b6;
  --accent-amber: #fbbf24;
  --accent-teal: #2dd4bf;
  --accent-cyan: #67e8f9;
  --ghost-opacity: 0.35;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: var(--bg-deep);
  color: var(--text-primary);
  font-family: system-ui, -apple-system, sans-serif;
  overflow: hidden;
  height: 100vh;
  width: 100vw;
}

#app {
  height: 100vh;
  width: 100vw;
  display: flex;
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: var(--bg-surface);
}

::-webkit-scrollbar-thumb {
  background: var(--border-subtle);
  border-radius: 3px;
}
```

- [ ] **Step 9: Create minimal App.svelte**

`ui/src/App.svelte`:
```svelte
<script lang="ts">
  // Placeholder — will be built out in later tasks
</script>

<div class="app-layout">
  <div class="galaxy-container">
    <p style="color: var(--text-muted); text-align: center; margin-top: 40vh;">
      Memory Galaxy — loading...
    </p>
  </div>
</div>

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .galaxy-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 10: Create Tauri backend — Cargo.toml**

`ui/src-tauri/Cargo.toml`:
```toml
[package]
name = "memory-graph-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
memory-graph = { path = "../..", features = ["ghost", "backup"] }
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 11: Create Tauri build.rs**

`ui/src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 12: Create tauri.conf.json**

`ui/src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://raw.githubusercontent.com/nicegui/tauri/main/crates/tauri-config-schema/schema.json",
  "productName": "Memory Galaxy",
  "version": "0.1.0",
  "identifier": "com.memory-graph.galaxy",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev"
  },
  "app": {
    "title": "Memory Galaxy",
    "windows": [
      {
        "title": "Memory Galaxy",
        "width": 1400,
        "height": 900,
        "minWidth": 800,
        "minHeight": 600,
        "decorations": true,
        "transparent": false
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

- [ ] **Step 13: Create Tauri main.rs stub**

`ui/src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_all_impulses,
            commands::get_all_connections,
            commands::get_memory_stats,
            commands::search_memories,
            commands::get_impulse_detail,
            commands::get_ghost_sources,
            commands::get_ghost_nodes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running memory-graph-ui");
}
```

- [ ] **Step 14: Create commands.rs stub**

`ui/src-tauri/src/commands.rs`:
```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct UiImpulse {
    pub id: String,
    pub content: String,
    pub impulse_type: String,
    pub weight: f64,
    pub emotional_valence: String,
    pub engagement_level: String,
    pub x: f64,
    pub y: f64,
}

#[tauri::command]
pub fn get_all_impulses() -> Result<Vec<UiImpulse>, String> {
    Ok(vec![]) // stub
}

#[tauri::command]
pub fn get_all_connections() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![]) // stub
}

#[tauri::command]
pub fn get_memory_stats() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({"total_impulses": 0, "total_connections": 0}))
}

#[tauri::command]
pub fn search_memories(query: String, max_results: Option<usize>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({"memories": [], "ghost_activations": []}))
}

#[tauri::command]
pub fn get_impulse_detail(id: String) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}

#[tauri::command]
pub fn get_ghost_sources() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}

#[tauri::command]
pub fn get_ghost_nodes(source_name: String) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![])
}
```

- [ ] **Step 15: Install npm dependencies and verify Tauri compiles**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && npm install
cd /Users/jwgrogan/GitHub/memory-graph/ui && cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: npm install succeeds, Tauri Rust backend compiles

- [ ] **Step 16: Commit**

```bash
git add ui/ src/lib.rs Cargo.toml
git commit -m "feat: scaffold Tauri v2 app with Svelte frontend and memory-graph crate integration"
```

---

### Task 3: Tauri Commands — Wire Backend Data

**Files:**
- Modify: `ui/src-tauri/src/commands.rs`

Replace stubs with real memory-graph API calls.

- [ ] **Step 1: Implement commands with real database access**

`ui/src-tauri/src/commands.rs`:
```rust
use memory_graph::db::Database;
use memory_graph::activation::ActivationEngine;
use memory_graph::models::*;
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
    // Get connections for all confirmed impulses
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
```

- [ ] **Step 2: Update main.rs to manage AppState**

`ui/src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use commands::AppState;

fn main() {
    let state = AppState::new().expect("Failed to initialize memory-graph database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::get_all_impulses,
            commands::get_all_connections,
            commands::get_memory_stats,
            commands::search_memories,
            commands::get_impulse_detail,
            commands::get_ghost_sources,
            commands::get_ghost_nodes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running memory-graph-ui");
}
```

Add `dirs` to Tauri Cargo.toml dependencies:
```toml
dirs = "5"
```

- [ ] **Step 3: Verify Tauri backend compiles**

Run: `cargo build --manifest-path ui/src-tauri/Cargo.toml 2>&1`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add ui/src-tauri/
git commit -m "feat: implement Tauri commands wrapping memory-graph API"
```

---

### Task 4: TypeScript Types and API Layer

**Files:**
- Create: `ui/src/lib/types.ts`
- Create: `ui/src/lib/api.ts`
- Create: `ui/src/lib/stores.ts`

- [ ] **Step 1: Create TypeScript types**

`ui/src/lib/types.ts`:
```typescript
export interface Impulse {
  id: string;
  content: string;
  impulse_type: string;
  weight: number;
  initial_weight: number;
  emotional_valence: string;
  engagement_level: string;
  source_type: string;
  source_ref: string;
  status: string;
  created_at: string;
  last_accessed_at: string;
}

export interface Connection {
  id: string;
  source_id: string;
  target_id: string;
  weight: number;
  relationship: string;
  traversal_count: number;
}

export interface GraphNode {
  impulse: Impulse;
  x: number;
  y: number;
  vx: number;
  vy: number;
  cluster: number;
  color: string;
  radius: number;
  isGhost: boolean;
}

export interface GraphEdge {
  connection: Connection;
  source: GraphNode;
  target: GraphNode;
}

export interface GhostSource {
  name: string;
  root_path: string;
  source_type: string;
  node_count: number;
  last_scanned_at: string | null;
}

export interface SearchResult {
  memories: {
    id: string;
    content: string;
    activation_score: number;
    activation_path: string[];
  }[];
  ghost_activations: {
    ghost_node_id: string;
    title: string;
    source_graph: string;
    activation_score: number;
  }[];
  total_activated: number;
}

export interface MemoryStats {
  total_impulses: number;
  confirmed_impulses: number;
  candidate_impulses: number;
  total_connections: number;
}

export interface ImpulseDetail {
  impulse: Impulse;
  connections: {
    id: string;
    other_id: string;
    other_content: string;
    relationship: string;
    weight: number;
    traversal_count: number;
  }[];
}

export type ZoomLevel = "galaxy" | "cluster" | "node";

export const NEBULA_COLORS = [
  "#6366f1", // indigo
  "#ec4899", // pink
  "#f59e0b", // amber
  "#14b8a6", // teal
  "#8b5cf6", // violet
  "#ef4444", // red
  "#06b6d4", // cyan
  "#22c55e", // green
  "#f97316", // orange
  "#a855f7", // purple
] as const;
```

- [ ] **Step 2: Create API wrapper**

`ui/src/lib/api.ts`:
```typescript
import { invoke } from "@tauri-apps/api/core";
import type {
  Impulse,
  Connection,
  MemoryStats,
  SearchResult,
  ImpulseDetail,
  GhostSource,
} from "./types";

export async function getAllImpulses(): Promise<Impulse[]> {
  return invoke<Impulse[]>("get_all_impulses");
}

export async function getAllConnections(): Promise<Connection[]> {
  return invoke<Connection[]>("get_all_connections");
}

export async function getMemoryStats(): Promise<MemoryStats> {
  return invoke<MemoryStats>("get_memory_stats");
}

export async function searchMemories(
  query: string,
  maxResults?: number
): Promise<SearchResult> {
  return invoke<SearchResult>("search_memories", {
    query,
    maxResults: maxResults ?? 20,
  });
}

export async function getImpulseDetail(id: string): Promise<ImpulseDetail> {
  return invoke<ImpulseDetail>("get_impulse_detail", { id });
}

export async function getGhostSources(): Promise<GhostSource[]> {
  return invoke<GhostSource[]>("get_ghost_sources");
}

export async function getGhostNodes(
  sourceName: string
): Promise<Record<string, unknown>[]> {
  return invoke<Record<string, unknown>[]>("get_ghost_nodes", { sourceName });
}
```

- [ ] **Step 3: Create Svelte stores**

`ui/src/lib/stores.ts`:
```typescript
import { writable, derived } from "svelte/store";
import type {
  GraphNode,
  GraphEdge,
  Impulse,
  Connection,
  ZoomLevel,
  ImpulseDetail,
  SearchResult,
} from "./types";

// Raw data from backend
export const impulses = writable<Impulse[]>([]);
export const connections = writable<Connection[]>([]);

// Graph state
export const nodes = writable<GraphNode[]>([]);
export const edges = writable<GraphEdge[]>([]);

// UI state
export const zoomLevel = writable<ZoomLevel>("galaxy");
export const selectedNodeId = writable<string | null>(null);
export const selectedDetail = writable<ImpulseDetail | null>(null);
export const focusedCluster = writable<number | null>(null);
export const searchOpen = writable(false);
export const searchResults = writable<SearchResult | null>(null);
export const activationPath = writable<Set<string>>(new Set());
export const currentView = writable<"galaxy" | "ghosts" | "stats">("galaxy");

// Camera state
export const camera = writable({
  x: 0,
  y: 0,
  zoom: 1,
});

// Derived
export const nodeCount = derived(nodes, ($nodes) => $nodes.length);
export const edgeCount = derived(edges, ($edges) => $edges.length);
```

- [ ] **Step 4: Verify frontend compiles**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && npx svelte-check
```

Expected: No errors (warnings OK)

- [ ] **Step 5: Commit**

```bash
git add ui/src/lib/
git commit -m "feat: add TypeScript types, API wrappers, and Svelte stores"
```

---

### Task 5: Pixi.js Renderer — Core Engine and Force Layout

**Files:**
- Create: `ui/src/lib/renderer/engine.ts`
- Create: `ui/src/lib/renderer/layout.ts`
- Create: `ui/src/lib/renderer/nodes.ts`
- Create: `ui/src/lib/renderer/connections.ts`
- Create: `ui/src/lib/renderer/camera.ts`
- Create: `ui/src/lib/renderer/clusters.ts`
- Modify: `ui/src/lib/Galaxy.svelte`

This is the core rendering engine. Nodes as star sprites, connections as lines, force-directed layout via d3-force, pan/zoom camera.

- [ ] **Step 1: Create the Pixi.js engine**

`ui/src/lib/renderer/engine.ts`:
```typescript
import { Application, Container } from "pixi.js";
import type { GraphNode, GraphEdge } from "../types";
import { renderNodes, updateNodes } from "./nodes";
import { renderConnections, updateConnections } from "./connections";
import { Camera } from "./camera";

export class GalaxyEngine {
  app: Application;
  camera: Camera;
  worldContainer: Container;
  connectionLayer: Container;
  nodeLayer: Container;
  private resizeObserver: ResizeObserver | null = null;

  constructor() {
    this.app = new Application();
    this.camera = new Camera();
    this.worldContainer = new Container();
    this.connectionLayer = new Container();
    this.nodeLayer = new Container();
  }

  async init(canvas: HTMLCanvasElement) {
    await this.app.init({
      canvas,
      background: 0x06060f,
      resizeTo: canvas.parentElement ?? undefined,
      antialias: true,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    });

    this.worldContainer.addChild(this.connectionLayer);
    this.worldContainer.addChild(this.nodeLayer);
    this.app.stage.addChild(this.worldContainer);

    // Enable interactivity
    this.app.stage.eventMode = "static";
    this.app.stage.hitArea = this.app.screen;

    this.setupPanZoom(canvas);
  }

  private setupPanZoom(canvas: HTMLCanvasElement) {
    let isDragging = false;
    let lastX = 0;
    let lastY = 0;

    canvas.addEventListener("mousedown", (e) => {
      isDragging = true;
      lastX = e.clientX;
      lastY = e.clientY;
    });

    canvas.addEventListener("mousemove", (e) => {
      if (!isDragging) return;
      const dx = e.clientX - lastX;
      const dy = e.clientY - lastY;
      this.camera.pan(dx, dy);
      lastX = e.clientX;
      lastY = e.clientY;
      this.applyCameraTransform();
    });

    canvas.addEventListener("mouseup", () => {
      isDragging = false;
    });

    canvas.addEventListener("mouseleave", () => {
      isDragging = false;
    });

    canvas.addEventListener("wheel", (e) => {
      e.preventDefault();
      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      this.camera.zoomAt(e.offsetX, e.offsetY, zoomFactor);
      this.applyCameraTransform();
    }, { passive: false });
  }

  applyCameraTransform() {
    this.worldContainer.x = this.camera.x;
    this.worldContainer.y = this.camera.y;
    this.worldContainer.scale.set(this.camera.zoom);
  }

  renderGraph(nodes: GraphNode[], edges: GraphEdge[]) {
    renderConnections(this.connectionLayer, edges);
    renderNodes(this.nodeLayer, nodes);
    this.centerOnGraph(nodes);
  }

  updateGraph(nodes: GraphNode[], edges: GraphEdge[]) {
    updateConnections(this.connectionLayer, edges);
    updateNodes(this.nodeLayer, nodes);
  }

  centerOnGraph(nodes: GraphNode[]) {
    if (nodes.length === 0) return;
    const cx = nodes.reduce((s, n) => s + n.x, 0) / nodes.length;
    const cy = nodes.reduce((s, n) => s + n.y, 0) / nodes.length;
    const screenW = this.app.screen.width;
    const screenH = this.app.screen.height;
    this.camera.x = screenW / 2 - cx * this.camera.zoom;
    this.camera.y = screenH / 2 - cy * this.camera.zoom;
    this.applyCameraTransform();
  }

  destroy() {
    this.resizeObserver?.disconnect();
    this.app.destroy(true);
  }
}
```

- [ ] **Step 2: Create camera module**

`ui/src/lib/renderer/camera.ts`:
```typescript
import type { ZoomLevel } from "../types";

export class Camera {
  x = 0;
  y = 0;
  zoom = 1;

  private minZoom = 0.05;
  private maxZoom = 8;

  pan(dx: number, dy: number) {
    this.x += dx;
    this.y += dy;
  }

  zoomAt(screenX: number, screenY: number, factor: number) {
    const newZoom = Math.max(this.minZoom, Math.min(this.maxZoom, this.zoom * factor));
    const ratio = newZoom / this.zoom;

    this.x = screenX - (screenX - this.x) * ratio;
    this.y = screenY - (screenY - this.y) * ratio;
    this.zoom = newZoom;
  }

  getZoomLevel(): ZoomLevel {
    if (this.zoom < 0.4) return "galaxy";
    if (this.zoom < 2.0) return "cluster";
    return "node";
  }

  zoomToFit(
    cx: number,
    cy: number,
    radius: number,
    screenWidth: number,
    screenHeight: number
  ) {
    const targetZoom = Math.min(screenWidth, screenHeight) / (radius * 3);
    this.zoom = Math.max(this.minZoom, Math.min(this.maxZoom, targetZoom));
    this.x = screenWidth / 2 - cx * this.zoom;
    this.y = screenHeight / 2 - cy * this.zoom;
  }
}
```

- [ ] **Step 3: Create cluster detection**

`ui/src/lib/renderer/clusters.ts`:
```typescript
import type { Impulse, Connection } from "../types";
import { NEBULA_COLORS } from "../types";

export interface Cluster {
  id: number;
  nodeIds: Set<string>;
  color: string;
  cx: number;
  cy: number;
}

/**
 * Detect clusters using connected components on strongly-weighted connections.
 * Threshold of 0.3 means only connections with weight >= 0.3 form cluster bonds.
 */
export function detectClusters(
  impulses: Impulse[],
  connections: Connection[],
  threshold = 0.3
): Map<string, number> {
  const nodeToCluster = new Map<string, number>();
  const adjacency = new Map<string, Set<string>>();

  // Build adjacency for strong connections only
  for (const impulse of impulses) {
    adjacency.set(impulse.id, new Set());
  }

  for (const conn of connections) {
    if (conn.weight >= threshold) {
      adjacency.get(conn.source_id)?.add(conn.target_id);
      adjacency.get(conn.target_id)?.add(conn.source_id);
    }
  }

  // BFS to find connected components
  let clusterId = 0;
  const visited = new Set<string>();

  for (const impulse of impulses) {
    if (visited.has(impulse.id)) continue;

    const queue = [impulse.id];
    visited.add(impulse.id);

    while (queue.length > 0) {
      const current = queue.shift()!;
      nodeToCluster.set(current, clusterId);

      for (const neighbor of adjacency.get(current) ?? []) {
        if (!visited.has(neighbor)) {
          visited.add(neighbor);
          queue.push(neighbor);
        }
      }
    }

    clusterId++;
  }

  return nodeToCluster;
}

export function getClusterColor(clusterId: number): string {
  return NEBULA_COLORS[clusterId % NEBULA_COLORS.length];
}

export function buildClusterInfo(
  nodeToCluster: Map<string, number>,
  positions: Map<string, { x: number; y: number }>
): Cluster[] {
  const clusters = new Map<number, Cluster>();

  for (const [nodeId, clusterId] of nodeToCluster) {
    if (!clusters.has(clusterId)) {
      clusters.set(clusterId, {
        id: clusterId,
        nodeIds: new Set(),
        color: getClusterColor(clusterId),
        cx: 0,
        cy: 0,
      });
    }
    clusters.get(clusterId)!.nodeIds.add(nodeId);
  }

  // Calculate cluster centers
  for (const cluster of clusters.values()) {
    let sumX = 0;
    let sumY = 0;
    let count = 0;
    for (const nodeId of cluster.nodeIds) {
      const pos = positions.get(nodeId);
      if (pos) {
        sumX += pos.x;
        sumY += pos.y;
        count++;
      }
    }
    if (count > 0) {
      cluster.cx = sumX / count;
      cluster.cy = sumY / count;
    }
  }

  return Array.from(clusters.values());
}
```

- [ ] **Step 4: Create force-directed layout**

`ui/src/lib/renderer/layout.ts`:
```typescript
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  type SimulationNodeDatum,
  type SimulationLinkDatum,
} from "d3-force";
import type { Impulse, Connection, GraphNode, GraphEdge } from "../types";
import { detectClusters, getClusterColor } from "./clusters";

interface SimNode extends SimulationNodeDatum {
  id: string;
  impulse: Impulse;
  cluster: number;
  color: string;
  radius: number;
  isGhost: boolean;
}

interface SimLink extends SimulationLinkDatum<SimNode> {
  connection: Connection;
}

export function computeLayout(
  impulses: Impulse[],
  connections: Connection[]
): { nodes: GraphNode[]; edges: GraphEdge[] } {
  const nodeToCluster = detectClusters(impulses, connections);

  const simNodes: SimNode[] = impulses.map((impulse) => {
    const cluster = nodeToCluster.get(impulse.id) ?? 0;
    const weight = impulse.weight;
    // Node radius: 3 to 12 based on weight
    const radius = 3 + weight * 9;

    return {
      id: impulse.id,
      impulse,
      cluster,
      color: getClusterColor(cluster),
      radius,
      isGhost: false,
      x: undefined,
      y: undefined,
    };
  });

  const nodeMap = new Map(simNodes.map((n) => [n.id, n]));

  const simLinks: SimLink[] = connections
    .filter(
      (c) => nodeMap.has(c.source_id) && nodeMap.has(c.target_id)
    )
    .map((c) => ({
      source: c.source_id,
      target: c.target_id,
      connection: c,
    }));

  // Run force simulation
  const simulation = forceSimulation<SimNode>(simNodes)
    .force(
      "link",
      forceLink<SimNode, SimLink>(simLinks)
        .id((d) => d.id)
        .distance(80)
        .strength((d) => d.connection.weight * 0.5)
    )
    .force("charge", forceManyBody().strength(-120))
    .force("center", forceCenter(0, 0))
    .force("collide", forceCollide<SimNode>().radius((d) => d.radius + 2))
    .stop();

  // Run synchronously for 300 ticks
  for (let i = 0; i < 300; i++) {
    simulation.tick();
  }

  // Convert to GraphNode/GraphEdge
  const graphNodes: GraphNode[] = simNodes.map((n) => ({
    impulse: n.impulse,
    x: n.x ?? 0,
    y: n.y ?? 0,
    vx: 0,
    vy: 0,
    cluster: n.cluster,
    color: n.color,
    radius: n.radius,
    isGhost: false,
  }));

  const graphNodeMap = new Map(graphNodes.map((n) => [n.impulse.id, n]));

  const graphEdges: GraphEdge[] = simLinks
    .map((l) => {
      const sourceNode = graphNodeMap.get(
        typeof l.source === "string" ? l.source : (l.source as SimNode).id
      );
      const targetNode = graphNodeMap.get(
        typeof l.target === "string" ? l.target : (l.target as SimNode).id
      );
      if (!sourceNode || !targetNode) return null;
      return {
        connection: l.connection,
        source: sourceNode,
        target: targetNode,
      };
    })
    .filter((e): e is GraphEdge => e !== null);

  return { nodes: graphNodes, edges: graphEdges };
}
```

- [ ] **Step 5: Create node renderer**

`ui/src/lib/renderer/nodes.ts`:
```typescript
import { Container, Graphics } from "pixi.js";
import type { GraphNode } from "../types";

export function renderNodes(layer: Container, nodes: GraphNode[]) {
  layer.removeChildren();

  for (const node of nodes) {
    const g = new Graphics();

    // Outer glow
    const glowAlpha = node.impulse.engagement_level === "high" ? 0.3 : 0.15;
    const glowRadius = node.radius * 3;
    g.circle(0, 0, glowRadius);
    g.fill({ color: node.color, alpha: glowAlpha });

    // Core star
    const coreAlpha = 0.5 + node.impulse.weight * 0.5;
    g.circle(0, 0, node.radius);
    g.fill({ color: node.color, alpha: coreAlpha });

    // Bright center
    g.circle(0, 0, node.radius * 0.4);
    g.fill({ color: 0xffffff, alpha: 0.6 + node.impulse.weight * 0.4 });

    g.x = node.x;
    g.y = node.y;

    // Store node ID for hit detection
    g.eventMode = "static";
    g.cursor = "pointer";
    g.label = node.impulse.id;

    layer.addChild(g);
  }
}

export function updateNodes(layer: Container, nodes: GraphNode[]) {
  // For now, just re-render. Optimize later with sprite pooling.
  renderNodes(layer, nodes);
}

export function highlightNode(layer: Container, nodeId: string, highlight: boolean) {
  for (const child of layer.children) {
    if (child.label === nodeId) {
      child.alpha = highlight ? 1.0 : 0.7;
      child.scale.set(highlight ? 1.3 : 1.0);
    }
  }
}
```

- [ ] **Step 6: Create connection renderer**

`ui/src/lib/renderer/connections.ts`:
```typescript
import { Container, Graphics } from "pixi.js";
import type { GraphEdge } from "../types";

export function renderConnections(layer: Container, edges: GraphEdge[]) {
  layer.removeChildren();

  const g = new Graphics();

  for (const edge of edges) {
    const weight = edge.connection.weight;
    const alpha = Math.max(0.05, weight * 0.4);
    const width = 0.5 + weight * 1.5;

    // Parse source color for the line
    const color = edge.source.color;

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
  }

  layer.addChild(g);
}

export function updateConnections(layer: Container, edges: GraphEdge[]) {
  renderConnections(layer, edges);
}

export function highlightPath(
  layer: Container,
  edges: GraphEdge[],
  pathNodeIds: Set<string>
) {
  layer.removeChildren();
  const g = new Graphics();

  for (const edge of edges) {
    const isOnPath =
      pathNodeIds.has(edge.source.impulse.id) &&
      pathNodeIds.has(edge.target.impulse.id);

    const alpha = isOnPath ? 0.8 : 0.05;
    const width = isOnPath ? 2 : 0.5;
    const color = isOnPath ? 0xfbbf24 : edge.source.color;

    g.moveTo(edge.source.x, edge.source.y);
    g.lineTo(edge.target.x, edge.target.y);
    g.stroke({ color, width, alpha });
  }

  layer.addChild(g);
}
```

- [ ] **Step 7: Create Galaxy.svelte component**

`ui/src/lib/Galaxy.svelte`:
```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { GalaxyEngine } from "./renderer/engine";
  import { computeLayout } from "./renderer/layout";
  import { nodes, edges, impulses, connections, selectedNodeId, zoomLevel } from "./stores";
  import { getAllImpulses, getAllConnections } from "./api";

  let canvasEl: HTMLCanvasElement;
  let engine: GalaxyEngine;

  onMount(async () => {
    engine = new GalaxyEngine();
    await engine.init(canvasEl);

    // Load data
    const impulseData = await getAllImpulses();
    const connectionData = await getAllConnections();
    impulses.set(impulseData);
    connections.set(connectionData);

    // Compute layout
    const layout = computeLayout(impulseData, connectionData);
    nodes.set(layout.nodes);
    edges.set(layout.edges);

    // Render
    engine.renderGraph(layout.nodes, layout.edges);

    // Handle node clicks
    engine.nodeLayer.on("pointerdown", (e) => {
      const target = e.target;
      if (target?.label) {
        selectedNodeId.set(target.label);
      }
    });
  });

  onDestroy(() => {
    engine?.destroy();
  });
</script>

<canvas bind:this={canvasEl} class="galaxy-canvas"></canvas>

<style>
  .galaxy-canvas {
    width: 100%;
    height: 100%;
    display: block;
  }
</style>
```

- [ ] **Step 8: Update App.svelte to include Galaxy**

`ui/src/App.svelte`:
```svelte
<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import { selectedNodeId, currentView } from "./lib/stores";
</script>

<div class="app-layout">
  <div class="galaxy-container">
    {#if $currentView === "galaxy"}
      <Galaxy />
    {:else}
      <p style="color: var(--text-muted); text-align: center; margin-top: 40vh;">
        {$currentView} view — coming soon
      </p>
    {/if}
  </div>
</div>

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .galaxy-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 9: Verify the app compiles and renders**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && npm run build
cargo build --manifest-path ui/src-tauri/Cargo.toml
```

Expected: Both frontend and backend compile. To visually verify, run `cd ui && cargo tauri dev` (requires a display).

- [ ] **Step 10: Commit**

```bash
git add ui/src/lib/renderer/ ui/src/lib/Galaxy.svelte ui/src/App.svelte
git commit -m "feat: implement Pixi.js galaxy renderer with force-directed layout and cluster detection"
```

---

### Task 6: Nebula Glow Effects

**Files:**
- Create: `ui/src/lib/renderer/nebula.ts`
- Modify: `ui/src/lib/renderer/engine.ts`

- [ ] **Step 1: Create nebula renderer**

`ui/src/lib/renderer/nebula.ts`:
```typescript
import { Container, Graphics } from "pixi.js";
import type { Cluster } from "./clusters";

/**
 * Render nebula glow effects behind node clusters.
 * Each cluster gets a radial gradient cloud.
 */
export function renderNebulae(
  layer: Container,
  clusters: Cluster[],
  nodePositions: Map<string, { x: number; y: number }>
) {
  layer.removeChildren();

  for (const cluster of clusters) {
    if (cluster.nodeIds.size < 2) continue;

    // Calculate cluster bounds
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;

    for (const nodeId of cluster.nodeIds) {
      const pos = nodePositions.get(nodeId);
      if (pos) {
        minX = Math.min(minX, pos.x);
        maxX = Math.max(maxX, pos.x);
        minY = Math.min(minY, pos.y);
        maxY = Math.max(maxY, pos.y);
      }
    }

    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    const rx = (maxX - minX) / 2 + 40;
    const ry = (maxY - minY) / 2 + 40;
    const radius = Math.max(rx, ry);

    // Draw multiple concentric circles with decreasing opacity for glow effect
    const g = new Graphics();
    const color = cluster.color;

    // Outer haze
    g.circle(cx, cy, radius * 1.5);
    g.fill({ color, alpha: 0.03 });

    // Mid glow
    g.circle(cx, cy, radius * 1.0);
    g.fill({ color, alpha: 0.06 });

    // Inner glow
    g.circle(cx, cy, radius * 0.6);
    g.fill({ color, alpha: 0.1 });

    // Core
    g.circle(cx, cy, radius * 0.3);
    g.fill({ color, alpha: 0.15 });

    layer.addChild(g);
  }
}
```

- [ ] **Step 2: Add nebula layer to engine**

Update `ui/src/lib/renderer/engine.ts` — add a nebulaLayer behind connections:

Add field:
```typescript
nebulaLayer: Container;
```

In constructor:
```typescript
this.nebulaLayer = new Container();
```

In init, add before connectionLayer:
```typescript
this.worldContainer.addChild(this.nebulaLayer);
```

Add method:
```typescript
renderNebulae(clusters: Cluster[], nodePositions: Map<string, { x: number; y: number }>) {
  renderNebulae(this.nebulaLayer, clusters, nodePositions);
}
```

- [ ] **Step 3: Update Galaxy.svelte to render nebulae**

After computing layout, add:
```typescript
import { buildClusterInfo, detectClusters } from "./renderer/clusters";

// After layout computation:
const nodeToCluster = detectClusters(impulseData, connectionData);
const posMap = new Map(layout.nodes.map(n => [n.impulse.id, { x: n.x, y: n.y }]));
const clusterInfo = buildClusterInfo(nodeToCluster, posMap);
engine.renderNebulae(clusterInfo, posMap);
```

- [ ] **Step 4: Verify visually**

Run: `cd /Users/jwgrogan/GitHub/memory-graph/ui && cargo tauri dev`
Expected: Nebula glows appear behind node clusters. Clusters have distinct colors.

- [ ] **Step 5: Commit**

```bash
git add ui/src/lib/renderer/nebula.ts ui/src/lib/renderer/engine.ts ui/src/lib/Galaxy.svelte
git commit -m "feat: add nebula glow effects behind node clusters"
```

---

### Task 7: Detail Panel

**Files:**
- Create: `ui/src/lib/DetailPanel.svelte`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: Create DetailPanel component**

`ui/src/lib/DetailPanel.svelte`:
```svelte
<script lang="ts">
  import { selectedNodeId, selectedDetail } from "./stores";
  import { getImpulseDetail } from "./api";
  import type { ImpulseDetail } from "./types";

  let detail: ImpulseDetail | null = null;
  let loading = false;

  $: if ($selectedNodeId) {
    loadDetail($selectedNodeId);
  } else {
    detail = null;
  }

  async function loadDetail(id: string) {
    loading = true;
    try {
      detail = await getImpulseDetail(id);
      selectedDetail.set(detail);
    } catch (e) {
      console.error("Failed to load detail:", e);
      detail = null;
    }
    loading = false;
  }

  function close() {
    selectedNodeId.set(null);
    selectedDetail.set(null);
  }

  function navigateTo(id: string) {
    selectedNodeId.set(id);
  }

  function engagementColor(level: string): string {
    switch (level) {
      case "high": return "#fbbf24";
      case "medium": return "#94a3b8";
      case "low": return "#475569";
      default: return "#64748b";
    }
  }

  function valenceColor(valence: string): string {
    switch (valence) {
      case "positive": return "#4ade80";
      case "negative": return "#f87171";
      case "neutral": return "#94a3b8";
      default: return "#64748b";
    }
  }

  function relativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const hours = Math.floor(diffMs / 3600000);
    if (hours < 1) return "just now";
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    return `${Math.floor(days / 30)}mo ago`;
  }
</script>

{#if $selectedNodeId && detail}
  <div class="detail-panel">
    <button class="close-btn" on:click={close}>x</button>

    <h2 class="content">{detail.impulse.content}</h2>

    <div class="meta-row">
      <div class="meta-item">
        <span class="label">Weight</span>
        <div class="weight-bar">
          <div class="weight-fill" style="width: {detail.impulse.weight * 100}%"></div>
        </div>
        <span class="value">{detail.impulse.weight.toFixed(2)}</span>
      </div>
    </div>

    <div class="meta-row">
      <div class="meta-item">
        <span class="label">Engagement</span>
        <span class="dot" style="background: {engagementColor(detail.impulse.engagement_level)}"></span>
        <span class="value">{detail.impulse.engagement_level}</span>
      </div>
      <div class="meta-item">
        <span class="label">Valence</span>
        <span class="dot" style="background: {valenceColor(detail.impulse.emotional_valence)}"></span>
        <span class="value">{detail.impulse.emotional_valence}</span>
      </div>
    </div>

    <div class="meta-row">
      <div class="meta-item">
        <span class="label">Type</span>
        <span class="value">{detail.impulse.impulse_type}</span>
      </div>
      <div class="meta-item">
        <span class="label">Source</span>
        <span class="value">{detail.impulse.source_type}</span>
      </div>
    </div>

    <div class="meta-row">
      <span class="label">Last accessed</span>
      <span class="value muted">{relativeTime(detail.impulse.last_accessed_at)}</span>
    </div>

    {#if detail.connections.length > 0}
      <div class="connections-section">
        <span class="label">Connections ({detail.connections.length})</span>
        {#each detail.connections as conn}
          <button
            class="connection-card"
            style="opacity: {Math.max(0.4, conn.weight)}"
            on:click={() => navigateTo(conn.other_id)}
          >
            <div class="conn-content">{conn.other_content.slice(0, 80)}{conn.other_content.length > 80 ? '...' : ''}</div>
            <div class="conn-meta">{conn.relationship} · weight {conn.weight.toFixed(2)}</div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .detail-panel {
    position: absolute;
    top: 0;
    right: 0;
    width: 320px;
    height: 100%;
    background: var(--bg-panel);
    border-left: 1px solid var(--border-subtle);
    padding: 20px;
    overflow-y: auto;
    z-index: 10;
  }

  .close-btn {
    position: absolute;
    top: 12px;
    right: 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
  }

  .content {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
    line-height: 1.5;
  }

  .meta-row {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .value {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .value.muted {
    color: var(--text-muted);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .weight-bar {
    width: 80px;
    height: 5px;
    background: rgba(99, 102, 241, 0.15);
    border-radius: 3px;
    overflow: hidden;
  }

  .weight-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-indigo), var(--accent-violet));
    border-radius: 3px;
  }

  .connections-section {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .connection-card {
    background: rgba(99, 102, 241, 0.08);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 10px;
    cursor: pointer;
    text-align: left;
    color: inherit;
    font: inherit;
  }

  .connection-card:hover {
    background: rgba(99, 102, 241, 0.15);
  }

  .conn-content {
    font-size: 12px;
    color: var(--accent-indigo);
    margin-bottom: 4px;
  }

  .conn-meta {
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 2: Add DetailPanel to App.svelte**

Update `ui/src/App.svelte`:
```svelte
<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import DetailPanel from "./lib/DetailPanel.svelte";
  import { selectedNodeId, currentView } from "./lib/stores";
</script>

<div class="app-layout">
  <div class="galaxy-container">
    {#if $currentView === "galaxy"}
      <Galaxy />
      <DetailPanel />
    {:else}
      <p style="color: var(--text-muted); text-align: center; margin-top: 40vh;">
        {$currentView} view — coming soon
      </p>
    {/if}
  </div>
</div>
```

- [ ] **Step 3: Verify compilation**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && npm run build
```

- [ ] **Step 4: Commit**

```bash
git add ui/src/lib/DetailPanel.svelte ui/src/App.svelte
git commit -m "feat: add detail panel with weight bar, engagement/valence indicators, and navigable connections"
```

---

### Task 8: Search Palette

**Files:**
- Create: `ui/src/lib/SearchPalette.svelte`
- Create: `ui/src/lib/renderer/search.ts`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: Create search highlighting module**

`ui/src/lib/renderer/search.ts`:
```typescript
import type { GraphEdge } from "../types";
import { highlightPath } from "./connections";
import { highlightNode } from "./nodes";
import type { Container } from "pixi.js";

export function applySearchHighlight(
  connectionLayer: Container,
  nodeLayer: Container,
  edges: GraphEdge[],
  activatedNodeIds: Set<string>
) {
  highlightPath(connectionLayer, edges, activatedNodeIds);

  for (const child of nodeLayer.children) {
    if (child.label) {
      const isActivated = activatedNodeIds.has(child.label);
      child.alpha = isActivated ? 1.0 : 0.15;
      child.scale.set(isActivated ? 1.2 : 0.8);
    }
  }
}

export function clearSearchHighlight(
  connectionLayer: Container,
  nodeLayer: Container,
  edges: GraphEdge[]
) {
  highlightPath(connectionLayer, edges, new Set());

  for (const child of nodeLayer.children) {
    child.alpha = 1.0;
    child.scale.set(1.0);
  }
}
```

- [ ] **Step 2: Create SearchPalette component**

`ui/src/lib/SearchPalette.svelte`:
```svelte
<script lang="ts">
  import { searchOpen, searchResults, activationPath } from "./stores";
  import { searchMemories } from "./api";
  import type { SearchResult } from "./types";

  let query = "";
  let loading = false;

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      searchOpen.update((v) => !v);
      if (!$searchOpen) {
        clearSearch();
      }
    }
    if (e.key === "Escape" && $searchOpen) {
      searchOpen.set(false);
      clearSearch();
    }
  }

  async function handleSearch() {
    if (!query.trim()) return;
    loading = true;
    try {
      const results = await searchMemories(query);
      searchResults.set(results);

      // Build activation path set
      const pathIds = new Set<string>();
      for (const mem of results.memories) {
        pathIds.add(mem.id);
        for (const pathId of mem.activation_path) {
          pathIds.add(pathId);
        }
      }
      activationPath.set(pathIds);
    } catch (e) {
      console.error("Search failed:", e);
    }
    loading = false;
  }

  function clearSearch() {
    query = "";
    searchResults.set(null);
    activationPath.set(new Set());
  }

  function selectResult(id: string) {
    // Dispatch a custom event that Galaxy.svelte can listen to
    window.dispatchEvent(new CustomEvent("navigate-to-node", { detail: { id } }));
    searchOpen.set(false);
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if $searchOpen}
  <div class="search-overlay" on:click={() => { searchOpen.set(false); clearSearch(); }}>
    <div class="search-palette" on:click|stopPropagation>
      <input
        class="search-input"
        placeholder="Search your memory..."
        bind:value={query}
        on:keydown={(e) => e.key === "Enter" && handleSearch()}
        autofocus
      />

      {#if loading}
        <div class="search-status">Searching...</div>
      {/if}

      {#if $searchResults}
        <div class="results">
          {#if $searchResults.memories.length === 0}
            <div class="search-status">No memories found</div>
          {/if}

          {#each $searchResults.memories as mem}
            <button class="result-item" on:click={() => selectResult(mem.id)}>
              <div class="result-content">{mem.content.slice(0, 100)}{mem.content.length > 100 ? '...' : ''}</div>
              <div class="result-meta">activation: {mem.activation_score.toFixed(3)}</div>
            </button>
          {/each}

          {#if $searchResults.ghost_activations.length > 0}
            <div class="ghost-section-label">Ghost Graph Matches</div>
            {#each $searchResults.ghost_activations as ghost}
              <div class="result-item ghost">
                <div class="result-content">{ghost.title}</div>
                <div class="result-meta">{ghost.source_graph} · score {ghost.activation_score.toFixed(3)}</div>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .search-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    padding-top: 15vh;
    z-index: 100;
  }

  .search-palette {
    width: 560px;
    max-height: 500px;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .search-input {
    width: 100%;
    padding: 16px 20px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
    font-size: 16px;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-status {
    padding: 12px 20px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .results {
    overflow-y: auto;
    max-height: 380px;
  }

  .result-item {
    width: 100%;
    padding: 12px 20px;
    border: none;
    border-bottom: 1px solid rgba(99, 102, 241, 0.08);
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }

  .result-item:hover {
    background: rgba(99, 102, 241, 0.1);
  }

  .result-item.ghost {
    cursor: default;
    opacity: 0.6;
  }

  .result-content {
    font-size: 13px;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .result-meta {
    font-size: 11px;
    color: var(--text-muted);
  }

  .ghost-section-label {
    padding: 8px 20px 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent-cyan);
    opacity: 0.6;
  }
</style>
```

- [ ] **Step 3: Add SearchPalette to App.svelte**

Add import and component:
```svelte
<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import DetailPanel from "./lib/DetailPanel.svelte";
  import SearchPalette from "./lib/SearchPalette.svelte";
  import { selectedNodeId, currentView } from "./lib/stores";
</script>

<div class="app-layout">
  <div class="galaxy-container">
    {#if $currentView === "galaxy"}
      <Galaxy />
      <DetailPanel />
    {:else}
      <p style="color: var(--text-muted); text-align: center; margin-top: 40vh;">
        {$currentView} view — coming soon
      </p>
    {/if}
  </div>
</div>

<SearchPalette />
```

- [ ] **Step 4: Commit**

```bash
git add ui/src/lib/SearchPalette.svelte ui/src/lib/renderer/search.ts ui/src/App.svelte
git commit -m "feat: add Cmd+K search palette with activation path highlighting"
```

---

### Task 9: Sidebar and Stats View

**Files:**
- Create: `ui/src/lib/Sidebar.svelte`
- Create: `ui/src/lib/StatsView.svelte`
- Create: `ui/src/lib/GhostList.svelte`
- Modify: `ui/src/App.svelte`

- [ ] **Step 1: Create Sidebar**

`ui/src/lib/Sidebar.svelte`:
```svelte
<script lang="ts">
  import { currentView } from "./stores";

  const views = [
    { id: "galaxy" as const, label: "Galaxy", icon: "✦" },
    { id: "ghosts" as const, label: "Ghost Graphs", icon: "◌" },
    { id: "stats" as const, label: "Stats", icon: "▦" },
  ];
</script>

<nav class="sidebar">
  <div class="logo">MG</div>
  {#each views as view}
    <button
      class="nav-item"
      class:active={$currentView === view.id}
      on:click={() => currentView.set(view.id)}
    >
      <span class="nav-icon">{view.icon}</span>
      <span class="nav-label">{view.label}</span>
    </button>
  {/each}
</nav>

<style>
  .sidebar {
    width: 56px;
    background: var(--bg-surface);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 12px;
    gap: 4px;
    flex-shrink: 0;
  }

  .logo {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-indigo);
    margin-bottom: 16px;
    opacity: 0.7;
  }

  .nav-item {
    width: 40px;
    height: 40px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    font: inherit;
  }

  .nav-item:hover {
    background: rgba(99, 102, 241, 0.1);
    color: var(--text-secondary);
  }

  .nav-item.active {
    background: rgba(99, 102, 241, 0.15);
    color: var(--accent-indigo);
  }

  .nav-icon {
    font-size: 16px;
  }

  .nav-label {
    font-size: 7px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
</style>
```

- [ ] **Step 2: Create StatsView**

`ui/src/lib/StatsView.svelte`:
```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { getMemoryStats } from "./api";
  import type { MemoryStats } from "./types";

  let stats: MemoryStats | null = null;

  onMount(async () => {
    stats = await getMemoryStats();
  });
</script>

<div class="stats-view">
  <h2>Memory Stats</h2>

  {#if stats}
    <div class="stat-grid">
      <div class="stat-card">
        <div class="stat-value">{stats.total_impulses}</div>
        <div class="stat-label">Total Memories</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.confirmed_impulses}</div>
        <div class="stat-label">Confirmed</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.candidate_impulses}</div>
        <div class="stat-label">Candidates</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.total_connections}</div>
        <div class="stat-label">Connections</div>
      </div>
    </div>
  {:else}
    <p class="loading">Loading stats...</p>
  {/if}
</div>

<style>
  .stats-view {
    padding: 32px;
    max-width: 600px;
  }

  h2 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 20px;
  }

  .stat-value {
    font-size: 32px;
    font-weight: 700;
    color: var(--accent-indigo);
  }

  .stat-label {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .loading {
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 3: Create GhostList**

`ui/src/lib/GhostList.svelte`:
```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { getGhostSources } from "./api";
  import type { GhostSource } from "./types";

  let sources: GhostSource[] = [];

  onMount(async () => {
    sources = await getGhostSources();
  });
</script>

<div class="ghost-list">
  <h2>Ghost Graphs</h2>

  {#if sources.length === 0}
    <p class="empty">No ghost graphs registered. Use the MCP tools to register an external knowledge base.</p>
  {:else}
    {#each sources as source}
      <div class="ghost-card">
        <div class="ghost-name">{source.name}</div>
        <div class="ghost-meta">
          <span>{source.source_type}</span>
          <span>{source.node_count} nodes</span>
          {#if source.last_scanned_at}
            <span>scanned {new Date(source.last_scanned_at).toLocaleDateString()}</span>
          {/if}
        </div>
        <div class="ghost-path">{source.root_path}</div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .ghost-list {
    padding: 32px;
    max-width: 600px;
  }

  h2 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .empty {
    color: var(--text-muted);
    font-size: 14px;
  }

  .ghost-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
  }

  .ghost-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--accent-cyan);
    margin-bottom: 6px;
  }

  .ghost-meta {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .ghost-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: monospace;
  }
</style>
```

- [ ] **Step 4: Update App.svelte with sidebar and views**

`ui/src/App.svelte`:
```svelte
<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import DetailPanel from "./lib/DetailPanel.svelte";
  import SearchPalette from "./lib/SearchPalette.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import StatsView from "./lib/StatsView.svelte";
  import GhostList from "./lib/GhostList.svelte";
  import { currentView } from "./lib/stores";
</script>

<div class="app-layout">
  <Sidebar />

  <div class="main-content">
    {#if $currentView === "galaxy"}
      <Galaxy />
      <DetailPanel />
    {:else if $currentView === "stats"}
      <StatsView />
    {:else if $currentView === "ghosts"}
      <GhostList />
    {/if}
  </div>
</div>

<SearchPalette />

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .main-content {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
</style>
```

- [ ] **Step 5: Verify compilation**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && npm run build
```

- [ ] **Step 6: Commit**

```bash
git add ui/src/lib/Sidebar.svelte ui/src/lib/StatsView.svelte ui/src/lib/GhostList.svelte ui/src/App.svelte
git commit -m "feat: add sidebar navigation, stats view, and ghost graph list"
```

---

### Task 10: Ghost Graph Ethereal Rendering

**Files:**
- Create: `ui/src/lib/renderer/ghost.ts`
- Modify: `ui/src/lib/renderer/engine.ts`

- [ ] **Step 1: Create ghost node renderer**

`ui/src/lib/renderer/ghost.ts`:
```typescript
import { Container, Graphics } from "pixi.js";
import type { GraphNode } from "../types";

export function renderGhostNodes(layer: Container, ghostNodes: GraphNode[]) {
  for (const node of ghostNodes) {
    const g = new Graphics();

    // Ethereal outer glow — very faint
    g.circle(0, 0, node.radius * 2.5);
    g.fill({ color: 0x67e8f9, alpha: 0.06 });

    // Ghost core — translucent
    g.circle(0, 0, node.radius);
    g.fill({ color: 0x67e8f9, alpha: 0.2 + node.impulse.weight * 0.2 });

    // Tiny bright center for pulled-through nodes
    if (node.impulse.weight > 0.4) {
      g.circle(0, 0, node.radius * 0.3);
      g.fill({ color: 0xa5f3fc, alpha: 0.5 });
    }

    g.x = node.x;
    g.y = node.y;
    g.eventMode = "static";
    g.cursor = "pointer";
    g.label = node.impulse.id;

    layer.addChild(g);
  }
}

export function renderGhostConnections(
  layer: Container,
  edges: { sourceX: number; sourceY: number; targetX: number; targetY: number; weight: number }[]
) {
  const g = new Graphics();

  for (const edge of edges) {
    const alpha = Math.max(0.03, edge.weight * 0.15);
    g.moveTo(edge.sourceX, edge.sourceY);
    g.lineTo(edge.targetX, edge.targetY);
    g.stroke({ color: 0x67e8f9, width: 0.5, alpha });
  }

  layer.addChild(g);
}

export function renderBridgeConnections(
  layer: Container,
  bridges: { sourceX: number; sourceY: number; targetX: number; targetY: number }[]
) {
  const g = new Graphics();

  for (const bridge of bridges) {
    // Dashed effect via short segments
    const dx = bridge.targetX - bridge.sourceX;
    const dy = bridge.targetY - bridge.sourceY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const segments = Math.floor(dist / 8);
    const ux = dx / dist;
    const uy = dy / dist;

    for (let i = 0; i < segments; i += 2) {
      const sx = bridge.sourceX + ux * i * 8;
      const sy = bridge.sourceY + uy * i * 8;
      const ex = bridge.sourceX + ux * Math.min((i + 1) * 8, dist);
      const ey = bridge.sourceY + uy * Math.min((i + 1) * 8, dist);

      g.moveTo(sx, sy);
      g.lineTo(ex, ey);
      g.stroke({ color: 0x67e8f9, width: 0.5, alpha: 0.12 });
    }
  }

  layer.addChild(g);
}
```

- [ ] **Step 2: Add ghost layer to engine**

Add to `GalaxyEngine`:
```typescript
ghostLayer: Container;
```

In constructor:
```typescript
this.ghostLayer = new Container();
```

In init, add between nebulaLayer and connectionLayer:
```typescript
this.worldContainer.addChild(this.ghostLayer);
```

- [ ] **Step 3: Commit**

```bash
git add ui/src/lib/renderer/ghost.ts ui/src/lib/renderer/engine.ts
git commit -m "feat: add ghost node ethereal rendering with bridge connections"
```

---

### Task 11: Full Build and Visual Verification

- [ ] **Step 1: Build frontend**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && npm run build
```

Expected: No errors

- [ ] **Step 2: Build Tauri backend**

```bash
cargo build --manifest-path ui/src-tauri/Cargo.toml
```

Expected: Compiles successfully

- [ ] **Step 3: Run the full app (if display available)**

```bash
cd /Users/jwgrogan/GitHub/memory-graph/ui && cargo tauri dev
```

Expected: Window opens with dark cosmos background. If the database has memories, they render as a galaxy. If empty, the canvas is dark with no nodes (expected).

- [ ] **Step 4: Run existing memory-graph tests to confirm no regressions**

```bash
cd /Users/jwgrogan/GitHub/memory-graph && cargo test
```

Expected: All existing Phase 1-4 tests still pass

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: Phase 5 Tauri UI complete — Memory Galaxy with orbital zoom, detail panel, search, sidebar"
```

---

## Post-Phase-5 Notes

After completing all tasks:

1. Tauri v2 app builds and launches with dark cosmos aesthetic
2. Force-directed graph layout positions nodes with cluster grouping
3. Nebula glow effects render behind clusters
4. Three zoom levels reveal progressive detail
5. Detail panel shows node content, weight, engagement, connections
6. Cmd+K search runs spreading activation and highlights results
7. Sidebar switches between Galaxy, Ghost Graphs, and Stats views
8. Ghost nodes render with ethereal treatment
9. All existing backend tests still pass

**Visual verification requires a display** — the agent loop can build and compile but visual confirmation needs a human looking at the screen. After the loop completes, run `cd ui && cargo tauri dev` to verify.
