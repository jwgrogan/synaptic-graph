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
  <h2>External Graphs</h2>
  <p class="subtitle">Connect your knowledge bases — Obsidian vaults, repos, or any markdown directory.</p>

  {#if sources.length === 0}
    <p class="empty">No external graphs connected. Use the MCP tool <code>register_ghost_graph</code> with a name and path to your vault, or register one below.</p>

    <div class="register-hint">
      <h3>Quick Connect — Obsidian</h3>
      <p>Tell your AI assistant:</p>
      <code class="hint-code">"Register my Obsidian vault as a ghost graph. Path: ~/path/to/your/vault"</code>
    </div>
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
    margin-bottom: 4px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 13px;
    margin-bottom: 24px;
  }

  .empty {
    color: var(--text-muted);
    font-size: 14px;
    margin-bottom: 16px;
  }

  .register-hint {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 16px;
    margin-top: 12px;
  }

  .register-hint h3 {
    font-size: 13px;
    color: var(--accent-cyan);
    margin-bottom: 6px;
  }

  .register-hint p {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .hint-code {
    display: block;
    font-size: 11px;
    color: var(--accent-indigo);
    background: var(--bg-deep);
    padding: 8px 12px;
    border-radius: 4px;
    font-family: monospace;
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
