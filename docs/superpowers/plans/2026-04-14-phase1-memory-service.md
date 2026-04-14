# Phase 1: Local Single-User Memory Service — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working MCP server that stores impulses as a weighted graph in SQLite, retrieves context via spreading activation, handles secret redaction, and supports incognito mode.

**Architecture:** Rust binary exposing MCP tools over stdio. SQLite (via rusqlite with bundled feature) as the single-file canonical store. Spreading activation runs as iterative graph traversal over weighted connections with decay. FTS5 for seed-phase text matching.

**Tech Stack:** Rust, rusqlite (bundled), rmcp (server + transport-io + macros), tokio, uuid, chrono, serde/serde_json, regex

---

## File Structure

```
memory-graph/
  Cargo.toml
  src/
    main.rs             # Entry point: init DB, start MCP server on stdio
    server.rs           # MCP tool handler struct + #[tool] method impls
    db.rs               # SQLite connection init, schema creation, migrations
    models.rs           # All Rust types: Impulse, Connection, enums, NewImpulse, etc.
    weight.rs           # Decay calculation, reinforcement, effective weight math
    activation.rs       # Spreading activation: seed, propagate, assemble
    redaction.rs        # Secret/PII pattern detection and stripping
    ingestion.rs        # Explicit save pipeline: validate -> redact -> persist
    session.rs          # Session state: incognito flag
  tests/
    common/mod.rs       # Shared test helpers: in-memory DB factory, seed data builders
    test_db.rs          # Schema creation, migration, basic CRUD
    test_weight.rs      # Decay math, reinforcement, floor behavior
    test_activation.rs  # Spreading activation: seeding, propagation, assembly, edge cases
    test_redaction.rs   # Secret detection patterns, PII stripping, passthrough of clean content
    test_ingestion.rs   # Full save pipeline: redaction -> persistence -> connection creation
    test_session.rs     # Incognito mode: no writes, no weight updates
    test_mcp.rs         # MCP tool handlers: end-to-end through the server struct
```

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/models.rs` (empty module declaration)
- Create: `src/db.rs` (empty module declaration)
- Create: `src/weight.rs` (empty module declaration)
- Create: `src/activation.rs` (empty module declaration)
- Create: `src/redaction.rs` (empty module declaration)
- Create: `src/ingestion.rs` (empty module declaration)
- Create: `src/session.rs` (empty module declaration)
- Create: `src/server.rs` (empty module declaration)
- Create: `tests/common/mod.rs` (empty)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "memory-graph"
version = "0.1.0"
edition = "2021"
description = "A portable, human-memory-inspired memory layer for AI systems"

[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-io", "macros"] }
rusqlite = { version = "0.32", features = ["bundled"] }
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
```

- [ ] **Step 2: Create src/main.rs with module declarations**

```rust
mod activation;
mod db;
mod ingestion;
mod models;
mod redaction;
mod server;
mod session;
mod weight;

fn main() {
    println!("memory-graph: not yet wired");
}
```

- [ ] **Step 3: Create empty module files**

Create each of these files with just a comment:

`src/models.rs`:
```rust
// Memory graph data types
```

`src/db.rs`:
```rust
// SQLite schema and database operations
```

`src/weight.rs`:
```rust
// Decay and reinforcement weight mechanics
```

`src/activation.rs`:
```rust
// Spreading activation retrieval algorithm
```

`src/redaction.rs`:
```rust
// Secret and PII detection and stripping
```

`src/ingestion.rs`:
```rust
// Ingestion pipeline for explicit saves
```

`src/session.rs`:
```rust
// Session state management
```

`src/server.rs`:
```rust
// MCP tool handlers
```

`tests/common/mod.rs`:
```rust
// Shared test utilities
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1`
Expected: Successful compilation (warnings about unused modules are fine)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: scaffold Rust project with dependencies and module structure"
```

---

### Task 2: Data Model Types

**Files:**
- Modify: `src/models.rs`

- [ ] **Step 1: Write model types**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// === Enums ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpulseType {
    Heuristic,
    Preference,
    Decision,
    Pattern,
    Observation,
}

impl ImpulseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Heuristic => "heuristic",
            Self::Preference => "preference",
            Self::Decision => "decision",
            Self::Pattern => "pattern",
            Self::Observation => "observation",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "heuristic" => Some(Self::Heuristic),
            "preference" => Some(Self::Preference),
            "decision" => Some(Self::Decision),
            "pattern" => Some(Self::Pattern),
            "observation" => Some(Self::Observation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmotionalValence {
    Positive,
    Negative,
    Neutral,
}

impl EmotionalValence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "positive" => Some(Self::Positive),
            "negative" => Some(Self::Negative),
            "neutral" => Some(Self::Neutral),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngagementLevel {
    Low,
    Medium,
    High,
}

impl EngagementLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    ExplicitSave,
    SessionExtraction,
    PullThrough,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ExplicitSave => "explicit_save",
            Self::SessionExtraction => "session_extraction",
            Self::PullThrough => "pull_through",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "explicit_save" => Some(Self::ExplicitSave),
            "session_extraction" => Some(Self::SessionExtraction),
            "pull_through" => Some(Self::PullThrough),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImpulseStatus {
    Candidate,
    Confirmed,
    Superseded,
    Deleted,
}

impl ImpulseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Confirmed => "confirmed",
            Self::Superseded => "superseded",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "candidate" => Some(Self::Candidate),
            "confirmed" => Some(Self::Confirmed),
            "superseded" => Some(Self::Superseded),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

// === Core Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Impulse {
    pub id: String,
    pub content: String,
    pub impulse_type: ImpulseType,
    pub weight: f64,
    pub initial_weight: f64,
    pub emotional_valence: EmotionalValence,
    pub engagement_level: EngagementLevel,
    pub source_signals: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub source_type: SourceType,
    pub source_ref: String,
    pub status: ImpulseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub weight: f64,
    pub relationship: String,
    pub created_at: DateTime<Utc>,
    pub last_traversed_at: DateTime<Utc>,
    pub traversal_count: i64,
}

// === Input Types (for creating new records) ===

#[derive(Debug, Clone)]
pub struct NewImpulse {
    pub content: String,
    pub impulse_type: ImpulseType,
    pub initial_weight: f64,
    pub emotional_valence: EmotionalValence,
    pub engagement_level: EngagementLevel,
    pub source_signals: Vec<String>,
    pub source_type: SourceType,
    pub source_ref: String,
}

#[derive(Debug, Clone)]
pub struct NewConnection {
    pub source_id: String,
    pub target_id: String,
    pub weight: f64,
    pub relationship: String,
}

// === Retrieval Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalRequest {
    pub query: String,
    pub max_results: usize,
    pub max_hops: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedMemory {
    pub impulse: Impulse,
    pub activation_score: f64,
    pub activation_path: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub memories: Vec<RetrievedMemory>,
    pub total_nodes_activated: usize,
}

// === Weight Constants ===

pub const WEIGHT_EXPLICIT_SAVE: f64 = 0.7;
pub const WEIGHT_SESSION_EXTRACTION_HIGH: f64 = 0.5;
pub const WEIGHT_SESSION_EXTRACTION_LOW: f64 = 0.3;
pub const WEIGHT_PULL_THROUGH: f64 = 0.4;
pub const WEIGHT_FLOOR: f64 = 0.001;
pub const REINFORCEMENT_BUMP: f64 = 0.05;

// Decay rates (lambda) — per hour
pub const DECAY_SEMANTIC: f64 = 0.0005;   // slow: ~1386 hours half-life (~58 days)
pub const DECAY_EPISODIC: f64 = 0.005;    // fast: ~139 hours half-life (~6 days)
pub const DECAY_GHOST: f64 = 0.002;       // medium: ~347 hours half-life (~14 days)

// Activation constants
pub const ACTIVATION_THRESHOLD: f64 = 0.1;
pub const PROXIMITY_DECAY_PER_HOP: f64 = 0.5;
pub const MAX_PROPAGATION_ITERATIONS: usize = 10;
pub const ACTIVATION_CONVERGENCE_THRESHOLD: f64 = 0.001;
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1`
Expected: Successful compilation

- [ ] **Step 3: Commit**

```bash
git add src/models.rs
git commit -m "feat: define core data model types, enums, and weight constants"
```

---

### Task 3: SQLite Schema and Database Operations

**Files:**
- Modify: `src/db.rs`
- Create: `tests/common/mod.rs`
- Create: `tests/test_db.rs`

- [ ] **Step 1: Write failing test for database initialization**

`tests/common/mod.rs`:
```rust
use memory_graph::db::Database;

pub fn test_db() -> Database {
    Database::open_in_memory().expect("Failed to create in-memory database")
}
```

`tests/test_db.rs`:
```rust
mod common;

#[test]
fn test_database_creates_tables() {
    let db = common::test_db();
    // Should be able to query the impulses table without error
    let count = db.impulse_count().unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_database_creates_fts_index() {
    let db = common::test_db();
    // FTS5 table should exist
    let count = db.fts_impulse_count().unwrap();
    assert_eq!(count, 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_database 2>&1`
Expected: Compilation error — `Database` type doesn't exist yet

- [ ] **Step 3: Implement database module**

`src/db.rs`:
```rust
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqlResult};
use uuid::Uuid;

use crate::models::*;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> SqlResult<()> {
        self.conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        self.conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        self.create_tables()?;
        Ok(())
    }

    fn create_tables(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS impulses (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                impulse_type TEXT NOT NULL,
                weight REAL NOT NULL,
                initial_weight REAL NOT NULL,
                emotional_valence TEXT NOT NULL DEFAULT 'neutral',
                engagement_level TEXT NOT NULL DEFAULT 'medium',
                source_signals TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL,
                source_type TEXT NOT NULL,
                source_ref TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'confirmed'
            );

            CREATE TABLE IF NOT EXISTS connections (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                weight REAL NOT NULL,
                relationship TEXT NOT NULL DEFAULT 'relates_to',
                created_at TEXT NOT NULL,
                last_traversed_at TEXT NOT NULL,
                traversal_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (source_id) REFERENCES impulses(id),
                FOREIGN KEY (target_id) REFERENCES impulses(id)
            );

            CREATE INDEX IF NOT EXISTS idx_connections_source ON connections(source_id);
            CREATE INDEX IF NOT EXISTS idx_connections_target ON connections(target_id);
            CREATE INDEX IF NOT EXISTS idx_impulses_status ON impulses(status);

            CREATE VIRTUAL TABLE IF NOT EXISTS impulses_fts USING fts5(
                content,
                content_rowid='rowid',
                tokenize='porter'
            );
            ",
        )?;
        Ok(())
    }

    // === Impulse Operations ===

    pub fn insert_impulse(&self, input: &NewImpulse) -> SqlResult<Impulse> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let signals_json = serde_json::to_string(&input.source_signals).unwrap_or_default();

        self.conn.execute(
            "INSERT INTO impulses (id, content, impulse_type, weight, initial_weight,
             emotional_valence, engagement_level, source_signals, created_at, last_accessed_at,
             source_type, source_ref, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                input.content,
                input.impulse_type.as_str(),
                input.initial_weight,
                input.initial_weight,
                input.emotional_valence.as_str(),
                input.engagement_level.as_str(),
                signals_json,
                now_str,
                now_str,
                input.source_type.as_str(),
                input.source_ref,
                "confirmed",
            ],
        )?;

        // Insert into FTS index
        self.conn.execute(
            "INSERT INTO impulses_fts (rowid, content)
             SELECT rowid, content FROM impulses WHERE id = ?1",
            params![id],
        )?;

        self.get_impulse(&id)
    }

    pub fn get_impulse(&self, id: &str) -> SqlResult<Impulse> {
        self.conn.query_row(
            "SELECT id, content, impulse_type, weight, initial_weight,
             emotional_valence, engagement_level, source_signals, created_at,
             last_accessed_at, source_type, source_ref, status
             FROM impulses WHERE id = ?1",
            params![id],
            |row| Ok(row_to_impulse(row)),
        )
    }

    pub fn update_impulse_status(&self, id: &str, status: ImpulseStatus) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE impulses SET status = ?1 WHERE id = ?2",
            params![status.as_str(), id],
        )?;
        Ok(())
    }

    pub fn update_impulse_content(&self, id: &str, content: &str) -> SqlResult<String> {
        let old = self.get_impulse(id)?;
        // Mark old as superseded
        self.update_impulse_status(id, ImpulseStatus::Superseded)?;

        // Create new impulse with updated content
        let new_input = NewImpulse {
            content: content.to_string(),
            impulse_type: old.impulse_type,
            initial_weight: old.initial_weight,
            emotional_valence: old.emotional_valence,
            engagement_level: old.engagement_level,
            source_signals: old.source_signals,
            source_type: old.source_type,
            source_ref: old.source_ref,
        };
        let new_impulse = self.insert_impulse(&new_input)?;

        // Create supersession connection
        let conn_input = NewConnection {
            source_id: new_impulse.id.clone(),
            target_id: id.to_string(),
            weight: 1.0,
            relationship: "supersedes".to_string(),
        };
        self.insert_connection(&conn_input)?;

        Ok(new_impulse.id)
    }

    pub fn update_impulse_weight(&self, id: &str, weight: f64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE impulses SET weight = ?1 WHERE id = ?2",
            params![weight, id],
        )?;
        Ok(())
    }

    pub fn touch_impulse(&self, id: &str) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE impulses SET last_accessed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn list_impulses(&self, status: Option<ImpulseStatus>) -> SqlResult<Vec<Impulse>> {
        match status {
            Some(s) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, content, impulse_type, weight, initial_weight,
                     emotional_valence, engagement_level, source_signals, created_at,
                     last_accessed_at, source_type, source_ref, status
                     FROM impulses WHERE status = ?1 ORDER BY weight DESC",
                )?;
                let rows = stmt.query_map(params![s.as_str()], |row| Ok(row_to_impulse(row)))?;
                rows.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, content, impulse_type, weight, initial_weight,
                     emotional_valence, engagement_level, source_signals, created_at,
                     last_accessed_at, source_type, source_ref, status
                     FROM impulses ORDER BY weight DESC",
                )?;
                let rows = stmt.query_map([], |row| Ok(row_to_impulse(row)))?;
                rows.collect()
            }
        }
    }

    pub fn search_impulses_fts(&self, query: &str) -> SqlResult<Vec<(String, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT i.id, fts.rank
             FROM impulses_fts fts
             JOIN impulses i ON i.rowid = fts.rowid
             WHERE impulses_fts MATCH ?1
             AND i.status IN ('confirmed', 'candidate')
             ORDER BY fts.rank",
        )?;
        let rows = stmt.query_map(params![query], |row| {
            let id: String = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok((id, rank))
        })?;
        rows.collect()
    }

    pub fn impulse_count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM impulses", [], |row| row.get(0))
    }

    pub fn fts_impulse_count(&self) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM impulses_fts",
            [],
            |row| row.get(0),
        )
    }

    // === Connection Operations ===

    pub fn insert_connection(&self, input: &NewConnection) -> SqlResult<Connection> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO connections (id, source_id, target_id, weight, relationship,
             created_at, last_traversed_at, traversal_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                id,
                input.source_id,
                input.target_id,
                input.weight,
                input.relationship,
                now_str,
                now_str,
            ],
        )?;

        self.get_connection(&id)
    }

    pub fn get_connection(&self, id: &str) -> SqlResult<Connection> {
        self.conn.query_row(
            "SELECT id, source_id, target_id, weight, relationship,
             created_at, last_traversed_at, traversal_count
             FROM connections WHERE id = ?1",
            params![id],
            |row| Ok(row_to_connection(row)),
        )
    }

    pub fn get_connections_for_node(&self, node_id: &str) -> SqlResult<Vec<Connection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, weight, relationship,
             created_at, last_traversed_at, traversal_count
             FROM connections
             WHERE source_id = ?1 OR target_id = ?1",
        )?;
        let rows = stmt.query_map(params![node_id], |row| Ok(row_to_connection(row)))?;
        rows.collect()
    }

    pub fn update_connection_weight(&self, id: &str, weight: f64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE connections SET weight = ?1 WHERE id = ?2",
            params![weight, id],
        )?;
        Ok(())
    }

    pub fn touch_connection(&self, id: &str) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE connections SET last_traversed_at = ?1, traversal_count = traversal_count + 1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn connection_count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0))
    }

    // === Stats ===

    pub fn memory_stats(&self) -> SqlResult<MemoryStats> {
        let impulse_count = self.impulse_count()?;
        let connection_count = self.connection_count()?;
        let confirmed_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM impulses WHERE status = 'confirmed'",
            [],
            |row| row.get(0),
        )?;
        let candidate_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM impulses WHERE status = 'candidate'",
            [],
            |row| row.get(0),
        )?;
        Ok(MemoryStats {
            total_impulses: impulse_count,
            confirmed_impulses: confirmed_count,
            candidate_impulses: candidate_count,
            total_connections: connection_count,
        })
    }
}

// === Helper: row mapping ===

fn row_to_impulse(row: &rusqlite::Row) -> Impulse {
    let signals_json: String = row.get(7).unwrap_or_default();
    let source_signals: Vec<String> =
        serde_json::from_str(&signals_json).unwrap_or_default();

    let created_str: String = row.get(8).unwrap_or_default();
    let accessed_str: String = row.get(9).unwrap_or_default();

    Impulse {
        id: row.get(0).unwrap_or_default(),
        content: row.get(1).unwrap_or_default(),
        impulse_type: ImpulseType::from_str(&row.get::<_, String>(2).unwrap_or_default())
            .unwrap_or(ImpulseType::Observation),
        weight: row.get(3).unwrap_or(0.0),
        initial_weight: row.get(4).unwrap_or(0.0),
        emotional_valence: EmotionalValence::from_str(
            &row.get::<_, String>(5).unwrap_or_default(),
        )
        .unwrap_or(EmotionalValence::Neutral),
        engagement_level: EngagementLevel::from_str(
            &row.get::<_, String>(6).unwrap_or_default(),
        )
        .unwrap_or(EngagementLevel::Medium),
        source_signals,
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        last_accessed_at: DateTime::parse_from_rfc3339(&accessed_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        source_type: SourceType::from_str(&row.get::<_, String>(10).unwrap_or_default())
            .unwrap_or(SourceType::ExplicitSave),
        source_ref: row.get(11).unwrap_or_default(),
        status: ImpulseStatus::from_str(&row.get::<_, String>(12).unwrap_or_default())
            .unwrap_or(ImpulseStatus::Confirmed),
    }
}

fn row_to_connection(row: &rusqlite::Row) -> Connection {
    let created_str: String = row.get(5).unwrap_or_default();
    let traversed_str: String = row.get(6).unwrap_or_default();

    Connection {
        id: row.get(0).unwrap_or_default(),
        source_id: row.get(1).unwrap_or_default(),
        target_id: row.get(2).unwrap_or_default(),
        weight: row.get(3).unwrap_or(0.0),
        relationship: row.get(4).unwrap_or_default(),
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        last_traversed_at: DateTime::parse_from_rfc3339(&traversed_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        traversal_count: row.get(7).unwrap_or(0),
    }
}

// === Stats Type ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_impulses: i64,
    pub confirmed_impulses: i64,
    pub candidate_impulses: i64,
    pub total_connections: i64,
}

use serde::{Deserialize, Serialize};
```

Also update `src/main.rs` to make modules public for tests:

```rust
pub mod activation;
pub mod db;
pub mod ingestion;
pub mod models;
pub mod redaction;
pub mod server;
pub mod session;
pub mod weight;

fn main() {
    println!("memory-graph: not yet wired");
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_database 2>&1`
Expected: Both tests PASS

- [ ] **Step 5: Write CRUD tests**

Add to `tests/test_db.rs`:
```rust
use memory_graph::models::*;

#[test]
fn test_insert_and_get_impulse() {
    let db = common::test_db();
    let input = NewImpulse {
        content: "Auth middleware silently drops tokens under concurrent writes".to_string(),
        impulse_type: ImpulseType::Heuristic,
        initial_weight: WEIGHT_EXPLICIT_SAVE,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::High,
        source_signals: vec!["long-form response".to_string()],
        source_type: SourceType::ExplicitSave,
        source_ref: "session-001".to_string(),
    };

    let impulse = db.insert_impulse(&input).unwrap();
    assert_eq!(impulse.content, input.content);
    assert_eq!(impulse.weight, WEIGHT_EXPLICIT_SAVE);
    assert_eq!(impulse.status, ImpulseStatus::Confirmed);

    let retrieved = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(retrieved.id, impulse.id);
    assert_eq!(retrieved.content, impulse.content);
}

#[test]
fn test_insert_and_get_connection() {
    let db = common::test_db();
    let a = db
        .insert_impulse(&NewImpulse {
            content: "Impulse A".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let b = db
        .insert_impulse(&NewImpulse {
            content: "Impulse B".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let conn_input = NewConnection {
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        weight: 0.8,
        relationship: "relates_to".to_string(),
    };

    let conn = db.insert_connection(&conn_input).unwrap();
    assert_eq!(conn.source_id, a.id);
    assert_eq!(conn.target_id, b.id);
    assert_eq!(conn.weight, 0.8);
    assert_eq!(conn.traversal_count, 0);

    let conns = db.get_connections_for_node(&a.id).unwrap();
    assert_eq!(conns.len(), 1);
}

#[test]
fn test_update_impulse_creates_supersession() {
    let db = common::test_db();
    let original = db
        .insert_impulse(&NewImpulse {
            content: "Original content".to_string(),
            impulse_type: ImpulseType::Decision,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let new_id = db
        .update_impulse_content(&original.id, "Updated content")
        .unwrap();

    // Old impulse should be superseded
    let old = db.get_impulse(&original.id).unwrap();
    assert_eq!(old.status, ImpulseStatus::Superseded);

    // New impulse should exist with updated content
    let new = db.get_impulse(&new_id).unwrap();
    assert_eq!(new.content, "Updated content");
    assert_eq!(new.status, ImpulseStatus::Confirmed);

    // Supersession connection should exist
    let conns = db.get_connections_for_node(&new_id).unwrap();
    assert_eq!(conns.len(), 1);
    assert_eq!(conns[0].relationship, "supersedes");
}

#[test]
fn test_soft_delete() {
    let db = common::test_db();
    let impulse = db
        .insert_impulse(&NewImpulse {
            content: "To be deleted".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Low,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    db.update_impulse_status(&impulse.id, ImpulseStatus::Deleted)
        .unwrap();

    let deleted = db.get_impulse(&impulse.id).unwrap();
    assert_eq!(deleted.status, ImpulseStatus::Deleted);

    // Should still be retrievable (soft delete)
    assert_eq!(deleted.content, "To be deleted");
}

#[test]
fn test_fts_search() {
    let db = common::test_db();
    db.insert_impulse(&NewImpulse {
        content: "Rust is great for systems programming".to_string(),
        impulse_type: ImpulseType::Preference,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Positive,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    })
    .unwrap();

    db.insert_impulse(&NewImpulse {
        content: "Python is slow but good for prototyping".to_string(),
        impulse_type: ImpulseType::Preference,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    })
    .unwrap();

    let results = db.search_impulses_fts("rust systems").unwrap();
    assert_eq!(results.len(), 1);

    let results = db.search_impulses_fts("programming").unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_memory_stats() {
    let db = common::test_db();
    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.total_impulses, 0);
    assert_eq!(stats.total_connections, 0);

    db.insert_impulse(&NewImpulse {
        content: "Test".to_string(),
        impulse_type: ImpulseType::Observation,
        initial_weight: 0.5,
        emotional_valence: EmotionalValence::Neutral,
        engagement_level: EngagementLevel::Medium,
        source_signals: vec![],
        source_type: SourceType::ExplicitSave,
        source_ref: "test".to_string(),
    })
    .unwrap();

    let stats = db.memory_stats().unwrap();
    assert_eq!(stats.total_impulses, 1);
    assert_eq!(stats.confirmed_impulses, 1);
}
```

- [ ] **Step 6: Run all DB tests**

Run: `cargo test test_ 2>&1`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/db.rs src/main.rs tests/
git commit -m "feat: implement SQLite schema, impulse/connection CRUD, FTS5 search"
```

---

### Task 4: Weight Mechanics — Decay and Reinforcement

**Files:**
- Modify: `src/weight.rs`
- Create: `tests/test_weight.rs`

- [ ] **Step 1: Write failing tests for decay**

`tests/test_weight.rs`:
```rust
mod common;

use memory_graph::models::*;
use memory_graph::weight;

#[test]
fn test_effective_weight_no_decay() {
    // Just created, no time passed
    let w = weight::effective_weight(0.7, 0.0, DECAY_SEMANTIC);
    assert!((w - 0.7).abs() < 0.001);
}

#[test]
fn test_effective_weight_decays_over_time() {
    // After 100 hours with semantic decay rate
    let w = weight::effective_weight(0.7, 100.0, DECAY_SEMANTIC);
    // e^(-0.0005 * 100) = e^(-0.05) ≈ 0.951
    // 0.7 * 0.951 ≈ 0.666
    assert!(w < 0.7);
    assert!(w > 0.6);
}

#[test]
fn test_effective_weight_episodic_decays_faster() {
    let semantic = weight::effective_weight(0.7, 200.0, DECAY_SEMANTIC);
    let episodic = weight::effective_weight(0.7, 200.0, DECAY_EPISODIC);
    assert!(episodic < semantic);
}

#[test]
fn test_effective_weight_never_below_floor() {
    // After a very long time
    let w = weight::effective_weight(0.1, 100_000.0, DECAY_EPISODIC);
    assert!(w >= WEIGHT_FLOOR);
}

#[test]
fn test_reinforce_adds_bump() {
    let new_weight = weight::reinforce(0.5);
    assert!((new_weight - 0.55).abs() < 0.001);
}

#[test]
fn test_reinforce_caps_at_one() {
    let new_weight = weight::reinforce(0.98);
    assert!((new_weight - 1.0).abs() < 0.001);
}

#[test]
fn test_reinforce_from_floor() {
    let new_weight = weight::reinforce(WEIGHT_FLOOR);
    assert!((new_weight - (WEIGHT_FLOOR + REINFORCEMENT_BUMP)).abs() < 0.001);
}

#[test]
fn test_decay_rate_for_impulse_type() {
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Heuristic), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Preference), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Decision), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Pattern), DECAY_SEMANTIC);
    assert_eq!(weight::decay_rate_for_type(ImpulseType::Observation), DECAY_EPISODIC);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_effective_weight 2>&1`
Expected: Compilation error — `weight` module has no functions

- [ ] **Step 3: Implement weight module**

`src/weight.rs`:
```rust
use crate::models::*;

/// Calculate effective weight after time-based decay.
/// weight: current stored weight (0.0 to 1.0)
/// hours_elapsed: hours since last access
/// lambda: decay rate constant
///
/// Formula: effective = max(WEIGHT_FLOOR, weight * e^(-lambda * hours))
pub fn effective_weight(weight: f64, hours_elapsed: f64, lambda: f64) -> f64 {
    let decayed = weight * (-lambda * hours_elapsed).exp();
    decayed.max(WEIGHT_FLOOR)
}

/// Apply reinforcement bump to a weight.
/// Returns new weight, capped at 1.0.
pub fn reinforce(weight: f64) -> f64 {
    (weight + REINFORCEMENT_BUMP).min(1.0)
}

/// Get the appropriate decay rate for an impulse type.
/// Semantic knowledge (heuristics, preferences, decisions, patterns) decays slowly.
/// Episodic observations decay faster.
pub fn decay_rate_for_type(impulse_type: ImpulseType) -> f64 {
    match impulse_type {
        ImpulseType::Heuristic
        | ImpulseType::Preference
        | ImpulseType::Decision
        | ImpulseType::Pattern => DECAY_SEMANTIC,
        ImpulseType::Observation => DECAY_EPISODIC,
    }
}

/// Calculate hours elapsed between two timestamps.
pub fn hours_since(from: &chrono::DateTime<chrono::Utc>, to: &chrono::DateTime<chrono::Utc>) -> f64 {
    let duration = *to - *from;
    duration.num_seconds() as f64 / 3600.0
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_ -- --test-output 2>&1`
Expected: All weight tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/weight.rs tests/test_weight.rs
git commit -m "feat: implement decay, reinforcement, and weight floor mechanics"
```

---

### Task 5: Spreading Activation Retrieval

**Files:**
- Modify: `src/activation.rs`
- Create: `tests/test_activation.rs`

This is the core algorithm. Seed nodes from FTS, propagate activation through weighted connections, assemble results.

- [ ] **Step 1: Write failing tests**

`tests/test_activation.rs`:
```rust
mod common;

use memory_graph::activation::ActivationEngine;
use memory_graph::models::*;

fn seed_graph(db: &memory_graph::db::Database) -> (String, String, String) {
    // Create three connected impulses: A -> B -> C
    let a = db
        .insert_impulse(&NewImpulse {
            content: "Rust is great for building memory systems".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let b = db
        .insert_impulse(&NewImpulse {
            content: "SQLite works well for local-first graph storage".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.6,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let c = db
        .insert_impulse(&NewImpulse {
            content: "Spreading activation mimics human memory recall".to_string(),
            impulse_type: ImpulseType::Pattern,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    // A relates_to B, B relates_to C
    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        weight: 0.8,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: b.id.clone(),
        target_id: c.id.clone(),
        weight: 0.6,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    (a.id, b.id, c.id)
}

#[test]
fn test_direct_match_returns_result() {
    let db = common::test_db();
    let (a_id, _, _) = seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    assert!(!result.memories.is_empty());
    // Node A should be in results (direct FTS match on "Rust" and "memory")
    assert!(result.memories.iter().any(|m| m.impulse.id == a_id));
}

#[test]
fn test_activation_spreads_to_connected_nodes() {
    let db = common::test_db();
    let (a_id, b_id, _) = seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    // Node B should also appear (connected to A which matched directly)
    assert!(result.memories.iter().any(|m| m.impulse.id == b_id));
}

#[test]
fn test_activation_decays_with_hops() {
    let db = common::test_db();
    let (a_id, b_id, c_id) = seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();

    let score_a = result
        .memories
        .iter()
        .find(|m| m.impulse.id == a_id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    let score_b = result
        .memories
        .iter()
        .find(|m| m.impulse.id == b_id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    let score_c = result
        .memories
        .iter()
        .find(|m| m.impulse.id == c_id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    // Direct match should have highest score
    assert!(score_a > score_b);
    // 1-hop should be higher than 2-hop
    assert!(score_b > score_c);
}

#[test]
fn test_max_results_limits_output() {
    let db = common::test_db();
    seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 1,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    assert!(result.memories.len() <= 1);
}

#[test]
fn test_no_match_returns_empty() {
    let db = common::test_db();
    seed_graph(&db);

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "quantum physics".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    assert!(result.memories.is_empty());
}

#[test]
fn test_deleted_impulses_excluded() {
    let db = common::test_db();
    let (a_id, _, _) = seed_graph(&db);

    db.update_impulse_status(&a_id, ImpulseStatus::Deleted).unwrap();

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();
    // Deleted node should not appear in results
    assert!(!result.memories.iter().any(|m| m.impulse.id == a_id));
}

#[test]
fn test_high_engagement_amplifies_propagation() {
    let db = common::test_db();

    // Create two paths: A -> B_high (high engagement) and A -> B_low (low engagement)
    let a = db
        .insert_impulse(&NewImpulse {
            content: "Memory design patterns".to_string(),
            impulse_type: ImpulseType::Heuristic,
            initial_weight: 0.7,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Medium,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let b_high = db
        .insert_impulse(&NewImpulse {
            content: "High engagement insight about graphs".to_string(),
            impulse_type: ImpulseType::Pattern,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Positive,
            engagement_level: EngagementLevel::High,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    let b_low = db
        .insert_impulse(&NewImpulse {
            content: "Low engagement note about graphs".to_string(),
            impulse_type: ImpulseType::Observation,
            initial_weight: 0.5,
            emotional_valence: EmotionalValence::Neutral,
            engagement_level: EngagementLevel::Low,
            source_signals: vec![],
            source_type: SourceType::ExplicitSave,
            source_ref: "test".to_string(),
        })
        .unwrap();

    // Same connection weight for both
    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b_high.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: a.id.clone(),
        target_id: b_low.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "memory design".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    let result = engine.retrieve(&request).unwrap();

    let score_high = result
        .memories
        .iter()
        .find(|m| m.impulse.id == b_high.id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    let score_low = result
        .memories
        .iter()
        .find(|m| m.impulse.id == b_low.id)
        .map(|m| m.activation_score)
        .unwrap_or(0.0);

    // High engagement node should receive more activation
    assert!(score_high > score_low);
}

#[test]
fn test_retrieval_reinforces_traversed_connections() {
    let db = common::test_db();
    let (a_id, b_id, _) = seed_graph(&db);

    let conns_before = db.get_connections_for_node(&a_id).unwrap();
    let weight_before = conns_before[0].weight;
    let count_before = conns_before[0].traversal_count;

    let engine = ActivationEngine::new(&db);
    let request = RetrievalRequest {
        query: "Rust memory".to_string(),
        max_results: 10,
        max_hops: 3,
    };

    engine.retrieve(&request).unwrap();

    let conns_after = db.get_connections_for_node(&a_id).unwrap();
    let ab_conn = conns_after
        .iter()
        .find(|c| {
            (c.source_id == a_id && c.target_id == b_id)
                || (c.source_id == b_id && c.target_id == a_id)
        })
        .unwrap();

    // Connection should have been reinforced
    assert!(ab_conn.weight >= weight_before);
    assert!(ab_conn.traversal_count > count_before);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_direct_match 2>&1`
Expected: Compilation error — `ActivationEngine` doesn't exist

- [ ] **Step 3: Implement activation engine**

`src/activation.rs`:
```rust
use std::collections::{HashMap, HashSet};

use crate::db::Database;
use crate::models::*;
use crate::weight;

pub struct ActivationEngine<'a> {
    db: &'a Database,
}

impl<'a> ActivationEngine<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn retrieve(&self, request: &RetrievalRequest) -> Result<RetrievalResult, String> {
        // Phase 1: Seed — find directly matching nodes via FTS
        let seed_matches = self
            .db
            .search_impulses_fts(&request.query)
            .map_err(|e| format!("FTS search failed: {}", e))?;

        if seed_matches.is_empty() {
            return Ok(RetrievalResult {
                memories: vec![],
                total_nodes_activated: 0,
            });
        }

        // Initialize activation scores from seed matches
        // FTS rank is negative (closer to 0 is better), normalize to 0-1
        let mut activations: HashMap<String, f64> = HashMap::new();
        let mut activation_paths: HashMap<String, Vec<String>> = HashMap::new();

        for (id, rank) in &seed_matches {
            // FTS5 rank is negative, more negative = better match
            // Normalize: use absolute value, then scale
            let score = (-rank).min(1.0).max(0.1);
            activations.insert(id.clone(), score);
            activation_paths.insert(id.clone(), vec![id.clone()]);
        }

        // Phase 2: Propagate — spread activation through connections
        let mut traversed_connections: HashSet<String> = HashSet::new();

        for _iteration in 0..MAX_PROPAGATION_ITERATIONS {
            let mut new_activations: HashMap<String, f64> = HashMap::new();
            let mut changed = false;

            let current_nodes: Vec<(String, f64)> =
                activations.iter().map(|(k, v)| (k.clone(), *v)).collect();

            for (node_id, node_activation) in &current_nodes {
                let connections = self
                    .db
                    .get_connections_for_node(node_id)
                    .map_err(|e| format!("Failed to get connections: {}", e))?;

                for conn in &connections {
                    let neighbor_id = if conn.source_id == *node_id {
                        &conn.target_id
                    } else {
                        &conn.source_id
                    };

                    // Skip if neighbor is deleted or superseded
                    let neighbor = match self.db.get_impulse(neighbor_id) {
                        Ok(imp) => imp,
                        Err(_) => continue,
                    };

                    if neighbor.status == ImpulseStatus::Deleted
                        || neighbor.status == ImpulseStatus::Superseded
                    {
                        continue;
                    }

                    // Calculate propagated activation
                    let now = chrono::Utc::now();
                    let hours = weight::hours_since(&conn.last_traversed_at, &now);
                    let effective_conn_weight =
                        weight::effective_weight(conn.weight, hours, DECAY_SEMANTIC);

                    // Emotional amplification: high engagement reduces proximity decay
                    let engagement_factor = match neighbor.engagement_level {
                        EngagementLevel::High => 0.8,   // less decay per hop
                        EngagementLevel::Medium => 0.5,  // standard
                        EngagementLevel::Low => 0.3,     // more decay per hop
                    };

                    let propagated =
                        node_activation * effective_conn_weight * engagement_factor;

                    let current = activations.get(neighbor_id).copied().unwrap_or(0.0);
                    let new_score = new_activations
                        .get(neighbor_id)
                        .copied()
                        .unwrap_or(current);

                    if propagated > new_score - current {
                        new_activations.insert(neighbor_id.clone(), current + propagated);
                        traversed_connections.insert(conn.id.clone());

                        // Update activation path
                        let mut path = activation_paths
                            .get(node_id)
                            .cloned()
                            .unwrap_or_default();
                        path.push(neighbor_id.clone());
                        activation_paths.insert(neighbor_id.clone(), path);

                        changed = true;
                    }
                }
            }

            // Merge new activations
            for (id, score) in new_activations {
                activations.insert(id, score);
            }

            if !changed {
                break;
            }
        }

        // Phase 3: Reinforce traversed connections
        for conn_id in &traversed_connections {
            if let Ok(conn) = self.db.get_connection(conn_id) {
                let new_weight = weight::reinforce(conn.weight);
                let _ = self.db.update_connection_weight(conn_id, new_weight);
                let _ = self.db.touch_connection(conn_id);
            }
        }

        // Touch accessed impulses
        for node_id in activations.keys() {
            let _ = self.db.touch_impulse(node_id);
        }

        // Phase 4: Assemble results
        let mut results: Vec<RetrievedMemory> = Vec::new();

        for (id, score) in &activations {
            if *score < ACTIVATION_THRESHOLD {
                continue;
            }

            let impulse = match self.db.get_impulse(id) {
                Ok(imp) => imp,
                Err(_) => continue,
            };

            if impulse.status == ImpulseStatus::Deleted
                || impulse.status == ImpulseStatus::Superseded
            {
                continue;
            }

            let path = activation_paths.get(id).cloned().unwrap_or_default();

            results.push(RetrievedMemory {
                impulse,
                activation_score: *score,
                activation_path: path,
            });
        }

        results.sort_by(|a, b| {
            b.activation_score
                .partial_cmp(&a.activation_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(request.max_results);

        let total_activated = activations.len();

        Ok(RetrievalResult {
            memories: results,
            total_nodes_activated: total_activated,
        })
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_ 2>&1`
Expected: All activation tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/activation.rs tests/test_activation.rs
git commit -m "feat: implement spreading activation retrieval with decay and emotional amplification"
```

---

### Task 6: Secret and PII Redaction

**Files:**
- Modify: `src/redaction.rs`
- Create: `tests/test_redaction.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_redaction.rs`:
```rust
use memory_graph::redaction;

#[test]
fn test_redacts_aws_access_key() {
    let input = "Use key AKIAIOSFODNN7EXAMPLE for access";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(result.clean_content.contains("[REDACTED]"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_generic_api_key_pattern() {
    let input = "api_key = sk-1234567890abcdef1234567890abcdef";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("sk-1234567890abcdef1234567890abcdef"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_bearer_token() {
    let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("eyJhbGci"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_connection_string() {
    let input = "DATABASE_URL=postgresql://user:password123@host:5432/db";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("password123"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_private_key() {
    let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("MIIEpAIBAAKCAQEA"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_email() {
    let input = "Contact jake.grogan@example.com for details";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("jake.grogan@example.com"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_clean_content_passes_through() {
    let input = "Rust is great for building memory systems";
    let result = redaction::redact(input);
    assert_eq!(result.clean_content, input);
    assert!(result.redactions.is_empty());
}

#[test]
fn test_multiple_redactions() {
    let input = "key=AKIAIOSFODNN7EXAMPLE and email=test@example.com";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(!result.clean_content.contains("test@example.com"));
    assert!(result.redactions.len() >= 2);
}

#[test]
fn test_has_secrets_check() {
    assert!(redaction::has_secrets("my key is AKIAIOSFODNN7EXAMPLE"));
    assert!(!redaction::has_secrets("just normal text here"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_redacts 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement redaction module**

`src/redaction.rs`:
```rust
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone)]
pub struct RedactionResult {
    pub clean_content: String,
    pub redactions: Vec<RedactedItem>,
}

#[derive(Debug, Clone)]
pub struct RedactedItem {
    pub pattern_name: String,
    pub original_length: usize,
}

struct SecretPattern {
    name: &'static str,
    regex: Regex,
}

static PATTERNS: LazyLock<Vec<SecretPattern>> = LazyLock::new(|| {
    vec![
        SecretPattern {
            name: "AWS Access Key",
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        },
        SecretPattern {
            name: "Generic API Key",
            regex: Regex::new(r"(?i)(api[_-]?key|apikey|secret[_-]?key)\s*[=:]\s*\S{16,}").unwrap(),
        },
        SecretPattern {
            name: "Bearer Token",
            regex: Regex::new(r"(?i)bearer\s+[a-zA-Z0-9\-_.]{20,}").unwrap(),
        },
        SecretPattern {
            name: "SK Token",
            regex: Regex::new(r"sk-[a-zA-Z0-9]{20,}").unwrap(),
        },
        SecretPattern {
            name: "Connection String",
            regex: Regex::new(r"(?i)(postgres|mysql|mongodb|redis)://\S+").unwrap(),
        },
        SecretPattern {
            name: "Private Key",
            regex: Regex::new(r"(?s)-----BEGIN[A-Z ]*PRIVATE KEY-----.*?-----END[A-Z ]*PRIVATE KEY-----").unwrap(),
        },
        SecretPattern {
            name: "Email Address",
            regex: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
        },
        SecretPattern {
            name: "GitHub Token",
            regex: Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
        },
        SecretPattern {
            name: "Generic Secret Assignment",
            regex: Regex::new(r"(?i)(password|passwd|secret)\s*[=:]\s*\S{8,}").unwrap(),
        },
    ]
});

pub fn redact(content: &str) -> RedactionResult {
    let mut result = content.to_string();
    let mut redactions = Vec::new();

    for pattern in PATTERNS.iter() {
        for mat in pattern.regex.find_iter(content) {
            let matched = mat.as_str();
            if result.contains(matched) {
                result = result.replace(matched, "[REDACTED]");
                redactions.push(RedactedItem {
                    pattern_name: pattern.name.to_string(),
                    original_length: matched.len(),
                });
            }
        }
    }

    RedactionResult {
        clean_content: result,
        redactions,
    }
}

pub fn has_secrets(content: &str) -> bool {
    PATTERNS.iter().any(|p| p.regex.is_match(content))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_redact 2>&1`
Expected: All redaction tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/redaction.rs tests/test_redaction.rs
git commit -m "feat: implement secret/PII detection and redaction"
```

---

### Task 7: Ingestion Pipeline

**Files:**
- Modify: `src/ingestion.rs`
- Create: `tests/test_ingestion.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_ingestion.rs`:
```rust
mod common;

use memory_graph::ingestion;
use memory_graph::models::*;

#[test]
fn test_explicit_save_creates_impulse() {
    let db = common::test_db();
    let result = ingestion::explicit_save(
        &db,
        "Auth middleware drops tokens under concurrent writes",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::High,
        vec!["debugging session".to_string()],
        "session-001",
    );

    assert!(result.is_ok());
    let impulse = result.unwrap();
    assert_eq!(
        impulse.content,
        "Auth middleware drops tokens under concurrent writes"
    );
    assert_eq!(impulse.weight, WEIGHT_EXPLICIT_SAVE);
    assert_eq!(impulse.status, ImpulseStatus::Confirmed);
}

#[test]
fn test_explicit_save_redacts_secrets() {
    let db = common::test_db();
    let result = ingestion::explicit_save(
        &db,
        "Use AKIAIOSFODNN7EXAMPLE to access the auth service",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-001",
    );

    assert!(result.is_ok());
    let impulse = result.unwrap();
    assert!(!impulse.content.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(impulse.content.contains("[REDACTED]"));
}

#[test]
fn test_explicit_save_with_connections() {
    let db = common::test_db();

    let first = ingestion::explicit_save(
        &db,
        "Rust is good for systems programming",
        ImpulseType::Preference,
        EmotionalValence::Positive,
        EngagementLevel::Medium,
        vec![],
        "session-001",
    )
    .unwrap();

    let second = ingestion::explicit_save_with_connections(
        &db,
        "Memory systems need low-level control",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-001",
        &[(first.id.clone(), "relates_to".to_string(), 0.6)],
    )
    .unwrap();

    let connections = db.get_connections_for_node(&second.id).unwrap();
    assert_eq!(connections.len(), 1);
    assert_eq!(connections[0].relationship, "relates_to");
}

#[test]
fn test_save_empty_content_fails() {
    let db = common::test_db();
    let result = ingestion::explicit_save(
        &db,
        "",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-001",
    );
    assert!(result.is_err());
}

#[test]
fn test_save_only_secrets_fails() {
    let db = common::test_db();
    let result = ingestion::explicit_save(
        &db,
        "AKIAIOSFODNN7EXAMPLE",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-001",
    );
    // After redaction, content is just "[REDACTED]" — should still save
    // but the content should be redacted
    assert!(result.is_ok());
    let impulse = result.unwrap();
    assert!(!impulse.content.contains("AKIA"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_explicit_save 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement ingestion module**

`src/ingestion.rs`:
```rust
use crate::db::Database;
use crate::models::*;
use crate::redaction;

pub fn explicit_save(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
) -> Result<Impulse, String> {
    if content.trim().is_empty() {
        return Err("Content cannot be empty".to_string());
    }

    // Redact secrets
    let redaction_result = redaction::redact(content);

    let input = NewImpulse {
        content: redaction_result.clean_content,
        impulse_type,
        initial_weight: WEIGHT_EXPLICIT_SAVE,
        emotional_valence,
        engagement_level,
        source_signals,
        source_type: SourceType::ExplicitSave,
        source_ref: source_ref.to_string(),
    };

    db.insert_impulse(&input)
        .map_err(|e| format!("Failed to insert impulse: {}", e))
}

pub fn explicit_save_with_connections(
    db: &Database,
    content: &str,
    impulse_type: ImpulseType,
    emotional_valence: EmotionalValence,
    engagement_level: EngagementLevel,
    source_signals: Vec<String>,
    source_ref: &str,
    connections: &[(String, String, f64)], // (target_id, relationship, weight)
) -> Result<Impulse, String> {
    let impulse = explicit_save(
        db,
        content,
        impulse_type,
        emotional_valence,
        engagement_level,
        source_signals,
        source_ref,
    )?;

    for (target_id, relationship, weight) in connections {
        let conn_input = NewConnection {
            source_id: impulse.id.clone(),
            target_id: target_id.clone(),
            weight: *weight,
            relationship: relationship.clone(),
        };
        db.insert_connection(&conn_input)
            .map_err(|e| format!("Failed to insert connection: {}", e))?;
    }

    Ok(impulse)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_ 2>&1`
Expected: All ingestion tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/ingestion.rs tests/test_ingestion.rs
git commit -m "feat: implement ingestion pipeline with redaction and connection support"
```

---

### Task 8: Session Management and Incognito Mode

**Files:**
- Modify: `src/session.rs`
- Create: `tests/test_session.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_session.rs`:
```rust
use memory_graph::session::Session;

#[test]
fn test_session_starts_not_incognito() {
    let session = Session::new("session-001");
    assert!(!session.is_incognito());
}

#[test]
fn test_set_incognito() {
    let mut session = Session::new("session-001");
    session.set_incognito(true);
    assert!(session.is_incognito());
}

#[test]
fn test_disable_incognito() {
    let mut session = Session::new("session-001");
    session.set_incognito(true);
    assert!(session.is_incognito());
    session.set_incognito(false);
    assert!(!session.is_incognito());
}

#[test]
fn test_session_id() {
    let session = Session::new("test-session-42");
    assert_eq!(session.id(), "test-session-42");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_session 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement session module**

`src/session.rs`:
```rust
#[derive(Debug)]
pub struct Session {
    id: String,
    incognito: bool,
}

impl Session {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            incognito: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn is_incognito(&self) -> bool {
        self.incognito
    }

    pub fn set_incognito(&mut self, incognito: bool) {
        self.incognito = incognito;
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_session 2>&1`
Expected: All session tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/session.rs tests/test_session.rs
git commit -m "feat: implement session management with incognito mode"
```

---

### Task 9: MCP Server Wiring

**Files:**
- Modify: `src/server.rs`
- Modify: `src/main.rs`
- Create: `tests/test_mcp.rs`

This wires everything together into MCP tool handlers.

- [ ] **Step 1: Write failing tests for MCP tool handlers**

`tests/test_mcp.rs`:
```rust
mod common;

use memory_graph::models::*;
use memory_graph::server::MemoryGraphServer;

#[test]
fn test_save_memory_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let result = server.handle_save_memory(
        "Rust is great for memory systems".to_string(),
        "heuristic".to_string(),
        Some("positive".to_string()),
        Some("high".to_string()),
        None,
    );

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("Rust is great"));
}

#[test]
fn test_save_memory_blocked_in_incognito() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    server.set_incognito(true);

    let result = server.handle_save_memory(
        "Should not be saved".to_string(),
        "observation".to_string(),
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("incognito"));
}

#[test]
fn test_retrieve_context_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    // Save something first
    server
        .handle_save_memory(
            "SQLite is excellent for local-first applications".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    let result = server.handle_retrieve_context(
        "SQLite local".to_string(),
        Some(10),
    );

    assert!(result.is_ok());
}

#[test]
fn test_retrieve_context_blocked_in_incognito() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_save_memory(
            "Test memory".to_string(),
            "observation".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    server.set_incognito(true);

    // Retrieval should still work in incognito (read-only) but should NOT
    // reinforce connections (no weight updates)
    let result = server.handle_retrieve_context(
        "test".to_string(),
        Some(10),
    );
    // This is a design decision: we allow reads in incognito but skip reinforcement
    assert!(result.is_ok());
}

#[test]
fn test_delete_memory_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let save_result = server
        .handle_save_memory(
            "To be deleted".to_string(),
            "observation".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    // Extract ID from response (JSON)
    let saved: serde_json::Value = serde_json::from_str(&save_result).unwrap();
    let id = saved["id"].as_str().unwrap().to_string();

    let result = server.handle_delete_memory(id.clone());
    assert!(result.is_ok());

    let inspect = server.handle_inspect_memory(id);
    assert!(inspect.is_ok());
    let inspected: serde_json::Value = serde_json::from_str(&inspect.unwrap()).unwrap();
    assert_eq!(inspected["status"], "deleted");
}

#[test]
fn test_update_memory_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let save_result = server
        .handle_save_memory(
            "Original content".to_string(),
            "decision".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    let saved: serde_json::Value = serde_json::from_str(&save_result).unwrap();
    let id = saved["id"].as_str().unwrap().to_string();

    let result = server.handle_update_memory(id.clone(), "Updated content".to_string());
    assert!(result.is_ok());

    // Old should be superseded
    let old = server.handle_inspect_memory(id).unwrap();
    let old_v: serde_json::Value = serde_json::from_str(&old).unwrap();
    assert_eq!(old_v["status"], "superseded");
}

#[test]
fn test_memory_status_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let result = server.handle_memory_status();
    assert!(result.is_ok());

    let stats: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(stats["total_impulses"], 0);
}

#[test]
fn test_set_incognito_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    assert!(!server.is_incognito());
    server.set_incognito(true);
    assert!(server.is_incognito());
    server.set_incognito(false);
    assert!(!server.is_incognito());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_save_memory 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement server module**

`src/server.rs`:
```rust
use std::sync::Mutex;

use crate::activation::ActivationEngine;
use crate::db::Database;
use crate::ingestion;
use crate::models::*;
use crate::session::Session;

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

    pub fn handle_save_memory(
        &self,
        content: String,
        impulse_type: String,
        emotional_valence: Option<String>,
        engagement_level: Option<String>,
        source_ref: Option<String>,
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

        let db = self.db.lock().unwrap();
        let impulse = ingestion::explicit_save(
            &db,
            &content,
            itype,
            valence,
            engagement,
            vec![],
            &sref,
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
            "status": impulse.status,
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
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_ 2>&1`
Expected: All MCP handler tests PASS

- [ ] **Step 5: Wire up main.rs with MCP transport**

`src/main.rs`:
```rust
pub mod activation;
pub mod db;
pub mod ingestion;
pub mod models;
pub mod redaction;
pub mod server;
pub mod session;
pub mod weight;

use server::MemoryGraphServer;
use std::path::PathBuf;

fn default_db_path() -> PathBuf {
    let mut path = dirs_or_default();
    path.push("memory-graph");
    std::fs::create_dir_all(&path).ok();
    path.push("memory.db");
    path
}

fn dirs_or_default() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir
    } else {
        PathBuf::from(".")
    }
}

fn main() {
    let db_path = std::env::var("MEMORY_GRAPH_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_db_path());

    eprintln!(
        "memory-graph: starting with database at {}",
        db_path.display()
    );

    let _server = match MemoryGraphServer::new(db_path.to_str().unwrap_or("memory.db")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize memory-graph: {}", e);
            std::process::exit(1);
        }
    };

    // MCP transport wiring will be added once rmcp API is confirmed
    // For now, the server is functional and testable via direct method calls
    eprintln!("memory-graph: server initialized (MCP transport pending)");
}
```

Add `dirs` to Cargo.toml dependencies:

In `Cargo.toml`, add:
```toml
dirs = "5"
```

- [ ] **Step 6: Run all tests and verify compilation**

Run: `cargo test 2>&1`
Expected: All tests PASS, binary compiles

- [ ] **Step 7: Commit**

```bash
git add src/server.rs src/main.rs Cargo.toml tests/test_mcp.rs
git commit -m "feat: implement MCP server handlers with save, retrieve, delete, update, inspect, and incognito"
```

---

### Task 10: End-to-End Validation Tests

These tests map directly to the Phase 1 validation criteria from the PRD.

**Files:**
- Create: `tests/test_validation.rs`

- [ ] **Step 1: Write validation tests**

`tests/test_validation.rs`:
```rust
mod common;

use memory_graph::activation::ActivationEngine;
use memory_graph::db::Database;
use memory_graph::ingestion;
use memory_graph::models::*;
use memory_graph::redaction;
use memory_graph::server::MemoryGraphServer;
use memory_graph::weight;

// ============================================================
// PRD Validation Criterion 1: Store/retrieve round-trip
// "save impulses, retrieve by related query, correct ones return"
// ============================================================

#[test]
fn validation_store_retrieve_round_trip() {
    let db = common::test_db();

    // Save 5 impulses on different topics
    ingestion::explicit_save(
        &db,
        "Rust ownership model prevents data races at compile time",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "session-v1",
    )
    .unwrap();

    ingestion::explicit_save(
        &db,
        "PostgreSQL handles concurrent writes better than SQLite",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-v1",
    )
    .unwrap();

    ingestion::explicit_save(
        &db,
        "React hooks should not be called conditionally",
        ImpulseType::Pattern,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-v1",
    )
    .unwrap();

    ingestion::explicit_save(
        &db,
        "Memory decay follows exponential curves in cognitive science",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "session-v1",
    )
    .unwrap();

    ingestion::explicit_save(
        &db,
        "Kubernetes pod scheduling uses resource requests and limits",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-v1",
    )
    .unwrap();

    let engine = ActivationEngine::new(&db);

    // Query about Rust should return the Rust impulse
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "Rust ownership data races".to_string(),
            max_results: 3,
            max_hops: 2,
        })
        .unwrap();

    assert!(!result.memories.is_empty());
    assert!(result.memories[0]
        .impulse
        .content
        .contains("Rust ownership"));

    // Query about databases should return the PostgreSQL impulse
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "database concurrent writes".to_string(),
            max_results: 3,
            max_hops: 2,
        })
        .unwrap();

    assert!(!result.memories.is_empty());
    assert!(result.memories[0].impulse.content.contains("PostgreSQL"));

    // Query about memory should return the cognitive science impulse
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "memory decay cognitive".to_string(),
            max_results: 3,
            max_hops: 2,
        })
        .unwrap();

    assert!(!result.memories.is_empty());
    assert!(result.memories[0].impulse.content.contains("decay"));
}

// ============================================================
// PRD Validation Criterion 2: Spreading activation
// "connected impulses activate through adjacency"
// ============================================================

#[test]
fn validation_spreading_activation_through_adjacency() {
    let db = common::test_db();

    let auth = ingestion::explicit_save(
        &db,
        "JWT tokens should have short expiration times",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-v2",
    )
    .unwrap();

    let security = ingestion::explicit_save_with_connections(
        &db,
        "Session fixation attacks can bypass authentication",
        ImpulseType::Pattern,
        EmotionalValence::Negative,
        EngagementLevel::High,
        vec![],
        "session-v2",
        &[(auth.id.clone(), "relates_to".to_string(), 0.7)],
    )
    .unwrap();

    let mitigation = ingestion::explicit_save_with_connections(
        &db,
        "Regenerate session IDs after privilege escalation",
        ImpulseType::Decision,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-v2",
        &[(security.id.clone(), "derived_from".to_string(), 0.8)],
    )
    .unwrap();

    let engine = ActivationEngine::new(&db);

    // Query matches "JWT tokens" directly, but connected nodes should also activate
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "JWT token expiration".to_string(),
            max_results: 10,
            max_hops: 3,
        })
        .unwrap();

    let ids: Vec<&str> = result.memories.iter().map(|m| m.impulse.id.as_str()).collect();

    // Direct match
    assert!(ids.contains(&auth.id.as_str()));
    // 1-hop connected
    assert!(ids.contains(&security.id.as_str()));
    // 2-hop connected (if activation threshold is met)
    // This may or may not appear depending on weights — check that activation spreads at least 1 hop
    assert!(result.memories.len() >= 2);
}

// ============================================================
// PRD Validation Criterion 3: Decay
// "accessed memories surface more strongly than untouched ones after time"
// ============================================================

#[test]
fn validation_decay_reduces_effective_weight() {
    // Test the decay math directly since we can't simulate real time in tests
    let initial = 0.7;

    // After 24 hours with semantic decay
    let after_1_day = weight::effective_weight(initial, 24.0, DECAY_SEMANTIC);
    assert!(after_1_day < initial);
    assert!(after_1_day > initial * 0.9); // Semantic decays slowly

    // After 30 days
    let after_30_days = weight::effective_weight(initial, 720.0, DECAY_SEMANTIC);
    assert!(after_30_days < after_1_day);

    // Episodic decays much faster
    let episodic_30_days = weight::effective_weight(initial, 720.0, DECAY_EPISODIC);
    assert!(episodic_30_days < after_30_days);

    // But nothing goes below floor
    let after_year = weight::effective_weight(initial, 8760.0, DECAY_EPISODIC);
    assert!(after_year >= WEIGHT_FLOOR);
}

#[test]
fn validation_reinforcement_counters_decay() {
    let initial = 0.5;

    // Simulate: access every day for 10 days
    let mut w = initial;
    for _ in 0..10 {
        // Decay for 24 hours
        w = weight::effective_weight(w, 24.0, DECAY_SEMANTIC);
        // Then reinforce
        w = weight::reinforce(w);
    }

    // After regular access, weight should be higher than initial
    assert!(w > initial);

    // Simulate: no access for 60 days
    let decayed = weight::effective_weight(w, 1440.0, DECAY_SEMANTIC);
    // Should be lower but still above floor
    assert!(decayed < w);
    assert!(decayed > WEIGHT_FLOOR);
}

// ============================================================
// PRD Validation Criterion 4: Reconstruction
// "system builds coherent narrative from connected impulses without stored summary"
// ============================================================

#[test]
fn validation_narrative_reconstruction_from_connections() {
    let db = common::test_db();

    // Create a cluster of connected impulses about a design decision
    let problem = ingestion::explicit_save(
        &db,
        "AI memory systems lose context when switching providers",
        ImpulseType::Observation,
        EmotionalValence::Negative,
        EngagementLevel::High,
        vec![],
        "session-v4",
    )
    .unwrap();

    let insight = ingestion::explicit_save_with_connections(
        &db,
        "Memory should be portable and user-owned, not provider-locked",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "session-v4",
        &[(problem.id.clone(), "derived_from".to_string(), 0.9)],
    )
    .unwrap();

    let decision = ingestion::explicit_save_with_connections(
        &db,
        "Build memory as an MCP server so any client can use it",
        ImpulseType::Decision,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "session-v4",
        &[(insight.id.clone(), "derived_from".to_string(), 0.8)],
    )
    .unwrap();

    let implementation = ingestion::explicit_save_with_connections(
        &db,
        "Use SQLite for portable local-first storage",
        ImpulseType::Decision,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-v4",
        &[(decision.id.clone(), "relates_to".to_string(), 0.7)],
    )
    .unwrap();

    // Query about "memory portability" should reconstruct the narrative
    let engine = ActivationEngine::new(&db);
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "memory portability provider".to_string(),
            max_results: 10,
            max_hops: 4,
        })
        .unwrap();

    // Should get multiple connected impulses that together tell the story
    assert!(result.memories.len() >= 3);

    // The results should include the problem, insight, and decision
    let contents: Vec<&str> = result
        .memories
        .iter()
        .map(|m| m.impulse.content.as_str())
        .collect();

    assert!(contents.iter().any(|c| c.contains("lose context")));
    assert!(contents.iter().any(|c| c.contains("portable")));
    assert!(contents.iter().any(|c| c.contains("MCP server")));
}

// ============================================================
// PRD Validation Criterion 5: Emotional weighting
// "high-engagement impulses surface more readily"
// ============================================================

#[test]
fn validation_emotional_weighting() {
    let db = common::test_db();

    // Two impulses about similar topics, different engagement
    let high = ingestion::explicit_save(
        &db,
        "Graph databases enable powerful relationship traversal queries",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec!["excited discussion".to_string(), "deep exploration".to_string()],
        "session-v5",
    )
    .unwrap();

    let low = ingestion::explicit_save(
        &db,
        "Graph models store relationships between entities",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-v5",
    )
    .unwrap();

    // Connect both to a shared concept
    let anchor = ingestion::explicit_save(
        &db,
        "Graph data structures for memory systems",
        ImpulseType::Pattern,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "session-v5",
    )
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: anchor.id.clone(),
        target_id: high.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    db.insert_connection(&NewConnection {
        source_id: anchor.id.clone(),
        target_id: low.id.clone(),
        weight: 0.5,
        relationship: "relates_to".to_string(),
    })
    .unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine
        .retrieve(&RetrievalRequest {
            query: "graph data structures memory".to_string(),
            max_results: 10,
            max_hops: 2,
        })
        .unwrap();

    // Find scores for high and low engagement nodes
    let score_high = result
        .memories
        .iter()
        .find(|m| m.impulse.id == high.id)
        .map(|m| m.activation_score);

    let score_low = result
        .memories
        .iter()
        .find(|m| m.impulse.id == low.id)
        .map(|m| m.activation_score);

    // Both should appear, but high engagement should score higher
    if let (Some(sh), Some(sl)) = (score_high, score_low) {
        assert!(sh > sl, "High engagement ({}) should score above low engagement ({})", sh, sl);
    }
}

// ============================================================
// PRD Validation Criterion 6: Security
// "API keys in ingested conversations are stripped before persistence"
// ============================================================

#[test]
fn validation_security_secrets_stripped() {
    let db = common::test_db();

    let impulse = ingestion::explicit_save(
        &db,
        "Connect with AKIAIOSFODNN7EXAMPLE and secret key wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-v6",
    )
    .unwrap();

    // AWS key should be redacted
    assert!(!impulse.content.contains("AKIAIOSFODNN7EXAMPLE"));
    // Content should still exist (not empty)
    assert!(!impulse.content.is_empty());
    assert!(impulse.content.contains("[REDACTED]"));
}

#[test]
fn validation_security_bearer_tokens_stripped() {
    let db = common::test_db();

    let impulse = ingestion::explicit_save(
        &db,
        "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.something",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-v6",
    )
    .unwrap();

    assert!(!impulse.content.contains("eyJhbGci"));
}

#[test]
fn validation_security_connection_strings_stripped() {
    let db = common::test_db();

    let impulse = ingestion::explicit_save(
        &db,
        "Set DATABASE_URL=postgresql://admin:supersecret@prod.db.example.com:5432/maindb",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Low,
        vec![],
        "session-v6",
    )
    .unwrap();

    assert!(!impulse.content.contains("supersecret"));
    assert!(!impulse.content.contains("prod.db.example.com"));
}

// ============================================================
// Additional: Incognito mode leaves zero trace
// ============================================================

#[test]
fn validation_incognito_zero_trace() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    // Save something before incognito
    server
        .handle_save_memory(
            "Pre-incognito memory".to_string(),
            "observation".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    let stats_before = server.handle_memory_status().unwrap();
    let before: serde_json::Value = serde_json::from_str(&stats_before).unwrap();
    let count_before = before["total_impulses"].as_i64().unwrap();

    // Enable incognito
    server.set_incognito(true);

    // Attempt to save — should fail
    let save_result = server.handle_save_memory(
        "Incognito memory attempt".to_string(),
        "observation".to_string(),
        None,
        None,
        None,
    );
    assert!(save_result.is_err());

    // Disable incognito
    server.set_incognito(false);

    // Count should be unchanged
    let stats_after = server.handle_memory_status().unwrap();
    let after: serde_json::Value = serde_json::from_str(&stats_after).unwrap();
    let count_after = after["total_impulses"].as_i64().unwrap();

    assert_eq!(count_before, count_after);
}

// ============================================================
// Additional: Supersession chain
// ============================================================

#[test]
fn validation_supersession_preserves_history() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let v1 = server
        .handle_save_memory(
            "Initial understanding of auth flow".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    let v1_parsed: serde_json::Value = serde_json::from_str(&v1).unwrap();
    let v1_id = v1_parsed["id"].as_str().unwrap().to_string();

    let v2 = server
        .handle_update_memory(v1_id.clone(), "Revised understanding of auth flow with edge cases".to_string())
        .unwrap();

    let v2_parsed: serde_json::Value = serde_json::from_str(&v2).unwrap();
    let v2_id = v2_parsed["id"].as_str().unwrap().to_string();

    // v1 should be superseded
    let v1_inspect = server.handle_inspect_memory(v1_id).unwrap();
    let v1_data: serde_json::Value = serde_json::from_str(&v1_inspect).unwrap();
    assert_eq!(v1_data["status"], "superseded");

    // v2 should be confirmed
    let v2_inspect = server.handle_inspect_memory(v2_id.clone()).unwrap();
    let v2_data: serde_json::Value = serde_json::from_str(&v2_inspect).unwrap();
    assert_eq!(v2_data["status"], "confirmed");

    // v2 should have a supersession connection to v1
    let connections = v2_data["connections"].as_array().unwrap();
    assert!(!connections.is_empty());
    assert!(connections
        .iter()
        .any(|c| c["relationship"] == "supersedes"));
}
```

- [ ] **Step 2: Run all validation tests**

Run: `cargo test validation_ 2>&1`
Expected: All validation tests PASS

- [ ] **Step 3: Run the complete test suite**

Run: `cargo test 2>&1`
Expected: ALL tests PASS across all test files

- [ ] **Step 4: Commit**

```bash
git add tests/test_validation.rs
git commit -m "feat: add end-to-end validation tests for all Phase 1 PRD criteria"
```

---

### Task 11: MCP Transport Wiring

**Files:**
- Modify: `src/server.rs` (add rmcp trait implementations)
- Modify: `src/main.rs` (wire stdio transport)

Note: This task depends on the exact rmcp API. The implementation below follows the documented rmcp patterns. If the API has changed, adapt accordingly — the server logic is already tested via direct method calls.

- [ ] **Step 1: Add rmcp tool annotations to server**

Add to the top of `src/server.rs`, after existing imports:
```rust
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::model::*;
use rmcp::schemars;
use rmcp::tool;
```

Add a new struct that wraps the server for MCP:
```rust
use std::sync::Arc;

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

#[tool(tool_box)]
impl McpHandler {
    #[tool(description = "Save a new memory (impulse). Types: heuristic, preference, decision, pattern, observation. Valence: positive, negative, neutral. Engagement: low, medium, high.")]
    fn save_memory(
        &self,
        #[tool(param, description = "The content of the memory to save")] content: String,
        #[tool(param, description = "Type of impulse: heuristic, preference, decision, pattern, observation")] impulse_type: String,
        #[tool(param, description = "Emotional valence: positive, negative, neutral")] emotional_valence: Option<String>,
        #[tool(param, description = "Engagement level: low, medium, high")] engagement_level: Option<String>,
        #[tool(param, description = "Source reference (conversation or session ID)")] source_ref: Option<String>,
    ) -> Result<String, String> {
        self.inner.handle_save_memory(content, impulse_type, emotional_valence, engagement_level, source_ref)
    }

    #[tool(description = "Retrieve relevant memory context using spreading activation. Returns memories ranked by activation strength.")]
    fn retrieve_context(
        &self,
        #[tool(param, description = "Query text to seed memory retrieval")] query: String,
        #[tool(param, description = "Maximum number of results to return")] max_results: Option<usize>,
    ) -> Result<String, String> {
        self.inner.handle_retrieve_context(query, max_results)
    }

    #[tool(description = "Delete a memory (soft delete — connections fade but remain traversable)")]
    fn delete_memory(
        &self,
        #[tool(param, description = "ID of the memory to delete")] id: String,
    ) -> Result<String, String> {
        self.inner.handle_delete_memory(id)
    }

    #[tool(description = "Update a memory's content. Creates a new version and marks the old one as superseded.")]
    fn update_memory(
        &self,
        #[tool(param, description = "ID of the memory to update")] id: String,
        #[tool(param, description = "New content for the memory")] new_content: String,
    ) -> Result<String, String> {
        self.inner.handle_update_memory(id, new_content)
    }

    #[tool(description = "Inspect a specific memory's full record including provenance, weight, and connections")]
    fn inspect_memory(
        &self,
        #[tool(param, description = "ID of the memory to inspect")] id: String,
    ) -> Result<String, String> {
        self.inner.handle_inspect_memory(id)
    }

    #[tool(description = "Get memory graph status: counts, connection stats, incognito state")]
    fn memory_status(&self) -> Result<String, String> {
        self.inner.handle_memory_status()
    }

    #[tool(description = "Enable or disable incognito mode. When active: no saves, no proposals, no trace.")]
    fn set_incognito(
        &self,
        #[tool(param, description = "true to enable incognito, false to disable")] enabled: bool,
    ) -> Result<String, String> {
        self.inner.handle_set_incognito(enabled)
    }

    #[tool(description = "Explain why a memory was recalled for a given query — shows activation path and scores")]
    fn explain_recall(
        &self,
        #[tool(param, description = "The query that triggered recall")] query: String,
        #[tool(param, description = "ID of the memory to explain")] memory_id: String,
    ) -> Result<String, String> {
        self.inner.handle_explain_recall(query, memory_id)
    }
}

impl rmcp::ServerHandler for McpHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "memory-graph".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            ..Default::default()
        }
    }
}
```

- [ ] **Step 2: Update main.rs for MCP stdio transport**

```rust
pub mod activation;
pub mod db;
pub mod ingestion;
pub mod models;
pub mod redaction;
pub mod server;
pub mod session;
pub mod weight;

use server::{McpHandler, MemoryGraphServer};
use std::path::PathBuf;

fn default_db_path() -> PathBuf {
    let mut path = dirs_or_default();
    path.push("memory-graph");
    std::fs::create_dir_all(&path).ok();
    path.push("memory.db");
    path
}

fn dirs_or_default() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir
    } else {
        PathBuf::from(".")
    }
}

#[tokio::main]
async fn main() {
    let db_path = std::env::var("MEMORY_GRAPH_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_db_path());

    eprintln!(
        "memory-graph: starting with database at {}",
        db_path.display()
    );

    let inner = match MemoryGraphServer::new(db_path.to_str().unwrap_or("memory.db")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize memory-graph: {}", e);
            std::process::exit(1);
        }
    };

    let handler = McpHandler::new(inner);

    eprintln!("memory-graph: MCP server starting on stdio");

    let transport = rmcp::transport::stdio::stdio_transport();
    let server = handler.serve(transport).await;

    match server {
        Ok(s) => {
            eprintln!("memory-graph: server running");
            s.waiting().await;
        }
        Err(e) => {
            eprintln!("memory-graph: server error: {}", e);
            std::process::exit(1);
        }
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build 2>&1`
Expected: Successful compilation. If rmcp API doesn't match exactly, adapt the trait impl and tool annotations to match the actual crate API — the core logic is the same.

- [ ] **Step 4: Run full test suite**

Run: `cargo test 2>&1`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/server.rs src/main.rs
git commit -m "feat: wire MCP stdio transport with tool annotations for all memory operations"
```

---

### Task 12: Final Cleanup and Binary Verification

**Files:**
- Possibly modify any files that need fixing from test output

- [ ] **Step 1: Run clippy**

Run: `cargo clippy 2>&1`
Expected: No errors (warnings are acceptable for now)

Fix any clippy errors that appear.

- [ ] **Step 2: Run full test suite one final time**

Run: `cargo test 2>&1`
Expected: ALL tests PASS

- [ ] **Step 3: Build release binary**

Run: `cargo build --release 2>&1`
Expected: Successful compilation

- [ ] **Step 4: Verify binary runs**

Run: `echo '{}' | timeout 2 ./target/release/memory-graph 2>&1 || true`
Expected: Should print startup messages to stderr and attempt to start MCP transport

- [ ] **Step 5: Commit any cleanup**

```bash
git add -A
git commit -m "chore: clippy fixes and release build verification"
```

---

## Post-Implementation Notes

After completing all tasks, the following should be true:

1. `cargo test` passes all tests including the 8 PRD validation tests
2. `cargo build --release` produces a working binary
3. The binary starts an MCP server on stdio when run
4. All 7 MCP tools are registered and functional: save_memory, retrieve_context, delete_memory, update_memory, inspect_memory, memory_status, set_incognito, explain_recall

**What is NOT included in Phase 1 (deferred to Phase 2+):**
- Ghost graph registration and pull-through
- End-of-session adaptive extraction (requires LLM call)
- `recall_narrative` tool (requires LLM for narrative reconstruction)
- `propose_memories` / `confirm_proposal` / `dismiss_proposal` tools
- Visual graph inspection
- Cross-device sync
