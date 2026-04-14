# Tauri UI Design Spec: Memory Galaxy

## Purpose

The Tauri UI is the visual face of memory-graph. Its primary purpose is not memory management — that happens through MCP tools in conversation. The UI is the **"see your brain"** experience: a living visualization of your knowledge graph that's inherently interesting to explore, reveals things about yourself you didn't know, and makes you want to keep the system running.

It's a marketing tool as much as a utility. The bar is: someone sees your screen and asks "what is that?"

## Visual Direction: Dark Cosmos

The knowledge graph is rendered as a galaxy. Dark background, nebula-colored clusters, nodes as stars, connections as light trails. Ghost graphs (Obsidian vaults, etc.) appear as ethereal translucent regions — visually distinct from core memory but part of the same cosmos.

Key visual properties:
- **Dark background** (#06060f to #0a0a1a range)
- **Clusters glow as nebulae** — auto-colored by topic, with radial gradients
- **Nodes are stars** — brightness/size proportional to weight
- **Connections are light trails** — opacity proportional to connection weight
- **Ghost graphs are ethereal** — translucent, dashed connections, dimmer nodes
- **Bridge connections** between memory and ghost regions are faint dashed light trails
- **Emotional engagement** maps to color warmth — high engagement nodes glow warmer

## Interaction Model: Orbital Zoom

The galaxy uses semantic zoom with three progressive detail levels:

### Level 1: Full Galaxy
- See the entire knowledge cosmos at once
- Core memory as vivid nebulae, ghost graphs as ethereal regions
- No individual node labels — just colored clouds and faint structure
- Cluster positions determined by force-directed layout; clusters detected via connected-component grouping on strongly-weighted connections (threshold TBD during implementation, start at 0.3)
- Colors auto-assigned per cluster (palette of 8-10 nebula colors, cycled)
- Click/scroll to zoom into a region

### Level 2: Cluster Focus
- Zoomed into a single cluster
- Individual nodes visible with labels
- Connections between nodes visible with varying opacity by weight
- Cluster metadata shown: label, node count, average weight, engagement level
- Click a node to zoom to Level 3

### Level 3: Node Detail
- Selected node glows brightly with a selection halo
- Connected nodes visible around it
- Detail panel slides in from the right
- Graph remains interactive — click connected nodes to traverse

## Detail Panel (Level 3)

Slides in from the right side of the canvas. Shows:
- **Content** — the impulse text
- **Weight** — visualized as a gradient bar (0.0 to 1.0), not just a number
- **Engagement** — color dot indicator (amber = high, gray = low)
- **Valence** — color dot indicator (green = positive, red = negative, gray = neutral)
- **Type** — heuristic, preference, decision, pattern, observation
- **Source** — how it was created (explicit save, extraction, pull-through) and source ref
- **Last accessed** — relative time
- **Connections** — list of connected nodes, clickable to navigate. Each shows relationship label and weight. Fading connections shown with reduced opacity.

## Ghost Graph Visualization

When ghost graphs are registered, they appear as distinct regions in the galaxy:
- **Visual treatment** — more translucent/ethereal than core memory
- **Pulled-through nodes** are brighter (they've crossed the barrier into memory)
- **Never-accessed nodes** are dim wisps
- **Internal structure** mirrors the source KB's link topology (wikilinks for Obsidian)
- **Clicking into a ghost region** shows the KB graph structure rendered in cosmos aesthetic — effectively an Obsidian graph view with learned weights overlaid
- **Bridge connections** between ghost and memory nodes show the activation pathways

## Sidebar

Minimal left sidebar for context switching:
- **Galaxy** (default) — the main visualization
- **Ghost Graphs** — list registered sources, node counts, scan status
- **Stats** — memory count, connection count, top clusters, engagement distribution
- **Settings** — ghost graph registration, DB path, theme (future)

## Search

Command-palette style, triggered by Cmd+K (or Ctrl+K):
- Runs `retrieve_context` against the memory graph
- Highlights the activation path in the galaxy view — nodes light up along the retrieval path
- Results listed in the palette, clicking a result navigates to that node in the galaxy
- Ghost node activations shown separately with source indicator

## Tech Stack

- **Tauri v2** — Rust backend, webview frontend, single binary
- **Svelte** — frontend framework (small bundle, reactive, good with canvas)
- **Pixi.js or d3-force** — graph rendering on HTML5 Canvas (not DOM nodes — needs to handle thousands of nodes smoothly)
- **memory-graph crate** — imported directly as a Rust library dependency. No MCP/HTTP between UI and backend.
- **Tauri commands** — the frontend calls typed Rust functions via Tauri's invoke system

## Data Flow

```
Svelte Frontend  <--tauri invoke-->  Tauri Commands  <--direct call-->  memory-graph crate
                                                                              |
                                                                         SQLite DB
```

No MCP protocol between the UI and backend. MCP is for external AI clients only. The Tauri app has direct Rust access to the memory-graph library, which means typed responses, no serialization overhead for internal operations, and the full power of the Rust API.

## What's in v1 of the UI

- Galaxy visualization with orbital zoom (3 levels)
- Force-directed graph layout with topic clustering
- Nebula visual effects (glow, gradients, light trails)
- Detail panel for node inspection
- Ghost graph regions (ethereal visual treatment)
- Search via command palette with activation path highlighting
- Sidebar navigation (galaxy, ghost graphs, stats)
- Responsive — works in the Tauri window at various sizes

## What's NOT in v1 of the UI

- Proposal review workflow (add when Phase 2 backend ships)
- Memory editing/deletion from the UI (use MCP tools)
- Settings beyond ghost graph registration
- 3D rendering
- Animations/transitions beyond basic zoom (can polish later)
- Mobile or web deployment (Tauri desktop only)

## Performance Targets

- **Render 10,000 nodes** at Level 1 without frame drops (Canvas/WebGL, not DOM)
- **Smooth zoom transitions** between levels
- **Search results** in under 200ms (matching backend retrieval target)
- **Startup** shows the galaxy within 2 seconds of app launch

## Build Sequence

1. Tauri v2 scaffold with Svelte frontend
2. Wire memory-graph crate as dependency, expose Tauri commands
3. Basic canvas renderer with force-directed layout (nodes + connections)
4. Add nebula visual effects (glow, gradients, cluster coloring)
5. Implement orbital zoom with three detail levels
6. Detail panel component
7. Ghost graph rendering (ethereal overlay)
8. Search with activation path highlighting
9. Sidebar and stats view
10. Performance tuning and visual polish
