// Cross-device sync for synaptic-graph

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::backup;
use crate::db::Database;
use crate::models::*;

// === Structs ===

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

#[derive(Debug, Clone)]
pub struct SyncMergeResult {
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
}

// === Public Functions ===

/// Export a snapshot of the local database into the sync directory, updating the manifest.
pub fn export_snapshot(
    db: &Database,
    sync_dir: &str,
    device_id: &str,
) -> Result<ExportResult, String> {
    // Ensure sync dir exists
    fs::create_dir_all(sync_dir)
        .map_err(|e| format!("Failed to create sync directory: {}", e))?;

    let snapshot_filename = format!("memory-graph-{}.db", device_id);
    let snapshot_path = Path::new(sync_dir)
        .join(&snapshot_filename)
        .to_string_lossy()
        .to_string();

    // Create backup into the sync directory
    let backup_result = backup::create_backup(db, &snapshot_path)?;

    // Update the manifest
    let mut manifest = read_manifest(sync_dir).unwrap_or_else(|_| SyncManifest {
        devices: HashMap::new(),
        last_updated: Utc::now(),
    });

    let entry = DeviceEntry {
        device_id: device_id.to_string(),
        snapshot_filename: snapshot_filename.clone(),
        checksum: backup_result.checksum.clone(),
        exported_at: Utc::now(),
        impulse_count: backup_result.impulse_count,
    };

    manifest.devices.insert(device_id.to_string(), entry);
    manifest.last_updated = Utc::now();

    write_manifest(sync_dir, &manifest)?;

    Ok(ExportResult {
        snapshot_path,
        checksum: backup_result.checksum,
    })
}

/// Check the sync directory for remote device updates.
pub fn check_sync_status(
    sync_dir: &str,
    local_device_id: &str,
) -> Result<SyncStatus, String> {
    let manifest = read_manifest(sync_dir)?;

    let remote_devices: Vec<String> = manifest
        .devices
        .keys()
        .filter(|id| id.as_str() != local_device_id)
        .cloned()
        .collect();

    // Find the latest remote device by exported_at
    let mut latest_device: Option<String> = None;
    let mut latest_time: Option<DateTime<Utc>> = None;

    for (id, entry) in &manifest.devices {
        if id == local_device_id {
            continue;
        }
        match latest_time {
            None => {
                latest_device = Some(id.clone());
                latest_time = Some(entry.exported_at);
            }
            Some(t) if entry.exported_at > t => {
                latest_device = Some(id.clone());
                latest_time = Some(entry.exported_at);
            }
            _ => {}
        }
    }

    // Determine if remote updates exist: any remote device exported after the local device
    let local_exported_at = manifest
        .devices
        .get(local_device_id)
        .map(|e| e.exported_at);

    let has_remote_updates = match (local_exported_at, latest_time) {
        (Some(local_t), Some(remote_t)) => remote_t > local_t,
        (None, Some(_)) => true,
        _ => false,
    };

    Ok(SyncStatus {
        has_remote_updates,
        remote_devices,
        latest_remote_device: latest_device,
        latest_remote_time: latest_time,
    })
}

/// Import a remote snapshot into the local database using ID-based merge (not overwrite).
/// For each remote impulse:
///   - If it exists locally, use newer-wins on last_accessed_at
///   - If it's new, insert it preserving the original ID
pub fn import_snapshot(
    snapshot_path: &str,
    local_db_path: &str,
    expected_checksum: &str,
) -> Result<SyncMergeResult, String> {
    // Verify snapshot integrity
    if !backup::verify_backup(snapshot_path, expected_checksum)? {
        return Err("Snapshot integrity check failed: checksum mismatch".to_string());
    }

    // Open both databases
    let remote_db = Database::open(snapshot_path)
        .map_err(|e| format!("Failed to open remote snapshot: {}", e))?;
    let local_db = Database::open(local_db_path)
        .map_err(|e| format!("Failed to open local database: {}", e))?;

    // Get all impulses from remote
    let remote_impulses = remote_db
        .list_impulses(None)
        .map_err(|e| format!("Failed to list remote impulses: {}", e))?;

    let mut inserted: usize = 0;
    let mut updated: usize = 0;
    let mut skipped: usize = 0;

    for remote_impulse in &remote_impulses {
        match local_db.get_impulse(&remote_impulse.id) {
            Ok(local_impulse) => {
                // Exists locally -- newer-wins on last_accessed_at
                if remote_impulse.last_accessed_at > local_impulse.last_accessed_at {
                    // Update local with remote's data (touch + update weight)
                    local_db
                        .touch_impulse(&remote_impulse.id)
                        .map_err(|e| format!("Failed to touch impulse: {}", e))?;
                    local_db
                        .update_impulse_weight(&remote_impulse.id, remote_impulse.weight)
                        .map_err(|e| format!("Failed to update impulse weight: {}", e))?;
                    updated += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(_) => {
                // New impulse -- insert preserving original ID
                let new_input = NewImpulse {
                    content: remote_impulse.content.clone(),
                    impulse_type: remote_impulse.impulse_type,
                    initial_weight: remote_impulse.initial_weight,
                    emotional_valence: remote_impulse.emotional_valence,
                    engagement_level: remote_impulse.engagement_level,
                    source_signals: remote_impulse.source_signals.clone(),
                    source_type: remote_impulse.source_type,
                    source_ref: remote_impulse.source_ref.clone(),
                };
                local_db
                    .insert_impulse_with_id(&remote_impulse.id, &new_input)
                    .map_err(|e| format!("Failed to insert impulse with id: {}", e))?;
                inserted += 1;
            }
        }
    }

    Ok(SyncMergeResult {
        inserted,
        updated,
        skipped,
    })
}

/// Read the sync manifest from the sync directory.
pub fn read_manifest(sync_dir: &str) -> Result<SyncManifest, String> {
    let manifest_path = Path::new(sync_dir).join("manifest.json");
    let data = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: SyncManifest =
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse manifest: {}", e))?;
    Ok(manifest)
}

// === Private Functions ===

fn write_manifest(sync_dir: &str, manifest: &SyncManifest) -> Result<(), String> {
    let manifest_path = Path::new(sync_dir).join("manifest.json");
    let data = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&manifest_path, data)
        .map_err(|e| format!("Failed to write manifest: {}", e))?;
    Ok(())
}
