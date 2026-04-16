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
        self.run_migrations()?;
        Ok(())
    }

    fn run_migrations(&self) -> SqlResult<()> {
        // Add source_provider and source_account columns if they don't exist
        let has_provider: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info('impulses') WHERE name='source_provider'",
            [],
            |row| row.get::<_, i64>(0),
        ).map(|c| c > 0).unwrap_or(false);

        if !has_provider {
            self.conn.execute_batch(
                "ALTER TABLE impulses ADD COLUMN source_provider TEXT NOT NULL DEFAULT 'unknown';
                 ALTER TABLE impulses ADD COLUMN source_account TEXT NOT NULL DEFAULT '';"
            )?;
        }
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

            CREATE TABLE IF NOT EXISTS ghost_nodes (
                id TEXT PRIMARY KEY,
                source_graph TEXT NOT NULL,
                external_ref TEXT NOT NULL,
                title TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                weight REAL NOT NULL,
                last_accessed_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(source_graph, external_ref)
            );

            CREATE INDEX IF NOT EXISTS idx_ghost_nodes_source_graph ON ghost_nodes(source_graph);

            CREATE TABLE IF NOT EXISTS ghost_connections (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                weight REAL NOT NULL,
                relationship TEXT NOT NULL DEFAULT 'relates_to',
                created_at TEXT NOT NULL,
                last_traversed_at TEXT NOT NULL,
                traversal_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (source_id) REFERENCES ghost_nodes(id),
                FOREIGN KEY (target_id) REFERENCES ghost_nodes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_ghost_connections_source_target ON ghost_connections(source_id, target_id);

            CREATE TABLE IF NOT EXISTS ghost_sources (
                name TEXT PRIMARY KEY,
                root_path TEXT NOT NULL,
                source_type TEXT NOT NULL,
                registered_at TEXT NOT NULL,
                last_scanned_at TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS ghost_nodes_fts USING fts5(
                title,
                content_rowid='rowid',
                tokenize='porter'
            );

            CREATE TABLE IF NOT EXISTS tags (
                name TEXT PRIMARY KEY,
                color TEXT NOT NULL DEFAULT '#8E99A4',
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS impulse_tags (
                impulse_id TEXT NOT NULL,
                tag_name TEXT NOT NULL,
                PRIMARY KEY (impulse_id, tag_name),
                FOREIGN KEY (impulse_id) REFERENCES impulses(id),
                FOREIGN KEY (tag_name) REFERENCES tags(name)
            );

            CREATE INDEX IF NOT EXISTS idx_impulse_tags_impulse ON impulse_tags(impulse_id);
            CREATE INDEX IF NOT EXISTS idx_impulse_tags_tag ON impulse_tags(tag_name);
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
             source_type, source_ref, status, source_provider, source_account)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                input.source_provider,
                input.source_account,
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

    pub fn insert_impulse_with_id(&self, id: &str, input: &NewImpulse) -> SqlResult<Impulse> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let signals_json = serde_json::to_string(&input.source_signals).unwrap_or_default();

        self.conn.execute(
            "INSERT INTO impulses (id, content, impulse_type, weight, initial_weight,
             emotional_valence, engagement_level, source_signals, created_at, last_accessed_at,
             source_type, source_ref, status, source_provider, source_account)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                input.source_provider,
                input.source_account,
            ],
        )?;

        // Insert into FTS index
        self.conn.execute(
            "INSERT INTO impulses_fts (rowid, content)
             SELECT rowid, content FROM impulses WHERE id = ?1",
            params![id],
        )?;

        self.get_impulse(id)
    }

    pub fn get_impulse(&self, id: &str) -> SqlResult<Impulse> {
        self.conn.query_row(
            "SELECT id, content, impulse_type, weight, initial_weight,
             emotional_valence, engagement_level, source_signals, created_at,
             last_accessed_at, source_type, source_ref, status,
             source_provider, source_account
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
            source_provider: old.source_provider,
            source_account: old.source_account,
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
                     last_accessed_at, source_type, source_ref, status,
                     source_provider, source_account
                     FROM impulses WHERE status = ?1 ORDER BY weight DESC",
                )?;
                let rows = stmt.query_map(params![s.as_str()], |row| Ok(row_to_impulse(row)))?;
                rows.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, content, impulse_type, weight, initial_weight,
                     emotional_valence, engagement_level, source_signals, created_at,
                     last_accessed_at, source_type, source_ref, status,
                     source_provider, source_account
                     FROM impulses ORDER BY weight DESC",
                )?;
                let rows = stmt.query_map([], |row| Ok(row_to_impulse(row)))?;
                rows.collect()
            }
        }
    }

    pub fn search_impulses_fts(&self, query: &str) -> SqlResult<Vec<(String, f64)>> {
        // Convert multi-word queries to OR terms for broader matching
        // FTS5 default is AND which is too strict for natural language queries
        let fts_query = sanitize_fts_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }

        let mut stmt = self.conn.prepare(
            "SELECT i.id, fts.rank
             FROM impulses_fts fts
             JOIN impulses i ON i.rowid = fts.rowid
             WHERE impulses_fts MATCH ?1
             AND i.status = 'confirmed'
             ORDER BY fts.rank",
        )?;
        let rows = stmt.query_map(params![fts_query], |row| {
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

    pub fn delete_connection(&self, id: &str) -> SqlResult<()> {
        self.conn.execute("DELETE FROM connections WHERE id = ?1", params![id])?;
        Ok(())
    }

    // === Ghost Node Operations ===

    pub fn insert_ghost_node(&self, input: &NewGhostNode) -> SqlResult<GhostNode> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let metadata_str = serde_json::to_string(&input.metadata).unwrap_or_else(|_| "{}".to_string());

        self.conn.execute(
            "INSERT INTO ghost_nodes (id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                input.source_graph,
                input.external_ref,
                input.title,
                metadata_str,
                input.initial_weight,
                now_str,
                now_str,
            ],
        )?;

        // Insert into FTS index
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
            |row| Ok(row_to_ghost_node(row)),
        )
    }

    pub fn get_ghost_node_by_ref(&self, source_graph: &str, external_ref: &str) -> SqlResult<GhostNode> {
        self.conn.query_row(
            "SELECT id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at
             FROM ghost_nodes WHERE source_graph = ?1 AND external_ref = ?2",
            params![source_graph, external_ref],
            |row| Ok(row_to_ghost_node(row)),
        )
    }

    pub fn list_ghost_nodes_by_source(&self, source_graph: &str) -> SqlResult<Vec<GhostNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_graph, external_ref, title, metadata, weight, last_accessed_at, created_at
             FROM ghost_nodes WHERE source_graph = ?1 ORDER BY weight DESC",
        )?;
        let rows = stmt.query_map(params![source_graph], |row| Ok(row_to_ghost_node(row)))?;
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
        // Delete FTS entries first
        self.conn.execute(
            "DELETE FROM ghost_nodes_fts WHERE rowid IN (SELECT rowid FROM ghost_nodes WHERE source_graph = ?1)",
            params![source_graph],
        )?;
        // Delete connections involving these nodes
        self.conn.execute(
            "DELETE FROM ghost_connections WHERE source_id IN (SELECT id FROM ghost_nodes WHERE source_graph = ?1)
             OR target_id IN (SELECT id FROM ghost_nodes WHERE source_graph = ?1)",
            params![source_graph],
        )?;
        let count = self.conn.execute(
            "DELETE FROM ghost_nodes WHERE source_graph = ?1",
            params![source_graph],
        )?;
        Ok(count)
    }

    pub fn search_ghost_nodes_fts(&self, query: &str) -> SqlResult<Vec<(String, f64)>> {
        let fts_query = sanitize_fts_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }
        let mut stmt = self.conn.prepare(
            "SELECT gn.id, fts.rank
             FROM ghost_nodes_fts fts
             JOIN ghost_nodes gn ON gn.rowid = fts.rowid
             WHERE ghost_nodes_fts MATCH ?1
             ORDER BY fts.rank",
        )?;
        let rows = stmt.query_map(params![fts_query], |row| {
            let id: String = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok((id, rank))
        })?;
        rows.collect()
    }

    // === Ghost Connection Operations ===

    pub fn insert_ghost_connection(&self, input: &NewGhostConnection) -> SqlResult<Connection> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO ghost_connections (id, source_id, target_id, weight, relationship, created_at, last_traversed_at, traversal_count)
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

        self.get_ghost_connection(&id)
    }

    fn get_ghost_connection(&self, id: &str) -> SqlResult<Connection> {
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
             FROM ghost_connections
             WHERE source_id = ?1 OR target_id = ?1",
        )?;
        let rows = stmt.query_map(params![node_id], |row| Ok(row_to_connection(row)))?;
        rows.collect()
    }

    // === Ghost Source Operations ===

    pub fn register_ghost_source(&self, name: &str, root_path: &str, source_type: &str) -> SqlResult<GhostSource> {
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT OR REPLACE INTO ghost_sources (name, root_path, source_type, registered_at, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, NULL)",
            params![name, root_path, source_type, now],
        )?;

        self.get_ghost_source(name)
    }

    fn get_ghost_source(&self, name: &str) -> SqlResult<GhostSource> {
        self.conn.query_row(
            "SELECT gs.name, gs.root_path, gs.source_type, gs.registered_at, gs.last_scanned_at,
             (SELECT COUNT(*) FROM ghost_nodes WHERE source_graph = gs.name) as node_count
             FROM ghost_sources gs WHERE gs.name = ?1",
            params![name],
            |row| Ok(row_to_ghost_source(row)),
        )
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
        let rows = stmt.query_map([], |row| Ok(row_to_ghost_source(row)))?;
        rows.collect()
    }

    // === Tag Operations ===

    pub fn create_tag(&self, tag: &NewTag) -> SqlResult<Tag> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO tags (name, color, created_at) VALUES (?1, ?2, ?3)",
            params![tag.name, tag.color, now_str],
        )?;

        self.get_tag(&tag.name)
    }

    pub fn get_tag(&self, name: &str) -> SqlResult<Tag> {
        self.conn.query_row(
            "SELECT name, color, created_at FROM tags WHERE name = ?1",
            params![name],
            |row| {
                let created_str: String = row.get(2)?;
                Ok(Tag {
                    name: row.get(0)?,
                    color: row.get(1)?,
                    created_at: DateTime::parse_from_rfc3339(&created_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            },
        )
    }

    pub fn list_tags(&self) -> SqlResult<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, color, created_at FROM tags ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            let created_str: String = row.get(2)?;
            Ok(Tag {
                name: row.get(0)?,
                color: row.get(1)?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect()
    }

    pub fn delete_tag(&self, name: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM impulse_tags WHERE tag_name = ?1",
            params![name],
        )?;
        self.conn.execute(
            "DELETE FROM tags WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    pub fn tag_impulse(&self, impulse_id: &str, tag_name: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO impulse_tags (impulse_id, tag_name) VALUES (?1, ?2)",
            params![impulse_id, tag_name],
        )?;
        Ok(())
    }

    pub fn untag_impulse(&self, impulse_id: &str, tag_name: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM impulse_tags WHERE impulse_id = ?1 AND tag_name = ?2",
            params![impulse_id, tag_name],
        )?;
        Ok(())
    }

    pub fn get_tags_for_impulse(&self, impulse_id: &str) -> SqlResult<Vec<Tag>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, t.color, t.created_at
             FROM tags t
             JOIN impulse_tags it ON it.tag_name = t.name
             WHERE it.impulse_id = ?1
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![impulse_id], |row| {
            let created_str: String = row.get(2)?;
            Ok(Tag {
                name: row.get(0)?,
                color: row.get(1)?,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;
        rows.collect()
    }

    pub fn get_impulses_for_tag(&self, tag_name: &str) -> SqlResult<Vec<Impulse>> {
        let mut stmt = self.conn.prepare(
            "SELECT i.id, i.content, i.impulse_type, i.weight, i.initial_weight,
             i.emotional_valence, i.engagement_level, i.source_signals, i.created_at,
             i.last_accessed_at, i.source_type, i.source_ref, i.status,
             i.source_provider, i.source_account
             FROM impulses i
             JOIN impulse_tags it ON it.impulse_id = i.id
             WHERE it.tag_name = ?1
             ORDER BY i.weight DESC",
        )?;
        let rows = stmt.query_map(params![tag_name], |row| Ok(row_to_impulse(row)))?;
        rows.collect()
    }

    // === Backup ===

    pub fn vacuum_into(&self, path: &str) -> SqlResult<()> {
        self.conn.execute_batch(&format!("VACUUM INTO '{}'", path.replace('\'', "''")))?;
        Ok(())
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
        source_provider: row.get(13).unwrap_or_else(|_| "unknown".to_string()),
        source_account: row.get(14).unwrap_or_else(|_| String::new()),
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

fn row_to_ghost_node(row: &rusqlite::Row) -> GhostNode {
    let metadata_str: String = row.get(4).unwrap_or_else(|_| "{}".to_string());
    let created_str: String = row.get(7).unwrap_or_default();
    let accessed_str: String = row.get(6).unwrap_or_default();

    GhostNode {
        id: row.get(0).unwrap_or_default(),
        source_graph: row.get(1).unwrap_or_default(),
        external_ref: row.get(2).unwrap_or_default(),
        title: row.get(3).unwrap_or_default(),
        metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::Value::Object(Default::default())),
        weight: row.get(5).unwrap_or(0.0),
        last_accessed_at: DateTime::parse_from_rfc3339(&accessed_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    }
}

fn row_to_ghost_source(row: &rusqlite::Row) -> GhostSource {
    let registered_str: String = row.get(3).unwrap_or_default();
    let scanned_str: Option<String> = row.get(4).unwrap_or(None);

    GhostSource {
        name: row.get(0).unwrap_or_default(),
        root_path: row.get(1).unwrap_or_default(),
        source_type: row.get(2).unwrap_or_default(),
        registered_at: DateTime::parse_from_rfc3339(&registered_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        last_scanned_at: scanned_str.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .map(|dt| dt.with_timezone(&Utc))
                .ok()
        }),
        node_count: row.get(5).unwrap_or(0),
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

// === FTS Query Helpers ===

const FTS_STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
    "have", "has", "had", "do", "does", "did", "will", "would", "could",
    "should", "may", "might", "can", "to", "of", "in", "for", "on", "with",
    "at", "by", "from", "as", "into", "through", "during", "before", "after",
    "and", "or", "but", "not", "no", "so", "if", "then", "than", "too",
    "very", "just", "about", "up", "out", "how", "what", "when", "where",
    "who", "which", "that", "this", "it", "i", "me", "my", "you", "your",
    "we", "our", "they", "them", "their", "he", "she", "his", "her",
    "want", "think", "know", "like", "get", "make", "going",
];

/// Convert a natural language query into an FTS5-compatible OR query.
/// Strips stop words, keeps significant terms, joins with OR.
fn sanitize_fts_query(query: &str) -> String {
    let stop: std::collections::HashSet<&str> = FTS_STOP_WORDS.iter().copied().collect();

    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 3 && !stop.contains(w.as_str()))
        .collect();

    if terms.is_empty() {
        return String::new();
    }

    // Join with OR for broad matching
    terms.join(" OR ")
}
