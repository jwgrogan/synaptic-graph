# Source Note

Imported from `/Users/jwgrogan/Downloads/graph-memory.md` on 2026-04-13. The original note is preserved below for provenance.

# Two 1-Pagers: Graph Memory Service and Homard Integration

## Summary
Split this into two distinct efforts:

1. a **standalone graph memory service** that owns user memory across providers and surfaces  
2. a **thin Homard integration** that reads/writes that service but does not own the memory model

This keeps Homard out of the memory-platform business and lets you return later once the service API is clearer.

## 1-Pager: Portable Graph Memory Service

### Title
Portable Memory Graph for LLMs

### What it is
A **user-owned memory layer for AI systems** that captures durable context across chat apps, coding agents, and providers. It stores memory as a graph of entities, relationships, events, artifacts, and preferences, then serves that context back through a local API.

### Core idea
Today, providers each keep partial memory inside their own product surfaces. Context gets lost:
- between chat and coding modes
- when switching providers
- when changing tools over time

This service makes memory **portable, inspectable, and reusable**. The user owns the memory graph; assistants plug into it.

### Product shape
Local-first system with:
- local canonical store
- open local API for assistants/tools
- optional Obsidian export or vault sync
- future freemium cloud sync for backup and cross-device portability

### Primary user value
- one memory across all AI tools
- preserved context between chat and coding
- portable personal + project memory
- transparent, user-auditable memory instead of opaque provider retention

### v1 scope
- single-user
- entity/relationship/event memory model
- ingestion from assistant interactions and coding sessions
- retrieval API for task-conditioned context assembly
- markdown/Obsidian view generation
- provenance, confidence, edit/delete/merge controls

### Non-goals for v1
- team collaboration semantics
- full enterprise permissions model
- deep hosted SaaS features
- provider-specific lock-in integrations

### Suggested architecture direction
- canonical store: SQLite with graph-shaped schema
- service boundary: local daemon or embeddable service with HTTP/JSON or MCP-style interface
- outputs: retrieval context, provenance, graph export, markdown notes
- sync: deferred until core ingestion/retrieval model is stable

### Open decisions to resolve later
- exact endpoint contract
- ingestion pipeline shape
- auto-save vs review thresholds
- Obsidian sync model: generated vault vs bidirectional sync
- identity and sync semantics for future cloud tier

## 1-Pager: Homard Integration

### Title
Homard as a Client of the Portable Memory Graph

### What it is
A **thin integration layer** that lets Homard consume and update the external graph memory service. Homard should act as a client, not as the owner of the memory system.

### Core idea
Homard already has lightweight markdown/FTS memory. That should remain simple or transitional. The richer cross-provider graph belongs outside Homard so it can serve other assistants too.

### Role of Homard
Homard should:
- send interaction events and explicit memory saves to the graph service
- request relevant context before chat/coding actions
- render returned memory context inside its own agent flows
- optionally show provenance and memory actions to the user

Homard should not:
- define the canonical graph schema
- become the system of record for portable memory
- tightly couple its internal prompts to a Homard-only memory format

### Integration shape
Treat the graph service like an external capability:
- `ingest_interaction`
- `retrieve_context`
- `save_memory`
- `update_memory`
- `delete_memory`
- `export_view` or `open_note`

Homard can map existing memory behaviors onto those calls while keeping its current simple memory as fallback or compatibility mode.

### User-facing value inside Homard
- context survives switching between providers and surfaces
- project and personal context can be pulled in on demand
- explicit “remember this” and “why did you recall this?” flows
- cleaner handoff between planning/chat/coding sessions

### v1 scope
- adapter/client for the graph service
- retrieval injection into system/context building
- memory write hooks from chat/coding flows
- fallback behavior if the service is unavailable
- minimal UI hooks for visibility and debugging

### Non-goals for v1
- implementing the graph platform inside Homard
- Obsidian sync logic inside Homard
- cloud sync logic inside Homard
- advanced graph browsing UI inside Homard

### Suggested implementation direction
- isolate integration behind a small client module
- preserve compatibility with current `memory_save` / `memory_search` semantics where useful
- keep `MEMORY.md` as optional local note/output, not the primary memory model
- design for graceful degradation when the graph service is not configured

## Assumptions And Defaults
- The graph memory product is a **separate tool/service**, not part of Homard’s core scope.
- Homard should integrate later once the service endpoints and data contract are clearer.
- Obsidian is an **adjacent interface/export target**, not necessarily the canonical backend.
- The right boundary is: **portable graph service first, Homard integration second**.
