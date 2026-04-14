// SQLite schema and database operations

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection as SqliteConnection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::*;

pub struct Database {
    conn: SqliteConnection,
}

impl Database {
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = SqliteConnection::open(path)?;
        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = SqliteConnection::open_in_memory()?;
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
                status TEXT NOT NULL DEFAULT 'candidate'
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
                "candidate",
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

    pub fn confirm_impulse(&self, id: &str) -> SqlResult<()> {
        self.update_impulse_status(id, ImpulseStatus::Confirmed)
    }

    pub fn dismiss_impulse(&self, id: &str) -> SqlResult<()> {
        self.update_impulse_status(id, ImpulseStatus::Deleted)
    }

    pub fn list_candidates(&self) -> SqlResult<Vec<Impulse>> {
        self.list_impulses(Some(ImpulseStatus::Candidate))
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
             AND i.status = 'confirmed'
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
