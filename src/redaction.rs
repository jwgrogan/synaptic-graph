// Secret and PII detection and stripping

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
            regex: Regex::new(r"(?i)(postgresql|postgres|mysql|mongodb|redis)://\S+").unwrap(),
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
