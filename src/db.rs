// SQLite schema and database operations

use chrono::{DateTime, Utc};
use rusqlite::{
    params, Connection as SqliteConnection, OpenFlags, OptionalExtension, Result as SqlResult,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::confidence::{effective_confidence, posterior_confidence};
use crate::graph::{
    current_feature_flags, CanonicalEdge, CanonicalNode, GhostPayload, GraphNodeKind,
    MemoryPayload, SchemaInfo, CURRENT_SCHEMA_VERSION,
};
use crate::models::*;

pub struct Database {
    conn: SqliteConnection,
}

const LEGACY_SCHEMA_VERSION: i64 = 1;

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
        let _ = self.purge_expired_evidence_sets()?;
        Ok(())
    }

    fn run_migrations(&self) -> SqlResult<()> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;

        let result = (|| {
            self.ensure_schema_version_row()?;
            self.migrate_legacy_provider_columns()?;

            let version = self.read_schema_version()?;
            if version > CURRENT_SCHEMA_VERSION {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISMATCH),
                    Some(format!(
                        "Database schema version {} is newer than supported version {}",
                        version, CURRENT_SCHEMA_VERSION
                    )),
                ));
            }

            self.create_canonical_graph_tables()?;

            if version < CURRENT_SCHEMA_VERSION {
                self.backfill_canonical_graph()?;
                self.write_schema_version(CURRENT_SCHEMA_VERSION)?;
            }

            Ok(())
        })();

        if let Err(err) = result {
            let _ = self.conn.execute_batch("ROLLBACK");
            return Err(err);
        }

        self.conn.execute_batch("COMMIT")?;
        Ok(())
    }

    fn detect_schema_version(conn: &SqliteConnection) -> SqlResult<i64> {
        let has_schema_table: bool = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_version'",
            [],
            |row| row.get::<_, i64>(0),
        )? > 0;

        if has_schema_table {
            let maybe_version =
                conn.query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                    row.get::<_, i64>(0)
                });
            if let Ok(version) = maybe_version {
                return Ok(version);
            }
        }

        let has_nodes: bool = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'nodes'",
            [],
            |row| row.get::<_, i64>(0),
        )? > 0;

        if has_nodes {
            Ok(CURRENT_SCHEMA_VERSION)
        } else {
            Ok(LEGACY_SCHEMA_VERSION)
        }
    }

    fn ensure_schema_version_row(&self) -> SqlResult<()> {
        let row_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM schema_version", [], |row| row.get(0))?;

        if row_count == 0 {
            let detected = Self::detect_schema_version(&self.conn)?;
            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                params![detected],
            )?;
        }

        Ok(())
    }

    fn read_schema_version(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
    }

    fn write_schema_version(&self, version: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM schema_version", [])?;
        self.conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![version],
        )?;
        Ok(())
    }

    fn migrate_legacy_provider_columns(&self) -> SqlResult<()> {
        let has_provider: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('impulses') WHERE name='source_provider'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_provider {
            self.conn.execute_batch(
                "ALTER TABLE impulses ADD COLUMN source_provider TEXT NOT NULL DEFAULT 'unknown';
                 ALTER TABLE impulses ADD COLUMN source_account TEXT NOT NULL DEFAULT '';",
            )?;
        }

        Ok(())
    }

    fn create_canonical_graph_tables(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS nodes (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                weight REAL NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                helpful_count INTEGER NOT NULL DEFAULT 0,
                unhelpful_count INTEGER NOT NULL DEFAULT 0,
                initial_weight REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_accessed_at TEXT NOT NULL,
                source_provider TEXT NOT NULL DEFAULT 'unknown',
                source_account TEXT NOT NULL DEFAULT ''
            );

            CREATE INDEX IF NOT EXISTS idx_nodes_kind_status ON nodes(kind, status);

            CREATE TABLE IF NOT EXISTS edges (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship TEXT NOT NULL,
                weight REAL NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                traversal_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_traversed_at TEXT NOT NULL,
                FOREIGN KEY (source_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (target_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);

            CREATE TABLE IF NOT EXISTS node_payload_memory (
                node_id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                impulse_type TEXT NOT NULL,
                emotional_valence TEXT NOT NULL,
                engagement_level TEXT NOT NULL,
                source_signals TEXT NOT NULL DEFAULT '[]',
                source_type TEXT NOT NULL,
                source_ref TEXT NOT NULL DEFAULT '',
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
                content,
                content_rowid='rowid',
                tokenize='porter'
            );

            CREATE TABLE IF NOT EXISTS node_payload_skill (
                node_id TEXT PRIMARY KEY,
                trigger TEXT NOT NULL DEFAULT '',
                steps_json TEXT NOT NULL DEFAULT '[]',
                constraints_json TEXT NOT NULL DEFAULT '[]',
                metadata TEXT NOT NULL DEFAULT '{}',
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS skill_fts USING fts5(
                trigger,
                content_rowid='rowid',
                tokenize='porter'
            );

            CREATE TABLE IF NOT EXISTS node_payload_ghost (
                node_id TEXT PRIMARY KEY,
                source_graph TEXT NOT NULL,
                external_ref TEXT NOT NULL,
                title TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                UNIQUE(source_graph, external_ref),
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS assessments (
                id TEXT PRIMARY KEY,
                subject_node_id TEXT NOT NULL,
                object_node_id TEXT,
                assessment_type TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'candidate',
                confidence REAL NOT NULL DEFAULT 0.5,
                rationale TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                dismissed_at TEXT,
                FOREIGN KEY (subject_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (object_node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS evidence_sets (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                response_hash TEXT NOT NULL DEFAULT '',
                node_ids TEXT NOT NULL DEFAULT '[]',
                edge_ids TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                expires_at TEXT
            );

            CREATE TABLE IF NOT EXISTS feedback_events (
                id TEXT PRIMARY KEY,
                evidence_set_id TEXT NOT NULL,
                target_node_id TEXT,
                target_edge_id TEXT,
                feedback_kind TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(evidence_set_id, idempotency_key),
                FOREIGN KEY (evidence_set_id) REFERENCES evidence_sets(id) ON DELETE CASCADE,
                FOREIGN KEY (target_node_id) REFERENCES nodes(id) ON DELETE CASCADE,
                FOREIGN KEY (target_edge_id) REFERENCES edges(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS node_versions (
                id TEXT PRIMARY KEY,
                node_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                snapshot_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(node_id, version),
                FOREIGN KEY (node_id) REFERENCES nodes(id) ON DELETE CASCADE
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_assessments_unique_pair
                ON assessments(assessment_type, subject_node_id, object_node_id);
            CREATE INDEX IF NOT EXISTS idx_assessments_status
                ON assessments(status, assessment_type);
            CREATE INDEX IF NOT EXISTS idx_assessments_subject
                ON assessments(subject_node_id);
            CREATE INDEX IF NOT EXISTS idx_assessments_object
                ON assessments(object_node_id);
            CREATE INDEX IF NOT EXISTS idx_assessments_contradiction_lookup
                ON assessments(assessment_type, status, subject_node_id, object_node_id);
            CREATE INDEX IF NOT EXISTS idx_evidence_sets_expires_at
                ON evidence_sets(expires_at);
            ",
        )?;
        Ok(())
    }

    fn backfill_canonical_graph(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            INSERT OR IGNORE INTO nodes (
                id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                initial_weight, created_at, updated_at, last_accessed_at,
                source_provider, source_account
            )
            SELECT
                id,
                'memory',
                status,
                weight,
                0.5,
                0,
                0,
                initial_weight,
                created_at,
                last_accessed_at,
                last_accessed_at,
                source_provider,
                source_account
            FROM impulses;

            INSERT OR IGNORE INTO node_payload_memory (
                node_id, content, impulse_type, emotional_valence,
                engagement_level, source_signals, source_type, source_ref
            )
            SELECT
                id, content, impulse_type, emotional_valence,
                engagement_level, source_signals, source_type, source_ref
            FROM impulses;

            INSERT OR IGNORE INTO nodes (
                id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                initial_weight, created_at, updated_at, last_accessed_at,
                source_provider, source_account
            )
            SELECT
                id,
                'ghost',
                'confirmed',
                weight,
                0.5,
                0,
                0,
                weight,
                created_at,
                last_accessed_at,
                last_accessed_at,
                'ghost',
                source_graph
            FROM ghost_nodes;

            INSERT OR IGNORE INTO node_payload_ghost (
                node_id, source_graph, external_ref, title, metadata
            )
            SELECT id, source_graph, external_ref, title, metadata
            FROM ghost_nodes;

            INSERT OR IGNORE INTO edges (
                id, source_id, target_id, relationship, weight, confidence,
                traversal_count, created_at, updated_at, last_traversed_at
            )
            SELECT
                id, source_id, target_id, relationship, weight, 0.5,
                traversal_count, created_at, last_traversed_at, last_traversed_at
            FROM connections;

            INSERT OR IGNORE INTO edges (
                id, source_id, target_id, relationship, weight, confidence,
                traversal_count, created_at, updated_at, last_traversed_at
            )
            SELECT
                id, source_id, target_id, relationship, weight, 0.5,
                traversal_count, created_at, last_traversed_at, last_traversed_at
            FROM ghost_connections;

            DELETE FROM memory_fts;
            INSERT INTO memory_fts (rowid, content)
            SELECT rowid, content FROM node_payload_memory;
            ",
        )?;
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

    pub fn schema_version(&self) -> SqlResult<i64> {
        self.read_schema_version()
    }

    pub fn schema_info(&self) -> SqlResult<SchemaInfo> {
        Ok(SchemaInfo {
            version: self.read_schema_version()?,
            feature_flags: current_feature_flags(),
        })
    }

    pub fn inspect_schema(path: &str) -> Result<SchemaInfo, String> {
        let conn = SqliteConnection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| format!("Failed to inspect schema at {}: {}", path, e))?;
        let version = Self::detect_schema_version(&conn)
            .map_err(|e| format!("Failed to read schema version at {}: {}", path, e))?;

        Ok(SchemaInfo {
            version,
            feature_flags: if version >= CURRENT_SCHEMA_VERSION {
                current_feature_flags()
            } else {
                Vec::new()
            },
        })
    }

    pub fn require_compatible_external_schema(path: &str) -> Result<SchemaInfo, String> {
        let info = Self::inspect_schema(path)?;
        if info.version != CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "Schema version {} at {} is incompatible with local version {}. Upgrade the external database before merge/import.",
                info.version, path, CURRENT_SCHEMA_VERSION
            ));
        }
        Ok(info)
    }

    pub fn canonical_node_count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))
    }

    pub fn canonical_edge_count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))
    }

    pub fn get_canonical_node(&self, id: &str) -> SqlResult<CanonicalNode> {
        self.conn.query_row(
            "SELECT id, kind, status, weight, confidence, helpful_count, unhelpful_count,
             initial_weight, created_at, updated_at, last_accessed_at, source_provider, source_account
             FROM nodes WHERE id = ?1",
            params![id],
            |row| Ok(row_to_canonical_node(row)),
        )
    }

    pub fn get_canonical_edge(&self, id: &str) -> SqlResult<CanonicalEdge> {
        self.conn.query_row(
            "SELECT id, source_id, target_id, relationship, weight, confidence,
             traversal_count, created_at, updated_at, last_traversed_at
             FROM edges WHERE id = ?1",
            params![id],
            |row| Ok(row_to_canonical_edge(row)),
        )
    }

    pub fn get_canonical_edges_for_node(&self, node_id: &str) -> SqlResult<Vec<CanonicalEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, relationship, weight, confidence,
             traversal_count, created_at, updated_at, last_traversed_at
             FROM edges
             WHERE source_id = ?1 OR target_id = ?1
             ORDER BY weight DESC, created_at ASC",
        )?;
        let rows = stmt.query_map(params![node_id], |row| Ok(row_to_canonical_edge(row)))?;
        rows.collect()
    }

    pub fn list_canonical_nodes(
        &self,
        kind: Option<GraphNodeKind>,
    ) -> SqlResult<Vec<CanonicalNode>> {
        match kind {
            Some(kind) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                     initial_weight, created_at, updated_at, last_accessed_at, source_provider, source_account
                     FROM nodes WHERE kind = ?1 ORDER BY updated_at DESC, created_at ASC",
                )?;
                let rows =
                    stmt.query_map(params![kind.as_str()], |row| Ok(row_to_canonical_node(row)))?;
                rows.collect()
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                     initial_weight, created_at, updated_at, last_accessed_at, source_provider, source_account
                     FROM nodes ORDER BY updated_at DESC, created_at ASC",
                )?;
                let rows = stmt.query_map([], |row| Ok(row_to_canonical_node(row)))?;
                rows.collect()
            }
        }
    }

    pub fn list_canonical_edges(&self) -> SqlResult<Vec<CanonicalEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, relationship, weight, confidence,
             traversal_count, created_at, updated_at, last_traversed_at
             FROM edges ORDER BY updated_at DESC, created_at ASC",
        )?;
        let rows = stmt.query_map([], |row| Ok(row_to_canonical_edge(row)))?;
        rows.collect()
    }

    pub fn find_canonical_edge_between(
        &self,
        first_node_id: &str,
        second_node_id: &str,
    ) -> SqlResult<Option<CanonicalEdge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, relationship, weight, confidence,
             traversal_count, created_at, updated_at, last_traversed_at
             FROM edges
             WHERE (source_id = ?1 AND target_id = ?2)
                OR (source_id = ?2 AND target_id = ?1)
             ORDER BY weight DESC, created_at ASC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![first_node_id, second_node_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_canonical_edge(row)))
        } else {
            Ok(None)
        }
    }

    pub fn get_canonical_memory_payload(&self, node_id: &str) -> SqlResult<MemoryPayload> {
        self.conn.query_row(
            "SELECT node_id, content, impulse_type, emotional_valence,
             engagement_level, source_signals, source_type, source_ref
             FROM node_payload_memory WHERE node_id = ?1",
            params![node_id],
            |row| Ok(row_to_memory_payload(row)),
        )
    }

    pub fn get_canonical_ghost_payload(&self, node_id: &str) -> SqlResult<GhostPayload> {
        self.conn.query_row(
            "SELECT node_id, source_graph, external_ref, title, metadata
             FROM node_payload_ghost WHERE node_id = ?1",
            params![node_id],
            |row| Ok(row_to_ghost_payload(row)),
        )
    }

    pub fn get_canonical_memory_impulse(&self, node_id: &str) -> SqlResult<Impulse> {
        let node = self.get_canonical_node(node_id)?;
        let payload = self.get_canonical_memory_payload(node_id)?;

        Ok(Impulse {
            id: node.id,
            content: payload.content,
            impulse_type: ImpulseType::from_str(&payload.impulse_type)
                .unwrap_or(ImpulseType::Observation),
            weight: node.weight,
            initial_weight: node.initial_weight,
            emotional_valence: EmotionalValence::from_str(&payload.emotional_valence)
                .unwrap_or(EmotionalValence::Neutral),
            engagement_level: EngagementLevel::from_str(&payload.engagement_level)
                .unwrap_or(EngagementLevel::Medium),
            source_signals: payload.source_signals,
            created_at: node.created_at,
            last_accessed_at: node.last_accessed_at,
            source_type: SourceType::from_str(&payload.source_type)
                .unwrap_or(SourceType::ExplicitSave),
            source_ref: payload.source_ref,
            status: ImpulseStatus::from_str(&node.status).unwrap_or(ImpulseStatus::Confirmed),
            source_provider: node.source_provider,
            source_account: node.source_account,
        })
    }

    pub fn list_canonical_memory_impulses(
        &self,
        status: Option<ImpulseStatus>,
    ) -> SqlResult<Vec<Impulse>> {
        let mut nodes = self.list_canonical_nodes(Some(GraphNodeKind::Memory))?;
        if let Some(status) = status {
            nodes.retain(|node| node.status == status.as_str());
        }

        nodes
            .into_iter()
            .map(|node| self.get_canonical_memory_impulse(&node.id))
            .collect()
    }

    pub fn search_canonical_memory_fts(&self, query: &str) -> SqlResult<Vec<(String, f64)>> {
        let fts_query = sanitize_fts_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }

        let mut stmt = self.conn.prepare(
            "SELECT m.node_id, fts.rank
             FROM memory_fts fts
             JOIN node_payload_memory m ON m.rowid = fts.rowid
             JOIN nodes n ON n.id = m.node_id
             WHERE memory_fts MATCH ?1
             AND n.kind = 'memory'
             AND n.status = 'confirmed'
             ORDER BY fts.rank",
        )?;
        let rows = stmt.query_map(params![fts_query], |row| {
            let id: String = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok((id, rank))
        })?;
        rows.collect()
    }

    pub fn touch_canonical_node(&self, id: &str) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE nodes SET last_accessed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn update_canonical_edge_weight(&self, id: &str, weight: f64) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE edges SET weight = ?1, updated_at = ?2 WHERE id = ?3",
            params![weight, now, id],
        )?;
        Ok(())
    }

    pub fn touch_canonical_edge(&self, id: &str) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE edges
             SET last_traversed_at = ?1,
                 updated_at = ?1,
                 traversal_count = traversal_count + 1
             WHERE id = ?2",
            params![now, id],
        )?;
        Ok(())
    }

    pub fn upsert_canonical_node_from_sync(&self, node: &CanonicalNode) -> SqlResult<(bool, bool)> {
        let existing = self.get_canonical_node(&node.id).optional()?;
        match existing {
            None => {
                self.conn.execute(
                    "INSERT INTO nodes (
                        id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                        initial_weight, created_at, updated_at, last_accessed_at,
                        source_provider, source_account
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        node.id,
                        node.kind.as_str(),
                        node.status,
                        node.weight,
                        node.confidence,
                        node.helpful_count,
                        node.unhelpful_count,
                        node.initial_weight,
                        node.created_at.to_rfc3339(),
                        node.updated_at.to_rfc3339(),
                        node.last_accessed_at.to_rfc3339(),
                        node.source_provider,
                        node.source_account,
                    ],
                )?;
                Ok((true, false))
            }
            Some(current) if node.updated_at > current.updated_at => {
                self.conn.execute(
                    "UPDATE nodes
                     SET kind = ?1,
                         status = ?2,
                         weight = ?3,
                         confidence = ?4,
                         helpful_count = ?5,
                         unhelpful_count = ?6,
                         initial_weight = ?7,
                         created_at = ?8,
                         updated_at = ?9,
                         last_accessed_at = ?10,
                         source_provider = ?11,
                         source_account = ?12
                     WHERE id = ?13",
                    params![
                        node.kind.as_str(),
                        node.status,
                        node.weight,
                        node.confidence,
                        node.helpful_count,
                        node.unhelpful_count,
                        node.initial_weight,
                        node.created_at.to_rfc3339(),
                        node.updated_at.to_rfc3339(),
                        node.last_accessed_at.to_rfc3339(),
                        node.source_provider,
                        node.source_account,
                        node.id,
                    ],
                )?;
                Ok((false, true))
            }
            Some(_) => Ok((false, false)),
        }
    }

    pub fn force_upsert_canonical_node(&self, node: &CanonicalNode) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO nodes (
                id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                initial_weight, created_at, updated_at, last_accessed_at,
                source_provider, source_account
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                status = excluded.status,
                weight = excluded.weight,
                confidence = excluded.confidence,
                helpful_count = excluded.helpful_count,
                unhelpful_count = excluded.unhelpful_count,
                initial_weight = excluded.initial_weight,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at,
                source_provider = excluded.source_provider,
                source_account = excluded.source_account",
            params![
                node.id,
                node.kind.as_str(),
                node.status,
                node.weight,
                node.confidence,
                node.helpful_count,
                node.unhelpful_count,
                node.initial_weight,
                node.created_at.to_rfc3339(),
                node.updated_at.to_rfc3339(),
                node.last_accessed_at.to_rfc3339(),
                node.source_provider,
                node.source_account,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_canonical_memory_payload(&self, payload: &MemoryPayload) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO node_payload_memory (
                node_id, content, impulse_type, emotional_valence,
                engagement_level, source_signals, source_type, source_ref
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(node_id) DO UPDATE SET
                content = excluded.content,
                impulse_type = excluded.impulse_type,
                emotional_valence = excluded.emotional_valence,
                engagement_level = excluded.engagement_level,
                source_signals = excluded.source_signals,
                source_type = excluded.source_type,
                source_ref = excluded.source_ref",
            params![
                payload.node_id,
                payload.content,
                payload.impulse_type,
                payload.emotional_valence,
                payload.engagement_level,
                serde_json::to_string(&payload.source_signals).unwrap_or_else(|_| "[]".to_string()),
                payload.source_type,
                payload.source_ref,
            ],
        )?;
        self.refresh_memory_fts(&payload.node_id)?;
        Ok(())
    }

    pub fn upsert_skill_payload(&self, skill: &SkillPayload) -> SqlResult<()> {
        let steps_json = serde_json::to_string(&skill.steps).unwrap_or_else(|_| "[]".to_string());
        let constraints_json =
            serde_json::to_string(&skill.constraints).unwrap_or_else(|_| "[]".to_string());
        self.conn.execute(
            "INSERT INTO node_payload_skill (node_id, trigger, steps_json, constraints_json, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id) DO UPDATE SET
                trigger = excluded.trigger,
                steps_json = excluded.steps_json,
                constraints_json = excluded.constraints_json,
                metadata = excluded.metadata",
            params![
                skill.node_id,
                skill.trigger,
                steps_json,
                constraints_json,
                skill.metadata.to_string(),
            ],
        )?;
        let index_content = format!("{} {} {}", skill.name, skill.description, skill.trigger);
        self.refresh_skill_fts(&skill.node_id, &index_content)?;
        Ok(())
    }

    pub fn upsert_canonical_ghost_payload(&self, payload: &GhostPayload) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO node_payload_ghost (node_id, source_graph, external_ref, title, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id) DO UPDATE SET
                source_graph = excluded.source_graph,
                external_ref = excluded.external_ref,
                title = excluded.title,
                metadata = excluded.metadata",
            params![
                payload.node_id,
                payload.source_graph,
                payload.external_ref,
                payload.title,
                payload.metadata.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn upsert_canonical_edge_from_sync(&self, edge: &CanonicalEdge) -> SqlResult<(bool, bool)> {
        let existing = self.get_canonical_edge(&edge.id).optional()?;
        match existing {
            None => {
                self.conn.execute(
                    "INSERT INTO edges (
                        id, source_id, target_id, relationship, weight, confidence,
                        traversal_count, created_at, updated_at, last_traversed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        edge.id,
                        edge.source_id,
                        edge.target_id,
                        edge.relationship,
                        edge.weight,
                        edge.confidence,
                        edge.traversal_count,
                        edge.created_at.to_rfc3339(),
                        edge.updated_at.to_rfc3339(),
                        edge.last_traversed_at.to_rfc3339(),
                    ],
                )?;
                Ok((true, false))
            }
            Some(current) if edge.updated_at > current.updated_at => {
                self.conn.execute(
                    "UPDATE edges
                     SET source_id = ?1,
                         target_id = ?2,
                         relationship = ?3,
                         weight = ?4,
                         confidence = ?5,
                         traversal_count = ?6,
                         created_at = ?7,
                         updated_at = ?8,
                         last_traversed_at = ?9
                     WHERE id = ?10",
                    params![
                        edge.source_id,
                        edge.target_id,
                        edge.relationship,
                        edge.weight,
                        edge.confidence,
                        edge.traversal_count,
                        edge.created_at.to_rfc3339(),
                        edge.updated_at.to_rfc3339(),
                        edge.last_traversed_at.to_rfc3339(),
                        edge.id,
                    ],
                )?;
                Ok((false, true))
            }
            Some(_) => Ok((false, false)),
        }
    }

    fn refresh_skill_fts(&self, node_id: &str, index_content: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM skill_fts
             WHERE rowid = (SELECT rowid FROM node_payload_skill WHERE node_id = ?1)",
            params![node_id],
        )?;
        self.conn.execute(
            "INSERT INTO skill_fts (rowid, trigger)
             SELECT rowid, ?2 FROM node_payload_skill WHERE node_id = ?1",
            params![node_id, index_content],
        )?;
        Ok(())
    }

    pub fn create_skill(
        &self,
        name: &str,
        description: &str,
        trigger: &str,
        steps: &[String],
        constraints: &[String],
        evidence_set_id: &str,
        evidence_node_ids: &[String],
        source_provider: &str,
        source_account: &str,
    ) -> SqlResult<SkillPayload> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let steps_json = serde_json::to_string(steps).unwrap_or_else(|_| "[]".to_string());
        let constraints_json =
            serde_json::to_string(constraints).unwrap_or_else(|_| "[]".to_string());

        let evidence_nodes = evidence_node_ids
            .iter()
            .filter_map(|node_id| self.get_canonical_node(node_id).ok())
            .collect::<Vec<_>>();
        let derived_weight = if evidence_nodes.is_empty() {
            0.7
        } else {
            evidence_nodes.iter().map(|node| node.weight).sum::<f64>() / evidence_nodes.len() as f64
        };
        let derived_confidence = if evidence_nodes.is_empty() {
            0.5
        } else {
            evidence_nodes
                .iter()
                .map(|node| effective_confidence(node.helpful_count, node.unhelpful_count))
                .sum::<f64>()
                / evidence_nodes.len() as f64
        };

        let metadata = serde_json::json!({
            "name": name,
            "description": description,
            "origin_evidence_set_id": evidence_set_id,
        });

        self.conn.execute(
            "INSERT INTO nodes (
                id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                initial_weight, created_at, updated_at, last_accessed_at,
                source_provider, source_account
             ) VALUES (?1, 'skill', 'confirmed', ?2, ?3, 0, 0, ?4, ?5, ?5, ?5, ?6, ?7)",
            params![
                id,
                derived_weight,
                derived_confidence,
                derived_weight,
                now.to_rfc3339(),
                source_provider,
                source_account,
            ],
        )?;

        self.conn.execute(
            "INSERT INTO node_payload_skill (node_id, trigger, steps_json, constraints_json, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, trigger, steps_json, constraints_json, metadata.to_string()],
        )?;

        let index_content = format!("{} {} {}", name, description, trigger);
        self.refresh_skill_fts(&id, &index_content)?;

        for evidence_node_id in evidence_node_ids {
            let edge_id = Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO edges (
                    id, source_id, target_id, relationship, weight, confidence,
                    traversal_count, created_at, updated_at, last_traversed_at
                 ) VALUES (?1, ?2, ?3, 'evidence_for', 1.0, ?4, 0, ?5, ?5, ?5)",
                params![
                    edge_id,
                    evidence_node_id,
                    id,
                    derived_confidence,
                    now.to_rfc3339(),
                ],
            )?;
        }

        self.get_skill(&id)
    }

    pub fn get_skill(&self, node_id: &str) -> SqlResult<SkillPayload> {
        self.conn.query_row(
            "SELECT node_id, trigger, steps_json, constraints_json, metadata
             FROM node_payload_skill WHERE node_id = ?1",
            params![node_id],
            |row| Ok(row_to_skill_payload(row)),
        )
    }

    pub fn get_skill_evidence_node_ids(&self, skill_id: &str) -> SqlResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_id
             FROM edges
             WHERE target_id = ?1 AND relationship = 'evidence_for'
             ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(params![skill_id], |row| row.get::<_, String>(0))?;
        rows.collect()
    }

    pub fn search_skills_fts(&self, query: &str) -> SqlResult<Vec<(String, f64)>> {
        let fts_query = sanitize_fts_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }

        let mut stmt = self.conn.prepare(
            "SELECT s.node_id, fts.rank
             FROM skill_fts fts
             JOIN node_payload_skill s ON s.rowid = fts.rowid
             JOIN nodes n ON n.id = s.node_id
             WHERE skill_fts MATCH ?1
             AND n.kind = 'skill'
             AND n.status = 'confirmed'
             ORDER BY fts.rank",
        )?;
        let rows = stmt.query_map(params![fts_query], |row| {
            let id: String = row.get(0)?;
            let rank: f64 = row.get(1)?;
            Ok((id, rank))
        })?;
        rows.collect()
    }

    pub fn create_evidence_set(
        &self,
        query: &str,
        response_hash: &str,
        node_ids: &[String],
        edge_ids: &[String],
        ttl_hours: Option<i64>,
    ) -> SqlResult<EvidenceSet> {
        let _ = self.purge_expired_evidence_sets()?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = ttl_hours.map(|hours| now + chrono::Duration::hours(hours));
        let node_ids_json = serde_json::to_string(node_ids).unwrap_or_else(|_| "[]".to_string());
        let edge_ids_json = serde_json::to_string(edge_ids).unwrap_or_else(|_| "[]".to_string());

        self.conn.execute(
            "INSERT INTO evidence_sets (
                id, query, response_hash, node_ids, edge_ids, created_at, expires_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                query,
                response_hash,
                node_ids_json,
                edge_ids_json,
                now.to_rfc3339(),
                expires_at.as_ref().map(DateTime::<Utc>::to_rfc3339),
            ],
        )?;

        self.get_evidence_set(&id)
    }

    pub fn purge_expired_evidence_sets(&self) -> SqlResult<usize> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "DELETE FROM evidence_sets
             WHERE expires_at IS NOT NULL AND expires_at < ?1",
            params![now],
        )
    }

    pub fn get_evidence_set(&self, id: &str) -> SqlResult<EvidenceSet> {
        self.conn.query_row(
            "SELECT id, query, response_hash, node_ids, edge_ids, created_at, expires_at
             FROM evidence_sets WHERE id = ?1",
            params![id],
            |row| Ok(row_to_evidence_set(row)),
        )
    }

    pub fn apply_feedback_to_node(
        &self,
        node_id: &str,
        feedback_kind: FeedbackKind,
    ) -> SqlResult<CanonicalNode> {
        let node = self.get_canonical_node(node_id)?;
        let (helpful_count, unhelpful_count) = match feedback_kind {
            FeedbackKind::Helpful => (node.helpful_count + 1, node.unhelpful_count),
            FeedbackKind::Unhelpful => (node.helpful_count, node.unhelpful_count + 1),
        };
        let confidence = posterior_confidence(helpful_count, unhelpful_count);
        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "UPDATE nodes
             SET helpful_count = ?1,
                 unhelpful_count = ?2,
                 confidence = ?3,
                 updated_at = ?4
             WHERE id = ?5",
            params![helpful_count, unhelpful_count, confidence, now, node_id,],
        )?;

        self.get_canonical_node(node_id)
    }

    pub fn create_feedback_record(
        &self,
        evidence_set_id: &str,
        target_node_id: Option<&str>,
        target_edge_id: Option<&str>,
        feedback_kind: FeedbackKind,
        idempotency_key: &str,
    ) -> SqlResult<Option<FeedbackRecord>> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let inserted = self.conn.execute(
            "INSERT OR IGNORE INTO feedback_events (
                id, evidence_set_id, target_node_id, target_edge_id,
                feedback_kind, idempotency_key, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                evidence_set_id,
                target_node_id,
                target_edge_id,
                feedback_kind.as_str(),
                idempotency_key,
                now.to_rfc3339(),
            ],
        )?;

        if inserted == 0 {
            return Ok(None);
        }

        Ok(Some(FeedbackRecord {
            id,
            evidence_set_id: evidence_set_id.to_string(),
            target_node_id: target_node_id.map(ToOwned::to_owned),
            target_edge_id: target_edge_id.map(ToOwned::to_owned),
            feedback_kind,
            idempotency_key: idempotency_key.to_string(),
            created_at: now,
        }))
    }

    pub fn get_assessment(&self, id: &str) -> SqlResult<Assessment> {
        self.conn.query_row(
            "SELECT id, subject_node_id, object_node_id, assessment_type, status,
             confidence, rationale, created_at, updated_at, dismissed_at
             FROM assessments WHERE id = ?1",
            params![id],
            |row| Ok(row_to_assessment(row)),
        )
    }

    pub fn find_assessment_for_pair(
        &self,
        assessment_type: AssessmentType,
        left_node_id: &str,
        right_node_id: &str,
    ) -> SqlResult<Option<Assessment>> {
        let (subject_node_id, object_node_id) =
            canonicalize_assessment_pair(left_node_id, right_node_id);

        self.conn
            .query_row(
                "SELECT id, subject_node_id, object_node_id, assessment_type, status,
                 confidence, rationale, created_at, updated_at, dismissed_at
                 FROM assessments
                 WHERE assessment_type = ?1
                 AND subject_node_id = ?2
                 AND object_node_id = ?3",
                params![assessment_type.as_str(), subject_node_id, object_node_id,],
                |row| Ok(row_to_assessment(row)),
            )
            .optional()
    }

    pub fn upsert_pair_assessment(
        &self,
        assessment_type: AssessmentType,
        left_node_id: &str,
        right_node_id: &str,
        confidence: f64,
        rationale: &str,
    ) -> SqlResult<Assessment> {
        let (subject_node_id, object_node_id) =
            canonicalize_assessment_pair(left_node_id, right_node_id);
        let now = Utc::now().to_rfc3339();

        if let Some(existing) =
            self.find_assessment_for_pair(assessment_type, &subject_node_id, &object_node_id)?
        {
            let next_status = if existing.status == AssessmentStatus::Confirmed {
                AssessmentStatus::Confirmed
            } else {
                AssessmentStatus::Candidate
            };

            self.conn.execute(
                "UPDATE assessments
                 SET status = ?1,
                     confidence = ?2,
                     rationale = ?3,
                     updated_at = ?4,
                     dismissed_at = NULL
                 WHERE id = ?5",
                params![
                    next_status.as_str(),
                    confidence,
                    rationale,
                    now,
                    existing.id,
                ],
            )?;
            return self.get_assessment(&existing.id);
        }

        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO assessments (
                id, subject_node_id, object_node_id, assessment_type, status,
                confidence, rationale, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
            params![
                id,
                subject_node_id,
                object_node_id,
                assessment_type.as_str(),
                AssessmentStatus::Candidate.as_str(),
                confidence,
                rationale,
                now,
            ],
        )?;

        self.get_assessment(&id)
    }

    pub fn set_assessment_status(
        &self,
        id: &str,
        status: AssessmentStatus,
    ) -> SqlResult<Assessment> {
        let now = Utc::now().to_rfc3339();
        let dismissed_at = if status == AssessmentStatus::Dismissed {
            Some(now.clone())
        } else {
            None
        };

        self.conn.execute(
            "UPDATE assessments
             SET status = ?1,
                 updated_at = ?2,
                 dismissed_at = ?3
             WHERE id = ?4",
            params![status.as_str(), now, dismissed_at, id],
        )?;

        self.get_assessment(id)
    }

    pub fn upsert_assessment_from_sync(&self, assessment: &Assessment) -> SqlResult<(bool, bool)> {
        let existing = self.get_assessment(&assessment.id).optional()?;
        match existing {
            None => {
                self.conn.execute(
                    "INSERT INTO assessments (
                        id, subject_node_id, object_node_id, assessment_type, status,
                        confidence, rationale, created_at, updated_at, dismissed_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        assessment.id,
                        assessment.subject_node_id,
                        assessment.object_node_id,
                        assessment.assessment_type.as_str(),
                        assessment.status.as_str(),
                        assessment.confidence,
                        assessment.rationale,
                        assessment.created_at.to_rfc3339(),
                        assessment.updated_at.to_rfc3339(),
                        assessment.dismissed_at.map(|value| value.to_rfc3339()),
                    ],
                )?;
                Ok((true, false))
            }
            Some(current) if assessment.updated_at > current.updated_at => {
                self.conn.execute(
                    "UPDATE assessments
                     SET subject_node_id = ?1,
                         object_node_id = ?2,
                         assessment_type = ?3,
                         status = ?4,
                         confidence = ?5,
                         rationale = ?6,
                         created_at = ?7,
                         updated_at = ?8,
                         dismissed_at = ?9
                     WHERE id = ?10",
                    params![
                        assessment.subject_node_id,
                        assessment.object_node_id,
                        assessment.assessment_type.as_str(),
                        assessment.status.as_str(),
                        assessment.confidence,
                        assessment.rationale,
                        assessment.created_at.to_rfc3339(),
                        assessment.updated_at.to_rfc3339(),
                        assessment.dismissed_at.map(|value| value.to_rfc3339()),
                        assessment.id,
                    ],
                )?;
                Ok((false, true))
            }
            Some(_) => Ok((false, false)),
        }
    }

    pub fn list_assessments(
        &self,
        assessment_type: Option<AssessmentType>,
        status: Option<AssessmentStatus>,
        node_id: Option<&str>,
    ) -> SqlResult<Vec<Assessment>> {
        let mut sql = String::from(
            "SELECT id, subject_node_id, object_node_id, assessment_type, status,
             confidence, rationale, created_at, updated_at, dismissed_at
             FROM assessments WHERE 1 = 1",
        );
        let mut params_vec = Vec::new();

        if let Some(assessment_type) = assessment_type {
            sql.push_str(" AND assessment_type = ?");
            params_vec.push(assessment_type.as_str().to_string());
        }
        if let Some(status) = status {
            sql.push_str(" AND status = ?");
            params_vec.push(status.as_str().to_string());
        }
        if let Some(node_id) = node_id {
            sql.push_str(" AND (subject_node_id = ? OR object_node_id = ?)");
            params_vec.push(node_id.to_string());
            params_vec.push(node_id.to_string());
        }

        sql.push_str(" ORDER BY confidence DESC, updated_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_vec.iter()), |row| {
            Ok(row_to_assessment(row))
        })?;
        rows.collect()
    }

    pub fn list_assessments_for_node_ids(&self, node_ids: &[String]) -> SqlResult<Vec<Assessment>> {
        if node_ids.is_empty() {
            return Ok(Vec::new());
        }

        let node_set: std::collections::HashSet<&str> =
            node_ids.iter().map(String::as_str).collect();
        let assessments = self.list_assessments(None, None, None)?;

        Ok(assessments
            .into_iter()
            .filter(|assessment| {
                node_set.contains(assessment.subject_node_id.as_str())
                    || assessment
                        .object_node_id
                        .as_deref()
                        .is_some_and(|node_id| node_set.contains(node_id))
            })
            .collect())
    }

    pub fn count_confirmed_contradictions_for_node(&self, node_id: &str) -> SqlResult<i64> {
        self.conn.query_row(
            "SELECT COUNT(*)
             FROM assessments
             WHERE assessment_type = ?1
               AND status = ?2
               AND (subject_node_id = ?3 OR object_node_id = ?3)",
            params![
                AssessmentType::Contradiction.as_str(),
                AssessmentStatus::Confirmed.as_str(),
                node_id,
            ],
            |row| row.get(0),
        )
    }

    fn refresh_memory_fts(&self, node_id: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM memory_fts
             WHERE rowid = (SELECT rowid FROM node_payload_memory WHERE node_id = ?1)",
            params![node_id],
        )?;
        self.conn.execute(
            "INSERT INTO memory_fts (rowid, content)
             SELECT rowid, content FROM node_payload_memory WHERE node_id = ?1",
            params![node_id],
        )?;
        Ok(())
    }

    fn mirror_impulse_to_canonical(&self, id: &str) -> SqlResult<()> {
        let impulse = self.get_impulse(id)?;
        let signals_json = serde_json::to_string(&impulse.source_signals).unwrap_or_default();
        let updated_at = impulse.last_accessed_at.to_rfc3339();

        self.conn.execute(
            "INSERT INTO nodes (
                id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                initial_weight, created_at, updated_at, last_accessed_at,
                source_provider, source_account
             ) VALUES (?1, ?2, ?3, ?4, 0.5, 0, 0, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                status = excluded.status,
                weight = excluded.weight,
                initial_weight = excluded.initial_weight,
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at,
                source_provider = excluded.source_provider,
                source_account = excluded.source_account",
            params![
                impulse.id,
                GraphNodeKind::Memory.as_str(),
                impulse.status.as_str(),
                impulse.weight,
                impulse.initial_weight,
                impulse.created_at.to_rfc3339(),
                updated_at,
                impulse.last_accessed_at.to_rfc3339(),
                impulse.source_provider,
                impulse.source_account,
            ],
        )?;

        self.conn.execute(
            "INSERT INTO node_payload_memory (
                node_id, content, impulse_type, emotional_valence,
                engagement_level, source_signals, source_type, source_ref
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(node_id) DO UPDATE SET
                content = excluded.content,
                impulse_type = excluded.impulse_type,
                emotional_valence = excluded.emotional_valence,
                engagement_level = excluded.engagement_level,
                source_signals = excluded.source_signals,
                source_type = excluded.source_type,
                source_ref = excluded.source_ref",
            params![
                impulse.id,
                impulse.content,
                impulse.impulse_type.as_str(),
                impulse.emotional_valence.as_str(),
                impulse.engagement_level.as_str(),
                signals_json,
                impulse.source_type.as_str(),
                impulse.source_ref,
            ],
        )?;

        self.refresh_memory_fts(id)?;
        Ok(())
    }

    fn mirror_connection_to_canonical(&self, id: &str) -> SqlResult<()> {
        let conn = self.get_connection(id)?;

        self.conn.execute(
            "INSERT INTO edges (
                id, source_id, target_id, relationship, weight, confidence,
                traversal_count, created_at, updated_at, last_traversed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 0.5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                source_id = excluded.source_id,
                target_id = excluded.target_id,
                relationship = excluded.relationship,
                weight = excluded.weight,
                traversal_count = excluded.traversal_count,
                updated_at = excluded.updated_at,
                last_traversed_at = excluded.last_traversed_at",
            params![
                conn.id,
                conn.source_id,
                conn.target_id,
                conn.relationship,
                conn.weight,
                conn.traversal_count,
                conn.created_at.to_rfc3339(),
                conn.last_traversed_at.to_rfc3339(),
                conn.last_traversed_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    fn mirror_ghost_node_to_canonical(&self, id: &str) -> SqlResult<()> {
        let ghost = self.get_ghost_node(id)?;

        self.conn.execute(
            "INSERT INTO nodes (
                id, kind, status, weight, confidence, helpful_count, unhelpful_count,
                initial_weight, created_at, updated_at, last_accessed_at,
                source_provider, source_account
             ) VALUES (?1, ?2, 'confirmed', ?3, 0.5, 0, 0, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                kind = excluded.kind,
                status = excluded.status,
                weight = excluded.weight,
                initial_weight = excluded.initial_weight,
                updated_at = excluded.updated_at,
                last_accessed_at = excluded.last_accessed_at,
                source_provider = excluded.source_provider,
                source_account = excluded.source_account",
            params![
                ghost.id,
                GraphNodeKind::Ghost.as_str(),
                ghost.weight,
                ghost.weight,
                ghost.created_at.to_rfc3339(),
                ghost.last_accessed_at.to_rfc3339(),
                ghost.last_accessed_at.to_rfc3339(),
                "ghost",
                ghost.source_graph,
            ],
        )?;

        self.conn.execute(
            "INSERT INTO node_payload_ghost (node_id, source_graph, external_ref, title, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id) DO UPDATE SET
                source_graph = excluded.source_graph,
                external_ref = excluded.external_ref,
                title = excluded.title,
                metadata = excluded.metadata",
            params![
                ghost.id,
                ghost.source_graph,
                ghost.external_ref,
                ghost.title,
                serde_json::to_string(&ghost.metadata).unwrap_or_else(|_| "{}".to_string()),
            ],
        )?;

        Ok(())
    }

    fn mirror_ghost_connection_to_canonical(&self, id: &str) -> SqlResult<()> {
        let conn = self.get_ghost_connection(id)?;

        self.conn.execute(
            "INSERT INTO edges (
                id, source_id, target_id, relationship, weight, confidence,
                traversal_count, created_at, updated_at, last_traversed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 0.5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                source_id = excluded.source_id,
                target_id = excluded.target_id,
                relationship = excluded.relationship,
                weight = excluded.weight,
                traversal_count = excluded.traversal_count,
                updated_at = excluded.updated_at,
                last_traversed_at = excluded.last_traversed_at",
            params![
                conn.id,
                conn.source_id,
                conn.target_id,
                conn.relationship,
                conn.weight,
                conn.traversal_count,
                conn.created_at.to_rfc3339(),
                conn.last_traversed_at.to_rfc3339(),
                conn.last_traversed_at.to_rfc3339(),
            ],
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

        self.mirror_impulse_to_canonical(&id)?;

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

        self.mirror_impulse_to_canonical(id)?;

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
        self.mirror_impulse_to_canonical(id)?;
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
        self.mirror_impulse_to_canonical(id)?;
        Ok(())
    }

    pub fn touch_impulse(&self, id: &str) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE impulses SET last_accessed_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        self.mirror_impulse_to_canonical(id)?;
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
        self.conn
            .query_row("SELECT COUNT(*) FROM impulses_fts", [], |row| row.get(0))
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

        self.mirror_connection_to_canonical(&id)?;

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
        self.mirror_connection_to_canonical(id)?;
        Ok(())
    }

    pub fn touch_connection(&self, id: &str) -> SqlResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE connections SET last_traversed_at = ?1, traversal_count = traversal_count + 1
             WHERE id = ?2",
            params![now, id],
        )?;
        self.mirror_connection_to_canonical(id)?;
        Ok(())
    }

    pub fn connection_count(&self) -> SqlResult<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM connections", [], |row| row.get(0))
    }

    pub fn delete_connection(&self, id: &str) -> SqlResult<()> {
        self.conn
            .execute("DELETE FROM connections WHERE id = ?1", params![id])?;
        self.conn
            .execute("DELETE FROM edges WHERE id = ?1", params![id])?;
        Ok(())
    }

    // === Ghost Node Operations ===

    pub fn insert_ghost_node(&self, input: &NewGhostNode) -> SqlResult<GhostNode> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let metadata_str =
            serde_json::to_string(&input.metadata).unwrap_or_else(|_| "{}".to_string());

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

        self.mirror_ghost_node_to_canonical(&id)?;

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

    pub fn get_ghost_node_by_ref(
        &self,
        source_graph: &str,
        external_ref: &str,
    ) -> SqlResult<GhostNode> {
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
        self.mirror_ghost_node_to_canonical(id)?;
        Ok(())
    }

    pub fn update_ghost_node_weight(&self, id: &str, weight: f64) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE ghost_nodes SET weight = ?1 WHERE id = ?2",
            params![weight, id],
        )?;
        self.mirror_ghost_node_to_canonical(id)?;
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
        self.conn.execute(
            "DELETE FROM nodes WHERE id IN (
                SELECT node_id FROM node_payload_ghost WHERE source_graph = ?1
            )",
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

        self.mirror_ghost_connection_to_canonical(&id)?;

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

    pub fn register_ghost_source(
        &self,
        name: &str,
        root_path: &str,
        source_type: &str,
    ) -> SqlResult<GhostSource> {
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
        let mut stmt = self
            .conn
            .prepare("SELECT name, color, created_at FROM tags ORDER BY name")?;
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
        self.conn
            .execute("DELETE FROM tags WHERE name = ?1", params![name])?;
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
        self.conn
            .execute_batch(&format!("VACUUM INTO '{}'", path.replace('\'', "''")))?;
        Ok(())
    }

    // === Stats ===

    pub fn memory_stats(&self) -> SqlResult<MemoryStats> {
        let total_memory_nodes: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind = 'memory'",
            [],
            |row| row.get(0),
        )?;
        let total_skill_nodes: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind = 'skill'",
            [],
            |row| row.get(0),
        )?;
        let total_ghost_nodes: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind = 'ghost'",
            [],
            |row| row.get(0),
        )?;
        let total_graph_edges: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
        let confirmed_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind = 'memory' AND status = 'confirmed'",
            [],
            |row| row.get(0),
        )?;
        let candidate_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM nodes WHERE kind = 'memory' AND status = 'candidate'",
            [],
            |row| row.get(0),
        )?;
        let total_assessments: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM assessments", [], |row| row.get(0))?;
        let total_evidence_sets: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM evidence_sets", [], |row| row.get(0))?;
        Ok(MemoryStats {
            total_impulses: total_memory_nodes,
            confirmed_impulses: confirmed_count,
            candidate_impulses: candidate_count,
            total_connections: total_graph_edges,
            total_memory_nodes,
            total_skill_nodes,
            total_ghost_nodes,
            total_graph_edges,
            total_assessments,
            total_evidence_sets,
        })
    }
}

// === Helper: row mapping ===

fn parse_timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn row_to_canonical_node(row: &rusqlite::Row) -> CanonicalNode {
    let created_str: String = row.get(8).unwrap_or_default();
    let updated_str: String = row.get(9).unwrap_or_default();
    let accessed_str: String = row.get(10).unwrap_or_default();

    CanonicalNode {
        id: row.get(0).unwrap_or_default(),
        kind: GraphNodeKind::from_str(&row.get::<_, String>(1).unwrap_or_default())
            .unwrap_or(GraphNodeKind::Memory),
        status: row.get(2).unwrap_or_default(),
        weight: row.get(3).unwrap_or(0.0),
        confidence: row.get(4).unwrap_or(0.5),
        helpful_count: row.get(5).unwrap_or(0),
        unhelpful_count: row.get(6).unwrap_or(0),
        initial_weight: row.get(7).unwrap_or(0.0),
        created_at: parse_timestamp(&created_str),
        updated_at: parse_timestamp(&updated_str),
        last_accessed_at: parse_timestamp(&accessed_str),
        source_provider: row.get(11).unwrap_or_else(|_| "unknown".to_string()),
        source_account: row.get(12).unwrap_or_default(),
    }
}

fn row_to_canonical_edge(row: &rusqlite::Row) -> CanonicalEdge {
    let created_str: String = row.get(7).unwrap_or_default();
    let updated_str: String = row.get(8).unwrap_or_default();
    let traversed_str: String = row.get(9).unwrap_or_default();

    CanonicalEdge {
        id: row.get(0).unwrap_or_default(),
        source_id: row.get(1).unwrap_or_default(),
        target_id: row.get(2).unwrap_or_default(),
        relationship: row.get(3).unwrap_or_default(),
        weight: row.get(4).unwrap_or(0.0),
        confidence: row.get(5).unwrap_or(0.5),
        traversal_count: row.get(6).unwrap_or(0),
        created_at: parse_timestamp(&created_str),
        updated_at: parse_timestamp(&updated_str),
        last_traversed_at: parse_timestamp(&traversed_str),
    }
}

fn row_to_memory_payload(row: &rusqlite::Row) -> MemoryPayload {
    let signals_json: String = row.get(5).unwrap_or_default();

    MemoryPayload {
        node_id: row.get(0).unwrap_or_default(),
        content: row.get(1).unwrap_or_default(),
        impulse_type: row.get(2).unwrap_or_default(),
        emotional_valence: row.get(3).unwrap_or_default(),
        engagement_level: row.get(4).unwrap_or_default(),
        source_signals: serde_json::from_str(&signals_json).unwrap_or_default(),
        source_type: row.get(6).unwrap_or_default(),
        source_ref: row.get(7).unwrap_or_default(),
    }
}

fn row_to_ghost_payload(row: &rusqlite::Row) -> GhostPayload {
    let metadata_str: String = row.get(4).unwrap_or_else(|_| "{}".to_string());

    GhostPayload {
        node_id: row.get(0).unwrap_or_default(),
        source_graph: row.get(1).unwrap_or_default(),
        external_ref: row.get(2).unwrap_or_default(),
        title: row.get(3).unwrap_or_default(),
        metadata: serde_json::from_str(&metadata_str)
            .unwrap_or(serde_json::Value::Object(Default::default())),
    }
}

fn row_to_evidence_set(row: &rusqlite::Row) -> EvidenceSet {
    let node_ids_json: String = row.get(3).unwrap_or_else(|_| "[]".to_string());
    let edge_ids_json: String = row.get(4).unwrap_or_else(|_| "[]".to_string());
    let created_str: String = row.get(5).unwrap_or_default();
    let expires_str: Option<String> = row.get(6).unwrap_or(None);

    EvidenceSet {
        id: row.get(0).unwrap_or_default(),
        query: row.get(1).unwrap_or_default(),
        response_hash: row.get(2).unwrap_or_default(),
        node_ids: serde_json::from_str(&node_ids_json).unwrap_or_default(),
        edge_ids: serde_json::from_str(&edge_ids_json).unwrap_or_default(),
        created_at: parse_timestamp(&created_str),
        expires_at: expires_str.map(|value| parse_timestamp(&value)),
    }
}

fn row_to_skill_payload(row: &rusqlite::Row) -> SkillPayload {
    let steps_json: String = row.get(2).unwrap_or_else(|_| "[]".to_string());
    let constraints_json: String = row.get(3).unwrap_or_else(|_| "[]".to_string());
    let metadata_str: String = row.get(4).unwrap_or_else(|_| "{}".to_string());
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata_str).unwrap_or_else(|_| serde_json::json!({}));

    SkillPayload {
        node_id: row.get(0).unwrap_or_default(),
        name: metadata
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        description: metadata
            .get("description")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        trigger: row.get(1).unwrap_or_default(),
        steps: serde_json::from_str(&steps_json).unwrap_or_default(),
        constraints: serde_json::from_str(&constraints_json).unwrap_or_default(),
        metadata,
    }
}

fn row_to_assessment(row: &rusqlite::Row) -> Assessment {
    let created_str: String = row.get(7).unwrap_or_default();
    let updated_str: String = row.get(8).unwrap_or_default();
    let dismissed_str: Option<String> = row.get(9).unwrap_or(None);

    Assessment {
        id: row.get(0).unwrap_or_default(),
        subject_node_id: row.get(1).unwrap_or_default(),
        object_node_id: row.get(2).unwrap_or(None),
        assessment_type: AssessmentType::from_str(&row.get::<_, String>(3).unwrap_or_default())
            .unwrap_or(AssessmentType::Contradiction),
        status: AssessmentStatus::from_str(&row.get::<_, String>(4).unwrap_or_default())
            .unwrap_or(AssessmentStatus::Candidate),
        confidence: row.get(5).unwrap_or(0.5),
        rationale: row.get(6).unwrap_or_default(),
        created_at: parse_timestamp(&created_str),
        updated_at: parse_timestamp(&updated_str),
        dismissed_at: dismissed_str.map(|value| parse_timestamp(&value)),
    }
}

fn row_to_impulse(row: &rusqlite::Row) -> Impulse {
    let signals_json: String = row.get(7).unwrap_or_default();
    let source_signals: Vec<String> = serde_json::from_str(&signals_json).unwrap_or_default();

    let created_str: String = row.get(8).unwrap_or_default();
    let accessed_str: String = row.get(9).unwrap_or_default();

    Impulse {
        id: row.get(0).unwrap_or_default(),
        content: row.get(1).unwrap_or_default(),
        impulse_type: ImpulseType::from_str(&row.get::<_, String>(2).unwrap_or_default())
            .unwrap_or(ImpulseType::Observation),
        weight: row.get(3).unwrap_or(0.0),
        initial_weight: row.get(4).unwrap_or(0.0),
        emotional_valence: EmotionalValence::from_str(&row.get::<_, String>(5).unwrap_or_default())
            .unwrap_or(EmotionalValence::Neutral),
        engagement_level: EngagementLevel::from_str(&row.get::<_, String>(6).unwrap_or_default())
            .unwrap_or(EngagementLevel::Medium),
        source_signals,
        created_at: parse_timestamp(&created_str),
        last_accessed_at: parse_timestamp(&accessed_str),
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
        created_at: parse_timestamp(&created_str),
        last_traversed_at: parse_timestamp(&traversed_str),
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
        metadata: serde_json::from_str(&metadata_str)
            .unwrap_or(serde_json::Value::Object(Default::default())),
        weight: row.get(5).unwrap_or(0.0),
        last_accessed_at: parse_timestamp(&accessed_str),
        created_at: parse_timestamp(&created_str),
    }
}

fn row_to_ghost_source(row: &rusqlite::Row) -> GhostSource {
    let registered_str: String = row.get(3).unwrap_or_default();
    let scanned_str: Option<String> = row.get(4).unwrap_or(None);

    GhostSource {
        name: row.get(0).unwrap_or_default(),
        root_path: row.get(1).unwrap_or_default(),
        source_type: row.get(2).unwrap_or_default(),
        registered_at: parse_timestamp(&registered_str),
        last_scanned_at: scanned_str.map(|s| parse_timestamp(&s)),
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
    pub total_memory_nodes: i64,
    pub total_skill_nodes: i64,
    pub total_ghost_nodes: i64,
    pub total_graph_edges: i64,
    pub total_assessments: i64,
    pub total_evidence_sets: i64,
}

// === FTS Query Helpers ===

const FTS_STOP_WORDS: &[&str] = &[
    "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "could", "should", "may", "might", "can", "to", "of",
    "in", "for", "on", "with", "at", "by", "from", "as", "into", "through", "during", "before",
    "after", "and", "or", "but", "not", "no", "so", "if", "then", "than", "too", "very", "just",
    "about", "up", "out", "how", "what", "when", "where", "who", "which", "that", "this", "it",
    "i", "me", "my", "you", "your", "we", "our", "they", "them", "their", "he", "she", "his",
    "her", "want", "think", "know", "like", "get", "make", "going",
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

fn canonicalize_assessment_pair(left_node_id: &str, right_node_id: &str) -> (String, String) {
    if left_node_id <= right_node_id {
        (left_node_id.to_string(), right_node_id.to_string())
    } else {
        (right_node_id.to_string(), left_node_id.to_string())
    }
}
