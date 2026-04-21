mod common;

use std::fs;
use synaptic_graph::server::MemoryGraphServer;
use tempfile::TempDir;

#[test]
fn test_save_memory_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let result = server.handle_save_memory(
        "Rust is great for memory systems".to_string(),
        "heuristic".to_string(),
        Some("positive".to_string()),
        Some("high".to_string()),
        None,
        None,
        None,
    );

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("Rust is great"));
}

#[test]
fn test_save_memory_blocked_in_incognito() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    server.set_incognito(true);

    let result = server.handle_save_memory(
        "Should not be saved".to_string(),
        "observation".to_string(),
        None,
        None,
        None,
        None,
        None,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("incognito"));
}

#[test]
fn test_retrieve_context_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    // Save something first
    let saved = server
        .handle_quick_save(
            "SQLite is excellent for local-first applications".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let saved_id = saved["id"].as_str().unwrap();

    let result = server.handle_retrieve_context("SQLite local".to_string(), Some(10));

    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(!response["memories"].as_array().unwrap().is_empty());
    assert!(response["evidence_set"]["id"].is_string());
    assert_eq!(response["evidence_set"]["query"], "SQLite local");
    assert!(response["evidence_set"]["node_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value.as_str() == Some(saved_id)));
}

#[test]
fn test_retrieve_context_blocked_in_incognito() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_save_memory(
            "Test memory".to_string(),
            "observation".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    server.set_incognito(true);

    // Retrieval should still work in incognito (read-only) but should NOT
    // reinforce connections (no weight updates)
    let result = server.handle_retrieve_context("test".to_string(), Some(10));
    // This is a design decision: we allow reads in incognito but skip reinforcement
    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(response["evidence_set"].is_null());
}

#[test]
fn test_feedback_recall_updates_confidence_idempotently() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let saved = server
        .handle_quick_save(
            "Use SQLite when you want local-first durability".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let memory_id = saved["id"].as_str().unwrap().to_string();

    let retrieved = server
        .handle_retrieve_context("SQLite durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let feedback = server
        .handle_feedback_recall(
            evidence_set_id.clone(),
            "helpful".to_string(),
            Some(vec![memory_id.clone()]),
            Some("replay-safe-key".to_string()),
        )
        .unwrap();
    let feedback: serde_json::Value = serde_json::from_str(&feedback).unwrap();
    assert_eq!(feedback["applied"].as_array().unwrap().len(), 1);

    let replay = server
        .handle_feedback_recall(
            evidence_set_id,
            "helpful".to_string(),
            Some(vec![memory_id.clone()]),
            Some("replay-safe-key".to_string()),
        )
        .unwrap();
    let replay: serde_json::Value = serde_json::from_str(&replay).unwrap();
    assert_eq!(replay["applied"].as_array().unwrap().len(), 0);
    assert_eq!(replay["skipped"].as_array().unwrap().len(), 1);

    let inspected = server.handle_inspect_memory(memory_id).unwrap();
    let inspected: serde_json::Value = serde_json::from_str(&inspected).unwrap();
    assert_eq!(inspected["helpful_count"], 1);
    assert_eq!(inspected["unhelpful_count"], 0);
    assert_eq!(inspected["node_kind"], "memory");
    assert!(inspected["confidence"].as_f64().unwrap() > 0.5);
    assert_eq!(inspected["effective_confidence"].as_f64().unwrap(), 0.5);
}

#[test]
fn test_reflect_context_returns_typed_packet() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Use SQLite for local-first durability".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    server
        .handle_quick_save(
            "Spreading activation helps connected recall".to_string(),
            "pattern".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("local durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"].as_str().unwrap();

    let reflected = server
        .handle_reflect_context(evidence_set_id.to_string(), Some(5), Some(5))
        .unwrap();
    let reflected: serde_json::Value = serde_json::from_str(&reflected).unwrap();

    assert_eq!(reflected["evidence_set_id"], evidence_set_id);
    assert_eq!(reflected["query"], "local durability");
    assert!(reflected["instruction"]
        .as_str()
        .unwrap()
        .contains("grounded"));
    assert!(!reflected["memory_items"].as_array().unwrap().is_empty());
    assert!(reflected["skill_items"].is_array());
    assert!(reflected["relationships"].is_array());
    assert!(reflected["assessment_items"].is_array());
}

#[test]
fn test_prepare_compression_strips_recalled_context_and_updates_status() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Use SQLite for local durability".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("SQLite durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let session_content = concat!(
        "The user wants to keep local-first sync practical.\n",
        "[Recalled memories — use these to inform your response naturally, never cite them directly]\n",
        "- (heuristic) Use SQLite for local durability\n",
        "They also want better multi-device conflict handling.\n"
    );

    let prepared = server
        .handle_prepare_compression(
            session_content.to_string(),
            Some(vec![evidence_set_id]),
            Some(45.0),
            Some("pre_compress".to_string()),
        )
        .unwrap();
    let prepared: serde_json::Value = serde_json::from_str(&prepared).unwrap();
    let sanitized = prepared["sanitized_session_content"].as_str().unwrap();

    assert!(!sanitized.contains("Use SQLite for local durability"));
    assert!(!sanitized.contains("[Recalled memories"));
    assert!(sanitized.contains("better multi-device conflict handling"));
    assert_eq!(prepared["reason"], "pre_compress");
    assert!(prepared["memory_proposal"]["instruction"]
        .as_str()
        .unwrap()
        .contains("Reflect on this session"));

    let status = server.handle_compression_status().unwrap();
    let status: serde_json::Value = serde_json::from_str(&status).unwrap();
    assert_eq!(status["compression_checkpoint"]["pre_compression_calls"], 1);
    assert_eq!(
        status["compression_checkpoint"]["last_pre_compression_reason"],
        "pre_compress"
    );
}

#[test]
fn test_prepare_compression_preserves_plain_user_prose_even_if_it_matches_memory_text() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Use SQLite for local durability".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("SQLite durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let prepared = server
        .handle_prepare_compression(
            "The user still says use SQLite for local durability in this fresh message."
                .to_string(),
            Some(vec![evidence_set_id]),
            Some(5.0),
            Some("pre_compress".to_string()),
        )
        .unwrap();
    let prepared: serde_json::Value = serde_json::from_str(&prepared).unwrap();

    assert!(prepared["sanitized_session_content"]
        .as_str()
        .unwrap()
        .contains("use SQLite for local durability"));
    assert!(prepared["suppressed_evidence_set_ids"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn test_retrieve_context_response_hash_is_stable_for_same_query_and_graph() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Stable evidence hashing matters for replay safety".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let first = server
        .handle_retrieve_context("replay safety".to_string(), Some(10))
        .unwrap();
    let first: serde_json::Value = serde_json::from_str(&first).unwrap();

    let second = server
        .handle_retrieve_context("replay safety".to_string(), Some(10))
        .unwrap();
    let second: serde_json::Value = serde_json::from_str(&second).unwrap();

    assert_eq!(
        first["evidence_set"]["response_hash"],
        second["evidence_set"]["response_hash"]
    );
}

#[test]
fn test_skill_lifecycle_from_evidence_set() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Use SQLite for local-first durability".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    server
        .handle_quick_save(
            "Prefer WAL mode for concurrent local writes".to_string(),
            "pattern".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("local durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let evidence_node_ids = retrieved["evidence_set"]["node_ids"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    let proposed = server
        .handle_propose_skills(evidence_set_id.clone(), Some(2))
        .unwrap();
    let proposed: serde_json::Value = serde_json::from_str(&proposed).unwrap();
    assert_eq!(proposed["evidence_set_id"], evidence_set_id);
    assert!(proposed["instruction"]
        .as_str()
        .unwrap()
        .contains("save_skill"));

    let saved = server
        .handle_save_skill(
            "Local SQLite Setup".to_string(),
            "Procedure for initializing a durable local SQLite database".to_string(),
            "When starting a local-first SQLite store".to_string(),
            vec![
                "Create the database file".to_string(),
                "Enable WAL mode".to_string(),
                "Enable foreign keys".to_string(),
            ],
            Some(vec![
                "Use for local-first or single-user graph storage".to_string()
            ]),
            evidence_set_id.clone(),
            evidence_node_ids.clone(),
            None,
            None,
        )
        .unwrap();
    let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let skill_id = saved["skill"]["node_id"].as_str().unwrap().to_string();
    assert_eq!(
        saved["evidence_node_ids"].as_array().unwrap().len(),
        evidence_node_ids.len()
    );

    let inspected = server.handle_inspect_skill(skill_id.clone()).unwrap();
    let inspected: serde_json::Value = serde_json::from_str(&inspected).unwrap();
    assert_eq!(inspected["skill"]["name"], "Local SQLite Setup");
    assert_eq!(
        inspected["evidence_node_ids"].as_array().unwrap().len(),
        evidence_node_ids.len()
    );

    let retrieved_skills = server
        .handle_retrieve_skills("SQLite WAL".to_string(), Some(10))
        .unwrap();
    let retrieved_skills: serde_json::Value = serde_json::from_str(&retrieved_skills).unwrap();
    assert!(!retrieved_skills.as_array().unwrap().is_empty());
    assert_eq!(
        retrieved_skills[0]["skill"]["node_id"].as_str().unwrap(),
        skill_id
    );

    let retrieve_context_with_skills = server
        .handle_retrieve_context("SQLite WAL".to_string(), Some(10))
        .unwrap();
    let retrieve_context_with_skills: serde_json::Value =
        serde_json::from_str(&retrieve_context_with_skills).unwrap();
    assert!(!retrieve_context_with_skills["skills"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        retrieve_context_with_skills["skills"][0]["skill"]["node_id"]
            .as_str()
            .unwrap(),
        skill_id
    );
}

#[test]
fn test_save_skill_rejects_fabricated_evidence_ids() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Use SQLite for local-first durability".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("local durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let err = server
        .handle_save_skill(
            "Bad Skill".to_string(),
            "Should not save".to_string(),
            "Never".to_string(),
            vec!["Invent evidence".to_string()],
            None,
            evidence_set_id,
            vec!["fabricated-node-id".to_string()],
            None,
            None,
        )
        .unwrap_err();

    assert!(err.contains("not present in evidence set"));
}

#[test]
fn test_detect_contradictions_persists_candidates_and_reflects_them() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Use SQLite for local durability".to_string(),
            "heuristic".to_string(),
            Some("positive".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
    server
        .handle_quick_save(
            "Avoid SQLite for local durability".to_string(),
            "heuristic".to_string(),
            Some("negative".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("SQLite durability".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let detected = server
        .handle_detect_contradictions(evidence_set_id.clone(), Some(5))
        .unwrap();
    let detected: serde_json::Value = serde_json::from_str(&detected).unwrap();
    let assessments = detected["assessments"].as_array().unwrap();
    assert_eq!(detected["evidence_set_id"], evidence_set_id);
    assert!(!assessments.is_empty());
    assert_eq!(assessments[0]["assessment_type"], "contradiction");
    assert_eq!(assessments[0]["status"], "candidate");

    let reflected = server
        .handle_reflect_context(evidence_set_id, Some(10), Some(10))
        .unwrap();
    let reflected: serde_json::Value = serde_json::from_str(&reflected).unwrap();
    assert!(!reflected["assessment_items"].as_array().unwrap().is_empty());
}

#[test]
fn test_dismissed_contradictions_are_suppressed_until_memories_change() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    server
        .handle_quick_save(
            "Prefer WAL mode for SQLite".to_string(),
            "pattern".to_string(),
            Some("positive".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();
    server
        .handle_quick_save(
            "Avoid WAL mode for SQLite".to_string(),
            "pattern".to_string(),
            Some("negative".to_string()),
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let retrieved = server
        .handle_retrieve_context("SQLite WAL".to_string(), Some(10))
        .unwrap();
    let retrieved: serde_json::Value = serde_json::from_str(&retrieved).unwrap();
    let evidence_set_id = retrieved["evidence_set"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let detected = server
        .handle_detect_contradictions(evidence_set_id.clone(), Some(5))
        .unwrap();
    let detected: serde_json::Value = serde_json::from_str(&detected).unwrap();
    let assessment_id = detected["assessments"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let dismissed = server.handle_dismiss_assessment(assessment_id).unwrap();
    let dismissed: serde_json::Value = serde_json::from_str(&dismissed).unwrap();
    assert_eq!(dismissed["status"], "dismissed");

    let redetected = server
        .handle_detect_contradictions(evidence_set_id, Some(5))
        .unwrap();
    let redetected: serde_json::Value = serde_json::from_str(&redetected).unwrap();
    assert!(redetected["assessments"].as_array().unwrap().is_empty());
}

#[test]
fn test_delete_memory_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let save_result = server
        .handle_save_memory(
            "To be deleted".to_string(),
            "observation".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    // Extract ID from response (JSON)
    let saved: serde_json::Value = serde_json::from_str(&save_result).unwrap();
    let id = saved["id"].as_str().unwrap().to_string();

    let result = server.handle_delete_memory(id.clone());
    assert!(result.is_ok());

    let inspect = server.handle_inspect_memory(id);
    assert!(inspect.is_ok());
    let inspected: serde_json::Value = serde_json::from_str(&inspect.unwrap()).unwrap();
    assert_eq!(inspected["status"], "deleted");
}

#[test]
fn test_update_memory_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let save_result = server
        .handle_save_memory(
            "Original content".to_string(),
            "decision".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let saved: serde_json::Value = serde_json::from_str(&save_result).unwrap();
    let id = saved["id"].as_str().unwrap().to_string();

    let result = server.handle_update_memory(id.clone(), "Updated content".to_string());
    assert!(result.is_ok());

    // Old should be superseded
    let old = server.handle_inspect_memory(id).unwrap();
    let old_v: serde_json::Value = serde_json::from_str(&old).unwrap();
    assert_eq!(old_v["status"], "superseded");
}

#[test]
fn test_memory_status_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    let result = server.handle_memory_status();
    assert!(result.is_ok());

    let stats: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(stats["total_impulses"], 0);
    assert_eq!(stats["total_memory_nodes"], 0);
    assert_eq!(stats["total_skill_nodes"], 0);
    assert_eq!(stats["total_graph_edges"], 0);
    assert_eq!(stats["total_assessments"], 0);
}

#[test]
fn test_set_incognito_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    assert!(!server.is_incognito());
    server.set_incognito(true);
    assert!(server.is_incognito());
    server.set_incognito(false);
    assert!(!server.is_incognito());
}

fn create_mcp_test_vault() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("test-note.md"),
        "# Test Note\n\nSome content about Rust and memory.\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("other.md"),
        "# Other\n\nLinks to [[test-note]].\n",
    )
    .unwrap();
    dir
}

#[test]
fn test_register_ghost_graph_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    let vault = create_mcp_test_vault();

    let result = server.handle_register_ghost_graph(
        "test-vault".to_string(),
        vault.path().to_str().unwrap().to_string(),
        Some("obsidian".to_string()),
        None,
    );

    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(response["name"], "test-vault");
    assert!(response["nodes_scanned"].as_i64().unwrap() >= 2);
}

#[test]
fn test_refresh_ghost_graph_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    let vault = create_mcp_test_vault();

    server
        .handle_register_ghost_graph(
            "test-vault".to_string(),
            vault.path().to_str().unwrap().to_string(),
            None,
            None,
        )
        .unwrap();

    let result = server.handle_refresh_ghost_graph("test-vault".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_pull_through_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);
    let vault = create_mcp_test_vault();

    server
        .handle_register_ghost_graph(
            "test-vault".to_string(),
            vault.path().to_str().unwrap().to_string(),
            None,
            None,
        )
        .unwrap();

    // Pull through by source and ref
    let result = server.handle_pull_through(
        "test-vault".to_string(),
        "test-note.md".to_string(),
        Some("session_only".to_string()),
    );

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("Rust"));
}

#[test]
fn test_backup_tool() {
    let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");
    let backup_path = tmp_dir.path().join("backup.db");

    let server = MemoryGraphServer::new(db_path.to_str().unwrap()).unwrap();

    // Save a memory so there's data
    server
        .handle_save_memory(
            "Backup test memory".to_string(),
            "observation".to_string(),
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();

    let result = server.handle_create_backup(backup_path.to_str().unwrap().to_string());
    assert!(result.is_ok());

    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert!(response["checksum"].as_str().unwrap().len() > 0);
    assert_eq!(response["impulse_count"], 1);
    assert!(response["size_bytes"].as_u64().unwrap() > 0);
}
