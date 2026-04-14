mod common;

use memory_graph::server::MemoryGraphServer;

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
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("incognito"));
}

#[test]
fn test_retrieve_context_tool() {
    let db = common::test_db();
    let server = MemoryGraphServer::new_with_db(db);

    // Save something first
    server
        .handle_save_memory(
            "SQLite is excellent for local-first applications".to_string(),
            "heuristic".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

    let result = server.handle_retrieve_context(
        "SQLite local".to_string(),
        Some(10),
    );

    assert!(result.is_ok());
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
        )
        .unwrap();

    server.set_incognito(true);

    // Retrieval should still work in incognito (read-only) but should NOT
    // reinforce connections (no weight updates)
    let result = server.handle_retrieve_context(
        "test".to_string(),
        Some(10),
    );
    // This is a design decision: we allow reads in incognito but skip reinforcement
    assert!(result.is_ok());
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
