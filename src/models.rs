// Memory graph data types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

    #[allow(clippy::should_implement_trait)]
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

    #[allow(clippy::should_implement_trait)]
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

    #[allow(clippy::should_implement_trait)]
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

    #[allow(clippy::should_implement_trait)]
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

    #[allow(clippy::should_implement_trait)]
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
    pub source_provider: String,
    pub source_account: String,
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
    pub source_provider: String,
    pub source_account: String,
}

#[derive(Debug, Clone)]
pub struct NewConnection {
    pub source_id: String,
    pub target_id: String,
    pub weight: f64,
    pub relationship: String,
}

// === Tag Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewTag {
    pub name: String,
    pub color: String,
}

// === Ghost Graph Types ===

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostActivation {
    pub ghost_node: GhostNode,
    pub activation_score: f64,
    pub source_graph: String,
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
    pub ghost_activations: Vec<GhostActivation>,
}

// === Weight Constants ===

pub const WEIGHT_EXPLICIT_SAVE: f64 = 0.7;
pub const WEIGHT_SESSION_EXTRACTION_HIGH: f64 = 0.5;
pub const WEIGHT_SESSION_EXTRACTION_LOW: f64 = 0.3;
pub const WEIGHT_PULL_THROUGH: f64 = 0.4;
pub const WEIGHT_FLOOR: f64 = 0.001;
pub const REINFORCEMENT_BUMP: f64 = 0.05;

// Decay rates (lambda) -- per hour
pub const DECAY_SEMANTIC: f64 = 0.0005;   // slow: ~1386 hours half-life (~58 days)
pub const DECAY_EPISODIC: f64 = 0.005;    // fast: ~139 hours half-life (~6 days)
pub const DECAY_GHOST: f64 = 0.002;       // medium: ~347 hours half-life (~14 days)

// Activation constants
pub const ACTIVATION_THRESHOLD: f64 = 0.1;
pub const PROXIMITY_DECAY_PER_HOP: f64 = 0.5;
pub const MAX_PROPAGATION_ITERATIONS: usize = 10;
pub const ACTIVATION_CONVERGENCE_THRESHOLD: f64 = 0.001;
