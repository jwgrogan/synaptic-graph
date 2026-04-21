// Backup and restore operations for synaptic-graph

use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};

use crate::db::Database;

#[derive(Debug, Clone)]
pub struct BackupResult {
    pub path: String,
    pub checksum: String,
    pub impulse_count: i64,
    pub connection_count: i64,
    pub size_bytes: u64,
}

/// Create a consistent backup of the database using SQLite VACUUM INTO.
/// Returns a BackupResult with path, checksum, and stats.
pub fn create_backup(db: &Database, backup_path: &str) -> Result<BackupResult, String> {
    // Get stats before backup
    let impulse_count = db
        .impulse_count()
        .map_err(|e| format!("Failed to count impulses: {}", e))?;
    let connection_count = db
        .connection_count()
        .map_err(|e| format!("Failed to count connections: {}", e))?;

    // Create consistent snapshot via VACUUM INTO
    db.vacuum_into(backup_path)
        .map_err(|e| format!("VACUUM INTO failed: {}", e))?;

    // Compute checksum
    let checksum =
        checksum_file(backup_path).map_err(|e| format!("Failed to compute checksum: {}", e))?;

    // Get file size
    let metadata = fs::metadata(backup_path)
        .map_err(|e| format!("Failed to read backup file metadata: {}", e))?;

    Ok(BackupResult {
        path: backup_path.to_string(),
        checksum,
        impulse_count,
        connection_count,
        size_bytes: metadata.len(),
    })
}

/// Verify a backup file's integrity by comparing its SHA-256 checksum.
pub fn verify_backup(backup_path: &str, expected_checksum: &str) -> Result<bool, String> {
    let actual =
        checksum_file(backup_path).map_err(|e| format!("Failed to compute checksum: {}", e))?;
    Ok(actual == expected_checksum)
}

/// Restore a backup to a new location after verifying integrity.
/// Opens the restored database to confirm it is valid.
pub fn restore_backup(
    backup_path: &str,
    restore_path: &str,
    expected_checksum: &str,
) -> Result<(), String> {
    // Verify integrity first
    if !verify_backup(backup_path, expected_checksum)? {
        return Err("Backup integrity check failed: checksum mismatch".to_string());
    }

    // Copy file to restore location
    fs::copy(backup_path, restore_path)
        .map_err(|e| format!("Failed to copy backup to restore path: {}", e))?;

    // Open the restored DB to verify it's valid
    Database::open(restore_path)
        .map_err(|e| format!("Restored database failed validation: {}", e))?;

    Ok(())
}

/// Compute SHA-256 checksum of a file, returned as a lowercase hex string.
pub fn checksum_file(path: &str) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}
