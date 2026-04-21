// Session state management

use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CompressionCheckpoint {
    pub pre_compression_calls: usize,
    pub last_pre_compression_at: Option<DateTime<Utc>>,
    pub last_pre_compression_reason: Option<String>,
    pub last_suppressed_evidence_set_ids: Vec<String>,
    pub last_stripped_char_count: usize,
}

#[derive(Debug)]
pub struct Session {
    id: String,
    incognito: bool,
    pre_compression_calls: usize,
    last_pre_compression_at: Option<DateTime<Utc>>,
    last_pre_compression_reason: Option<String>,
    last_suppressed_evidence_set_ids: Vec<String>,
    last_stripped_char_count: usize,
}

impl Session {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            incognito: false,
            pre_compression_calls: 0,
            last_pre_compression_at: None,
            last_pre_compression_reason: None,
            last_suppressed_evidence_set_ids: Vec::new(),
            last_stripped_char_count: 0,
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

    pub fn record_pre_compression(
        &mut self,
        reason: &str,
        suppressed_evidence_set_ids: Vec<String>,
        stripped_char_count: usize,
    ) {
        self.pre_compression_calls += 1;
        self.last_pre_compression_at = Some(Utc::now());
        self.last_pre_compression_reason = Some(reason.to_string());
        self.last_suppressed_evidence_set_ids = suppressed_evidence_set_ids;
        self.last_stripped_char_count = stripped_char_count;
    }

    pub fn compression_checkpoint(&self) -> CompressionCheckpoint {
        CompressionCheckpoint {
            pre_compression_calls: self.pre_compression_calls,
            last_pre_compression_at: self.last_pre_compression_at,
            last_pre_compression_reason: self.last_pre_compression_reason.clone(),
            last_suppressed_evidence_set_ids: self.last_suppressed_evidence_set_ids.clone(),
            last_stripped_char_count: self.last_stripped_char_count,
        }
    }
}
