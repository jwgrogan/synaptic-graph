# Technical Requirements Document

## System Overview

memory-graph is a local-first memory service deployed as an MCP server. It stores a graph of weighted, decaying connections between learned impulses and insights, supports spreading activation retrieval, and overlays external knowledge bases as ghost graphs. The canonical store is local SQLite.

## Data Model

### Core Node Types

**Impulse** — the atomic unit of memory. Represents something learned: a heuristic, preference, decision, pattern recognition.

```
impulse {
  id:               uuid
  content:          text        -- the learned thing
  type:             enum        -- heuristic, preference, decision, pattern, observation
  weight:           float       -- 0.0 to 1.0, current strength
  initial_weight:   float       -- set by creation method
  emotional_valence: enum       -- positive, negative, neutral
  engagement_level:  enum       -- low, medium, high
  source_signals:   text[]      -- what the valence/engagement were derived from
  created_at:       timestamp
  last_accessed_at: timestamp
  source_type:      enum        -- explicit_save, session_extraction, pull_through
  source_ref:       text        -- conversation ID, file path, session ID
  status:           enum        -- candidate, confirmed, superseded, deleted
}
```

**Ghost Node** — a lightweight reference to a node in an external knowledge graph. No content stored.

```
ghost_node {
  id:               uuid
  source_graph:     text        -- which external KB (e.g., "obsidian-main-vault")
  external_ref:     text        -- path, URL, or ID in the external system
  title:            text        -- human-readable label
  metadata:         jsonb       -- tags, last_modified, structural info from source
  weight:           float       -- learned relevance, 0.0 to 1.0
  last_accessed_at: timestamp
  created_at:       timestamp
}
```

### Connection Model

Connections are the core of the memory model. They carry weight, decay, and reinforce.

```
connection {
  id:               uuid
  source_id:        uuid        -- impulse or ghost_node
  target_id:        uuid        -- impulse or ghost_node
  weight:           float       -- 0.0 to 1.0
  relationship:     text        -- freeform label (e.g., "derived_from", "contradicts", "relates_to")
  created_at:       timestamp
  last_traversed_at: timestamp
  traversal_count:  integer
}
```

### Weight Mechanics

**Initial weight** is set by creation method:
- Explicit user save: 0.7
- Session extraction (high engagement): 0.5
- Session extraction (low engagement): 0.3
- Ghost node pull-through: 0.4

These are starting points. Exact values should be tuned during Phase 1.

**Decay function:**

```
effective_weight(t) = weight * e^(-λ * hours_since_last_access)
```

Where `λ` (decay rate) varies by content type:
- Semantic knowledge (heuristics, preferences, patterns): low λ, slow decay
- Episodic provenance (session refs, timestamps, source context): high λ, fast decay
- Ghost node connections: medium λ

**Reinforcement:**

Each traversal during retrieval adds a fixed bump:

```
weight = min(1.0, weight + reinforcement_bump)
last_traversed_at = now()
traversal_count += 1
```

All reinforcement is equal — explicit user confirmation, retrieval traversal, and task usage all add the same bump. The system learns from usage patterns, not declarations.

**Decay floor:**

Weights never reach exactly 0.0. There is a minimum floor (e.g., 0.001) that ensures every connection remains theoretically traversable if enough activation energy reaches it. This models the "smell triggers a decades-old memory" phenomenon.

## Spreading Activation Retrieval

### Seed Phase

The retrieval request provides seed signals: task description, question, workspace context, explicit entities, or any combination. The system identifies directly matching nodes in the memory graph and ghost graphs.

### Propagation Phase

```
activation(node) = direct_match_score + Σ(
  connection_weight * source_activation * proximity_decay(hops)
)
```

Where:
- `direct_match_score` is the relevance of the node to the query (semantic similarity or keyword match)
- `connection_weight` is the effective weight after decay
- `source_activation` is the activation level of the connected node
- `proximity_decay(hops)` drops off with graph distance, preventing runaway activation

Emotional weight amplifies propagation: connections with high engagement level carry energy further (lower proximity decay per hop).

Propagation runs iteratively until:
- A maximum number of iterations is reached, or
- No node's activation changes by more than a threshold

### Threshold and Assembly

- Nodes above the activation threshold are "recalled"
- Nodes just below threshold are "available" — included if context budget allows
- Ghost nodes above threshold trigger pull-through consideration

Recalled nodes are ranked by activation strength. The system assembles a context package within the caller's budget, preferring fewer strongly-activated pieces over many weak fragments.

Each recalled piece includes provenance: where it came from, the connection path that activated it, and emotional context.

### Reinforcement

Every connection traversed during a successful retrieval gets a weight bump. Connections traversed in retrievals where the user dismisses the result or the context goes unused do not get bumped (and may get a negative signal in future iterations).

## Ghost Graph System

### Registration

An external KB is registered by providing:
- A source identifier (e.g., "obsidian-main-vault")
- A root path or access method
- A scan configuration (what file types, what structural relationships to map)

### Topology Scan

On registration (and periodic refresh), the system:
1. Walks the external KB structure
2. Creates ghost nodes for each meaningful unit (note, file, document)
3. Maps structural relationships between ghost nodes (links, folder hierarchy, imports)
4. Stores metadata (title, tags, last modified) but not content

This scan is cheap — metadata only, no content reading or LLM calls.

### Pull-Through

When a ghost node activates during retrieval:
1. The system fetches the actual content from the external source
2. An LLM call evaluates relevance to the current retrieval context
3. If relevant, the content is made available to the caller

Pull-through operates in two modes:
- **Session-only:** Content is available for the current session, then released. Ghost node connection weights are still updated (the access pattern is remembered even if the content isn't kept).
- **Permanent:** Content is extracted into impulses and added to the core memory graph with provenance back to the ghost node. The ghost node relationship becomes a real connection.

### Ghost Pull as Subagent

Ghost node pull-through is asynchronous. The main retrieval returns core memory results immediately. A subagent handles ghost node fetching and reports back with supplemental context. This ensures retrieval latency is not gated on external KB reads.

## Ingestion Pipeline

### Explicit Save

User says "remember this" during a session. The system:
1. Extracts the relevant impulse from the current context
2. Runs secret/PII detection and strips sensitive content
3. Creates a candidate impulse with source provenance
4. Attaches derived emotional state from current session signals
5. Presents to user for confirmation

Initial weight: 0.7.

### End-of-Session Extraction

At session end, the system:
1. Evaluates session engagement level from heuristics:
   - Turn length and depth
   - Session duration and sustained focus
   - Decision density
   - Topic novelty
   - Explicit emotional signals
   - Conversational arc (building ideas vs isolated questions)
2. Scales extraction depth by engagement level
3. Runs an LLM call to extract candidate impulses and proposed connections
4. Runs secret/PII detection on all candidates
5. Presents proposals to user with emotional context attached
6. User confirms, edits, or dismisses each proposal
7. Confirmed proposals become memories with full provenance

### Secret and PII Handling

Before any content is persisted:
- Pattern matching for API keys, tokens, connection strings, credentials
- PII detection for emails, phone numbers, addresses
- Detected secrets are stripped or redacted
- User is notified of what was redacted

### Incognito Mode

When active:
- No ingestion of any kind
- No end-of-session proposals
- No ghost node pull-throughs persisted
- No session metadata stored
- No connection weights updated
- The memory graph has zero record of the session

## MCP Tool Surface

The memory graph exposes these capabilities as MCP tools:

### Memory Operations
- `save_memory` — explicit user save, creates candidate impulse
- `retrieve_context` — spreading activation retrieval with context budget
- `recall_narrative` — reconstruct a story/narrative from connected impulses for a given topic
- `update_memory` — modify an existing impulse's content (creates supersession chain)
- `delete_memory` — soft-delete, sets status to deleted, connections fade but remain
- `inspect_memory` — view a specific memory's full record including provenance, weight, connections

### Ghost Graph Operations
- `register_ghost_graph` — register an external KB for ghost graph mapping
- `refresh_ghost_graph` — re-scan external KB topology
- `pull_through` — explicitly pull ghost node content (permanent or session-only)

### Session Operations
- `set_incognito` — enable/disable incognito mode for current session
- `propose_memories` — trigger end-of-session extraction manually
- `confirm_proposal` — confirm a proposed memory
- `dismiss_proposal` — dismiss a proposed memory (negative signal)

### Inspection
- `explain_recall` — explain why a specific piece of context was recalled (activation path, connection weights)
- `memory_status` — overview of graph state (node count, ghost graphs, connection statistics)

## Persistence

### SQLite Schema

The canonical store is a single SQLite database file. SQLite is chosen for:
- Portability (single file, no daemon dependency beyond the MCP server)
- Inspectability (standard tooling can query it directly)
- Embeddability (no separate database process)
- Sufficient performance for single-user graph traversal

### Schema Migration

Schema versions are tracked in a metadata table. Migrations run automatically on service startup. Backward-incompatible changes require explicit user confirmation and backup.

### Backup

The SQLite file can be copied for backup. The MCP server should expose a backup tool that creates a consistent snapshot. Restore replaces the file and restarts the service.

## Performance Constraints

### Retrieval Latency
- Phase 1 core retrieval (memory graph only): target under 200ms
- Phase 2 with ghost pull-through: core results under 200ms, ghost results stream async

### Graph Scale
- Phase 1 target: up to 10,000 impulses, 50,000 connections
- Spreading activation should remain under 200ms at this scale on SQLite
- If scale demands exceed SQLite, evaluate migration to embedded graph DB (deferred)

### Ingestion
- Explicit saves: effectively instant (single insert + connection creation)
- End-of-session extraction: LLM call, acceptable latency is seconds (not in critical path)

## Deferred Decisions

These are intentionally left for later phases:

- Sync protocol for cross-device portability (Phase 3)
- Cloud backup service (Phase 3)
- Exact decay rate constants (tune during Phase 1 based on real usage)
- Reinforcement bump magnitude (tune during Phase 1)
- Activation threshold values (tune during Phase 1)
- Visual graph inspection UI (nice-to-have, not required for validation)
- Multiple user support
