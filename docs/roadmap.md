# Roadmap

## Phase 0: Repo Framing

- establish the service-first boundary for portable memory
- document the core problem, thesis, and BYOKB stance
- preserve initial source notes for provenance
- create a clear starting point for future architectural decisions

## Phase 1: Local Single-User Memory Service

- define the first graph-shaped local data model
- implement local persistence and memory lifecycle operations
- support ingestion from assistant interactions and coding sessions
- support task-conditioned retrieval with provenance

## Phase 2: KB Views And Assistant Integration

- generate note or markdown views over the graph
- support source-preserving KB workflows and unified KB generation
- build a thin assistant client for retrieval and memory writes
- validate fallback behavior when the service is unavailable

## Phase 3: Sync And Portability

- explore backup and cross-device portability
- define identity and sync semantics
- evaluate optional cloud sync without making it the canonical source of truth
- preserve local-first guarantees as the system expands
