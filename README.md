# memory-graph

A portable, human-memory-inspired memory layer for AI systems. User-owned, local-first, inspectable.

## What This Is

memory-graph is a personal memory prosthetic deployed as an MCP server. It stores what was learned from AI interactions as a graph of weighted, decaying connections — replicating how human memory works rather than how databases work. Memories strengthen with use, fade with neglect, and reconstruct narratives on demand instead of replaying stored summaries.

It overlays any external knowledge base (Obsidian, codebases, conversation archives) as a ghost graph — learning which parts of your existing knowledge matter most without copying or modifying the source.

## Core Ideas

- **Impulses, not episodes.** The system stores what was learned, not what happened. Episodes are provenance, not memory.
- **Connections are the memory.** Weighted edges between impulses carry the real value. They decay without use and strengthen through retrieval.
- **Demand-driven synthesis.** Narratives are reconstructed at query time, never stored. No automatic compilation, no bloat, no hallucinated summaries.
- **Ghost graphs.** External KBs are overlaid as shadow topologies. Content is pulled on demand, never copied. The source stays clean.
- **User owns everything.** Local-first, inspectable, exportable, deletable. Incognito mode means zero trace.

## Current Status

Docs-first. No runnable service yet. The design philosophy, product requirements, and technical direction are documented and ready for Phase 1 implementation.

## Documentation

- [Philosophy](./docs/philosophy.md) — the human-memory-inspired design thinking behind every decision
- [PRD](./docs/PRD.md) — product requirements, capabilities, roadmap with validation gates
- [TRD](./docs/TRD.md) — architecture, data model, activation math, MCP tool surface
- [Source Note](./docs/source-notes/graph-memory.md) — original planning document, preserved for provenance

## Repo Layout

```text
.
├── README.md
└── docs/
    ├── philosophy.md
    ├── PRD.md
    ├── TRD.md
    └── source-notes/
        └── graph-memory.md
```
