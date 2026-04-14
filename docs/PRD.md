# Product Requirements Document

## Problem

Memory in AI systems is fragmented. Providers retain partial context inside their own products. Context gets lost between chat, coding, and planning surfaces. When users switch providers or tools, they start from zero. Existing knowledge bases are hard to reuse inside AI workflows without polluting the source.

Users do significant intellectual work inside AI conversations — design thinking, decision-making, philosophical exploration — and it's all lost when the session ends.

## Thesis

Memory should belong to the user. A portable, inspectable memory layer should sit outside any single provider or assistant, accepting contributions from any source and serving context to any client. The user owns it, controls it, and takes it with them.

## Product Concept

memory-graph is a personal memory prosthetic: a local-first memory service that replicates human memory patterns. It stores what was learned (not what happened), maintains weighted connections that strengthen with use and fade with neglect, and reconstructs narratives on demand rather than storing static summaries.

It is deployed as an MCP server or plugin, loadable by any compatible AI client — Claude, GPT, open-source models, or any future system that supports the protocol.

For the design philosophy behind these choices, see [philosophy.md](./philosophy.md).

## Core Capabilities

### Memory Storage
- Store impulses (atomic learned things) and insights (connected impulses forming mental models)
- Every memory carries provenance (where it came from), emotional context (engagement level, valence), and a weight (0.0 to 1.0)
- Connections between memories are weighted, decay over time, and strengthen through use
- Nothing is hard-deleted. Connections fade toward zero but remain traversable.

### Ingestion
- Explicit user saves ("remember this") at any point during a session
- Adaptive end-of-session extraction: system proposes what it thinks was worth remembering
- Extraction intensity scales with session engagement — high-engagement sessions get deeper extraction
- All proposals require user confirmation. No silent auto-save.
- Secret and PII detection and stripping before any persistence

### Retrieval
- Spreading activation model: query seeds activate nodes, energy propagates through weighted connections
- Context assembly ranked by activation strength, fitted to caller's context budget
- Two-phase: fast synchronous core recall, followed by async ghost graph pull-through via subagent
- Narratives reconstructed at query time from co-activated impulses, never served from stored summaries
- Usage-based quality feedback: dismissed recalls weaken connections, used recalls strengthen them

### Ghost Graphs (BYOKB)
- External knowledge bases (Obsidian, repos, any structured KB) are mapped as shadow topologies
- Structure is known, content is not ingested — only metadata and relationships
- The memory graph's ontology (weights, activation, decay) overlays the ghost graph
- Content is pulled on demand when ghost nodes activate during retrieval
- Pulled content enters as session-only (released after use) or permanent (promoted to full memory node)
- External KBs are never modified. One-directional. Source integrity preserved.
- Ghost graph weights learn over time which parts of external knowledge are most relevant

### Incognito Mode
- Full blackout: no ingestion, no proposals, no ghost pulls persisted, no metadata stored
- The memory graph has no record that the session occurred

## Deployment Model

memory-graph runs as an MCP server or plugin. Availability is binary:
- Connected: full memory capability
- Not loaded: no memory capability, assistant operates without memory
- No fallback, no queuing, no local shadow memory. The MCP server is the single authority.

## User Experience

### During a Session
- Assistant has access to memory tools via MCP
- Relevant context surfaces automatically when the assistant requests it for a task
- User can explicitly save memories at any time
- User can ask "why was this recalled?" and get provenance
- User can ask "what do I know about X?" and get reconstructed context

### End of Session
- System proposes extracted impulses and insights based on session content
- Proposal depth scales with session engagement
- User confirms, edits, or dismisses each proposal
- Confirmed proposals become memories with provenance back to the session

### Memory Management
- User can inspect all stored memories, their connections, and weights
- User can edit or delete any memory
- User can see which ghost graphs are mapped and their learned weights
- All operations are transparent and auditable

## BYOKB: Bring Your Own Knowledge Base

The memory graph is a universal memory ontology that overlays any knowledge graph. It doesn't replace existing tools — it learns from them.

- Obsidian vaults, codebases, conversation archives, or any structured KB can be registered as ghost graphs
- The graph maps their topology and learns which parts matter through usage
- Content is fetched on demand, not copied
- Multiple ghost graphs can coexist, each with independent learned weights
- The same activation model works across all sources

The user's existing knowledge ecosystem stays intact. memory-graph sits alongside it and makes it more useful.

## Roadmap

### Phase 0: Repo Framing (Current)
- Document problem, thesis, and design philosophy
- Resolve architectural decisions through adversarial review
- Produce PRD and TRD

**Gate:** A second person can read the docs and understand what's being built and why.

### Phase 1: Local Single-User Memory Service
- Implement graph data model with weighted connections
- Implement spreading activation retrieval
- Implement decay and reinforcement mechanics
- Implement ingestion pipeline with extraction and secret stripping
- Implement incognito mode
- MCP tool surface for basic memory operations

**Validation criteria:**
- Store/retrieve round-trip: save impulses, retrieve by related query, correct ones return
- Spreading activation: connected impulses activate through adjacency
- Decay: accessed memories surface more strongly than untouched ones after time
- Reconstruction: system builds coherent narrative from connected impulses without stored summary
- Emotional weighting: high-engagement impulses surface more readily
- Security: API keys in ingested conversations are stripped before persistence

**Kill criterion:** If retrieval quality after tuning is no better than keyword search over a flat document store, the graph model isn't earning its complexity.

**User testing:** Self-use for two weeks. Does it surface useful context?

### Phase 2: Ghost Graphs and Multi-Client Validation
- Ghost graph registration and topology mapping
- On-demand pull-through (session-only and permanent)
- Adaptive end-of-session extraction with engagement heuristics
- Validate MCP tools work across multiple AI clients

**Validation criteria:**
- Ghost graph maps Obsidian vault topology without content ingestion
- Pull-through activates on relevant query, fetches content on demand
- Session-only pulls leave no persistent trace
- Permanent pulls create full memory nodes with provenance
- MCP tools work in at least two different AI clients
- Adaptive extraction proposes more from engaging sessions, less from routine ones

**Kill criterion:** If users don't find surfaced context useful enough to keep the MCP loaded, retrieval or extraction quality isn't there.

**User testing:** One other person uses it and gets value without the philosophy explained to them.

### Phase 3: Sync and Portability
- Backup and cross-device portability
- Identity and sync semantics
- Multiple ghost graph sources (Obsidian + repos + conversations)
- Optional cloud sync without compromising local-first guarantees

**Validation criteria:**
- Cross-device access with consistency
- Backup/restore preserves full graph integrity
- Cross-source activation works across multiple ghost graphs

**Kill criterion:** If sync degrades the local-first experience, defer further.

**User testing:** Works across someone's real multi-device workflow.

## Non-Goals

- Team collaboration semantics
- Enterprise permissions or policy systems
- Hosted SaaS-first product design
- Cloud sync as a prerequisite for usefulness
- Provider-specific lock-in integrations
- Advanced visual graph browsing as a requirement for early validation
- Bidirectional sync with external KBs
- Automatic memory compilation without user-initiated or task-initiated trigger
