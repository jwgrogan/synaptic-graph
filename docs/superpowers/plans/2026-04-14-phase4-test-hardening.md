# Phase 4: Test Hardening and E2E Validation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run the full test suite, fix every failure, add missing edge case coverage, stress test the spreading activation engine at scale, and iterate until the suite is completely clean across multiple consecutive runs.

**Architecture:** No new features. This phase is exclusively about test quality, edge case coverage, and confirming the system works end-to-end under realistic conditions.

**Tech Stack:** Same as Phases 1-3. No new dependencies.

**Depends on:** Phases 1, 2, and 3 complete.

---

### Task 1: Full Test Suite — First Pass

- [ ] **Step 1: Run the complete test suite**

Run: `cargo test 2>&1`

Capture the full output. Note every failure.

- [ ] **Step 2: Fix every compilation error**

Read each error carefully. Common issues:
- Missing imports after Phase 2/3 model changes
- Type mismatches from model evolution across phases
- Missing method implementations referenced in tests

Fix each error. Do not skip or comment out tests.

- [ ] **Step 3: Fix every test failure**

For each failing test:
1. Read the assertion that failed
2. Determine if the test is wrong or the code is wrong
3. Fix the correct side
4. Re-run that specific test to confirm the fix

- [ ] **Step 4: Run full suite again**

Run: `cargo test 2>&1`
Expected: Fewer failures than Step 1. If not zero, repeat Step 3.

- [ ] **Step 5: Commit fixes**

```bash
git add -A
git commit -m "fix: resolve test failures from cross-phase integration"
```

---

### Task 2: Clippy Clean Pass

- [ ] **Step 1: Run clippy with all warnings**

Run: `cargo clippy -- -W clippy::all 2>&1`

- [ ] **Step 2: Fix all clippy warnings**

Address each warning. Common ones:
- Unnecessary clones
- Unused variables or imports
- Missing error handling
- Redundant closures
- Non-idiomatic patterns

- [ ] **Step 3: Run clippy again**

Run: `cargo clippy -- -W clippy::all 2>&1`
Expected: Zero warnings

- [ ] **Step 4: Run tests after clippy fixes**

Run: `cargo test 2>&1`
Expected: All PASS (clippy fixes should not break tests)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "fix: resolve all clippy warnings"
```

---

### Task 3: Edge Case Tests — Weight Mechanics

- [ ] **Step 1: Add edge case tests**

Add to `tests/test_weight.rs`:
```rust
#[test]
fn test_decay_with_zero_hours() {
    let w = weight::effective_weight(0.5, 0.0, DECAY_SEMANTIC);
    assert!((w - 0.5).abs() < 0.0001);
}

#[test]
fn test_decay_with_negative_hours_treated_as_zero() {
    // Should not panic or produce NaN
    let w = weight::effective_weight(0.5, -10.0, DECAY_SEMANTIC);
    assert!(w.is_finite());
    assert!(w > 0.0);
}

#[test]
fn test_decay_with_zero_weight() {
    let w = weight::effective_weight(0.0, 100.0, DECAY_SEMANTIC);
    assert_eq!(w, WEIGHT_FLOOR);
}

#[test]
fn test_decay_with_max_weight() {
    let w = weight::effective_weight(1.0, 0.0, DECAY_SEMANTIC);
    assert!((w - 1.0).abs() < 0.0001);
}

#[test]
fn test_reinforce_at_exactly_one() {
    let w = weight::reinforce(1.0);
    assert_eq!(w, 1.0);
}

#[test]
fn test_reinforce_at_zero() {
    let w = weight::reinforce(0.0);
    assert!((w - REINFORCEMENT_BUMP).abs() < 0.0001);
}

#[test]
fn test_decay_over_one_year() {
    let semantic = weight::effective_weight(1.0, 8760.0, DECAY_SEMANTIC);
    let episodic = weight::effective_weight(1.0, 8760.0, DECAY_EPISODIC);

    // Semantic should still be meaningful after a year
    assert!(semantic > 0.01, "Semantic memory should persist over a year: {}", semantic);

    // Episodic should be near floor
    assert!(episodic < 0.01, "Episodic memory should fade within a year: {}", episodic);

    // Both above floor
    assert!(semantic >= WEIGHT_FLOOR);
    assert!(episodic >= WEIGHT_FLOOR);
}

#[test]
fn test_repeated_reinforcement_convergence() {
    // Reinforce 100 times — should cap at 1.0
    let mut w = 0.1;
    for _ in 0..100 {
        w = weight::reinforce(w);
    }
    assert_eq!(w, 1.0);
}
```

- [ ] **Step 2: Run weight tests**

Run: `cargo test test_weight 2>&1`
Expected: All PASS. If any fail, fix the weight module.

- [ ] **Step 3: Commit**

```bash
git add tests/test_weight.rs src/weight.rs
git commit -m "test: add edge case tests for weight mechanics"
```

---

### Task 4: Edge Case Tests — Activation Engine

- [ ] **Step 1: Add stress and edge case tests**

Add to `tests/test_activation.rs`:
```rust
#[test]
fn test_activation_with_empty_graph() {
    let db = common::test_db();
    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "anything".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();
    assert!(result.memories.is_empty());
}

#[test]
fn test_activation_with_single_node() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db, "Single isolated node",
        ImpulseType::Observation, EmotionalValence::Neutral,
        EngagementLevel::Medium, vec![], "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "single isolated".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();
    assert_eq!(result.memories.len(), 1);
}

#[test]
fn test_activation_with_disconnected_clusters() {
    let db = common::test_db();

    // Cluster 1: A -> B
    let a = ingestion::save_and_confirm(
        &db, "Rust ownership patterns",
        ImpulseType::Heuristic, EmotionalValence::Positive,
        EngagementLevel::High, vec![], "test",
    ).unwrap();
    ingestion::save_and_confirm_with_connections(
        &db, "Borrow checker prevents data races",
        ImpulseType::Pattern, EmotionalValence::Positive,
        EngagementLevel::Medium, vec![], "test",
        &[(a.id.clone(), "relates_to".to_string(), 0.8)],
    ).unwrap();

    // Cluster 2: C -> D (disconnected from cluster 1)
    let c = ingestion::save_and_confirm(
        &db, "PostgreSQL connection pooling",
        ImpulseType::Heuristic, EmotionalValence::Neutral,
        EngagementLevel::Medium, vec![], "test",
    ).unwrap();
    ingestion::save_and_confirm_with_connections(
        &db, "Database connections are expensive to create",
        ImpulseType::Pattern, EmotionalValence::Neutral,
        EngagementLevel::Low, vec![], "test",
        &[(c.id.clone(), "relates_to".to_string(), 0.7)],
    ).unwrap();

    let engine = ActivationEngine::new(&db);

    // Query about Rust should NOT activate PostgreSQL cluster
    let result = engine.retrieve(&RetrievalRequest {
        query: "Rust ownership borrow".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();

    let has_postgres = result.memories.iter().any(|m| m.impulse.content.contains("PostgreSQL"));
    assert!(!has_postgres, "Disconnected cluster should not activate");
}

#[test]
fn test_activation_at_scale_100_nodes() {
    let db = common::test_db();
    let mut ids = Vec::new();

    // Create 100 impulses
    for i in 0..100 {
        let impulse = ingestion::save_and_confirm(
            &db,
            &format!("Memory node {} about topic {}", i, i % 10),
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Medium,
            vec![],
            "test",
        ).unwrap();
        ids.push(impulse.id);
    }

    // Create connections: each node to the next (chain)
    for i in 0..99 {
        db.insert_connection(&NewConnection {
            source_id: ids[i].clone(),
            target_id: ids[i + 1].clone(),
            weight: 0.5,
            relationship: "next".to_string(),
        }).unwrap();
    }

    let engine = ActivationEngine::new(&db);

    // Time the retrieval
    let start = std::time::Instant::now();
    let result = engine.retrieve(&RetrievalRequest {
        query: "Memory node 0 topic".to_string(),
        max_results: 10,
        max_hops: 5,
    }).unwrap();
    let duration = start.elapsed();

    assert!(!result.memories.is_empty());
    // Should complete in under 1 second at this scale
    assert!(duration.as_millis() < 1000, "Retrieval took {}ms at 100 nodes", duration.as_millis());
}

#[test]
fn test_activation_at_scale_1000_nodes() {
    let db = common::test_db();
    let mut ids = Vec::new();

    for i in 0..1000 {
        let impulse = ingestion::save_and_confirm(
            &db,
            &format!("Scale test node {} category {}", i, i % 50),
            ImpulseType::Observation,
            EmotionalValence::Neutral,
            EngagementLevel::Medium,
            vec![],
            "test",
        ).unwrap();
        ids.push(impulse.id);
    }

    // Create a more realistic graph: each node connected to 3 random others
    for i in 0..1000 {
        for offset in [1, 7, 23] {
            let target = (i + offset) % 1000;
            let _ = db.insert_connection(&NewConnection {
                source_id: ids[i].clone(),
                target_id: ids[target].clone(),
                weight: 0.4,
                relationship: "relates_to".to_string(),
            });
        }
    }

    let engine = ActivationEngine::new(&db);

    let start = std::time::Instant::now();
    let result = engine.retrieve(&RetrievalRequest {
        query: "Scale test node category".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();
    let duration = start.elapsed();

    assert!(!result.memories.is_empty());
    // TRD says target is under 200ms for 10K nodes; 1K should be well under
    assert!(duration.as_millis() < 500, "Retrieval took {}ms at 1000 nodes", duration.as_millis());
}

#[test]
fn test_activation_with_cycle() {
    let db = common::test_db();

    // A -> B -> C -> A (cycle)
    let a = ingestion::save_and_confirm(
        &db, "Cycle node A", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium, vec![], "test",
    ).unwrap();
    let b = ingestion::save_and_confirm(
        &db, "Cycle node B", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium, vec![], "test",
    ).unwrap();
    let c = ingestion::save_and_confirm(
        &db, "Cycle node C", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium, vec![], "test",
    ).unwrap();

    db.insert_connection(&NewConnection {
        source_id: a.id.clone(), target_id: b.id.clone(),
        weight: 0.8, relationship: "next".to_string(),
    }).unwrap();
    db.insert_connection(&NewConnection {
        source_id: b.id.clone(), target_id: c.id.clone(),
        weight: 0.8, relationship: "next".to_string(),
    }).unwrap();
    db.insert_connection(&NewConnection {
        source_id: c.id.clone(), target_id: a.id.clone(),
        weight: 0.8, relationship: "next".to_string(),
    }).unwrap();

    let engine = ActivationEngine::new(&db);

    // Should not infinite loop
    let start = std::time::Instant::now();
    let result = engine.retrieve(&RetrievalRequest {
        query: "Cycle node".to_string(),
        max_results: 10,
        max_hops: 10,
    }).unwrap();
    let duration = start.elapsed();

    assert!(!result.memories.is_empty());
    assert!(duration.as_millis() < 1000, "Cycle detection failed — took {}ms", duration.as_millis());
}

#[test]
fn test_activation_max_results_zero() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db, "Some memory", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium, vec![], "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "Some memory".to_string(),
        max_results: 0,
        max_hops: 3,
    }).unwrap();

    assert!(result.memories.is_empty());
}

#[test]
fn test_activation_empty_query() {
    let db = common::test_db();
    ingestion::save_and_confirm(
        &db, "Memory", ImpulseType::Observation,
        EmotionalValence::Neutral, EngagementLevel::Medium, vec![], "test",
    ).unwrap();

    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: "".to_string(),
        max_results: 10,
        max_hops: 3,
    }).unwrap();

    // Empty query should return empty results (FTS won't match)
    assert!(result.memories.is_empty());
}
```

- [ ] **Step 2: Run activation tests**

Run: `cargo test test_activation 2>&1`
Expected: All PASS. Fix any failures.

- [ ] **Step 3: Commit**

```bash
git add tests/test_activation.rs src/activation.rs
git commit -m "test: add edge case and stress tests for activation engine including cycles and scale"
```

---

### Task 5: Edge Case Tests — Redaction

- [ ] **Step 1: Add edge case tests**

Add to `tests/test_redaction.rs`:
```rust
#[test]
fn test_redact_empty_string() {
    let result = redaction::redact("");
    assert_eq!(result.clean_content, "");
    assert!(result.redactions.is_empty());
}

#[test]
fn test_redact_very_long_content() {
    let long = "a".repeat(100_000);
    let result = redaction::redact(&long);
    assert_eq!(result.clean_content.len(), 100_000);
    assert!(result.redactions.is_empty());
}

#[test]
fn test_redact_github_token() {
    let input = "Use token ghp_1234567890abcdefghijklmnopqrstuvwxyz12";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("ghp_"));
}

#[test]
fn test_redact_preserves_surrounding_text() {
    let input = "Before AKIAIOSFODNN7EXAMPLE after";
    let result = redaction::redact(input);
    assert!(result.clean_content.contains("Before"));
    assert!(result.clean_content.contains("after"));
    assert!(result.clean_content.contains("[REDACTED]"));
}

#[test]
fn test_redact_unicode_content() {
    let input = "Use key AKIAIOSFODNN7EXAMPLE for \u{1F600} emoji support";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(result.clean_content.contains("\u{1F600}"));
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test test_redact 2>&1`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add tests/test_redaction.rs
git commit -m "test: add edge case tests for redaction module"
```

---

### Task 6: Edge Case Tests — Ghost Graph and Backup

- [ ] **Step 1: Add edge cases for ghost operations**

Add to `tests/test_ghost.rs`:
```rust
#[test]
fn test_scan_empty_directory() {
    let dir = TempDir::new().unwrap();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };
    let result = scan_directory(dir.path(), &config).unwrap();
    assert!(result.nodes.is_empty());
    assert!(result.links.is_empty());
}

#[test]
fn test_scan_no_matching_extensions() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt"), "Not markdown").unwrap();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };
    let result = scan_directory(dir.path(), &config).unwrap();
    assert!(result.nodes.is_empty());
}

#[test]
fn test_register_same_source_twice() {
    let db = common::test_db();
    let vault = create_test_vault();
    let config = ScanConfig {
        extensions: vec!["md".to_string()],
        ignore_patterns: vec![],
    };

    ghost::register_and_scan(&db, "vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();
    // Register again — should not error, should update
    ghost::register_and_scan(&db, "vault", vault.path().to_str().unwrap(), "obsidian", &config).unwrap();

    let sources = db.list_ghost_sources().unwrap();
    assert_eq!(sources.len(), 1);
}
```

Add to `tests/test_backup.rs`:
```rust
#[test]
fn test_backup_empty_database() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("empty.db");
    let backup_path = tmp.path().join("backup.db");

    let db = Database::open(db_path.to_str().unwrap()).unwrap();
    let result = backup::create_backup(&db, backup_path.to_str().unwrap()).unwrap();

    assert_eq!(result.impulse_count, 0);
    assert!(backup_path.exists());
}

#[test]
fn test_restore_nonexistent_backup() {
    let result = backup::restore_backup("/nonexistent/path.db", "/tmp/out.db", "fake-checksum");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test 2>&1`
Expected: All PASS

- [ ] **Step 3: Commit**

```bash
git add tests/
git commit -m "test: add edge case tests for ghost graph and backup operations"
```

---

### Task 7: Consecutive Clean Runs

This is the final gate. The test suite must pass cleanly across multiple consecutive runs with no flaky tests.

- [ ] **Step 1: First clean run**

Run: `cargo test 2>&1`
Expected: ALL PASS, zero failures

If any fail, go back and fix. Do not proceed until this passes.

- [ ] **Step 2: Second clean run**

Run: `cargo test 2>&1`
Expected: ALL PASS, zero failures

This catches any tests that depend on global state or test ordering.

- [ ] **Step 3: Third clean run**

Run: `cargo test 2>&1`
Expected: ALL PASS, zero failures

Three consecutive clean passes means the suite is stable.

- [ ] **Step 4: Run with threads=1 to catch race conditions**

Run: `cargo test -- --test-threads=1 2>&1`
Expected: ALL PASS

This catches any concurrency issues between tests sharing state.

- [ ] **Step 5: Run release tests**

Run: `cargo test --release 2>&1`
Expected: ALL PASS

Release mode can expose different behavior due to optimizations.

- [ ] **Step 6: Final test count and summary**

Run: `cargo test 2>&1 | tail -5`
Expected: Report showing total test count and "0 failed"

- [ ] **Step 7: Build release binary**

Run: `cargo build --release 2>&1`
Expected: Clean build

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "test: all tests passing — 3 consecutive clean runs verified"
```

---

## Completion Criteria

Phase 4 is complete when:

1. `cargo test` passes with zero failures across 3 consecutive runs
2. `cargo test -- --test-threads=1` passes (no race conditions)
3. `cargo test --release` passes (no optimization-sensitive bugs)
4. `cargo clippy -- -W clippy::all` produces zero warnings
5. `cargo build --release` succeeds
6. Every PRD validation criterion from all three phases has a passing test

This is the quality gate before the system is ready for real self-use.
