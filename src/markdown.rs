// Markdown export for Obsidian-compatible vaults

use std::fs;
use std::path::Path;

use crate::db::Database;
use crate::models::*;

pub struct ExportResult {
    pub files_written: usize,
    pub output_dir: String,
}

pub fn export_to_markdown(db: &Database, output_dir: &str) -> Result<ExportResult, String> {
    let dir = Path::new(output_dir);
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create dir: {}", e))?;

    let impulses = db
        .list_impulses(Some(ImpulseStatus::Confirmed))
        .map_err(|e| format!("DB error: {}", e))?;

    let mut count = 0;

    for impulse in &impulses {
        let tags = db.get_tags_for_impulse(&impulse.id).unwrap_or_default();
        let connections = db
            .get_connections_for_node(&impulse.id)
            .unwrap_or_default();

        // Sanitize title from first 50 chars of content
        let title = sanitize_filename(&impulse.content, 50);
        let short_id = &impulse.id[..8.min(impulse.id.len())];
        let filename = format!("{}_{}.md", short_id, title);

        let tag_list = tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>();

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("id: {}\n", impulse.id));
        md.push_str(&format!("type: {}\n", impulse.impulse_type.as_str()));
        md.push_str(&format!("weight: {:.2}\n", impulse.weight));
        md.push_str(&format!(
            "created: {}\n",
            impulse.created_at.format("%Y-%m-%d")
        ));
        if !tag_list.is_empty() {
            md.push_str(&format!("tags: [{}]\n", tag_list.join(", ")));
        }
        if impulse.source_provider != "unknown" {
            md.push_str(&format!("source: {}\n", impulse.source_provider));
        }
        md.push_str("---\n\n");
        md.push_str(&impulse.content);
        md.push('\n');

        if !connections.is_empty() {
            md.push_str("\n## Connections\n\n");
            for conn in &connections {
                let other_id = if conn.source_id == impulse.id {
                    &conn.target_id
                } else {
                    &conn.source_id
                };
                let other_content = db
                    .get_impulse(other_id)
                    .map(|i| sanitize_filename(&i.content, 50))
                    .unwrap_or_else(|_| "unknown".to_string());
                let other_short = &other_id[..8.min(other_id.len())];
                md.push_str(&format!(
                    "- [[{}_{}.md|{}]] -- {} (weight: {:.2})\n",
                    other_short, other_content, other_content, conn.relationship, conn.weight
                ));
            }
        }

        let path = dir.join(&filename);
        fs::write(&path, &md).map_err(|e| format!("Write failed: {}", e))?;
        count += 1;
    }

    // Export tag index files
    let tags_dir = dir.join("tags");
    fs::create_dir_all(&tags_dir).ok();
    let all_tags = db.list_tags().unwrap_or_default();
    for tag in &all_tags {
        let tagged = db.get_impulses_for_tag(&tag.name).unwrap_or_default();
        let mut md = format!("# {}\n\n", tag.name);
        for imp in &tagged {
            let title = sanitize_filename(&imp.content, 50);
            let short_id = &imp.id[..8.min(imp.id.len())];
            md.push_str(&format!("- [[{}_{}.md|{}]]\n", short_id, title, title));
        }
        let path = tags_dir.join(format!("{}.md", sanitize_filename(&tag.name, 30)));
        fs::write(&path, &md).ok();
    }

    Ok(ExportResult {
        files_written: count,
        output_dir: output_dir.to_string(),
    })
}

fn sanitize_filename(text: &str, max_len: usize) -> String {
    text.chars()
        .take(max_len)
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}
