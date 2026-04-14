# Phase 2: Ghost Graphs, Adaptive Extraction, and Multi-Client Validation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add ghost graph overlays for external KBs (starting with Obsidian vaults and filesystem directories), implement on-demand pull-through with session-only and permanent modes, build adaptive end-of-session extraction, and validate the MCP tools work across multiple AI clients.

**Architecture:** Ghost graphs are lightweight metadata indexes stored in SQLite alongside the core memory graph. Pull-through reads file content on demand without caching. Adaptive extraction uses heuristics on session metadata to scale proposal depth. The MCP tool surface expands with ghost graph and session extraction tools.

**Tech Stack:** Same as Phase 1 (Rust, rusqlite, rmcp, tokio) plus `walkdir` for filesystem traversal, `pulldown-cmark` for markdown link extraction.

**Depends on:** Phase 1 complete — all Phase 1 tests passing.

---

## File Structure (additions to Phase 1)

```
src/
    ghost/
        mod.rs          # Ghost graph registration, refresh, pull-through orchestration
        scanner.rs      # Filesystem/vault topology scanning
        pull.rs         # On-demand content pull and relevance evaluation
    extraction.rs       # End-of-session adaptive extraction with engagement heuristics
tests/
    test_ghost.rs       # Ghost graph registration, scanning, pull-through, weight learning
    test_extraction.rs  # Adaptive extraction: engagement heuristics, proposal generation
    test_validation_p2.rs  # Phase 2 PRD validation criteria
```

---

### Task 1: Add Phase 2 Dependencies

**Files:**
- Modify: `Cargo.toml`
- Create: `src/ghost/mod.rs`
- Create: `src/ghost/scanner.rs`
- Create: `src/ghost/pull.rs`
- Create: `src/extraction.rs`

- [ ] **Step 1: Add dependencies to Cargo.toml**

Add to `[dependencies]`:
```toml
walkdir = "2"
pulldown-cmark = "0.10"
```

- [ ] **Step 2: Create module files**

`src/ghost/mod.rs`:
```rust
pub mod pull;
pub mod scanner;

use crate::db::Database;
use crate::models::*;

pub use pull::PullMode;
pub use scanner::ScanConfig;
```

`src/ghost/scanner.rs`:
```rust
// Ghost graph filesystem scanner
```

`src/ghost/pull.rs`:
```rust
// On-demand ghost node content pull-through
```

`src/extraction.rs`:
```rust
// Adaptive end-of-session extraction
```

- [ ] **Step 3: Add module declarations to main.rs**

Add to `src/main.rs`:
```rust
pub mod extraction;
pub mod ghost;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build 2>&1`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/ghost/ src/extraction.rs src/main.rs
git commit -m "feat: scaffold Phase 2 modules for ghost graphs and extraction"
```

---

### Task 2: Ghost Node Database Schema

**Files:**
- Modify: `src/db.rs`
- Modify: `src/models.rs`

- [ ] **Step 1: Write failing test for ghost node operations**

Add to `tests/test_db.rs`:
```rust
#[test]
fn test_insert_and_get_ghost_node() {
    let db = common::test_db();
    let input = NewGhostNode {
        source_graph: "obsidian-main-vault".to_string(),
        external_ref: "/notes/design-philosophy.md".to_string(),
        title: "Design Philosophy".to_string(),
        metadata: serde_json::json!({"tags": ["philosophy", "design"], "last_modified": "2026-04-14"}),
        initial_weight: 0.3,
    };

    let node = db.insert_ghost_node(&input).unwrap();
    assert_eq!(node.source_graph, "obsidian-main-vault");
    assert_eq!(node.title, "Design Philosophy");
    assert_eq!(node.weight, 0.3);

    let retrieved = db.get_ghost_node(&node.id).unwrap();
    assert_eq!(retrieved.id, node.id);
}

#[test]
fn test_list_ghost_nodes_by_source() {
    let db = common::test_db();

    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault-a".to_string(),
        external_ref: "/note1.md".to_string(),
        title: "Note 1".to_string(),
        metadata: serde_json::json!({}),
        initial_weight: 0.3,
    }).unwrap();

    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault-a".to_string(),
        external_ref: "/note2.md".to_string(),
        title: "Note 2".to_string(),
        metadata: serde_json::json!({}),
        initial_weight: 0.3,
    }).unwrap();

    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault-b".to_string(),
        external_ref: "/other.md".to_string(),
        title: "Other".to_string(),
        metadata: serde_json::json!({}),
        initial_weight: 0.3,
    }).unwrap();

    let nodes = db.list_ghost_nodes_by_source("vault-a").unwrap();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn test_ghost_node_connections() {
    let db = common::test_db();

    let g1 = db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault".to_string(),
        external_ref: "/a.md".to_string(),
        title: "A".to_string(),
        metadata: serde_json::json!({}),
        initial_weight: 0.3,
    }).unwrap();

    let g2 = db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault".to_string(),
        external_ref: "/b.md".to_string(),
        title: "B".to_string(),
        metadata: serde_json::json!({}),
        initial_weight: 0.3,
    }).unwrap();

    // Ghost-to-ghost connection (structural link from vault)
    db.insert_ghost_connection(&NewGhostConnection {
        source_id: g1.id.clone(),
        target_id: g2.id.clone(),
        weight: 0.5,
        relationship: "links_to".to_string(),
    }).unwrap();

    let conns = db.get_ghost_connections_for_node(&g1.id).unwrap();
    assert_eq!(conns.len(), 1);
}

#[test]
fn test_ghost_source_registry() {
    let db = common::test_db();

    db.register_ghost_source("obsidian-vault", "/Users/jake/vault", "obsidian").unwrap();

    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].name, "obsidian-vault");
    assert_eq!(sources[0].root_path, "/Users/jake/vault");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_insert_and_get_ghost 2>&1`
Expected: Compilation error

- [ ] **Step 3: Add ghost node types to models.rs**

Add to `src/models.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostNode {
    pub id: String,
    pub source_graph: String,
    pub external_ref: String,
    pub title: String,
    pub metadata: serde_json::Value,
    pub weight: f64,
    pub last_accessed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewGhostNode {
    pub source_graph: String,
    pub external_ref: String,
    pub title: String,
    pub metadata: serde_json::Value,
    pub initial_weight: f64,
}

#[derive(Debug, Clone)]
pub struct NewGhostConnection {
    pub source_id: String,
    pub target_id: String,
    pub weight: f64,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostSource {
    pub name: String,
    pub root_path: String,
    pub source_type: String,
    pub registered_at: DateTime<Utc>,
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub node_count: i64,
}
```

- [ ] **Step 4: Add ghost node schema and CRUD to db.rs**

Add to the `create_tables` method in `src/db.rs`:
```rust
CREATE TABLE IF NOT EXISTS ghost_nodes (
    id TEXT PRIMARY KEY,
    source_graph TEXT NOT NULL,
    external_ref TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    metadata TEXT NOT NULL DEFAULT '{}',
    weight REAL NOT NULL DEFAULT 0.3,
    last_accessed_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(source_graph, external_ref)
);

CREATE TABLE IF NOT EXISTS ghost_connections (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 0.5,
    relationship TEXT NOT NULL DEFAULT 'links_to',
    created_at TEXT NOT NULL,
    last_traversed_at TEXT NOT NULL,
    traversal_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ghost_sources (
    name TEXT PRIMARY KEY,
    root_path TEXT NOT NULL,
    source_type TEXT NOT NULL DEFAULT 'directory',
    registered_at TEXT NOT NULL,
    last_scanned_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_ghost_nodes_source ON ghost_nodes(source_graph);
CREATE INDEX IF NOT EXISTS idx_ghost_connections_source ON ghost_connections(source_id);
CREATE INDEX IF NOT EXISTS idx_ghost_connections_target ON ghost_connections(target_id);

CREATE VIRTUAL TABLE IF NOT EXISTS ghost_nodes_fts USING fts5(
    title,
    content_rowid='rowid',
    tokenize='porter'
);
```

Add CRUD methods to `Database`:
```rust
// === Ghost Node Operations ===

pub fn insert_ghost_node(&self, input: &NewGhostNode) -> SqlResult<GhostNode> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let metadata_str = serde_json::to_string(&input.metadata).unwrap_or_default();

    self.conn.execute(
        "INSERT OR REPLACE INTO ghost_nodes (id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, input.source_graph, input.external_ref, input.title, metadata_str, input.initial_weight, now, now],
    )?;

    self.conn.execute(
        "INSERT INTO ghost_nodes_fts (rowid, title)
         SELECT rowid, title FROM ghost_nodes WHERE id = ?1",
        params![id],
    )?;

    self.get_ghost_node(&id)
}

pub fn get_ghost_node(&self, id: &str) -> SqlResult<GhostNode> {
    self.conn.query_row(
        "SELECT id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at
         FROM ghost_nodes WHERE id = ?1",
        params![id],
        |row| {
            let metadata_str: String = row.get(4).unwrap_or_default();
            let created_str: String = row.get(7).unwrap_or_default();
            let accessed_str: String = row.get(6).unwrap_or_default();
            Ok(GhostNode {
                id: row.get(0)?,
                source_graph: row.get(1)?,
                external_ref: row.get(2)?,
                title: row.get(3).unwrap_or_default(),
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                weight: row.get(5).unwrap_or(0.3),
                last_accessed_at: DateTime::parse_from_rfc3339(&accessed_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        },
    )
}

pub fn get_ghost_node_by_ref(&self, source_graph: &str, external_ref: &str) -> SqlResult<GhostNode> {
    self.conn.query_row(
        "SELECT id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at
         FROM ghost_nodes WHERE source_graph = ?1 AND external_ref = ?2",
        params![source_graph, external_ref],
        |row| {
            let metadata_str: String = row.get(4).unwrap_or_default();
            let created_str: String = row.get(7).unwrap_or_default();
            let accessed_str: String = row.get(6).unwrap_or_default();
            Ok(GhostNode {
                id: row.get(0)?,
                source_graph: row.get(1)?,
                external_ref: row.get(2)?,
                title: row.get(3).unwrap_or_default(),
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                weight: row.get(5).unwrap_or(0.3),
                last_accessed_at: DateTime::parse_from_rfc3339(&accessed_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        },
    )
}

pub fn list_ghost_nodes_by_source(&self, source_graph: &str) -> SqlResult<Vec<GhostNode>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at
         FROM ghost_nodes WHERE source_graph = ?1 ORDER BY weight DESC",
    )?;
    let rows = stmt.query_map(params![source_graph], |row| {
        let metadata_str: String = row.get(4).unwrap_or_default();
        let created_str: String = row.get(7).unwrap_or_default();
        let accessed_str: String = row.get(6).unwrap_or_default();
        Ok(GhostNode {
            id: row.get(0)?,
            source_graph: row.get(1)?,
            external_ref: row.get(2)?,
            title: row.get(3).unwrap_or_default(),
            metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
            weight: row.get(5).unwrap_or(0.3),
            last_accessed_at: DateTime::parse_from_rfc3339(&accessed_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    })?;
    rows.collect()
}

pub fn touch_ghost_node(&self, id: &str) -> SqlResult<()> {
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "UPDATE ghost_nodes SET last_accessed_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn update_ghost_node_weight(&self, id: &str, weight: f64) -> SqlResult<()> {
    self.conn.execute(
        "UPDATE ghost_nodes SET weight = ?1 WHERE id = ?2",
        params![weight, id],
    )?;
    Ok(())
}

pub fn delete_ghost_nodes_by_source(&self, source_graph: &str) -> SqlResult<usize> {
    let count = self.conn.execute(
        "DELETE FROM ghost_nodes WHERE source_graph = ?1",
        params![source_graph],
    )?;
    Ok(count)
}

pub fn search_ghost_nodes_fts(&self, query: &str) -> SqlResult<Vec<(String, f64)>> {
    let mut stmt = self.conn.prepare(
        "SELECT g.id, fts.rank
         FROM ghost_nodes_fts fts
         JOIN ghost_nodes g ON g.rowid = fts.rowid
         WHERE ghost_nodes_fts MATCH ?1
         ORDER BY fts.rank",
    )?;
    let rows = stmt.query_map(params![query], |row| {
        let id: String = row.get(0)?;
        let rank: f64 = row.get(1)?;
        Ok((id, rank))
    })?;
    rows.collect()
}

// === Ghost Connection Operations ===

pub fn insert_ghost_connection(&self, input: &NewGhostConnection) -> SqlResult<Connection> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    self.conn.execute(
        "INSERT INTO ghost_connections (id, source_id, target_id, weight, relationship, created_at, last_traversed_at, traversal_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![id, input.source_id, input.target_id, input.weight, input.relationship, now, now],
    )?;

    // Return as a Connection type (same shape)
    self.conn.query_row(
        "SELECT id, source_id, target_id, weight, relationship, created_at, last_traversed_at, traversal_count
         FROM ghost_connections WHERE id = ?1",
        params![id],
        |row| Ok(row_to_connection(row)),
    )
}

pub fn get_ghost_connections_for_node(&self, node_id: &str) -> SqlResult<Vec<Connection>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, source_id, target_id, weight, relationship, created_at, last_traversed_at, traversal_count
         FROM ghost_connections WHERE source_id = ?1 OR target_id = ?1",
    )?;
    let rows = stmt.query_map(params![node_id], |row| Ok(row_to_connection(row)))?;
    rows.collect()
}

// === Ghost Source Registry ===

pub fn register_ghost_source(&self, name: &str, root_path: &str, source_type: &str) -> SqlResult<()> {
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "INSERT OR REPLACE INTO ghost_sources (name, root_path, source_type, registered_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![name, root_path, source_type, now],
    )?;
    Ok(())
}

pub fn update_ghost_source_scanned(&self, name: &str) -> SqlResult<()> {
    let now = Utc::now().to_rfc3339();
    self.conn.execute(
        "UPDATE ghost_sources SET last_scanned_at = ?1 WHERE name = ?2",
        params![now, name],
    )?;
    Ok(())
}

pub fn list_ghost_sources(&self) -> SqlResult<Vec<GhostSource>> {
    let mut stmt = self.conn.prepare(
        "SELECT gs.name, gs.root_path, gs.source_type, gs.registered_at, gs.last_scanned_at,
         (SELECT COUNT(*) FROM ghost_nodes WHERE source_graph = gs.name) as node_count
         FROM ghost_sources gs ORDER BY gs.name",
    )?;
    let rows = stmt.query_map([], |row| {
        let registered_str: String = row.get(3).unwrap_or_default();
        let scanned_str: Option<String> = row.get(4).ok();
        Ok(GhostSource {
            name: row.get(0)?,
            root_path: row.get(1)?,
            source_type: row.get(2).unwrap_or_default(),
            registered_at: DateTime::parse_from_rfc3339(&registered_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_scanned_at: scanned_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            node_count: row.get(5).unwrap_or(0),
        })
    })?;
    rows.collect()
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test test_ghost 2>&1`
Expected: All ghost node DB tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/db.rs src/models.rs tests/test_db.rs
git commit -m "feat: add ghost node schema, CRUD operations, and source registry"
```

---

### Task 3: Ghost Graph Filesystem Scanner

**Files:**
- Modify: `src/ghost/scanner.rs`
- Create: `tests/test_ghost.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_ghost.rs`:
```rust
mod common;

use memory_graph::ghost::scanner::{ScanConfig, scan_directory};
use std::fs;
use tempfile::TempDir;

fn create_test_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create some markdown files with links
    fs::write(root.join("index.md"), "# Index\n\nSee [[design]] and [[architecture]].\n").unwrap();
    fs::write(root.join("design.md"), "# Design\n\nRelates to [[architecture]].\nTagged: #philosophy #design\n").unwrap();
    fs::write(root.join("architecture.md"), "# Architecture\n\nSQLite-based system.\n").unwrap();

    // Create a subdirectory
    fs::create_dir(root.join("notes")).unwrap();
    fs::write(root.join("notes/daily.md"), "# Daily Note\n\nNothing important.\n").unwrap();

    // Create a non-markdown file (should be ignored by default)
    fs::write(root.join("image.png"), "fake png data").unwrap();

    dir
}

#[test]
fn test_scan_discovers_markdown_files() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    assert_eq!(result.nodes.len(), 4); // index, design, architecture, daily
}

#[test]
fn test_scan_extracts_titles() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    let titles: Vec<&str> = result.nodes.iter().map(|n| n.title.as_str()).collect();
    assert!(titles.contains(&"Index"));
    assert!(titles.contains(&"Design"));
    assert!(titles.contains(&"Architecture"));
}

#[test]
fn test_scan_extracts_wikilinks() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    // index.md links to design and architecture
    assert!(result.links.len() >= 3);
}

#[test]
fn test_scan_ignores_non_matching_extensions() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    let refs: Vec<&str> = result.nodes.iter().map(|n| n.external_ref.as_str()).collect();
    assert!(!refs.iter().any(|r| r.contains("image.png")));
}

#[test]
fn test_scan_respects_ignore_patterns() {
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec!["notes/".to_string()],
    };

    let result = scan_directory(vault.path(), &config).unwrap();
    assert_eq!(result.nodes.len(), 3); // daily.md excluded
}
```

Add to Cargo.toml:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_scan 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement scanner**

`src/ghost/scanner.rs`:
```rust
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub extensions: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug)]
pub struct ScanResult {
    pub nodes: Vec<ScannedNode>,
    pub links: Vec<ScannedLink>,
}

#[derive(Debug)]
pub struct ScannedNode {
    pub external_ref: String,
    pub title: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug)]
pub struct ScannedLink {
    pub from_ref: String,
    pub to_ref: String,
    pub link_type: String,
}

static WIKILINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").unwrap()
});

static HEADING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#\s+(.+)$").unwrap()
});

static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#([a-zA-Z][a-zA-Z0-9_-]+)").unwrap()
});

pub fn scan_directory(root: &Path, config: &ScanConfig) -> Result<ScanResult, String> {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy().to_string();

        // Check ignore patterns
        if config.ignore_patterns.iter().any(|p| rel_str.contains(p)) {
            continue;
        }

        // Check extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !config.extensions.iter().any(|e| e == ext) {
            continue;
        }

        // Read file for metadata extraction (titles, tags, links)
        let content = fs::read_to_string(path).unwrap_or_default();

        // Extract title from first heading
        let title = content
            .lines()
            .find_map(|line| {
                HEADING_RE.captures(line).map(|c| c[1].trim().to_string())
            })
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string()
            });

        // Extract tags
        let tags: Vec<String> = TAG_RE
            .captures_iter(&content)
            .map(|c| c[1].to_string())
            .collect();

        // Extract wikilinks
        for cap in WIKILINK_RE.captures_iter(&content) {
            let target = cap[1].trim().to_string();
            // Resolve wikilink to a relative path
            let target_ref = resolve_wikilink(&target, root, &config.extensions);
            if let Some(target_ref) = target_ref {
                links.push(ScannedLink {
                    from_ref: rel_str.clone(),
                    to_ref: target_ref,
                    link_type: "wikilink".to_string(),
                });
            }
        }

        let metadata = serde_json::json!({
            "tags": tags,
            "extension": ext,
        });

        nodes.push(ScannedNode {
            external_ref: rel_str,
            title,
            metadata,
        });
    }

    Ok(ScanResult { nodes, links })
}

fn resolve_wikilink(target: &str, root: &Path, extensions: &[String]) -> Option<String> {
    // Try to find a matching file
    for ext in extensions {
        let candidate = format!("{}.{}", target, ext);
        // Check in root
        if root.join(&candidate).exists() {
            return Some(candidate);
        }
        // Check recursively
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let name = entry.path().file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if name.eq_ignore_ascii_case(target) {
                    let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
                    return Some(rel.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_scan 2>&1`
Expected: All scanner tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ghost/scanner.rs tests/test_ghost.rs Cargo.toml
git commit -m "feat: implement ghost graph filesystem scanner with wikilink extraction"
```

---

### Task 4: Ghost Graph Registration and Refresh Orchestration

**Files:**
- Modify: `src/ghost/mod.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/test_ghost.rs`:
```rust
use memory_graph::ghost;
use memory_graph::ghost::scanner::ScanConfig;

#[test]
fn test_register_and_scan_ghost_graph() {
    let db = common::test_db();
    let vault = create_test_vault();

    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(
        &db,
        "test-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    )
    .unwrap();

    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].name, "test-vault");

    let nodes = db.list_ghost_nodes_by_source("test-vault").unwrap();
    assert_eq!(nodes.len(), 4);
}

#[test]
fn test_refresh_updates_ghost_graph() {
    let db = common::test_db();
    let vault = create_test_vault();

    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    // Add a new file
    std::fs::write(vault.path().join("new-note.md"), "# New Note\n\nFresh content.\n").unwrap();

    ghost::refresh(&db, "test-vault", &config).unwrap();

    let nodes = db.list_ghost_nodes_by_source("test-vault").unwrap();
    assert_eq!(nodes.len(), 5);
}

#[test]
fn test_scan_creates_ghost_connections_from_wikilinks() {
    let db = common::test_db();
    let vault = create_test_vault();

    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    // index.md links to design.md and architecture.md
    let index_node = db.get_ghost_node_by_ref("test-vault", "index.md").unwrap();
    let conns = db.get_ghost_connections_for_node(&index_node.id).unwrap();
    assert!(conns.len() >= 2);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_register 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement ghost graph orchestration**

`src/ghost/mod.rs`:
```rust
pub mod pull;
pub mod scanner;

use std::path::Path;

use crate::db::Database;
use crate::models::*;
use scanner::{ScanConfig, scan_directory};

pub use pull::PullMode;
pub use scanner::ScanConfig as GhostScanConfig;

pub fn register_and_scan(
    db: &Database,
    name: &str,
    root_path: &str,
    source_type: &str,
    config: &ScanConfig,
) -> Result<usize, String> {
    db.register_ghost_source(name, root_path, source_type)
        .map_err(|e| format!("Failed to register source: {}", e))?;

    scan_and_store(db, name, root_path, config)
}

pub fn refresh(
    db: &Database,
    name: &str,
    config: &ScanConfig,
) -> Result<usize, String> {
    let sources = db.list_ghost_sources()
        .map_err(|e| format!("Failed to list sources: {}", e))?;

    let source = sources.iter().find(|s| s.name == name)
        .ok_or_else(|| format!("Ghost source '{}' not found", name))?;

    let root_path = source.root_path.clone();

    // Clear existing nodes for this source
    db.delete_ghost_nodes_by_source(name)
        .map_err(|e| format!("Failed to clear old nodes: {}", e))?;

    let count = scan_and_store(db, name, &root_path, config)?;

    db.update_ghost_source_scanned(name)
        .map_err(|e| format!("Failed to update scan timestamp: {}", e))?;

    Ok(count)
}

fn scan_and_store(
    db: &Database,
    name: &str,
    root_path: &str,
    config: &ScanConfig,
) -> Result<usize, String> {
    let result = scan_directory(Path::new(root_path), config)?;

    // Insert ghost nodes
    for node in &result.nodes {
        db.insert_ghost_node(&NewGhostNode {
            source_graph: name.to_string(),
            external_ref: node.external_ref.clone(),
            title: node.title.clone(),
            metadata: node.metadata.clone(),
            initial_weight: 0.3,
        })
        .map_err(|e| format!("Failed to insert ghost node: {}", e))?;
    }

    // Insert ghost connections from wikilinks
    for link in &result.links {
        let from = db.get_ghost_node_by_ref(name, &link.from_ref);
        let to = db.get_ghost_node_by_ref(name, &link.to_ref);

        if let (Ok(from_node), Ok(to_node)) = (from, to) {
            let _ = db.insert_ghost_connection(&NewGhostConnection {
                source_id: from_node.id,
                target_id: to_node.id,
                weight: 0.5,
                relationship: link.link_type.clone(),
            });
        }
    }

    db.update_ghost_source_scanned(name)
        .map_err(|e| format!("Failed to update scan timestamp: {}", e))?;

    Ok(result.nodes.len())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_ 2>&1`
Expected: All ghost graph tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ghost/mod.rs tests/test_ghost.rs
git commit -m "feat: implement ghost graph registration, scan, and refresh orchestration"
```

---

### Task 5: Ghost Node Pull-Through

**Files:**
- Modify: `src/ghost/pull.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/test_ghost.rs`:
```rust
use memory_graph::ghost::pull::{PullMode, pull_ghost_content};

#[test]
fn test_pull_through_reads_file_content() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let node = db.get_ghost_node_by_ref("test-vault", "design.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    let root = &sources[0].root_path;

    let content = pull_ghost_content(&db, &node, root, PullMode::SessionOnly).unwrap();
    assert!(content.contains("Design"));
    assert!(content.contains("architecture"));
}

#[test]
fn test_pull_through_permanent_creates_impulse() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let node = db.get_ghost_node_by_ref("test-vault", "design.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    let root = &sources[0].root_path;

    let before_count = db.impulse_count().unwrap();

    let content = pull_ghost_content(&db, &node, root, PullMode::Permanent).unwrap();
    assert!(!content.is_empty());

    let after_count = db.impulse_count().unwrap();
    assert!(after_count > before_count);
}

#[test]
fn test_pull_through_session_only_no_impulse() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let node = db.get_ghost_node_by_ref("test-vault", "architecture.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    let root = &sources[0].root_path;

    let before_count = db.impulse_count().unwrap();

    pull_ghost_content(&db, &node, root, PullMode::SessionOnly).unwrap();

    let after_count = db.impulse_count().unwrap();
    assert_eq!(before_count, after_count);
}

#[test]
fn test_pull_through_updates_ghost_weight() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let node = db.get_ghost_node_by_ref("test-vault", "design.md").unwrap();
    let weight_before = node.weight;
    let sources = db.list_ghost_sources().unwrap();
    let root = &sources[0].root_path;

    pull_ghost_content(&db, &node, root, PullMode::SessionOnly).unwrap();

    let node_after = db.get_ghost_node(&node.id).unwrap();
    assert!(node_after.weight > weight_before);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_pull 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement pull-through**

`src/ghost/pull.rs`:
```rust
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
        // Create an impulse from the pulled content
        let input = NewImpulse {
            content: content.clone(),
            impulse_type: ImpulseType::Observation,
            initial_weight: WEIGHT_PULL_THROUGH,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![format!("ghost_pull:{}", ghost_node.source_graph)],
            source_type: SourceType::PullThrough,
            source_ref: format!("{}:{}", ghost_node.source_graph, ghost_node.external_ref),
        };

        db.insert_impulse(&input)
            .map_err(|e| format!("Failed to create impulse from pull: {}", e))?;
    }

    Ok(content)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_pull 2>&1`
Expected: All pull-through tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ghost/pull.rs tests/test_ghost.rs
git commit -m "feat: implement ghost node pull-through with session-only and permanent modes"
```

---

### Task 6: Integrate Ghost Nodes into Spreading Activation

**Files:**
- Modify: `src/activation.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/test_activation.rs`:
```rust
use memory_graph::ghost;
use memory_graph::ghost::scanner::ScanConfig;
use tempfile::TempDir;
use std::fs;

fn create_test_vault_for_activation() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("rust-memory.md"),
        "# Rust Memory Patterns\n\nRust ownership and memory management patterns for building memory systems.\n",
    ).unwrap();
    fs::write(
        dir.path().join("sqlite-patterns.md"),
        "# SQLite Patterns\n\nUsing SQLite for local-first graph storage.\n",
    ).unwrap();
    dir
}

#[test]
fn test_activation_includes_ghost_nodes() {
    let db = common::test_db();
    let vault = create_test_vault_for_activation();

    // Register ghost graph
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };
    ghost::register_and_scan(&db, "test-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    // Also add a regular impulse about Rust
    ingestion::explicit_save(
        &db,
        "Rust is great for building memory systems",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 2,
    }).unwrap();

    // Should include the regular impulse
    assert!(!result.memories.is_empty());

    // Should also flag ghost nodes that matched
    assert!(!result.ghost_activations.is_empty());
}
```

- [ ] **Step 2: Add ghost activation fields to RetrievalResult**

Update `RetrievalResult` in `src/models.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostActivation {
    pub ghost_node: GhostNode,
    pub activation_score: f64,
    pub source_graph: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub memories: Vec<RetrievedMemory>,
    pub ghost_activations: Vec<GhostActivation>,
    pub total_nodes_activated: usize,
}
```

- [ ] **Step 3: Update activation engine to include ghost node FTS search**

In `src/activation.rs`, update the `retrieve` method to also search ghost_nodes_fts and include ghost activations in the result. The ghost nodes participate in spreading activation but are reported separately so the caller can decide whether to pull through.

Add after the impulse FTS search in the seed phase:
```rust
// Also seed from ghost node FTS
let ghost_matches = self
    .db
    .search_ghost_nodes_fts(&request.query)
    .map_err(|e| format!("Ghost FTS search failed: {}", e))?;

let mut ghost_activations_map: HashMap<String, f64> = HashMap::new();
for (id, rank) in &ghost_matches {
    let score = (-rank).min(1.0).max(0.1);
    ghost_activations_map.insert(id.clone(), score);
}
```

And in the assembly phase, build the ghost_activations list:
```rust
let mut ghost_activations = Vec::new();
for (id, score) in &ghost_activations_map {
    if *score >= ACTIVATION_THRESHOLD {
        if let Ok(gn) = self.db.get_ghost_node(id) {
            ghost_activations.push(GhostActivation {
                source_graph: gn.source_graph.clone(),
                activation_score: *score,
                ghost_node: gn,
            });
        }
    }
}
ghost_activations.sort_by(|a, b| b.activation_score.partial_cmp(&a.activation_score).unwrap_or(std::cmp::Ordering::Equal));
```

Update the return to include `ghost_activations`.

- [ ] **Step 4: Fix any existing tests that break due to the new field**

Update all existing test assertions on `RetrievalResult` to account for the new `ghost_activations` field.

- [ ] **Step 5: Run all tests**

Run: `cargo test 2>&1`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/activation.rs src/models.rs tests/
git commit -m "feat: integrate ghost nodes into spreading activation retrieval"
```

---

### Task 7: Adaptive End-of-Session Extraction

**Files:**
- Modify: `src/extraction.rs`
- Create: `tests/test_extraction.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_extraction.rs`:
```rust
use memory_graph::extraction::{EngagementSignals, assess_engagement, ExtractionDepth};

#[test]
fn test_low_engagement_session() {
    let signals = EngagementSignals {
        total_turns: 3,
        avg_user_message_length: 20.0,
        avg_assistant_message_length: 50.0,
        session_duration_minutes: 2.0,
        explicit_save_count: 0,
        topic_count: 1,
        decision_keywords_found: 0,
        emotional_keywords_found: 0,
    };

    let depth = assess_engagement(&signals);
    assert_eq!(depth, ExtractionDepth::Minimal);
}

#[test]
fn test_high_engagement_session() {
    let signals = EngagementSignals {
        total_turns: 30,
        avg_user_message_length: 200.0,
        avg_assistant_message_length: 500.0,
        session_duration_minutes: 90.0,
        explicit_save_count: 3,
        topic_count: 5,
        decision_keywords_found: 8,
        emotional_keywords_found: 4,
    };

    let depth = assess_engagement(&signals);
    assert_eq!(depth, ExtractionDepth::Deep);
}

#[test]
fn test_medium_engagement_session() {
    let signals = EngagementSignals {
        total_turns: 12,
        avg_user_message_length: 80.0,
        avg_assistant_message_length: 200.0,
        session_duration_minutes: 20.0,
        explicit_save_count: 1,
        topic_count: 2,
        decision_keywords_found: 3,
        emotional_keywords_found: 1,
    };

    let depth = assess_engagement(&signals);
    assert_eq!(depth, ExtractionDepth::Standard);
}

#[test]
fn test_extraction_depth_max_proposals() {
    assert_eq!(ExtractionDepth::Minimal.max_proposals(), 1);
    assert_eq!(ExtractionDepth::Standard.max_proposals(), 5);
    assert_eq!(ExtractionDepth::Deep.max_proposals(), 15);
}

#[test]
fn test_engagement_score_calculation() {
    let signals = EngagementSignals {
        total_turns: 20,
        avg_user_message_length: 150.0,
        avg_assistant_message_length: 400.0,
        session_duration_minutes: 60.0,
        explicit_save_count: 2,
        topic_count: 4,
        decision_keywords_found: 5,
        emotional_keywords_found: 3,
    };

    let score = signals.engagement_score();
    assert!(score > 0.0);
    assert!(score <= 1.0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_low_engagement 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement extraction module**

`src/extraction.rs`:
```rust
#[derive(Debug, Clone)]
pub struct EngagementSignals {
    pub total_turns: usize,
    pub avg_user_message_length: f64,
    pub avg_assistant_message_length: f64,
    pub session_duration_minutes: f64,
    pub explicit_save_count: usize,
    pub topic_count: usize,
    pub decision_keywords_found: usize,
    pub emotional_keywords_found: usize,
}

impl EngagementSignals {
    /// Calculate a 0.0 to 1.0 engagement score from the signals.
    pub fn engagement_score(&self) -> f64 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        // Turn count (more turns = more engagement)
        max_score += 1.0;
        score += (self.total_turns as f64 / 30.0).min(1.0);

        // User message length (longer = more engaged)
        max_score += 1.0;
        score += (self.avg_user_message_length / 200.0).min(1.0);

        // Session duration
        max_score += 1.0;
        score += (self.session_duration_minutes / 60.0).min(1.0);

        // Explicit saves (user actively chose to remember things)
        max_score += 1.0;
        score += (self.explicit_save_count as f64 / 3.0).min(1.0);

        // Topic count (breadth of discussion)
        max_score += 0.5;
        score += (self.topic_count as f64 / 5.0).min(1.0) * 0.5;

        // Decision keywords (indicates decisions were made)
        max_score += 1.0;
        score += (self.decision_keywords_found as f64 / 5.0).min(1.0);

        // Emotional keywords (indicates emotional engagement)
        max_score += 0.5;
        score += (self.emotional_keywords_found as f64 / 3.0).min(1.0) * 0.5;

        (score / max_score).min(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionDepth {
    Minimal,  // Quick Q&A, routine task
    Standard, // Normal working session
    Deep,     // High-engagement, philosophical, many decisions
}

impl ExtractionDepth {
    pub fn max_proposals(&self) -> usize {
        match self {
            Self::Minimal => 1,
            Self::Standard => 5,
            Self::Deep => 15,
        }
    }
}

pub fn assess_engagement(signals: &EngagementSignals) -> ExtractionDepth {
    let score = signals.engagement_score();

    if score >= 0.6 {
        ExtractionDepth::Deep
    } else if score >= 0.3 {
        ExtractionDepth::Standard
    } else {
        ExtractionDepth::Minimal
    }
}

/// Keywords that suggest decisions were made in the conversation.
pub const DECISION_KEYWORDS: &[&str] = &[
    "decided", "decision", "agreed", "resolved", "chose", "picking",
    "going with", "let's use", "we'll do", "settled on", "commit to",
];

/// Keywords that suggest emotional engagement.
pub const EMOTIONAL_KEYWORDS: &[&str] = &[
    "love", "hate", "excited", "frustrated", "amazing", "terrible",
    "fun", "boring", "interesting", "fascinating", "annoying",
    "great", "awful", "brilliant", "painful",
];

/// Count keyword occurrences in text.
pub fn count_keywords(text: &str, keywords: &[&str]) -> usize {
    let lower = text.to_lowercase();
    keywords.iter().filter(|kw| lower.contains(**kw)).count()
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_ 2>&1`
Expected: All extraction tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/extraction.rs tests/test_extraction.rs
git commit -m "feat: implement adaptive extraction with engagement heuristics"
```

---

### Task 8: Expand MCP Tool Surface for Phase 2

**Files:**
- Modify: `src/server.rs`

- [ ] **Step 1: Write failing tests for new MCP tools**

Add to `tests/test_mcp.rs`:
```rust
use tempfile::TempDir;
use std::fs;

fn create_mcp_test_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("test-note.md"), "# Test Note\n\nSome content about Rust and memory.\n").unwrap();
    fs::write(dir.path().join("other.md"), "# Other\n\nLinks to [[test-note]].\n").unwrap();
    dir
}

#[test]
fn test_register_ghost_graph_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    let vault = create_mcp_test_vault();

    let result = server.handle_register_ghost_graph(
        "test-vault".to_string(),
        vault.path().to_str().unwrap().to_string(),
        Some("obsidian".to_string()),
        None,
    );

    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(response["name"], "test-vault");
    assert!(response["nodes_scanned"].as_i64().unwrap() >= 2);
}

#[test]
fn test_refresh_ghost_graph_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    let vault = create_mcp_test_vault();

    server.handle_register_ghost_graph(
        "test-vault".to_string(),
        vault.path().to_str().unwrap().to_string(),
        None,
        None,
    ).unwrap();

    let result = server.handle_refresh_ghost_graph("test-vault".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_pull_through_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    let vault = create_mcp_test_vault();

    server.handle_register_ghost_graph(
        "test-vault".to_string(),
        vault.path().to_str().unwrap().to_string(),
        None,
        None,
    ).unwrap();

    // Get a ghost node ID
    let status = server.handle_memory_status().unwrap();

    // Pull through by source and ref
    let result = server.handle_pull_through(
        "test-vault".to_string(),
        "test-note.md".to_string(),
        Some("session_only".to_string()),
    );

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("Rust"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_register_ghost 2>&1`
Expected: Compilation error

- [ ] **Step 3: Add ghost graph handlers to server.rs**

Add these methods to `MemoryGraphServer`:
```rust
pub fn handle_register_ghost_graph(
    &self,
    name: String,
    root_path: String,
    source_type: Option<String>,
    ignore_patterns: Option<Vec<String>>,
) -> Result<String, String> {
    let stype = source_type.unwrap_or_else(|| "directory".to_string());
    let config = crate::ghost::scanner::ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: ignore_patterns.unwrap_or_default(),
    };

    let db = self.db.lock().unwrap();
    let count = crate::ghost::register_and_scan(&db, &name, &root_path, &stype, &config)?;

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
    let config = crate::ghost::scanner::ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    let db = self.db.lock().unwrap();
    let count = crate::ghost::refresh(&db, &name, &config)?;

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
        Some("permanent") => crate::ghost::PullMode::Permanent,
        _ => crate::ghost::PullMode::SessionOnly,
    };

    let db = self.db.lock().unwrap();
    let ghost_node = db.get_ghost_node_by_ref(&source_graph, &external_ref)
        .map_err(|e| format!("Ghost node not found: {}", e))?;

    let sources = db.list_ghost_sources()
        .map_err(|e| format!("Failed to list sources: {}", e))?;

    let source = sources.iter().find(|s| s.name == source_graph)
        .ok_or_else(|| format!("Source '{}' not found", source_graph))?;

    let content = crate::ghost::pull::pull_ghost_content(&db, &ghost_node, &source.root_path, pull_mode)?;

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
```

Also add the corresponding MCP tool annotations to `McpHandler`:
```rust
#[tool(description = "Register an external knowledge base as a ghost graph. Scans the directory for markdown files and maps their structure without ingesting content.")]
fn register_ghost_graph(
    &self,
    #[tool(param, description = "Name for this ghost graph (e.g., 'obsidian-vault')")] name: String,
    #[tool(param, description = "Root path to the knowledge base directory")] root_path: String,
    #[tool(param, description = "Source type (e.g., 'obsidian', 'directory')")] source_type: Option<String>,
    #[tool(param, description = "Path patterns to ignore during scan")] ignore_patterns: Option<Vec<String>>,
) -> Result<String, String> {
    self.inner.handle_register_ghost_graph(name, root_path, source_type, ignore_patterns)
}

#[tool(description = "Refresh a ghost graph by re-scanning the external knowledge base for changes")]
fn refresh_ghost_graph(
    &self,
    #[tool(param, description = "Name of the ghost graph to refresh")] name: String,
) -> Result<String, String> {
    self.inner.handle_refresh_ghost_graph(name)
}

#[tool(description = "Pull content from a ghost node. 'session_only' releases after session. 'permanent' creates a memory node.")]
fn pull_through(
    &self,
    #[tool(param, description = "Name of the ghost graph source")] source_graph: String,
    #[tool(param, description = "External reference path of the ghost node")] external_ref: String,
    #[tool(param, description = "Pull mode: 'session_only' (default) or 'permanent'")] mode: Option<String>,
) -> Result<String, String> {
    self.inner.handle_pull_through(source_graph, external_ref, mode)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test 2>&1`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/server.rs tests/test_mcp.rs
git commit -m "feat: add ghost graph MCP tools — register, refresh, pull_through"
```

---

### Task 9: Phase 2 Validation Tests

**Files:**
- Create: `tests/test_validation_p2.rs`

- [ ] **Step 1: Write Phase 2 validation tests**

`tests/test_validation_p2.rs`:
```rust
mod common;

use memory_graph::activation::ActivationEngine;
use memory_graph::db::Database;
use memory_graph::extraction::{self, EngagementSignals, ExtractionDepth};
use memory_graph::ghost;
use memory_graph::ghost::pull::{pull_ghost_content, PullMode};
use memory_graph::ghost::scanner::ScanConfig;
use memory_graph::ingestion;
use memory_graph::models::*;
use memory_graph::server::MemoryGraphServer;
use std::fs;
use tempfile::TempDir;

fn create_validation_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    fs::write(
        root.join("design-philosophy.md"),
        "# Design Philosophy\n\nMemory should be portable and user-owned.\nSee [[architecture]] for implementation.\n",
    ).unwrap();

    fs::write(
        root.join("architecture.md"),
        "# Architecture\n\nSQLite-based local-first service with graph model.\nSee [[design-philosophy]] for principles.\n",
    ).unwrap();

    fs::write(
        root.join("rust-patterns.md"),
        "# Rust Patterns\n\nOwnership, borrowing, and lifetimes for safe memory management.\n",
    ).unwrap();

    fs::create_dir(root.join("private")).unwrap();
    fs::write(
        root.join("private/secrets.md"),
        "# Secrets\n\nAPI key: AKIAIOSFODNN7EXAMPLE\nDo not share.\n",
    ).unwrap();

    dir
}

// ============================================================
// PRD P2 Criterion 1: Ghost graph maps topology without content
// ============================================================

#[test]
fn validation_p2_ghost_maps_topology_no_content() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(
        &db,
        "validation-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    ).unwrap();

    let nodes = db.list_ghost_nodes_by_source("validation-vault").unwrap();

    // Should have all 4 markdown files mapped
    assert_eq!(nodes.len(), 4);

    // No impulses should have been created (content not ingested)
    assert_eq!(db.impulse_count().unwrap(), 0);

    // Nodes should have titles and metadata but not full content
    let design = nodes.iter().find(|n| n.title == "Design Philosophy").unwrap();
    assert!(!design.external_ref.is_empty());
}

// ============================================================
// PRD P2 Criterion 2: Pull-through activates on relevant query
// ============================================================

#[test]
fn validation_p2_pull_through_on_query() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(
        &db,
        "v-vault",
        vault.path().to_str().unwrap(),
        "obsidian",
        &config,
    ).unwrap();

    // Add a related impulse to the memory graph
    ingestion::explicit_save(
        &db,
        "Memory systems should be portable across providers",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "design philosophy portable".to_string(),
        max_results: 10,
        max_hops: 2,
    }).unwrap();

    // Should have ghost activations for the design philosophy note
    assert!(
        !result.ghost_activations.is_empty(),
        "Ghost nodes should activate on relevant query"
    );

    // Pull the activated ghost node content
    let sources = db.list_ghost_sources().unwrap();
    let root = &sources[0].root_path;
    let ghost_node = &result.ghost_activations[0].ghost_node;

    let content = pull_ghost_content(&db, ghost_node, root, PullMode::SessionOnly).unwrap();
    assert!(!content.is_empty());
    assert!(content.contains("portable") || content.contains("Philosophy"));
}

// ============================================================
// PRD P2 Criterion 3: Session-only pulls leave no persistent trace
// ============================================================

#[test]
fn validation_p2_session_only_no_trace() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let before = db.impulse_count().unwrap();

    let node = db.get_ghost_node_by_ref("v-vault", "architecture.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    pull_ghost_content(&db, &node, &sources[0].root_path, PullMode::SessionOnly).unwrap();

    let after = db.impulse_count().unwrap();
    assert_eq!(before, after, "Session-only pull should not create impulses");
}

// ============================================================
// PRD P2 Criterion 4: Permanent pulls create full memory nodes
// ============================================================

#[test]
fn validation_p2_permanent_pull_creates_node() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let before = db.impulse_count().unwrap();

    let node = db.get_ghost_node_by_ref("v-vault", "rust-patterns.md").unwrap();
    let sources = db.list_ghost_sources().unwrap();
    pull_ghost_content(&db, &node, &sources[0].root_path, PullMode::Permanent).unwrap();

    let after = db.impulse_count().unwrap();
    assert!(after > before, "Permanent pull should create an impulse");

    // The created impulse should have pull_through source type
    let impulses = db.list_impulses(None).unwrap();
    let pulled = impulses.iter().find(|i| i.source_type == SourceType::PullThrough);
    assert!(pulled.is_some());
    assert!(pulled.unwrap().source_ref.contains("v-vault"));
}

// ============================================================
// PRD P2 Criterion 6: Adaptive extraction scales with engagement
// ============================================================

#[test]
fn validation_p2_adaptive_extraction_scales() {
    // Low engagement
    let low = EngagementSignals {
        total_turns: 2,
        avg_user_message_length: 15.0,
        avg_assistant_message_length: 40.0,
        session_duration_minutes: 1.0,
        explicit_save_count: 0,
        topic_count: 1,
        decision_keywords_found: 0,
        emotional_keywords_found: 0,
    };
    assert_eq!(extraction::assess_engagement(&low), ExtractionDepth::Minimal);
    assert_eq!(ExtractionDepth::Minimal.max_proposals(), 1);

    // High engagement (like this design conversation)
    let high = EngagementSignals {
        total_turns: 25,
        avg_user_message_length: 180.0,
        avg_assistant_message_length: 450.0,
        session_duration_minutes: 75.0,
        explicit_save_count: 4,
        topic_count: 7,
        decision_keywords_found: 10,
        emotional_keywords_found: 5,
    };
    assert_eq!(extraction::assess_engagement(&high), ExtractionDepth::Deep);
    assert_eq!(ExtractionDepth::Deep.max_proposals(), 15);

    // Deep should propose more than minimal
    assert!(ExtractionDepth::Deep.max_proposals() > ExtractionDepth::Minimal.max_proposals());
}

// ============================================================
// Ghost node weight learning
// ============================================================

#[test]
fn validation_p2_ghost_weight_learns_from_access() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let node = db.get_ghost_node_by_ref("v-vault", "design-philosophy.md").unwrap();
    let initial_weight = node.weight;
    let sources = db.list_ghost_sources().unwrap();

    // Pull multiple times
    for _ in 0..5 {
        let n = db.get_ghost_node_by_ref("v-vault", "design-philosophy.md").unwrap();
        pull_ghost_content(&db, &n, &sources[0].root_path, PullMode::SessionOnly).unwrap();
    }

    let node_after = db.get_ghost_node_by_ref("v-vault", "design-philosophy.md").unwrap();
    assert!(
        node_after.weight > initial_weight,
        "Ghost node weight should increase with repeated access: {} > {}",
        node_after.weight,
        initial_weight
    );
}

// ============================================================
// End-to-end: ghost + memory graph integrated retrieval
// ============================================================

#[test]
fn validation_p2_integrated_retrieval() {
    let db = common::test_db();
    let vault = create_validation_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "v-vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    // Add memory graph impulses
    ingestion::explicit_save(
        &db,
        "Spreading activation mimics human memory recall patterns",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    ).unwrap();

    ingestion::explicit_save(
        &db,
        "SQLite is excellent for portable local-first storage",
        ImpulseType::Decision,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "architecture SQLite local".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();

    // Should get results from memory graph
    assert!(!result.memories.is_empty());

    // Should also get ghost activations from the vault
    // (architecture.md mentions SQLite)
    assert!(
        !result.ghost_activations.is_empty(),
        "Should activate ghost nodes for matching vault content"
    );
}
```

- [ ] **Step 2: Run validation tests**

Run: `cargo test validation_p2 2>&1`
Expected: All Phase 2 validation tests PASS

- [ ] **Step 3: Run complete test suite**

Run: `cargo test 2>&1`
Expected: ALL tests PASS (Phase 1 + Phase 2)

- [ ] **Step 4: Commit**

```bash
git add tests/test_validation_p2.rs
git commit -m "feat: add Phase 2 validation tests for ghost graphs, pull-through, and adaptive extraction"
```

---

### Task 10: Phase 2 Cleanup and Build Verification

- [ ] **Step 1: Run clippy**

Run: `cargo clippy 2>&1`
Fix any errors.

- [ ] **Step 2: Run full test suite**

Run: `cargo test 2>&1`
Expected: ALL tests PASS

- [ ] **Step 3: Build release**

Run: `cargo build --release 2>&1`
Expected: Successful compilation

- [ ] **Step 4: Commit cleanup**

```bash
git add -A
git commit -m "chore: Phase 2 clippy fixes and build verification"
```

---

## Post-Phase-2 Notes

After completing all tasks:

1. Ghost graph registration, scanning, and refresh work for filesystem/Obsidian vaults
2. Pull-through works in session-only and permanent modes
3. Ghost nodes participate in spreading activation retrieval
4. Ghost node weights learn from access patterns
5. Adaptive extraction heuristics assess session engagement
6. MCP tools expanded: register_ghost_graph, refresh_ghost_graph, pull_through
7. All Phase 1 + Phase 2 validation tests pass

**Deferred to Phase 3:**
- Cross-device sync
- Cloud backup
- Multiple concurrent ghost graph sources in integrated retrieval testing
- Identity semantics
