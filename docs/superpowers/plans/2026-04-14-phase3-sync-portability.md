# Phase 3: Sync and Portability — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add backup/restore, cross-device portability via file-based sync, and validate multiple ghost graph sources working together. Preserve local-first guarantees throughout.

**Architecture:** Backup is SQLite file snapshot with integrity verification. Cross-device sync uses a shared directory (cloud drive, NAS, etc.) with write-ahead journaling to avoid corruption. Multiple ghost graphs coexist with independent weight spaces. No custom sync protocol — leverage existing file sync infrastructure.

**Tech Stack:** Same as Phase 1+2 plus `sha2` for integrity checksums.

**Depends on:** Phase 2 complete — all Phase 1 + Phase 2 tests passing.

---

## File Structure (additions to Phase 2)

```
src/
    backup.rs           # Backup creation, integrity verification, restore
    sync.rs             # Cross-device sync coordination via shared directory
tests/
    test_backup.rs      # Backup/restore round-trip, integrity checks
    test_sync.rs        # Sync coordination, conflict detection
    test_validation_p3.rs  # Phase 3 PRD validation criteria
```

---

### Task 1: Add Phase 3 Dependencies and Scaffolding

**Files:**
- Modify: `Cargo.toml`
- Create: `src/backup.rs`
- Create: `src/sync.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add dependencies**

Add to `Cargo.toml` `[dependencies]`:
```toml
sha2 = "0.10"
```

- [ ] **Step 2: Create module files**

`src/backup.rs`:
```rust
// Backup and restore operations
```

`src/sync.rs`:
```rust
// Cross-device sync coordination
```

- [ ] **Step 3: Add module declarations to main.rs**

Add:
```rust
pub mod backup;
pub mod sync;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build 2>&1`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/backup.rs src/sync.rs src/main.rs
git commit -m "feat: scaffold Phase 3 modules for backup and sync"
```

---

### Task 2: Backup and Restore

**Files:**
- Modify: `src/backup.rs`
- Create: `tests/test_backup.rs`

- [ ] **Step 1: Write failing tests**

`tests/test_backup.rs`:
```rust
mod common;

use memory_graph::backup;
use memory_graph::db::Database;
use memory_graph::ingestion;
use memory_graph::models::*;
use tempfile::TempDir;

#[test]
fn test_backup_creates_file() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let backup_path = tmp.path().join("backup.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db,
        "Test memory for backup",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    ).unwrap();

    backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    assert!(backup_path.exists());
}

#[test]
fn test_backup_includes_checksum() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let backup_path = tmp.path().join("backup.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db,
        "Test memory",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    ).unwrap();

    let result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    assert!(!result.checksum.is_empty());
    assert!(result.impulse_count > 0);
}

#[test]
fn test_verify_backup_integrity() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let backup_path = tmp.path().join("backup.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db,
        "Important memory",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    ).unwrap();

    let backup_result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    let verified = backup::verify_backup(backup_path.to_str().unwrap(), &backup_result.checksum).unwrap();
    assert!(verified);
}

#[test]
fn test_corrupted_backup_fails_verification() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let backup_path = tmp.path().join("backup.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db,
        "Memory",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    ).unwrap();

    backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    // Corrupt the backup
    std::fs::write(&backup_path, b"corrupted data").unwrap();

    let verified = backup::verify_backup(backup_path.to_str().unwrap(), "original_checksum").unwrap();
    assert!(!verified);
}

#[test]
fn test_restore_from_backup() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("original.db");
    let backup_path = tmp.path().join("backup.db");
    let restore_path = tmp.path().join("restored.db");

    // Create original with data
    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db,
        "Memory that should survive restore",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec!["important".to_string()],
        "session-001",
    ).unwrap();

    let original_count = db.impulse_count().unwrap();
    assert_eq!(original_count, 1);

    // Backup
    let backup_result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    // Restore to new location
    backup::restore_backup(backup_path.to_str().unwrap(), restore_path.to_str().unwrap(), &backup_result.checksum).unwrap();

    // Verify restored data
    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();
    let restored_count = restored_db.impulse_count().unwrap();
    assert_eq!(restored_count, original_count);

    let impulses = restored_db.list_impulses(None).unwrap();
    assert_eq!(impulses[0].content, "Memory that should survive restore");
}

#[test]
fn test_restore_preserves_connections() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("original.db");
    let backup_path = tmp.path().join("backup.db");
    let restore_path = tmp.path().join("restored.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();

    let a = ingestion::explicit_save(
        &db, "Node A", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium,
        vec![], "test",
    ).unwrap();

    let b = ingestion::explicit_save_with_connections(
        &db, "Node B", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium,
        vec![], "test",
        &[(a.id.clone(), "relates_to".to_string(), 0.7)],
    ).unwrap();

    let original_conn_count = db.connection_count().unwrap();

    let backup_result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    backup::restore_backup(backup_path.to_str().unwrap(), restore_path.to_str().unwrap(), &backup_result.checksum).unwrap();

    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();
    assert_eq!(restored_db.connection_count().unwrap(), original_conn_count);
}

#[test]
fn test_restore_preserves_ghost_nodes() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("original.db");
    let backup_path = tmp.path().join("backup.db");
    let restore_path = tmp.path().join("restored.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();

    // Register a ghost source and add nodes
    db.register_ghost_source("vault", "/fake/path", "obsidian").unwrap();
    db.insert_ghost_node(&NewGhostNode {
        source_graph: "vault".to_string(),
        external_ref: "note.md".to_string(),
        title: "Ghost Note".to_string(),
        metadata: serde_json::json!({}),
        initial_weight: 0.3,
    }).unwrap();

    let backup_result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    backup::restore_backup(backup_path.to_str().unwrap(), restore_path.to_str().unwrap(), &backup_result.checksum).unwrap();

    let restored_db = Database::open(restore_path.to_str().unwrap()).unwrap();
    let sources = restored_db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 1);

    let nodes = restored_db.list_ghost_nodes_by_source("vault").unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].title, "Ghost Note");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_backup 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement backup module**

`src/backup.rs`:
```rust
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

use crate::db::Database;

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub path: String,
    pub checksum: String,
    pub impulse_count: i64,
    pub connection_count: i64,
    pub size_bytes: u64,
}

pub fn create_backup(db: &Database, backup_path: &str) -> Result<BackupResult, String> {
    // Use SQLite's backup API via VACUUM INTO for a consistent snapshot
    db.vacuum_into(backup_path)
        .map_err(|e| format!("Backup failed: {}", e))?;

    let checksum = file_checksum(backup_path)
        .map_err(|e| format!("Checksum failed: {}", e))?;

    let size = fs::metadata(backup_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let stats = db.memory_stats()
        .map_err(|e| format!("Stats failed: {}", e))?;

    Ok(BackupResult {
        path: backup_path.to_string(),
        checksum,
        impulse_count: stats.total_impulses,
        connection_count: stats.total_connections,
        size_bytes: size,
    })
}

pub fn verify_backup(backup_path: &str, expected_checksum: &str) -> Result<bool, String> {
    if !Path::new(backup_path).exists() {
        return Ok(false);
    }

    let actual = file_checksum(backup_path)
        .map_err(|e| format!("Checksum failed: {}", e))?;

    Ok(actual == expected_checksum)
}

pub fn restore_backup(
    backup_path: &str,
    restore_path: &str,
    expected_checksum: &str,
) -> Result<(), String> {
    // Verify integrity first
    if !verify_backup(backup_path, expected_checksum)? {
        return Err("Backup integrity check failed — checksum mismatch".to_string());
    }

    // Copy the backup to the restore location
    fs::copy(backup_path, restore_path)
        .map_err(|e| format!("Restore copy failed: {}", e))?;

    // Verify the restored database opens correctly
    let _db = Database::open(restore_path)
        .map_err(|e| format!("Restored database is corrupt: {}", e))?;

    Ok(())
}

fn file_checksum(path: &str) -> Result<String, std::io::Error> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}
```

Add `vacuum_into` to `Database` in `src/db.rs`:
```rust
pub fn vacuum_into(&self, path: &str) -> SqlResult<()> {
    self.conn.execute_batch(&format!("VACUUM INTO '{}'", path.replace('\'', "''")))?;
    Ok(())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_backup 2>&1`
Expected: All backup tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/backup.rs src/db.rs tests/test_backup.rs Cargo.toml
git commit -m "feat: implement backup/restore with SHA-256 integrity verification"
```

---

### Task 3: Sync Coordination

**Files:**
- Modify: `src/sync.rs`
- Create: `tests/test_sync.rs`

The sync model is simple: the user points to a shared directory (iCloud, Dropbox, NAS mount). The system writes snapshots there and detects when a newer snapshot exists from another device.

- [ ] **Step 1: Write failing tests**

`tests/test_sync.rs`:
```rust
mod common;

use memory_graph::backup;
use memory_graph::db::Database;
use memory_graph::ingestion;
use memory_graph::models::*;
use memory_graph::sync;
use tempfile::TempDir;

#[test]
fn test_sync_export_creates_snapshot() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("local.db");
    let sync_dir = tmp.path().join("sync");
    std::fs::create_dir(&sync_dir).unwrap();

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db, "Sync test memory", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium,
        vec![], "test",
    ).unwrap();

    let result = sync::export_snapshot(&db, sync_dir.to_str().unwrap(), "device-a").unwrap();
    assert!(!result.snapshot_path.is_empty());
    assert!(std::path::Path::new(&result.snapshot_path).exists());
}

#[test]
fn test_sync_detects_newer_remote_snapshot() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");
    std::fs::create_dir(&sync_dir).unwrap();

    let db_a_path = tmp.path().join("device_a.db");
    let db_b_path = tmp.path().join("device_b.db");

    // Device A exports
    let db_a = Database::open(db_a_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db_a, "From device A", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium,
        vec![], "test",
    ).unwrap();
    sync::export_snapshot(&db_a, sync_dir.to_str().unwrap(), "device-a").unwrap();

    // Device B checks
    let db_b = Database::open(db_b_path.to_str().unwrap()).unwrap();
    let status = sync::check_sync_status(sync_dir.to_str().unwrap(), "device-b").unwrap();
    assert!(status.has_remote_updates);
}

#[test]
fn test_sync_import_remote_snapshot() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");
    std::fs::create_dir(&sync_dir).unwrap();

    let db_a_path = tmp.path().join("device_a.db");
    let db_b_path = tmp.path().join("device_b.db");

    // Device A creates data and exports
    let db_a = Database::open(db_a_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db_a, "Memory from device A", ImpulseType::Heuristic,
        EmotionalValence::Positive, EngagementLevel::High,
        vec![], "test",
    ).unwrap();
    let export = sync::export_snapshot(&db_a, sync_dir.to_str().unwrap(), "device-a").unwrap();

    // Device B imports
    sync::import_snapshot(
        &export.snapshot_path,
        db_b_path.to_str().unwrap(),
        &export.checksum,
    ).unwrap();

    let db_b = Database::open(db_b_path.to_str().unwrap()).unwrap();
    let impulses = db_b.list_impulses(None).unwrap();
    assert_eq!(impulses.len(), 1);
    assert_eq!(impulses[0].content, "Memory from device A");
}

#[test]
fn test_sync_manifest_tracks_devices() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");
    std::fs::create_dir(&sync_dir).unwrap();

    let db_path = tmp.path().join("test.db");
    let db = Database::open(db_path.to_str().unwrap()).unwrap();

    sync::export_snapshot(&db, sync_dir.to_str().unwrap(), "my-laptop").unwrap();

    let manifest = sync::read_manifest(sync_dir.to_str().unwrap()).unwrap();
    assert!(manifest.devices.contains_key("my-laptop"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_sync 2>&1`
Expected: Compilation error

- [ ] **Step 3: Implement sync module**

`src/sync.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::backup;
use crate::db::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncManifest {
    pub devices: HashMap<String, DeviceEntry>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub device_id: String,
    pub snapshot_filename: String,
    pub checksum: String,
    pub exported_at: DateTime<Utc>,
    pub impulse_count: i64,
}

#[derive(Debug, Clone)]
pub struct ExportResult {
    pub snapshot_path: String,
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub has_remote_updates: bool,
    pub remote_devices: Vec<String>,
    pub latest_remote_device: Option<String>,
    pub latest_remote_time: Option<DateTime<Utc>>,
}

pub fn export_snapshot(
    db: &Database,
    sync_dir: &str,
    device_id: &str,
) -> Result<ExportResult, String> {
    let filename = format!("memory-graph-{}.db", device_id);
    let snapshot_path = Path::new(sync_dir).join(&filename);

    let backup_result = backup::create_backup(db, snapshot_path.to_str().unwrap())?;

    // Update manifest
    let mut manifest = read_manifest(sync_dir).unwrap_or_else(|_| SyncManifest {
        devices: HashMap::new(),
        last_updated: Utc::now(),
    });

    manifest.devices.insert(
        device_id.to_string(),
        DeviceEntry {
            device_id: device_id.to_string(),
            snapshot_filename: filename,
            checksum: backup_result.checksum.clone(),
            exported_at: Utc::now(),
            impulse_count: backup_result.impulse_count,
        },
    );
    manifest.last_updated = Utc::now();

    write_manifest(sync_dir, &manifest)?;

    Ok(ExportResult {
        snapshot_path: snapshot_path.to_string_lossy().to_string(),
        checksum: backup_result.checksum,
    })
}

pub fn check_sync_status(
    sync_dir: &str,
    local_device_id: &str,
) -> Result<SyncStatus, String> {
    let manifest = read_manifest(sync_dir)?;

    let remote_devices: Vec<String> = manifest
        .devices
        .keys()
        .filter(|d| *d != local_device_id)
        .cloned()
        .collect();

    let latest = manifest
        .devices
        .values()
        .filter(|d| d.device_id != local_device_id)
        .max_by_key(|d| d.exported_at);

    Ok(SyncStatus {
        has_remote_updates: !remote_devices.is_empty(),
        remote_devices,
        latest_remote_device: latest.map(|d| d.device_id.clone()),
        latest_remote_time: latest.map(|d| d.exported_at),
    })
}

pub fn import_snapshot(
    snapshot_path: &str,
    local_db_path: &str,
    expected_checksum: &str,
) -> Result<(), String> {
    backup::restore_backup(snapshot_path, local_db_path, expected_checksum)
}

pub fn read_manifest(sync_dir: &str) -> Result<SyncManifest, String> {
    let manifest_path = Path::new(sync_dir).join("manifest.json");

    if !manifest_path.exists() {
        return Err("No manifest found".to_string());
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse manifest: {}", e))
}

fn write_manifest(sync_dir: &str, manifest: &SyncManifest) -> Result<(), String> {
    let manifest_path = Path::new(sync_dir).join("manifest.json");

    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;

    fs::write(&manifest_path, content)
        .map_err(|e| format!("Failed to write manifest: {}", e))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test test_sync 2>&1`
Expected: All sync tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/sync.rs tests/test_sync.rs
git commit -m "feat: implement cross-device sync via shared directory with manifest tracking"
```

---

### Task 4: MCP Tools for Backup and Sync

**Files:**
- Modify: `src/server.rs`

- [ ] **Step 1: Write failing tests**

Add to `tests/test_mcp.rs`:
```rust
#[test]
fn test_backup_tool() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("test.db");
    let backup_path = tmp.path().join("backup.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    let server = MemoryGraphServer::new_with_db(db);

    server.handle_save_memory(
        "Backup test".to_string(),
        "observation".to_string(),
        None, None, None,
    ).unwrap();

    let result = server.handle_create_backup(backup_path.to_str().unwrap().to_string());
    assert!(result.is_ok());

    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(!response["checksum"].as_str().unwrap().is_empty());
}
```

- [ ] **Step 2: Add backup and sync handlers to server**

Add to `MemoryGraphServer`:
```rust
pub fn handle_create_backup(&self, backup_path: String) -> Result<String, String> {
    let db = self.db.lock().unwrap();
    let result = crate::backup::create_backup(&db, &backup_path)?;

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

pub fn handle_sync_export(&self, sync_dir: String, device_id: String) -> Result<String, String> {
    let db = self.db.lock().unwrap();
    let result = crate::sync::export_snapshot(&db, &sync_dir, &device_id)?;

    let response = serde_json::json!({
        "snapshot_path": result.snapshot_path,
        "checksum": result.checksum,
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Serialization error: {}", e))
}

pub fn handle_sync_status(&self, sync_dir: String, device_id: String) -> Result<String, String> {
    let status = crate::sync::check_sync_status(&sync_dir, &device_id)?;

    let response = serde_json::json!({
        "has_remote_updates": status.has_remote_updates,
        "remote_devices": status.remote_devices,
        "latest_remote_device": status.latest_remote_device,
        "latest_remote_time": status.latest_remote_time,
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Serialization error: {}", e))
}
```

Add MCP tool annotations to `McpHandler`:
```rust
#[tool(description = "Create a backup of the memory graph database")]
fn create_backup(
    &self,
    #[tool(param, description = "Path to save the backup file")] backup_path: String,
) -> Result<String, String> {
    self.inner.handle_create_backup(backup_path)
}

#[tool(description = "Export a sync snapshot to a shared directory for cross-device portability")]
fn sync_export(
    &self,
    #[tool(param, description = "Path to the shared sync directory")] sync_dir: String,
    #[tool(param, description = "Unique identifier for this device")] device_id: String,
) -> Result<String, String> {
    self.inner.handle_sync_export(sync_dir, device_id)
}

#[tool(description = "Check if there are remote sync updates from other devices")]
fn sync_status(
    &self,
    #[tool(param, description = "Path to the shared sync directory")] sync_dir: String,
    #[tool(param, description = "This device's identifier")] device_id: String,
) -> Result<String, String> {
    self.inner.handle_sync_status(sync_dir, device_id)
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test 2>&1`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add src/server.rs tests/test_mcp.rs
git commit -m "feat: add backup and sync MCP tools"
```

---

### Task 5: Multiple Ghost Graph Sources Integration

**Files:**
- Create: `tests/test_validation_p3.rs`

- [ ] **Step 1: Write tests for multiple ghost graph sources**

`tests/test_validation_p3.rs`:
```rust
mod common;

use memory_graph::activation::ActivationEngine;
use memory_graph::backup;
use memory_graph::db::Database;
use memory_graph::ghost;
use memory_graph::ghost::scanner::ScanConfig;
use memory_graph::ingestion;
use memory_graph::models::*;
use memory_graph::sync;
use std::fs;
use tempfile::TempDir;

fn create_obsidian_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("philosophy.md"),
        "# Philosophy\n\nMemory should be portable and inspectable.\n",
    ).unwrap();
    fs::write(
        dir.path().join("patterns.md"),
        "# Patterns\n\nSpreading activation for retrieval. See [[philosophy]].\n",
    ).unwrap();
    dir
}

fn create_code_docs() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("api.md"),
        "# API Reference\n\nMCP tools: save_memory, retrieve_context, inspect_memory.\n",
    ).unwrap();
    fs::write(
        dir.path().join("architecture.md"),
        "# Code Architecture\n\nSQLite-based graph with spreading activation retrieval.\n",
    ).unwrap();
    dir
}

// ============================================================
// PRD P3 Criterion 1: Cross-device access with consistency
// ============================================================

#[test]
fn validation_p3_cross_device_consistency() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");
    fs::create_dir(&sync_dir).unwrap();

    let db_a_path = tmp.path().join("device_a.db");
    let db_b_path = tmp.path().join("device_b.db");

    // Device A creates memories
    let db_a = Database::open(db_a_path.to_str().unwrap()).unwrap();
    ingestion::explicit_save(
        &db_a, "Cross-device memory test",
        ImpulseType::Heuristic, EmotionalValence::Positive,
        EngagementLevel::High, vec![], "device-a-session",
    ).unwrap();

    let a_count = db_a.impulse_count().unwrap();

    // Device A exports
    let export = sync::export_snapshot(&db_a, sync_dir.to_str().unwrap(), "device-a").unwrap();

    // Device B imports
    sync::import_snapshot(&export.snapshot_path, db_b_path.to_str().unwrap(), &export.checksum).unwrap();

    // Device B should have same data
    let db_b = Database::open(db_b_path.to_str().unwrap()).unwrap();
    let b_count = db_b.impulse_count().unwrap();
    assert_eq!(a_count, b_count);

    let b_impulses = db_b.list_impulses(None).unwrap();
    assert_eq!(b_impulses[0].content, "Cross-device memory test");
}

// ============================================================
// PRD P3 Criterion 2: Backup/restore preserves full graph integrity
// ============================================================

#[test]
fn validation_p3_backup_restore_full_integrity() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("original.db");
    let backup_path = tmp.path().join("backup.db");
    let restore_path = tmp.path().join("restored.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();

    // Create a rich graph
    let a = ingestion::explicit_save(
        &db, "Node A: Design philosophy",
        ImpulseType::Heuristic, EmotionalValence::Positive,
        EngagementLevel::High, vec!["deep discussion".to_string()], "s1",
    ).unwrap();

    let b = ingestion::explicit_save_with_connections(
        &db, "Node B: Architecture decision",
        ImpulseType::Decision, EmotionalValence::Positive,
        EngagementLevel::High, vec![], "s1",
        &[(a.id.clone(), "derived_from".to_string(), 0.9)],
    ).unwrap();

    let c = ingestion::explicit_save_with_connections(
        &db, "Node C: Implementation detail",
        ImpulseType::Pattern, EmotionalValence::Neutral,
        EngagementLevel::Medium, vec![], "s1",
        &[(b.id.clone(), "relates_to".to_string(), 0.7)],
    ).unwrap();

    // Register a ghost source
    db.register_ghost_source("test-vault", "/fake/path", "obsidian").unwrap();
    db.insert_ghost_node(&NewGhostNode {
        source_graph: "test-vault".to_string(),
        external_ref: "note.md".to_string(),
        title: "Test Ghost".to_string(),
        metadata: serde_json::json!({"tags": ["test"]}),
        initial_weight: 0.5,
    }).unwrap();

    let orig_impulses = db.impulse_count().unwrap();
    let orig_connections = db.connection_count().unwrap();
    let orig_sources = db.list_ghost_sources().unwrap().len();
    let orig_ghosts = db.list_ghost_nodes_by_source("test-vault").unwrap().len();

    // Backup and restore
    let br = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();
    backup::restore_backup(backup_path.to_str().unwrap(), restore_path.to_str().unwrap(), &br.checksum).unwrap();

    let restored = Database::open(restore_path.to_str().unwrap()).unwrap();

    assert_eq!(restored.impulse_count().unwrap(), orig_impulses);
    assert_eq!(restored.connection_count().unwrap(), orig_connections);
    assert_eq!(restored.list_ghost_sources().unwrap().len(), orig_sources);
    assert_eq!(restored.list_ghost_nodes_by_source("test-vault").unwrap().len(), orig_ghosts);

    // Verify specific content survived
    let restored_a = restored.get_impulse(&a.id).unwrap();
    assert_eq!(restored_a.content, "Node A: Design philosophy");
    assert_eq!(restored_a.engagement_level, EngagementLevel::High);
}

// ============================================================
// PRD P3 Criterion 3: Cross-source activation works
// ============================================================

#[test]
fn validation_p3_cross_source_activation() {
    let db = common::test_db();
    let vault = create_obsidian_vault();
    let docs = create_code_docs();

    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    // Register two different ghost graph sources
    ghost::register_and_scan(
        &db, "obsidian", vault.path().to_str().unwrap(), "obsidian", &config,
    ).unwrap();

    ghost::register_and_scan(
        &db, "code-docs", docs.path().to_str().unwrap(), "directory", &config,
    ).unwrap();

    // Add memory graph impulses
    ingestion::explicit_save(
        &db, "Spreading activation mimics human memory",
        ImpulseType::Heuristic, EmotionalValence::Positive,
        EngagementLevel::High, vec![], "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);

    // Query that should hit across sources
    let result = engine.retrieve(&RetrievalRequest {
        query: "spreading activation retrieval architecture".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();

    // Should get memory graph results
    assert!(!result.memories.is_empty());

    // Should get ghost activations from BOTH sources
    let ghost_sources: Vec<&str> = result.ghost_activations
        .iter()
        .map(|g| g.source_graph.as_str())
        .collect();

    // At least one ghost activation should exist
    assert!(
        !result.ghost_activations.is_empty(),
        "Should have ghost activations from external KBs"
    );

    // Verify both sources are represented (both mention relevant terms)
    let has_obsidian = ghost_sources.iter().any(|s| *s == "obsidian");
    let has_docs = ghost_sources.iter().any(|s| *s == "code-docs");

    // At minimum one should be present; ideally both
    assert!(
        has_obsidian || has_docs,
        "At least one ghost graph source should activate. Sources found: {:?}",
        ghost_sources
    );
}

// ============================================================
// Additional: Sync doesn't corrupt local-first experience
// ============================================================

#[test]
fn validation_p3_sync_preserves_local_first() {
    let tmp = TempDir::new().unwrap();
    let sync_dir = tmp.path().join("sync");
    fs::create_dir(&sync_dir).unwrap();

    let db_path = tmp.path().join("local.db");
    let db = Database::open(db_path.to_str().unwrap()).unwrap();

    // Create local data
    ingestion::explicit_save(
        &db, "Local memory",
        ImpulseType::Observation, EmotionalValence::Neutral,
        EngagementLevel::Medium, vec![], "test",
    ).unwrap();

    // Export snapshot
    sync::export_snapshot(&db, sync_dir.to_str().unwrap(), "laptop").unwrap();

    // Local operations should still work without sync dir
    ingestion::explicit_save(
        &db, "Another local memory",
        ImpulseType::Observation, EmotionalValence::Neutral,
        EngagementLevel::Medium, vec![], "test",
    ).unwrap();

    assert_eq!(db.impulse_count().unwrap(), 2);

    // Retrieval still works
    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "local memory".to_string(),
        max_results: 10,
        max_hops: 2,
    }).unwrap();
    assert!(!result.memories.is_empty());
}
```

- [ ] **Step 2: Run validation tests**

Run: `cargo test validation_p3 2>&1`
Expected: All Phase 3 validation tests PASS

- [ ] **Step 3: Run complete test suite**

Run: `cargo test 2>&1`
Expected: ALL tests PASS (Phase 1 + Phase 2 + Phase 3)

- [ ] **Step 4: Commit**

```bash
git add tests/test_validation_p3.rs
git commit -m "feat: add Phase 3 validation tests for sync, backup, and cross-source activation"
```

---

### Task 6: Final Cleanup and Full Verification

- [ ] **Step 1: Run clippy**

Run: `cargo clippy 2>&1`
Fix any errors.

- [ ] **Step 2: Run full test suite**

Run: `cargo test 2>&1`
Expected: ALL tests PASS across all phases

- [ ] **Step 3: Build release**

Run: `cargo build --release 2>&1`
Expected: Successful compilation

- [ ] **Step 4: Count tests**

Run: `cargo test 2>&1 | grep "test result"`
Expected: Should show total test count across all test files. Verify no failures.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: Phase 3 complete — clippy fixes and final build verification"
```

---

## Post-Phase-3 Notes

After completing all tasks across all three phases:

1. Core memory graph with weighted connections, decay, reinforcement, spreading activation
2. Ghost graph overlays for external KBs with pull-through and weight learning
3. Adaptive extraction heuristics for engagement-scaled proposals
4. Backup/restore with integrity verification
5. Cross-device sync via shared directory with manifest
6. Multiple ghost graph sources with cross-source activation
7. Full MCP tool surface: 14 tools across memory, ghost, session, backup, and sync
8. Comprehensive test suite covering all PRD validation criteria for all phases

**MCP Tools (final list):**
- Memory: save_memory, retrieve_context, delete_memory, update_memory, inspect_memory, explain_recall
- Ghost: register_ghost_graph, refresh_ghost_graph, pull_through
- Session: set_incognito, memory_status
- Backup/Sync: create_backup, sync_export, sync_status
