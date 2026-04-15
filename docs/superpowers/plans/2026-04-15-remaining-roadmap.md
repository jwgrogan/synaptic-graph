# Remaining Roadmap — Comprehensive Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete all remaining features: graph UI polish (labels, filtering, linking), Obsidian sync, LLM-powered extraction, and multi-client validation.

**Current state:** 144 tests, 24 MCP tools, Tauri UI with live force graph, tags, source provider tracking.

**Tech Stack:** Rust, Svelte, Pixi.js, d3-force, Tauri v2, rmcp

---

## Phase 6: Graph UI Polish

### Task 1: Node Labels (Hover + Zoom)

**Files:**
- Modify: `ui/src/lib/renderer/nodes.ts`
- Modify: `ui/src/lib/renderer/engine.ts`
- Modify: `ui/src/lib/Galaxy.svelte`

- [ ] **Step 1: Add text labels to nodes**

In `nodes.ts`, after drawing each node circle, add a Pixi `Text` object:

```typescript
import { Container, Graphics, Circle, Text, TextStyle } from "pixi.js";

// Inside renderNodes, after the circle drawing:
const labelStyle = new TextStyle({
  fontFamily: "DM Sans, system-ui, sans-serif",
  fontSize: 10,
  fill: "#666666",
  wordWrap: true,
  wordWrapWidth: 120,
});

const label = new Text({
  text: node.impulse.content.slice(0, 40) + (node.impulse.content.length > 40 ? "..." : ""),
  style: labelStyle,
});
label.anchor.set(0.5, 0);
label.x = 0;
label.y = node.radius + 4;
label.alpha = 0; // Hidden by default — shown on hover/zoom
label.label = "label";
g.addChild(label);
```

- [ ] **Step 2: Show labels on hover**

Add `pointerover`/`pointerout` events to each node Graphics in `renderNodes`:

```typescript
g.on("pointerover", () => {
  const lbl = g.children.find(c => c.label === "label");
  if (lbl) lbl.alpha = 1;
  g.scale.set(1.1);
});
g.on("pointerout", () => {
  const lbl = g.children.find(c => c.label === "label");
  if (lbl) lbl.alpha = 0;
  g.scale.set(1.0);
});
```

- [ ] **Step 3: Show all labels when zoomed in**

In `Galaxy.svelte`, add a zoom level check in the animation tick. When camera zoom > 2.0 (node detail level), set all labels visible:

```typescript
// In tick function:
const zoomLevel = engine.camera.getZoomLevel();
for (const child of engine.nodeLayer.children) {
  const lbl = child.children?.find((c: any) => c.label === "label");
  if (lbl) lbl.alpha = zoomLevel === "node" ? 1 : 0;
}
```

- [ ] **Step 4: Verify and commit**

Run: `cd ui && npx vite build`
Commit: "feat: node labels on hover and zoom-to-detail"

---

### Task 2: Tag-Based Filtering in Graph UI

**Files:**
- Create: `ui/src/lib/FilterBar.svelte`
- Modify: `ui/src/lib/Galaxy.svelte`
- Modify: `ui/src/lib/stores.ts`
- Modify: `ui/src/lib/App.svelte`
- Modify: `ui/src/lib/api.ts`
- Modify: `ui/src-tauri/src/commands.rs`
- Modify: `ui/src-tauri/src/main.rs`

- [ ] **Step 1: Add Tauri commands for tags**

In `ui/src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn get_all_tags(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let tags = db.list_tags().map_err(|e| format!("DB: {}", e))?;
    Ok(tags.iter().map(|t| serde_json::json!({
        "name": t.name, "color": t.color
    })).collect())
}

#[tauri::command]
pub fn get_impulse_tags(state: State<AppState>, impulse_id: String) -> Result<Vec<serde_json::Value>, String> {
    let db = state.db.lock().map_err(|e| format!("Lock: {}", e))?;
    let tags = db.get_tags_for_impulse(&impulse_id).map_err(|e| format!("DB: {}", e))?;
    Ok(tags.iter().map(|t| serde_json::json!({
        "name": t.name, "color": t.color
    })).collect())
}
```

Register in `main.rs` invoke_handler.

- [ ] **Step 2: Add API wrappers**

In `ui/src/lib/api.ts`:
```typescript
export async function getAllTags(): Promise<{ name: string; color: string }[]> {
  return invoke("get_all_tags");
}
```

- [ ] **Step 3: Add filter store**

In `ui/src/lib/stores.ts`:
```typescript
export const activeTagFilters = writable<Set<string>>(new Set());
export const activeProviderFilters = writable<Set<string>>(new Set());
```

- [ ] **Step 4: Create FilterBar component**

`ui/src/lib/FilterBar.svelte`:

A thin horizontal bar above the graph canvas. Shows tag pills (colored dots + name) that toggle filtering. Also shows source provider pills. When a filter is active, only matching nodes are shown (others fade to very low opacity).

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { getAllTags } from "./api";
  import { activeTagFilters } from "./stores";

  let tags: { name: string; color: string }[] = [];

  onMount(async () => {
    tags = await getAllTags();
  });

  function toggleTag(name: string) {
    activeTagFilters.update(s => {
      const next = new Set(s);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }
</script>

{#if tags.length > 0}
<div class="filter-bar">
  {#each tags as tag}
    <button
      class="filter-pill"
      class:active={$activeTagFilters.has(tag.name)}
      on:click={() => toggleTag(tag.name)}
    >
      <span class="pill-dot" style="background: {tag.color}"></span>
      {tag.name}
    </button>
  {/each}
</div>
{/if}
```

Style: thin bar, pills are small rounded buttons with a colored dot.

- [ ] **Step 5: Wire filtering into Galaxy.svelte**

Subscribe to `activeTagFilters`. When filters are active, set non-matching node opacity to 0.1 in the render loop. This requires knowing which impulse has which tags — either fetch tags per impulse on load, or add a bulk endpoint.

- [ ] **Step 6: Add FilterBar to App.svelte above the Galaxy**

- [ ] **Step 7: Verify and commit**

Commit: "feat: tag-based filtering with colored filter pills"

---

### Task 3: Source Provider Badges

**Files:**
- Modify: `ui/src/lib/renderer/nodes.ts`
- Modify: `ui/src/lib/DetailPanel.svelte`

- [ ] **Step 1: Add provider indicator to nodes**

In `renderNodes`, if the impulse has a `source_provider` other than "unknown", draw a tiny colored circle indicator at the top-right of the node:

```typescript
const providerColors: Record<string, number> = {
  "claude": 0xD97757,    // Claude orange
  "openai": 0x10A37F,    // OpenAI green
  "gemini": 0x4285F4,    // Google blue
  "import": 0x8E99A4,    // grey
  "ghost": 0x7B9E87,     // sage
};

if (node.impulse.source_provider && node.impulse.source_provider !== "unknown") {
  const provColor = providerColors[node.impulse.source_provider] || 0x8E99A4;
  g.circle(node.radius * 0.7, -node.radius * 0.7, 2.5);
  g.fill({ color: provColor, alpha: 0.9 });
}
```

- [ ] **Step 2: Show provider in DetailPanel**

Add a provider pill to the detail panel metadata section:
```svelte
{#if detail.impulse.source_provider && detail.impulse.source_provider !== "unknown"}
  <div class="meta-item">
    <span class="label">Source</span>
    <span class="provider-pill" style="background: {providerColor(detail.impulse.source_provider)}">
      {detail.impulse.source_provider}
    </span>
  </div>
{/if}
```

- [ ] **Step 3: Verify and commit**

Commit: "feat: source provider badges on nodes and detail panel"

---

### Task 4: Tag Management UI

**Files:**
- Create: `ui/src/lib/TagManager.svelte`
- Modify: `ui/src/lib/Sidebar.svelte`
- Modify: `ui/src/lib/App.svelte`
- Modify: `ui/src/lib/stores.ts`
- Modify: `ui/src-tauri/src/commands.rs`
- Modify: `ui/src-tauri/src/main.rs`

- [ ] **Step 1: Add Tauri commands for tag CRUD**

```rust
#[tauri::command]
pub fn create_tag(state: State<AppState>, name: String, color: String) -> Result<serde_json::Value, String>

#[tauri::command]
pub fn delete_tag(state: State<AppState>, name: String) -> Result<serde_json::Value, String>

#[tauri::command]
pub fn tag_impulse(state: State<AppState>, impulse_id: String, tag_name: String) -> Result<serde_json::Value, String>

#[tauri::command]
pub fn untag_impulse(state: State<AppState>, impulse_id: String, tag_name: String) -> Result<serde_json::Value, String>
```

- [ ] **Step 2: Create TagManager.svelte**

Full page view for managing tags:
- List all tags with color swatches and memory counts
- Create new tag: name input + color picker (simple preset palette)
- Delete tag button with confirmation
- Click a tag to see all memories with that tag

- [ ] **Step 3: Add "Tags" to sidebar and App.svelte routing**

Add a new sidebar icon for Tags between Fading and Stats.

- [ ] **Step 4: Verify and commit**

Commit: "feat: tag management UI with create/delete and color picker"

---

### Task 5: UI for Linking/Unlinking Nodes

**Files:**
- Modify: `ui/src/lib/Galaxy.svelte`
- Modify: `ui/src/lib/DetailPanel.svelte`
- Modify: `ui/src-tauri/src/commands.rs`
- Modify: `ui/src-tauri/src/main.rs`

- [ ] **Step 1: Add Tauri commands for link/unlink**

```rust
#[tauri::command]
pub fn link_memories(state: State<AppState>, source_id: String, target_id: String, relationship: Option<String>) -> Result<serde_json::Value, String>

#[tauri::command]
pub fn unlink_memories(state: State<AppState>, connection_id: String) -> Result<serde_json::Value, String>
```

- [ ] **Step 2: Add link mode to Galaxy**

When user holds Shift and clicks two nodes in sequence, create a connection between them. Show a visual indicator (dotted line following cursor) when the first node is selected.

Store `linkModeSourceId` in a local variable. On shift+click of first node, set it. On shift+click of second node, call `link_memories` and refresh the graph.

- [ ] **Step 3: Add unlink button to DetailPanel connections**

Each connection in the detail panel gets a small "×" button that calls `unlink_memories`.

- [ ] **Step 4: Verify and commit**

Commit: "feat: shift-click to link nodes, × to unlink from detail panel"

---

## Phase 7: Obsidian Markdown Sync

### Task 6: Markdown Export Layer

**Files:**
- Create: `src/markdown.rs`
- Modify: `src/lib.rs`
- Modify: `src/server.rs`

- [ ] **Step 1: Create markdown.rs module**

```rust
pub fn export_to_markdown(db: &Database, output_dir: &str) -> Result<ExportResult, String>
```

For each confirmed impulse:
- Create a `.md` file named `{id_short}_{sanitized_title}.md`
- File content:
  ```markdown
  ---
  id: {uuid}
  type: {impulse_type}
  weight: {weight}
  created: {date}
  tags: [tag1, tag2]
  source: {provider}
  ---

  {content}

  ## Connections
  - [[{connected_id_short}_{title}]] — {relationship} (weight: {weight})
  ```
- Connections become wikilinks

For each tag, create a `tags/{tag_name}.md` that lists all tagged impulses.

- [ ] **Step 2: Create MCP tool**

`export_to_obsidian(output_dir: String) -> Result<String, String>`

Returns count of files exported.

- [ ] **Step 3: Add periodic re-export option**

`sync_to_obsidian(vault_path: String)` — compares timestamps, only re-exports changed impulses. Does not modify files the user has edited (checks modification time vs last export).

- [ ] **Step 4: Tests**

- test_export_creates_markdown_files
- test_export_includes_connections_as_wikilinks
- test_export_includes_frontmatter
- test_sync_only_exports_changed

- [ ] **Step 5: Verify and commit**

Commit: "feat: markdown export layer for Obsidian vault sync"

---

### Task 7: Obsidian Link UI

**Files:**
- Modify: `ui/src/lib/GhostList.svelte`
- Modify: `ui/src-tauri/src/commands.rs`
- Modify: `ui/src-tauri/src/main.rs`

- [ ] **Step 1: Add "Link Obsidian Vault" section to External Graphs**

Two modes displayed in the UI:
1. **Read-only (Ghost Graph)** — existing functionality, folder picker
2. **Linked (Sync)** — folder picker + toggle for "auto-sync changes"

For linked mode, add a Tauri command:
```rust
#[tauri::command]
pub async fn sync_to_obsidian(state: State<'_, AppState>, vault_path: String) -> Result<serde_json::Value, String>
```

- [ ] **Step 2: Add sync status indicator**

Show when the vault was last synced, how many files are out of date.

- [ ] **Step 3: Add manual "Sync Now" button**

- [ ] **Step 4: Verify and commit**

Commit: "feat: Obsidian vault linking with read-only and sync modes"

---

## Phase 8: LLM-Powered Features

### Task 8: recall_narrative Tool

**Files:**
- Modify: `src/server.rs`

- [ ] **Step 1: Implement handle_recall_narrative**

This tool takes a topic, runs retrieve_context to get relevant impulses, then assembles them into a narrative prompt and returns it. The actual narrative generation happens in the LLM — the tool provides the structured context.

```rust
pub fn handle_recall_narrative(&self, topic: String) -> Result<String, String> {
    let db = self.db.lock().unwrap();
    let engine = ActivationEngine::new(&db);
    let result = engine.retrieve(&RetrievalRequest {
        query: topic.clone(),
        max_results: 20,
        max_hops: 5,
    })?;

    // Assemble narrative context
    let mut narrative_parts = Vec::new();
    for mem in &result.memories {
        let tags = db.get_tags_for_impulse(&mem.impulse.id).unwrap_or_default();
        let tag_str = tags.iter().map(|t| t.name.clone()).collect::<Vec<_>>().join(", ");
        let conns = db.get_connections_for_node(&mem.impulse.id).unwrap_or_default();

        narrative_parts.push(serde_json::json!({
            "content": mem.impulse.content,
            "type": mem.impulse.impulse_type.as_str(),
            "weight": mem.impulse.weight,
            "activation_score": mem.activation_score,
            "tags": tag_str,
            "connections": conns.len(),
            "engagement": mem.impulse.engagement_level.as_str(),
        }));
    }

    let response = serde_json::json!({
        "topic": topic,
        "impulse_count": result.memories.len(),
        "narrative_context": narrative_parts,
        "instruction": "Reconstruct a coherent narrative from these connected impulses. Tell the story of what was learned, decided, and understood about this topic. Use the activation scores to prioritize more relevant pieces."
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Serialization error: {}", e))
}
```

- [ ] **Step 2: Add MCP tool annotation**

- [ ] **Step 3: Test manually**

Ask an AI: "recall the narrative of how synaptic-graph was designed" — it should call recall_narrative, get structured context, and weave a story.

- [ ] **Step 4: Commit**

Commit: "feat: recall_narrative tool for LLM-powered narrative reconstruction"

---

### Task 9: End-of-Session Extraction (propose_memories)

**Files:**
- Modify: `src/server.rs`
- Modify: `src/extraction.rs`

- [ ] **Step 1: Implement handle_propose_memories**

Takes session content (conversation text), runs engagement assessment, extracts candidate impulses based on depth.

```rust
pub fn handle_propose_memories(
    &self,
    session_content: String,
    session_duration_minutes: Option<f64>,
) -> Result<String, String> {
    // Assess engagement
    let word_count = session_content.split_whitespace().count();
    let decision_count = extraction::count_keywords(&session_content, extraction::DECISION_KEYWORDS);
    let emotional_count = extraction::count_keywords(&session_content, extraction::EMOTIONAL_KEYWORDS);

    let signals = extraction::EngagementSignals {
        total_turns: word_count / 50, // rough estimate
        avg_user_message_length: (word_count / ((word_count / 50).max(1))) as f64,
        avg_assistant_message_length: 0.0,
        session_duration_minutes: session_duration_minutes.unwrap_or(30.0),
        explicit_save_count: 0,
        topic_count: 1,
        decision_keywords_found: decision_count,
        emotional_keywords_found: emotional_count,
    };

    let depth = extraction::assess_engagement(&signals);

    let response = serde_json::json!({
        "engagement_score": signals.engagement_score(),
        "depth": format!("{:?}", depth),
        "max_proposals": depth.max_proposals(),
        "instruction": format!(
            "Extract up to {} key impulses from this session. Focus on: decisions made, insights discovered, preferences expressed, and patterns observed. Return each as a separate quick_save call with appropriate type and engagement_level.",
            depth.max_proposals()
        ),
        "session_stats": {
            "word_count": word_count,
            "decision_keywords": decision_count,
            "emotional_keywords": emotional_count,
        }
    });

    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Serialization error: {}", e))
}
```

- [ ] **Step 2: Add MCP tool**

- [ ] **Step 3: Update MCP instructions**

Add to the ServerInfo instructions:
```
"At the end of long sessions, call propose_memories with the session summary to extract key learnings."
```

- [ ] **Step 4: Commit**

Commit: "feat: propose_memories tool for end-of-session extraction"

---

## Phase 9: Quality & Validation

### Task 10: Multi-Client MCP Validation

**Files:**
- Create: `tests/test_mcp_integration.sh`

- [ ] **Step 1: Create a shell script that tests the MCP handshake**

```bash
#!/bin/bash
# Test MCP server responds to initialize and tools/list

echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | \
  timeout 5 ./target/release/synaptic-graph 2>/dev/null | \
  python3 -c "import sys,json; d=json.loads(sys.stdin.readline()); assert 'result' in d; print('PASS: initialize')"

echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | \
  timeout 5 ./target/release/synaptic-graph 2>/dev/null | \
  python3 -c "import sys,json; d=json.loads(sys.stdin.readline()); tools=[t['name'] for t in d['result']['tools']]; print(f'PASS: {len(tools)} tools registered'); assert len(tools) >= 24"
```

- [ ] **Step 2: Test with Claude Code**

Manually verify: start a new Claude Code session, confirm synaptic-graph tools appear in `/mcp`, test quick_save and retrieve_context.

- [ ] **Step 3: Test with Claude Desktop**

Manually verify: open Claude Desktop, confirm tools available, test save and recall.

- [ ] **Step 4: Document results**

Create `docs/validation/multi-client-results.md` with pass/fail for each client.

- [ ] **Step 5: Commit**

Commit: "test: multi-client MCP validation script and results"

---

### Task 11: Final Test Hardening

- [ ] **Step 1: Run full test suite 3x consecutively**

```bash
cargo test && cargo test && cargo test
```

- [ ] **Step 2: Single-threaded run**

```bash
cargo test -- --test-threads=1
```

- [ ] **Step 3: Release mode**

```bash
cargo test --release
```

- [ ] **Step 4: Clippy clean**

```bash
cargo clippy -- -W clippy::all
```

- [ ] **Step 5: Build release binary**

```bash
cargo build --release
```

- [ ] **Step 6: Build Tauri release**

```bash
cd ui && npx tauri build
```

- [ ] **Step 7: Commit**

Commit: "chore: final validation — all tests passing, release builds clean"

---

## Summary

| Phase | Tasks | Focus |
|-------|-------|-------|
| 6 | 1-5 | Graph UI: labels, filtering, provider badges, tag manager, node linking |
| 7 | 6-7 | Obsidian: markdown export, vault linking with sync |
| 8 | 8-9 | LLM: recall_narrative, propose_memories |
| 9 | 10-11 | Validation: multi-client, test hardening, release builds |

Total: 11 tasks. Phases 6 and 8 can run in parallel (UI work vs backend tools).
