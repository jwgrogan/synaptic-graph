# Synaptic Graph

A portable, human-memory-inspired memory layer for AI systems. Your AI conversations generate knowledge — Synaptic Graph captures it, connects it, and makes it available everywhere.

## What it does

Synaptic Graph is an MCP server that gives any AI assistant persistent, portable memory. Memories are stored as a weighted graph of impulses (learned things) connected by relationships that strengthen with use and fade with neglect — just like human memory.

### Key features

- **Spreading activation retrieval** — queries light up matching memories and propagate through connections, surfacing related context you didn't explicitly search for
- **Weighted decay** — memories strengthen when accessed and fade when neglected. Nothing is deleted — even old memories can resurface with the right trigger
- **Ghost graphs** — overlay your existing knowledge bases (Obsidian vaults, markdown directories) without copying or modifying them
- **Tags and source tracking** — organize memories by topic, see which AI provider contributed each memory
- **Auto-linking** — new memories automatically connect to existing ones via keyword overlap
- **Obsidian export** — export your memory graph as linked markdown files with frontmatter and wikilinks
- **Desktop app** — Tauri-based galaxy visualization with interactive force-directed graph
- **Incognito mode** — full blackout, zero trace when you need privacy

### MCP Tools (28)

Memory: `save_memory`, `quick_save`, `retrieve_context`, `recall_narrative`, `update_memory`, `delete_memory`, `inspect_memory`, `confirm_proposal`, `dismiss_proposal`, `list_candidates`, `propose_memories`, `explain_recall`

Graph: `link_memories`, `unlink_memories`, `register_ghost_graph`, `refresh_ghost_graph`, `pull_through`, `export_to_obsidian`

Tags: `create_tag`, `list_tags`, `tag_memory`, `untag_memory`

System: `memory_status`, `set_incognito`, `create_backup`, `sync_export`, `sync_status`

## Install

### As an MCP server (Claude, GPT, any MCP client)

```bash
# Build from source
git clone https://github.com/jwgrogan/synaptic-graph.git
cd synaptic-graph
cargo build --release
```

Add to your MCP client config:
```json
{
  "mcpServers": {
    "synaptic-graph": {
      "command": "/path/to/synaptic-graph/target/release/synaptic-graph"
    }
  }
}
```

### Desktop app

```bash
cd ui
npm install
npx tauri dev     # development
npx tauri build   # production .dmg/.exe
```

## How it works

Memories are stored as **impulses** — atomic units of knowledge (not raw conversation transcripts). When you save a memory, the system:

1. Redacts secrets and PII
2. Creates a candidate impulse
3. Auto-links to existing memories via keyword overlap
4. Confirms and indexes for full-text search

When you retrieve context, **spreading activation** propagates through the weighted graph — directly matching memories activate first, then energy spreads through connections to surface related context. Frequently accessed connections strengthen. Unused ones fade.

## Architecture

- **Rust** core with SQLite (single portable file)
- **MCP server** over stdio (works with any MCP-compatible client)
- **Tauri v2** desktop app with Svelte + Pixi.js
- **148 tests** across all modules

## Documentation

- [Philosophy](./docs/philosophy.md) — the human-memory-inspired design thinking
- [PRD](./docs/PRD.md) — product requirements and roadmap
- [TRD](./docs/TRD.md) — technical architecture and data model

## License

Apache 2.0 — see [LICENSE](./LICENSE)
