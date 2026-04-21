use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: i64 = 2;
pub const FEATURE_CANONICAL_GRAPH: &str = "canonical_graph";
pub const FEATURE_SCHEMA_GATED_SYNC: &str = "schema_gated_sync";
pub const FEATURE_EVIDENCE_SETS: &str = "evidence_sets";

pub fn current_feature_flags() -> Vec<String> {
    vec![
        FEATURE_CANONICAL_GRAPH.to_string(),
        FEATURE_SCHEMA_GATED_SYNC.to_string(),
        FEATURE_EVIDENCE_SETS.to_string(),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphNodeKind {
    Memory,
    Skill,
    Ghost,
}

impl GraphNodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Skill => "skill",
            Self::Ghost => "ghost",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "memory" => Some(Self::Memory),
            "skill" => Some(Self::Skill),
            "ghost" => Some(Self::Ghost),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalNode {
    pub id: String,
    pub kind: GraphNodeKind,
    pub status: String,
    pub weight: f64,
    pub confidence: f64,
    pub helpful_count: i64,
    pub unhelpful_count: i64,
    pub initial_weight: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub source_provider: String,
    pub source_account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relationship: String,
    pub weight: f64,
    pub confidence: f64,
    pub traversal_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_traversed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPayload {
    pub node_id: String,
    pub content: String,
    pub impulse_type: String,
    pub emotional_valence: String,
    pub engagement_level: String,
    pub source_signals: Vec<String>,
    pub source_type: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostPayload {
    pub node_id: String,
    pub source_graph: String,
    pub external_ref: String,
    pub title: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaInfo {
    pub version: i64,
    pub feature_flags: Vec<String>,
}
