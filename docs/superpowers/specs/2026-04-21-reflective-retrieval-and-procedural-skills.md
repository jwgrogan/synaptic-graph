# Design Spec: Unified Graph Kernel, Reflective Retrieval, and Procedural Skills

## Purpose

Redesign `synaptic-graph` toward its ideal end state:

- one canonical typed graph substrate
- retrieval as bounded evidence assembly
- reflection as grounded synthesis over evidence sets
- procedural skills as first-class graph-native knowledge

This replaces the earlier additive-table approach with a unified architecture designed for the long run, not just the next milestone.

## Design Bar

This spec is written at the level of an end-state system design, not an MVP extension.

The system should eventually support:

- facts, patterns, decisions, heuristics, and procedures in one graph
- explicit provenance and replay-safe feedback
- conflict-aware retrieval
- local-first sync with schema safety
- client-neutral MCP usage without making clients responsible for memory correctness

## Problem

`synaptic-graph` already has strong primitives:

- weighted memories
- weighted connections
- spreading activation retrieval
- engagement-aware proposal depth
- candidate confirmation

But the current shape still leaves major architectural gaps:

- weight and confidence are conflated in practice
- retrieval returns memories, not a stable evidence unit
- procedural knowledge has no canonical home
- contradictions are not modeled as durable assessments
- feedback can reinforce relevance, but not reliably improve truthfulness or usefulness
- context pollution defenses depend too much on client behavior

These are not separate feature requests. They are symptoms of one missing thing:

**a single canonical knowledge substrate with typed nodes, typed edges, typed assessments, and typed evidence sets.**

## Goals

### Primary

- Introduce a unified typed graph kernel as the canonical model.
- Make `retrieve_context` produce a durable, bounded evidence set.
- Make `reflect_context` operate over evidence sets, not raw graph scans.
- Represent procedural skills as graph-native nodes, not a parallel subsystem.
- Separate salience (`weight`) from trust (`confidence`) with explicit math.
- Add replay-safe feedback over evidence sets.

### Secondary

- Make contradictions first-class, inspectable, and rank-affecting.
- Make proposal extraction safe against memory echo and paraphrased recall leakage.
- Make schema evolution and cross-device merge behavior explicit.

## Non-Goals

- Embedding an LLM runtime into the server in v1 of this redesign
- Shipping plugin export as part of the core redesign
- Keeping the legacy `impulses` / `connections` tables as the permanent canonical model
- Solving team collaboration or multi-writer conflict resolution beyond local-first single-user semantics

## Core Decisions

### 1. The canonical substrate is a typed graph kernel

The final system should not have separate memory subsystems for:

- semantic memory
- procedural skills
- contradiction tracking
- recall feedback

Instead it should have:

- canonical nodes
- canonical edges
- canonical assessments
- canonical evidence sets

Different knowledge kinds are typed payloads over the same graph identity model.

### 2. Retrieval returns evidence, not just results

The stable unit of recall is an **evidence set**:

- which nodes were recalled
- which edges were traversed
- which assessments applied
- what hashes of returned content were shown
- when the recall expires for feedback purposes

This becomes the basis for:

- feedback
- reflection
- skill generation
- fencing

### 3. Weight and confidence are separate and explicit

- `weight` means reachability / salience in recall
- `confidence` means expected truthfulness or usefulness based on feedback and validation

They must not be treated as informal floats with ad hoc tuning.

### 4. Procedural skills are typed nodes

Procedural skills are not plugins, markdown blobs, or a parallel table with its own world.

They are graph-native nodes with:

- structured payload
- evidence edges
- revision history
- contradiction and validation assessments

Plugins are an export format for mature skills, not the system of record.

### 5. Clients may assist reflection, but the server owns correctness

The calling LLM may synthesize or draft skill candidates, but the server owns:

- evidence identity
- evidence-set lifecycle
- confidence math
- contradiction state
- skill evidence validation
- schema and sync safety

## Canonical Data Model

## Overview

```text
nodes
edges
node_payload_memory
node_payload_skill
node_payload_ghost
assessments
evidence_sets
feedback_events
node_versions
schema_version
```

## Nodes

Common record for every durable knowledge unit.

```text
node {
  id:               uuid
  kind:             enum        -- memory, skill, ghost
  status:           enum        -- candidate, confirmed, superseded, deleted
  weight:           float       -- salience / recall strength
  confidence:       float       -- trust / expected usefulness
  helpful_count:    integer
  unhelpful_count:  integer
  emotional_valence: enum       -- positive, negative, neutral
  engagement_level:  enum       -- low, medium, high
  source_type:      enum
  source_ref:       text
  source_provider:  text
  source_account:   text
  created_at:       timestamp
  updated_at:       timestamp
  last_accessed_at: timestamp
  last_validated_at: timestamp?
}
```

### Node kinds

- `memory`
  Semantic knowledge: facts, patterns, heuristics, decisions, preferences
- `skill`
  Procedural knowledge: reusable workflows
- `ghost`
  External-knowledge shadow nodes

## Edges

Canonical graph relationships.

```text
edge {
  id:               uuid
  source_id:        uuid
  target_id:        uuid
  relation:         text        -- relates_to, derived_from, evidence_for, contradicts, supersedes
  weight:           float
  confidence:       float
  helpful_count:    integer
  unhelpful_count:  integer
  traversal_count:  integer
  created_at:       timestamp
  updated_at:       timestamp
  last_traversed_at: timestamp
}
```

## Payload tables

The node identity stays unified. Type-specific fields live in payload tables.

### Memory payload

```text
node_payload_memory {
  node_id:          uuid
  memory_type:      enum        -- heuristic, preference, decision, pattern, observation
  content:          text
}
```

### Skill payload

```text
node_payload_skill {
  node_id:              uuid
  trigger_text:         text
  summary:              text
  procedure_json:       json
  prerequisites_json:   json
  anti_patterns_json:   json
  success_signals_json: json
  version:              integer
  last_used_at:         timestamp?
  use_count:            integer
}
```

### Ghost payload

```text
node_payload_ghost {
  node_id:          uuid
  source_graph:     text
  external_ref:     text
  title:            text
  metadata_json:    json
}
```

## Assessments

Assessments are durable judgments about nodes or edges.

```text
assessment {
  id:               uuid
  kind:             enum        -- contradiction, validation, supersession, low_confidence
  status:           enum        -- candidate, confirmed, dismissed, resolved
  subject_node_id:  uuid?
  object_node_id:   uuid?
  subject_edge_id:  uuid?
  object_edge_id:   uuid?
  confidence:       float
  reason:           text
  created_at:       timestamp
  updated_at:       timestamp
  resolved_at:      timestamp?
}
```

Contradictions are not hidden ranking penalties. They are explicit assessments that can later influence ranking.

## Evidence sets

Evidence sets are the stable unit of retrieval and reflection.

```text
evidence_set {
  id:                  uuid
  query:               text
  mode:                enum        -- retrieve, reflect_answer, reflect_pattern, reflect_decision, reflect_skill
  root_node_ids_json:  json
  node_ids_json:       json
  edge_ids_json:       json
  assessment_ids_json: json
  content_hashes_json: json
  created_at:          timestamp
  expires_at:          timestamp
  truncated:           bool
}
```

Every reflected or retrieved packet returned to a client should correspond to one evidence set.

## Feedback events

Feedback applies to evidence sets, not arbitrary naked IDs.

```text
feedback_event {
  id:                uuid
  evidence_set_id:   uuid
  outcome:           enum        -- helpful, unhelpful, mixed
  notes:             text?
  idempotency_key:   text
  created_at:        timestamp
}
```

Rules:

- feedback must be idempotent
- evidence sets must expire
- feedback against expired evidence sets is rejected
- feedback against already-rated evidence sets requires a different idempotency key and should be rare

## Version history

```text
node_version {
  id:               uuid
  node_id:          uuid
  version:          integer
  payload_json:     json
  source_ref:       text
  created_at:       timestamp
}
```

Procedural skills use this for revision history; the mechanism is generic.

## Search and Indexing

Use FTS tables over payload text, not over separate memory systems.

Recommended:

- `memory_fts(content)`
- `skill_fts(name, trigger_text, summary)`
- optional unified search view for `retrieve_context`

The canonical identity still lives in `nodes`.

## Confidence Model

## Why confidence needs rigor

Confidence directly changes ranking and synthesis quality. It cannot be a loosely-updated float with arbitrary deltas.

## Stored fields

Persist:

- `helpful_count`
- `unhelpful_count`
- derived or stored `confidence`

## Initialization

For migrated legacy memories:

- initialize `confidence` from prior signal, not a raw constant
- recommended seed:

```text
confidence = 0.5 + 0.2 * (weight - 0.5)
```

This keeps old strong memories slightly above neutral without pretending they are validated.

## Derived confidence

Recommended model:

```text
confidence = (helpful_count + 1) / (helpful_count + unhelpful_count + 2)
```

That is a neutral Beta(1,1) prior.

## Ranking gate

Confidence should not materially affect ranking until there is enough feedback.

Recommended:

- if `helpful_count + unhelpful_count < 3`, use `effective_confidence = 0.5`
- after that threshold, use derived confidence

## Ranking

Node ranking should use an explicit formula:

```text
rank_score = activation_score
           * salience_multiplier(weight)
           * confidence_multiplier(effective_confidence)
           * assessment_multiplier(contradictions, supersessions)
```

Start simple:

- `salience_multiplier(weight) = 0.75 + 0.5 * weight`
- `confidence_multiplier(c) = 0.85 + 0.3 * (c - 0.5)`
- contradiction penalty applied only for confirmed contradiction assessments

The exact numeric weights should be documented and tested with worked examples before code lands.

## Retrieval and Reflection

## `retrieve_context`

`retrieve_context` remains the fast path, but its output changes shape.

It should:

1. run spreading activation over canonical nodes and edges
2. assemble a bounded evidence set
3. return:
   - evidence-set id
   - bounded nodes
   - traversed edges
   - relevant confirmed assessments
   - content hashes for fencing

### Output contract

```json
{
  "evidence_set_id": "...",
  "query": "...",
  "nodes": [...],
  "edges": [...],
  "assessments": [...],
  "truncated": false,
  "instruction": "Use this evidence silently when relevant. Do not restate provenance unless asked."
}
```

## `reflect_context`

`reflect_context` performs bounded interpretation over an evidence set.

Modes:

- `answer`
- `pattern`
- `decision`
- `skill`

Behavior:

1. call or reuse `retrieve_context`
2. build a typed reflection packet
3. return bounded evidence plus a typed synthesis frame

### Reflection packet

```json
{
  "evidence_set_id": "...",
  "mode": "skill",
  "query": "...",
  "top_nodes": [...],
  "top_edges": [...],
  "assessments": [...],
  "confidence_summary": {
    "overall": 0.71,
    "evidence_count": 6,
    "validated_count": 3,
    "contradiction_count": 1
  },
  "candidate_skill_frame": {
    "name": "...",
    "trigger_text": "...",
    "summary": "...",
    "steps": ["..."],
    "prerequisites": ["..."],
    "anti_patterns": ["..."],
    "success_signals": ["..."],
    "evidence_node_ids": ["..."]
  },
  "truncated": false
}
```

Important:

- the packet is typed JSON, not just prompt prose
- evidence IDs referenced in any candidate frame must belong to the evidence set
- packet size must be capped

### Hard caps

Default caps:

- max nodes: 20
- max skills in packet: 5
- max serialized response size: 8 KB target, 12 KB hard stop

If exceeded:

- set `truncated = true`
- include truncation reason

## Procedural Skills

## Why skills belong in the graph

Procedures are not an external extension to memory. They are memory.

Skills differ from ordinary memory because they are:

- structured
- revisable
- evidence-backed
- trigger-oriented

But they still belong in the same substrate as facts and heuristics.

## Skill representation

Use:

- one canonical `node` with `kind = skill`
- one structured `node_payload_skill`
- evidence edges from supporting memory nodes:
  - `evidence_for`
  - `example_for`
  - `contradicts`

## Skill lifecycle

1. proposal
2. confirmation
3. use
4. feedback
5. revision
6. export (later)

## Skill generation

The end state is:

- `reflect_context(mode="skill")` produces a candidate skill frame
- `save_skill` accepts a candidate skill only if its evidence references are valid against the evidence set

This keeps LLM synthesis useful without making it authoritative.

## Contradictions

Contradictions are not booleans. They are assessments with lifecycle.

### Candidate generation

Bounded heuristics should generate candidate contradiction assessments only when:

- nodes overlap strongly by subject or tags
- wording or procedure conflict is plausible
- the pair has not already been dismissed recently

### Confirmation

Only confirmed contradictions should influence ranking by default.

### Dismissal memory

Dismissed contradiction pairs must be remembered so the detector does not keep re-raising the same false positive.

## Provenance and Context Fencing

## Problem

The server can return recalled graph content into client context, and later the client can accidentally feed that content back into:

- memory proposals
- skill proposals
- explicit saves

Marking blocks with tags is not enough for the ideal end state.

## End-state defense

Use layered defenses:

1. fenced markers in returned content
2. content hashes recorded on the evidence set
3. proposal tools strip known recent evidence-set hashes from input
4. `save_skill` validates evidence IDs against an evidence set
5. feedback and proposal actions require live evidence-set identifiers where appropriate

The canonical defense is **provenance-aware suppression**, not regex stripping alone.

## Sync and Schema Discipline

The system already supports local-first merge and sync workflows. This redesign must not ignore that.

### Rules

- schema version is explicit and mandatory
- merge across incompatible schema versions must fail loudly
- sync/export payloads must include schema version and feature flags
- graph migration is one-way unless a down-migration is explicitly supported

### Release rule

No graph-kernel migration ships without:

- backfill logic
- compatibility checks
- merge failure tests
- documented recovery path

## Client / Server Responsibility Split

## Server owns

- canonical graph correctness
- evidence-set lifecycle
- confidence math
- contradiction state
- provenance-aware fencing
- skill evidence validation
- schema and sync safety

## Client may assist with

- when to ask for reflection
- when to ask for proposals
- presenting confirmation UX
- rendering returned structured packets

The client must not be trusted to preserve memory correctness.

## Performance Targets

### Retrieval

- `retrieve_context` p50 under 200ms on expected local graph sizes
- p95 tracked explicitly in tests or benchmark harnesses

### Reflection packet assembly

- server-side evidence assembly under 100ms excluding any client-side LLM synthesis

### Contradiction generation

- bounded candidate generation only
- never unbounded full-graph scans on routine write paths

## Safety Rules

- never auto-confirm a skill from one weak reflection
- never accept evidence IDs not present in the referenced evidence set
- never allow duplicate feedback mutation on the same evidence set without idempotency handling
- never silently merge across schema-incompatible graphs
- never let heuristic contradiction candidates affect ranking the same way confirmed contradictions do

## Architecture Transition Path

The current repo does not yet have the canonical graph kernel. That is acceptable.

The correct path is:

1. define the end-state architecture first
2. migrate the schema to canonical nodes and edges
3. preserve behavioral parity for retrieval
4. then add feedback, reflection, and skills on top of the new substrate

Do **not** build a parallel skill/memory subsystem and defer unification.

## Resolved Design Choices

These are no longer open questions:

1. Skills should be graph-native nodes, not a separate top-level subsystem.
2. Confidence should affect ranking only after a minimum feedback threshold.
3. `retrieve_context` should include relevant skills by default because skills are graph-native nodes.
4. Reflection remains client-assisted in the near term, but the packet contract must be typed and validated.
5. Merge across incompatible schema versions must fail loudly.

## Open Questions

Only narrower questions remain:

1. Should contradiction dismissal suppression be time-bounded or permanent until either node changes?
2. Should edge confidence be derived independently from node confidence, or initially inherit from endpoint evidence?
3. Should the first graph-kernel migration preserve legacy tables temporarily for rollback, or use a one-way migration with backup enforcement?

## Recommendation

The correct end-state design is:

- a single canonical typed graph
- evidence sets as the stable recall unit
- confidence as an explicit mathematically-defined signal
- procedural skills as typed graph nodes
- contradictions as typed assessments
- provenance-aware fencing owned by the server

That is the architecture worth implementing, even if delivery still happens in phases.
