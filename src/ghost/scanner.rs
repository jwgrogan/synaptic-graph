// Ghost graph filesystem scanner — scans external sources for ghost nodes

use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub extensions: Vec<String>,
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug)]
pub struct ScanResult {
    pub nodes: Vec<ScannedNode>,
    pub links: Vec<ScannedLink>,
}

#[derive(Debug)]
pub struct ScannedNode {
    pub external_ref: String,
    pub title: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug)]
pub struct ScannedLink {
    pub from_ref: String,
    pub to_ref: String,
    pub link_type: String,
}

static WIKILINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").unwrap());

static HEADING_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#\s+(.+)$").unwrap());

static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#([a-zA-Z][a-zA-Z0-9_-]+)").unwrap());

pub fn scan_directory(root: &Path, config: &ScanConfig) -> Result<ScanResult, String> {
    let mut nodes = Vec::new();
    let mut links = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel_path.to_string_lossy().to_string();

        // Check ignore patterns
        if config.ignore_patterns.iter().any(|p| rel_str.contains(p)) {
            continue;
        }

        // Check extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !config.extensions.iter().any(|e| e == ext) {
            continue;
        }

        // Read file for metadata extraction (titles, tags, links)
        let content = fs::read_to_string(path).unwrap_or_default();

        // Extract title from first heading
        let title = content
            .lines()
            .find_map(|line| HEADING_RE.captures(line).map(|c| c[1].trim().to_string()))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string()
            });

        // Extract tags
        let tags: Vec<String> = TAG_RE
            .captures_iter(&content)
            .map(|c| c[1].to_string())
            .collect();

        // Extract wikilinks
        for cap in WIKILINK_RE.captures_iter(&content) {
            let target = cap[1].trim().to_string();
            // Resolve wikilink to a relative path
            let target_ref = resolve_wikilink(&target, root, &config.extensions);
            if let Some(target_ref) = target_ref {
                links.push(ScannedLink {
                    from_ref: rel_str.clone(),
                    to_ref: target_ref,
                    link_type: "wikilink".to_string(),
                });
            }
        }

        let metadata = serde_json::json!({
            "tags": tags,
            "extension": ext,
        });

        nodes.push(ScannedNode {
            external_ref: rel_str,
            title,
            metadata,
        });
    }

    Ok(ScanResult { nodes, links })
}

fn resolve_wikilink(target: &str, root: &Path, extensions: &[String]) -> Option<String> {
    // Try to find a matching file
    for ext in extensions {
        let candidate = format!("{}.{}", target, ext);
        // Check in root
        if root.join(&candidate).exists() {
            return Some(candidate);
        }
        // Check recursively
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let name = entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if name.eq_ignore_ascii_case(target) {
                    let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
                    return Some(rel.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}
