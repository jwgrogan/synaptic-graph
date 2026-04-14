<script lang="ts">
  import { onMount } from "svelte";
  import { getGhostSources } from "./api";
  import type { GhostSource } from "./types";

  let sources: GhostSource[] = [];

  onMount(async () => {
    sources = await getGhostSources();
  });
</script>

<div class="ghost-list">
  <h2>Ghost Graphs</h2>

  {#if sources.length === 0}
    <p class="empty">No ghost graphs registered. Use the MCP tools to register an external knowledge base.</p>
  {:else}
    {#each sources as source}
      <div class="ghost-card">
        <div class="ghost-name">{source.name}</div>
        <div class="ghost-meta">
          <span>{source.source_type}</span>
          <span>{source.node_count} nodes</span>
          {#if source.last_scanned_at}
            <span>scanned {new Date(source.last_scanned_at).toLocaleDateString()}</span>
          {/if}
        </div>
        <div class="ghost-path">{source.root_path}</div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .ghost-list {
    padding: 32px;
    max-width: 600px;
  }

  h2 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .empty {
    color: var(--text-muted);
    font-size: 14px;
  }

  .ghost-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
  }

  .ghost-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--accent-cyan);
    margin-bottom: 6px;
  }

  .ghost-meta {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .ghost-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: monospace;
  }
</style>
