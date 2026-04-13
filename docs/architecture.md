# Architecture Direction

## Summary

This document describes a likely technical direction for a portable memory graph service. It is intentionally conceptual. The transport, endpoint contract, and concrete schema remain undecided at this stage.

## Proposed Shape

The current working direction is a local-first service with:

- a local canonical store
- a graph-shaped persistence model, likely backed by SQLite
- ingestion paths for assistant interactions and related work artifacts
- retrieval logic that assembles task-conditioned context
- provenance and user controls for memory lifecycle operations
- note or markdown-oriented outputs for human inspection and external KB workflows

## Major Capability Areas

### Canonical Store

The canonical store should live locally and remain understandable to the user. A SQLite-backed design is the leading intuition because it is portable, inspectable, and easy to embed, while still allowing a graph-shaped model over structured tables and joins.

### Ingestion

Ingestion should accept candidate memories and interaction traces from assistants or related tools. That includes explicit user saves, implicit events from work sessions, and links to external artifacts.

### Retrieval And Context Assembly

Retrieval should return context that is conditioned on a task, workspace, conversation, or user intent. The goal is not to dump the graph back into a prompt, but to assemble the smallest relevant slice with provenance.

### Provenance And Controls

Every saved memory should be auditable. The system should preserve where a memory came from, what confidence it carries, and how a user can edit, merge, or delete it.

### Export And Views

The system should be able to produce human-readable views, including markdown or note-style outputs. Those views may support tools like Obsidian, but they should not force the source knowledge base to become the canonical backend.

## Conceptual Interface Surface

These names are placeholders for capability boundaries, not a committed API:

- `ingest_interaction`
- `retrieve_context`
- `save_memory`
- `update_memory`
- `delete_memory`
- `export_view`
- `open_note`

The repo does not currently commit to:

- HTTP
- MCP
- local RPC
- request and response schemas
- event formats
- sync protocols

## Data Model Direction

The memory model should be able to express:

- people, places, projects, repos, and assistants as entities
- links between those entities as relationships
- interactions and work history as events
- files, notes, messages, and documents as artifacts
- durable user preferences and policies as preferences

The value of the graph is not graph theory for its own sake. It is the ability to preserve durable structure, provenance, and reusable context across many surfaces.

## Deferred Decisions

These decisions are intentionally left open:

- transport and service packaging
- automatic save thresholds versus explicit review
- sync model and cloud portability semantics
- source KB synchronization strategy
- schema normalization depth and indexing strategy

Those decisions should follow clearer validation of the ingestion and retrieval loop.
