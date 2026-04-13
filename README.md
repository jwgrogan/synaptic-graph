# memory-graph

`memory-graph` is a docs-first exploration of a portable memory graph for LLMs: a user-owned, inspectable memory layer that can travel across assistants, providers, and surfaces without making any single tool the system of record.

The core thesis is simple: memory should belong to the user. Providers and assistants can contribute to it and retrieve from it, but the durable memory model should live outside any one chat app, coding agent, or hosted platform.

## Why this exists

Today, memory is fragmented.

- Providers retain partial context inside their own products.
- Context gets lost between chat, planning, and coding surfaces.
- Existing knowledge bases are hard to reuse cleanly inside AI workflows.
- Users often have to choose between keeping notes in source systems or polluting them with AI-generated artifacts.

This project explores a different model: a portable graph-shaped memory layer that can ingest interactions, preserve provenance, and assemble context for downstream assistants without forcing the source knowledge base to become the canonical backend.

## BYOKB, Without Source Pollution

One design goal is BYOKB: bring your own knowledge base.

That means the system should be able to:

- leverage existing tools such as Obsidian and other markdown-based knowledge bases
- reference or derive memory from those sources without making them the canonical storage layer
- generate a new unified knowledge base view from source material when that is useful
- avoid polluting the original source with assistant-generated structure unless the user explicitly wants that

## Product Boundary

This repo is intentionally framed around the portable memory service first.

- The memory graph is the primary product concept.
- Assistants are clients of that memory graph, not the owners of its schema or persistence.
- An assistant can act as an injection surface for new memory and a retrieval surface for relevant context.
- The exact integration mechanism is still open and is not fixed in this repo.

## Current Status

This is an exploratory, docs-first repository.

- No runnable service is included yet.
- No public transport or endpoint contract is locked.
- No schema, sync protocol, or provider-specific adapter is treated as final.
- The initial focus is product framing, architecture direction, and repo-level clarity.

## Conceptual Capabilities

These capability names are illustrative only. They describe the shape of the problem space, not a committed API:

- `ingest_interaction`
- `retrieve_context`
- `save_memory`
- `update_memory`
- `delete_memory`
- `export_view`
- `open_note`

## Repository Guide

- [Vision](./docs/vision.md)
- [Architecture Direction](./docs/architecture.md)
- [Assistant Client Note](./docs/integrations/assistant-client.md)
- [Roadmap](./docs/roadmap.md)
- [Source Note](./docs/source-notes/graph-memory.md)

## Repo Layout

```text
.
├── README.md
└── docs/
    ├── architecture.md
    ├── integrations/
    │   └── assistant-client.md
    ├── roadmap.md
    ├── source-notes/
    │   └── graph-memory.md
    └── vision.md
```
