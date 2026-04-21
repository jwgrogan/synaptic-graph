# Phase 11: Canonical Graph Kernel, Reflective Retrieval, and Procedural Skills — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate `synaptic-graph` from its current memory-specific schema to a canonical typed graph kernel, then layer evidence-set retrieval, validated reflection, and procedural skills on top of that substrate.

**Current state:** The repo has:

- `impulses` + `connections`
- spreading activation retrieval over confirmed impulses
- ghost graphs with separate tables
- candidate confirmation
- sync/export and cross-device merge
- MCP server + CLI

**Target state:** One canonical graph model:

- `nodes`
- `edges`
- payload tables
- `assessments`
- `evidence_sets`
- `feedback_events`
- `node_versions`

This plan is intentionally not an MVP extension. It is the migration path to the architecture described in [2026-04-21-reflective-retrieval-and-procedural-skills.md](/Users/jwgrogan/GitHub/memory-graph/docs/superpowers/specs/2026-04-21-reflective-retrieval-and-procedural-skills.md).

---

## Delivery Strategy

Implement in two layers:

1. **Foundation migration**
   Move to the canonical graph kernel while preserving current behavior.
2. **Capability layering**
   Add evidence sets, feedback, reflection, contradiction assessments, and procedural skills on top.

Do not build procedural skills as a parallel subsystem while legacy `impulses` remain canonical.

---

## File Structure

### Existing files to modify

```text
synaptic-graph/
  src/
    activation.rs
    db.rs
    extraction.rs
    ghost/
    lib.rs
    main.rs
    markdown.rs
    models.rs
    server.rs
    sync.rs
  tests/
    test_activation.rs
    test_db.rs
    test_extraction.rs
    test_ghost.rs
    test_mcp.rs
    test_sync.rs
```

### New files to create

```text
synaptic-graph/
  src/
    graph.rs            # node/edge CRUD and payload-aware graph helpers
    confidence.rs       # confidence derivation, ranking helpers, worked formulas
    evidence.rs         # evidence-set lifecycle, hashing, idempotent feedback
    reflection.rs       # reflection packet assembly over evidence sets
    assessments.rs      # contradiction/validation/supersession assessments
  tests/
    test_graph.rs
    test_confidence.rs
    test_evidence.rs
    test_reflection.rs
    test_assessments.rs
    test_validation_p4.rs
```

---

## Canonical Schema Plan

## Migration policy

Before any Phase 11 schema change lands:

- [ ] Add explicit schema version handling in [db.rs](/Users/jwgrogan/GitHub/memory-graph/src/db.rs)
- [ ] Make [main.rs](/Users/jwgrogan/GitHub/memory-graph/src/main.rs) `cli merge` fail loudly on incompatible schema versions
- [ ] Extend sync/export metadata in [sync.rs](/Users/jwgrogan/GitHub/memory-graph/src/sync.rs) to carry schema version and feature flags

### New canonical tables

- [ ] `nodes`
- [ ] `edges`
- [ ] `node_payload_memory`
- [ ] `node_payload_skill`
- [ ] `node_payload_ghost`
- [ ] `assessments`
- [ ] `evidence_sets`
- [ ] `feedback_events`
- [ ] `node_versions`
- [ ] `memory_fts`
- [ ] `skill_fts`

### Recommended schema

```sql
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                 -- memory, skill, ghost
    status TEXT NOT NULL,               -- candidate, confirmed, superseded, deleted
    weight REAL NOT NULL,
    confidence REAL NOT NULL,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    unhelpful_count INTEGER NOT NULL DEFAULT 0,
    emotional_valence TEXT NOT NULL DEFAULT 'neutral',
    engagement_level TEXT NOT NULL DEFAULT 'medium',
    source_type TEXT NOT NULL,
    source_ref TEXT NOT NULL DEFAULT '',
    source_provider TEXT NOT NULL DEFAULT 'unknown',
    source_account TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed_at TEXT NOT NULL,
    last_validated_at TEXT
);

CREATE TABLE IF NOT EXISTS edges (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    weight REAL NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.5,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    unhelpful_count INTEGER NOT NULL DEFAULT 0,
    traversal_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_traversed_at TEXT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES nodes(id),
    FOREIGN KEY (target_id) REFERENCES nodes(id)
);

CREATE TABLE IF NOT EXISTS node_payload_memory (
    node_id TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL,
    content TEXT NOT NULL,
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

CREATE TABLE IF NOT EXISTS node_payload_skill (
    node_id TEXT PRIMARY KEY,
    trigger_text TEXT NOT NULL,
    summary TEXT NOT NULL,
    procedure_json TEXT NOT NULL,
    prerequisites_json TEXT NOT NULL DEFAULT '[]',
    anti_patterns_json TEXT NOT NULL DEFAULT '[]',
    success_signals_json TEXT NOT NULL DEFAULT '[]',
    version INTEGER NOT NULL DEFAULT 1,
    last_used_at TEXT,
    use_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

CREATE TABLE IF NOT EXISTS node_payload_ghost (
    node_id TEXT PRIMARY KEY,
    source_graph TEXT NOT NULL,
    external_ref TEXT NOT NULL,
    title TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    FOREIGN KEY (node_id) REFERENCES nodes(id)
);

CREATE TABLE IF NOT EXISTS assessments (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,                 -- contradiction, validation, supersession, low_confidence
    status TEXT NOT NULL,               -- candidate, confirmed, dismissed, resolved
    subject_node_id TEXT,
    object_node_id TEXT,
    subject_edge_id TEXT,
    object_edge_id TEXT,
    confidence REAL NOT NULL DEFAULT 0.5,
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE TABLE IF NOT EXISTS evidence_sets (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    mode TEXT NOT NULL,
    root_node_ids_json TEXT NOT NULL DEFAULT '[]',
    node_ids_json TEXT NOT NULL DEFAULT '[]',
    edge_ids_json TEXT NOT NULL DEFAULT '[]',
    assessment_ids_json TEXT NOT NULL DEFAULT '[]',
    content_hashes_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    truncated INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS feedback_events (
    id TEXT PRIMARY KEY,
    evidence_set_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    notes TEXT,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(evidence_set_id, idempotency_key)
);

CREATE TABLE IF NOT EXISTS node_versions (
    id TEXT PRIMARY KEY,
    node_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    payload_json TEXT NOT NULL,
    source_ref TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);
```

---

## MCP Tool Surface

### New tools

- `reflect_context`
- `feedback_recall`
- `detect_contradictions`
- `propose_skills`
- `save_skill`
- `quick_save_skill`
- `confirm_skill_proposal`
- `dismiss_skill_proposal`
- `list_skill_candidates`
- `retrieve_skills`
- `inspect_skill`
- `update_skill`
- `delete_skill`

### Existing tools to migrate

- `save_memory`
- `quick_save`
- `retrieve_context`
- `inspect_memory`
- `link_memories`
- `unlink_memories`
- ghost graph tools
- `memory_status`

The user-facing tool names can stay stable even as the backend moves to `nodes` / `edges`.

---

## Param Struct Additions

Add these structs in [server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs) once the kernel migration is underway.

```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ReflectContextParams {
    pub query: String,
    #[schemars(default)]
    pub mode: Option<String>,          // answer | pattern | decision | skill
    #[schemars(default)]
    pub max_nodes: Option<usize>,
    #[schemars(default)]
    pub max_skills: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FeedbackRecallParams {
    pub evidence_set_id: String,
    pub outcome: String,               // helpful | unhelpful | mixed
    pub idempotency_key: String,
    #[schemars(default)]
    pub notes: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DetectContradictionsParams {
    #[schemars(default)]
    pub query: Option<String>,
    #[schemars(default)]
    pub node_id: Option<String>,
    #[schemars(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ProposeSkillsParams {
    pub session_content: String,
    #[schemars(default)]
    pub session_duration_minutes: Option<f64>,
    #[schemars(default)]
    pub reason: Option<String>,        // end_session | pre_compress | explicit_review
    #[schemars(default)]
    pub source_ref: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SaveSkillParams {
    pub evidence_set_id: String,
    pub name: String,
    pub trigger_text: String,
    pub summary: String,
    pub steps: Vec<String>,
    #[schemars(default)]
    pub prerequisites: Option<Vec<String>>,
    #[schemars(default)]
    pub anti_patterns: Option<Vec<String>>,
    #[schemars(default)]
    pub success_signals: Option<Vec<String>>,
    pub evidence_node_ids: Vec<String>,
    #[schemars(default)]
    pub source_ref: Option<String>,
}
```

`save_skill` and `quick_save_skill` should validate `evidence_node_ids` against the referenced `evidence_set_id`.

---

## Task 1: Schema Versioning and Merge Safety

**Files:**
- Modify: [src/db.rs](/Users/jwgrogan/GitHub/memory-graph/src/db.rs)
- Modify: [src/main.rs](/Users/jwgrogan/GitHub/memory-graph/src/main.rs)
- Modify: [src/sync.rs](/Users/jwgrogan/GitHub/memory-graph/src/sync.rs)
- Modify: [tests/test_db.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_db.rs)
- Modify: [tests/test_sync.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_sync.rs)

- [ ] **Step 1: Add schema version table enforcement**

Require:

- `schema_version` row to exist
- major incompatible versions to fail merge

- [ ] **Step 2: Gate CLI merge**

Update [main.rs](/Users/jwgrogan/GitHub/memory-graph/src/main.rs:104) so `cli_merge`:

- reads schema version from both DBs
- aborts loudly on mismatch
- prints actionable recovery guidance

- [ ] **Step 3: Include schema metadata in sync export**

Extend sync snapshots with:

- schema version
- enabled feature flags

- [ ] **Step 4: Add regression tests**

Verify:

- merge fails on incompatible schema versions
- sync metadata round-trips correctly

---

## Task 2: Canonical Graph Kernel Substrate

**Files:**
- Modify: [src/models.rs](/Users/jwgrogan/GitHub/memory-graph/src/models.rs)
- Modify: [src/db.rs](/Users/jwgrogan/GitHub/memory-graph/src/db.rs)
- Modify: [src/lib.rs](/Users/jwgrogan/GitHub/memory-graph/src/lib.rs)
- Create: [src/graph.rs](/Users/jwgrogan/GitHub/memory-graph/src/graph.rs)
- Create: [tests/test_graph.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_graph.rs)

- [ ] **Step 1: Add canonical types**

In `models.rs`, add:

- `NodeKind`
- `NodeStatus`
- `Node`
- `Edge`
- `MemoryPayload`
- `SkillPayload`
- `GhostPayload`
- `Assessment`
- `AssessmentKind`
- `AssessmentStatus`
- `NodeVersion`

- [ ] **Step 2: Create canonical schema**

Implement the new tables in `db.rs`.

- [ ] **Step 3: Add graph CRUD in `graph.rs`**

Implement:

- `insert_node`
- `get_node`
- `list_nodes`
- `insert_edge`
- `get_edge`
- `list_edges_for_node`
- `insert_memory_payload`
- `insert_skill_payload`
- `insert_ghost_payload`

- [ ] **Step 4: Add search indexes**

Implement:

- `memory_fts`
- `skill_fts`
- search helpers by canonical node id

- [ ] **Step 5: Add graph tests**

Verify:

- node/edge CRUD
- payload linkage
- FTS alignment

---

## Task 3: Legacy-to-Canonical Migration and Dual Read

**Files:**
- Modify: [src/db.rs](/Users/jwgrogan/GitHub/memory-graph/src/db.rs)
- Create: [src/graph.rs](/Users/jwgrogan/GitHub/memory-graph/src/graph.rs)
- Modify: [tests/test_db.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_db.rs)
- Create: [tests/test_validation_p4.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_validation_p4.rs)

- [ ] **Step 1: Backfill legacy tables into canonical tables**

Map:

- `impulses` -> `nodes(kind=memory)` + `node_payload_memory`
- `connections` -> `edges`
- `ghost_nodes` -> `nodes(kind=ghost)` + `node_payload_ghost`
- `ghost_connections` -> `edges`

- [ ] **Step 2: Seed initial confidence**

For migrated legacy nodes:

```text
confidence = 0.5 + 0.2 * (weight - 0.5)
```

- [ ] **Step 3: Add dual-read verification helpers**

Temporary validation functions should compare:

- old retrieval results
- new canonical retrieval results

on the same seeded datasets.

- [ ] **Step 4: Do not remove legacy tables yet**

Keep legacy tables temporarily for:

- backfill verification
- rollback fallback during development

This is transitional only.

---

## Task 4: Port Spreading Activation to Canonical Nodes and Edges

**Files:**
- Modify: [src/activation.rs](/Users/jwgrogan/GitHub/memory-graph/src/activation.rs)
- Modify: [src/graph.rs](/Users/jwgrogan/GitHub/memory-graph/src/graph.rs)
- Modify: [tests/test_activation.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_activation.rs)

- [ ] **Step 1: Rework retrieval to operate on nodes**

Replace impulse-specific access with canonical node access plus payload lookups.

- [ ] **Step 2: Keep behavior parity for memory retrieval**

Before any reflective features land, retrieval parity with current memory behavior must hold.

- [ ] **Step 3: Make retrieval aware of skill nodes**

Skills are nodes and should be retrievable through the same system, subject to mode-specific filtering.

- [ ] **Step 4: Return canonical node/edge IDs**

Future evidence-set assembly depends on this.

---

## Task 5: Confidence Model and Ranking

**Files:**
- Create: [src/confidence.rs](/Users/jwgrogan/GitHub/memory-graph/src/confidence.rs)
- Modify: [src/activation.rs](/Users/jwgrogan/GitHub/memory-graph/src/activation.rs)
- Modify: [src/graph.rs](/Users/jwgrogan/GitHub/memory-graph/src/graph.rs)
- Create: [tests/test_confidence.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_confidence.rs)

- [ ] **Step 1: Implement derived confidence**

Implement:

```rust
pub fn derived_confidence(helpful: i64, unhelpful: i64) -> f64
```

using the Beta(1,1) prior:

```text
(helpful + 1) / (helpful + unhelpful + 2)
```

- [ ] **Step 2: Implement confidence gate**

If total feedback count < 3:

- `effective_confidence = 0.5`

- [ ] **Step 3: Implement ranking helpers**

Implement deterministic helpers:

- `salience_multiplier(weight)`
- `confidence_multiplier(confidence, sample_count)`
- `assessment_multiplier(...)`

- [ ] **Step 4: Add worked-example tests**

Tests should verify ranking on explicit tuples, not just smoke behavior.

---

## Task 6: Evidence Sets and Replay-Safe Feedback

**Files:**
- Create: [src/evidence.rs](/Users/jwgrogan/GitHub/memory-graph/src/evidence.rs)
- Modify: [src/server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs)
- Modify: [src/activation.rs](/Users/jwgrogan/GitHub/memory-graph/src/activation.rs)
- Create: [tests/test_evidence.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_evidence.rs)

- [ ] **Step 1: Add evidence-set types**

Implement:

- `EvidenceSet`
- `FeedbackEvent`

- [ ] **Step 2: Create evidence sets from retrieval**

Every `retrieve_context` should persist:

- node ids
- edge ids
- assessment ids
- content hashes
- expiration timestamp

- [ ] **Step 3: Add feedback lifecycle**

Implement:

- TTL enforcement
- idempotency via `(evidence_set_id, idempotency_key)`
- audit trail

- [ ] **Step 4: Add `feedback_recall`**

Mutate:

- node helpful/unhelpful counts
- optionally edge helpful/unhelpful counts only after node behavior is validated

Do not allow replay.

---

## Task 7: Reflective Retrieval

**Files:**
- Create: [src/reflection.rs](/Users/jwgrogan/GitHub/memory-graph/src/reflection.rs)
- Modify: [src/server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs)
- Modify: [tests/test_mcp.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_mcp.rs)
- Create: [tests/test_reflection.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_reflection.rs)

- [ ] **Step 1: Add reflection packet type**

Implement a typed packet, not free-form prose.

- [ ] **Step 2: Add hard caps**

Defaults:

- max nodes = 20
- max skill nodes = 5
- payload target size <= 8KB

- [ ] **Step 3: Add `reflect_context`**

Reflection should:

- reuse or create an evidence set
- build a typed packet for `answer`, `pattern`, `decision`, or `skill`
- set `truncated` explicitly if needed

- [ ] **Step 4: Add CLI parity only if useful for testing**

This is optional, not required for MCP completeness.

---

## Task 8: Procedural Skills as Typed Nodes

**Files:**
- Modify: [src/graph.rs](/Users/jwgrogan/GitHub/memory-graph/src/graph.rs)
- Modify: [src/server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs)
- Modify: [src/models.rs](/Users/jwgrogan/GitHub/memory-graph/src/models.rs)
- Create: [tests/test_validation_p4.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_validation_p4.rs)

- [ ] **Step 1: Implement skill node helpers**

Implement:

- `save_skill_candidate`
- `save_and_confirm_skill`
- `get_skill`
- `update_skill`
- `delete_skill`
- `list_skill_candidates`

- [ ] **Step 2: Use edges for evidence**

Represent supporting evidence with canonical edges:

- `evidence_for`
- `example_for`
- `contradicts`

Do not create a separate `skill_evidence` subsystem.

- [ ] **Step 3: Add version history**

Persist old skill payload in `node_versions` on every update.

- [ ] **Step 4: Add retrieval**

`retrieve_skills(query)` should return skill nodes, but `retrieve_context` should also surface relevant skills by default because they are graph-native nodes.

---

## Task 9: Assessments and Contradictions

**Files:**
- Create: [src/assessments.rs](/Users/jwgrogan/GitHub/memory-graph/src/assessments.rs)
- Modify: [src/server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs)
- Create: [tests/test_assessments.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_assessments.rs)

- [ ] **Step 1: Implement contradiction candidate generation**

Bound candidate selection by:

- strong overlap
- same tags / subject neighborhood
- not recently dismissed

- [ ] **Step 2: Persist dismissal memory**

Dismissed contradiction assessments must suppress repeat noise until a subject node materially changes.

- [ ] **Step 3: Apply ranking penalty only for confirmed contradictions**

Heuristic candidates should surface in reflection but should not affect ranking like confirmed contradictions.

---

## Task 10: Proposal Pipeline and Provenance-Aware Fencing

**Files:**
- Modify: [src/extraction.rs](/Users/jwgrogan/GitHub/memory-graph/src/extraction.rs)
- Modify: [src/evidence.rs](/Users/jwgrogan/GitHub/memory-graph/src/evidence.rs)
- Modify: [src/server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs)
- Modify: [tests/test_extraction.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_extraction.rs)

- [ ] **Step 1: Strip fenced content and recent evidence hashes**

Proposal tools must:

- strip fenced graph blocks
- suppress content matching recent evidence-set hashes

- [ ] **Step 2: Add `propose_skills`**

Use the same engagement scoring as `propose_memories`, but target durable procedures.

- [ ] **Step 3: Validate skill saves against evidence sets**

`save_skill` must reject:

- evidence IDs not in the referenced evidence set
- expired evidence sets

- [ ] **Step 4: Add pre-compression observability**

Add a minimal checkpoint mechanism or status metric so the server can report whether clients are actually calling proposal tools before compression boundaries.

---

## Task 11: Cutover, Compatibility, and Legacy Decommissioning

**Files:**
- Modify: [src/server.rs](/Users/jwgrogan/GitHub/memory-graph/src/server.rs)
- Modify: [src/db.rs](/Users/jwgrogan/GitHub/memory-graph/src/db.rs)
- Modify: [src/markdown.rs](/Users/jwgrogan/GitHub/memory-graph/src/markdown.rs)
- Modify: [src/sync.rs](/Users/jwgrogan/GitHub/memory-graph/src/sync.rs)
- Modify: [tests/test_validation_p4.rs](/Users/jwgrogan/GitHub/memory-graph/tests/test_validation_p4.rs)

- [ ] **Step 1: Switch public operations to canonical reads**

Once parity is verified:

- all read paths should use `nodes` / `edges`
- legacy tables become migration scaffolding only

- [ ] **Step 2: Update export and sync**

Obsidian export, sync export, and merge behavior must use canonical graph identities.

- [ ] **Step 3: Decide legacy table retention**

Choose one:

- keep temporary compatibility views
- or remove legacy tables after a stabilized release

Document the decision explicitly.

---

## Documentation and Instructions

**Files:**
- Modify: [README.md](/Users/jwgrogan/GitHub/memory-graph/README.md)
- Modify: [AGENTS.md](/Users/jwgrogan/GitHub/memory-graph/AGENTS.md)
- Modify: [GEMINI.md](/Users/jwgrogan/GitHub/memory-graph/GEMINI.md)
- Modify: [docs/PRD.md](/Users/jwgrogan/GitHub/memory-graph/docs/PRD.md)
- Modify: [docs/TRD.md](/Users/jwgrogan/GitHub/memory-graph/docs/TRD.md)

- [ ] Update docs to reflect:
  - canonical graph kernel
  - evidence sets as recall unit
  - skills as graph-native nodes
  - feedback lifecycle
  - schema compatibility rules

---

## Validation Requirements

Add end-to-end scenarios for:

1. legacy memory migration into canonical nodes with retrieval parity
2. retrieval -> evidence set -> helpful feedback -> changed ranking
3. reflection packet generation with truncation flags
4. skill creation from valid evidence nodes
5. fabricated evidence IDs rejected by `save_skill`
6. contradiction candidate generation, dismissal, and suppression
7. merge failure on schema incompatibility
8. proposal extraction ignoring fenced or hashed recalled content

---

## Recommended Commit Sequence

- [ ] Commit 1: `feat: add schema version gating and canonical graph tables`
- [ ] Commit 2: `feat: backfill legacy graph into canonical nodes and edges`
- [ ] Commit 3: `feat: port retrieval to canonical graph and add confidence model`
- [ ] Commit 4: `feat: add evidence sets and replay-safe feedback`
- [ ] Commit 5: `feat: add reflect_context typed packets`
- [ ] Commit 6: `feat: add procedural skills as graph-native nodes`
- [ ] Commit 7: `feat: add contradiction assessments and fencing`
- [ ] Commit 8: `feat: cut over sync export and docs to canonical graph`

---

## Phase Gates

### Gate 1: Schema and sync safety

- schema versioning is enforced
- incompatible merge fails loudly
- canonical tables exist and backfill works

### Gate 2: Retrieval parity

- canonical retrieval matches legacy behavior on seeded scenarios

### Gate 3: Evidence and reflection

- every retrieval produces an evidence set
- feedback is replay-safe
- reflection packets are typed and bounded

### Gate 4: Procedural intelligence

- skills are graph-native nodes
- evidence validation is enforced
- contradictions are inspectable and ranking-aware

---

## Exit Criteria

This phase is complete when:

- the canonical graph kernel is the system of record
- retrieval emits evidence sets
- feedback safely changes confidence over time
- procedural skills live in the graph, not beside it
- reflection is grounded, typed, and bounded
- sync and merge behavior are explicitly safe under the new schema
