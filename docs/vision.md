# Vision

## Summary

`memory-graph` explores a local-first, user-owned memory layer for LLMs. The goal is to give users durable memory that can move with them across providers, assistants, and workflows while remaining inspectable, editable, and portable.

The project is centered on a portable memory graph, not on any single assistant. Assistants are expected to plug into this layer, contribute events or candidate memories, and retrieve context that is relevant to the task at hand.

## Core Product Idea

The memory layer stores durable context as a graph of:

- entities
- relationships
- events
- artifacts
- preferences

This is meant to support both personal and project memory without collapsing everything into a flat note archive or a provider-specific chat transcript history.

## Design Principles

- User-owned: memory should live under user control, not inside a single provider surface.
- Local-first: the canonical memory should start locally, with sync treated as an extension rather than a prerequisite.
- Inspectable: users should be able to audit what was saved, where it came from, and why it was recalled.
- Source-preserving: existing knowledge bases can inform the graph without automatically becoming its canonical store.
- Portable: assistants and providers should be able to participate without owning the memory model.

## BYOKB

An important part of the concept is BYOKB: bring your own knowledge base.

In practice, that means:

- existing KB tools such as Obsidian can act as inputs, views, or export targets
- the graph can preserve links back to source material instead of rewriting the source by default
- users can create a new unified KB from the graph and its source references when they want a synthesized view
- the original source can remain clean unless the user explicitly chooses to publish changes back

## V1 Scope

The first meaningful version should stay narrow:

- single-user local deployment
- graph-shaped memory model covering entities, relationships, events, artifacts, and preferences
- ingestion from assistant interactions and coding sessions
- task-conditioned retrieval and context assembly
- provenance, confidence, edit, delete, and merge controls
- markdown or note-oriented export and view generation

## Explicit Non-Goals For V1

- team collaboration semantics
- enterprise permissions or policy systems
- hosted SaaS-first product design
- cloud sync as a prerequisite for usefulness
- provider-specific lock-in integrations
- advanced visual graph browsing as a requirement for early validation

## Product Boundary

The portable memory graph should be treated as a separate product layer.

- It is not an internal memory subsystem that belongs to one assistant.
- It should be usable across chat, coding, and planning surfaces.
- Assistant-specific integrations should remain thin and replaceable.
- The integration mechanism can evolve later once the service boundary is clearer.
