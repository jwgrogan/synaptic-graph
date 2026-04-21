mod common;

use synaptic_graph::ingestion;
use synaptic_graph::markdown;
use synaptic_graph::models::*;
use tempfile::TempDir;

#[test]
fn test_export_creates_files() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db,
        "Test memory",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    let dir = TempDir::new().unwrap();
    let result = markdown::export_to_markdown(&db, dir.path().to_str().unwrap()).unwrap();
    assert_eq!(result.files_written, 1);

    // Check file exists
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    assert_eq!(files.len(), 1);
}

#[test]
fn test_export_includes_frontmatter() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db,
        "Important heuristic about Rust",
        ImpulseType::Heuristic,
        EmotionalValence::Positive,
        EngagementLevel::High,
        vec![],
        "test",
    )
    .unwrap();

    let dir = TempDir::new().unwrap();
    markdown::export_to_markdown(&db, dir.path().to_str().unwrap()).unwrap();

    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    let content = std::fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("---"));
    assert!(content.contains("type: heuristic"));
    assert!(content.contains("Important heuristic"));
}

#[test]
fn test_export_includes_connections_as_wikilinks() {
    let db = common::test_db();
    let a = ingestion::save_and_confirm(
        &db,
        "Node A content",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    ingestion::save_and_confirm_with_connections(
        &db,
        "Node B content",
        ImpulseType::Observation,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
        &[(a.id.clone(), "relates_to".to_string(), 0.7)],
    )
    .unwrap();

    let dir = TempDir::new().unwrap();
    markdown::export_to_markdown(&db, dir.path().to_str().unwrap()).unwrap();

    // Find files with wikilinks
    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();

    let has_wikilink = files.iter().any(|f| {
        let c = std::fs::read_to_string(f.path()).unwrap();
        c.contains("[[") && c.contains("relates_to")
    });
    assert!(has_wikilink);
}

#[test]
fn test_export_empty_db() {
    let db = common::test_db();
    let dir = TempDir::new().unwrap();
    let result = markdown::export_to_markdown(&db, dir.path().to_str().unwrap()).unwrap();
    assert_eq!(result.files_written, 0);
}

#[test]
fn test_export_ignores_skill_evidence_edges() {
    let db = common::test_db();
    let memory = ingestion::save_and_confirm(
        &db,
        "Grounding memory for skill export",
        ImpulseType::Heuristic,
        EmotionalValence::Neutral,
        EngagementLevel::Medium,
        vec![],
        "test",
    )
    .unwrap();

    let evidence = db
        .create_evidence_set(
            "skill export query",
            "stable-export-hash",
            std::slice::from_ref(&memory.id),
            &[],
            Some(24),
        )
        .unwrap();
    db.create_skill(
        "Export-side skill",
        "Skill should not appear as a markdown memory connection",
        "When exporting markdown",
        &["Keep skill links out of memory notes".to_string()],
        &[],
        &evidence.id,
        std::slice::from_ref(&memory.id),
        "test",
        "",
    )
    .unwrap();

    let dir = TempDir::new().unwrap();
    markdown::export_to_markdown(&db, dir.path().to_str().unwrap()).unwrap();

    let files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
        .collect();
    assert_eq!(files.len(), 1);

    let content = std::fs::read_to_string(files[0].path()).unwrap();
    assert!(content.contains("Grounding memory for skill export"));
    assert!(!content.contains("evidence_for"));
    assert!(!content.contains("Export-side skill"));
}
