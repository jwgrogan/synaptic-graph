<script lang="ts">
  import { searchOpen, searchResults, activationPath } from "./stores";
  import { searchMemories } from "./api";
  import type { SearchResult } from "./types";

  let query = "";
  let loading = false;

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      searchOpen.update((v) => !v);
      if (!$searchOpen) {
        clearSearch();
      }
    }
    if (e.key === "Escape" && $searchOpen) {
      searchOpen.set(false);
      clearSearch();
    }
  }

  async function handleSearch() {
    if (!query.trim()) return;
    loading = true;
    try {
      const results = await searchMemories(query);
      searchResults.set(results);

      // Build activation path set
      const pathIds = new Set<string>();
      for (const mem of results.memories) {
        pathIds.add(mem.id);
        for (const pathId of mem.activation_path) {
          pathIds.add(pathId);
        }
      }
      activationPath.set(pathIds);
    } catch (e) {
      console.error("Search failed:", e);
    }
    loading = false;
  }

  function clearSearch() {
    query = "";
    searchResults.set(null);
    activationPath.set(new Set());
  }

  function selectResult(id: string) {
    // Dispatch a custom event that Galaxy.svelte can listen to
    window.dispatchEvent(new CustomEvent("navigate-to-node", { detail: { id } }));
    searchOpen.set(false);
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if $searchOpen}
  <div class="search-overlay" on:click={() => { searchOpen.set(false); clearSearch(); }}>
    <div class="search-palette" on:click|stopPropagation>
      <input
        class="search-input"
        placeholder="Search your memory..."
        bind:value={query}
        on:keydown={(e) => e.key === "Enter" && handleSearch()}
        autofocus
      />

      {#if loading}
        <div class="search-status">Searching...</div>
      {/if}

      {#if $searchResults}
        <div class="results">
          {#if $searchResults.memories.length === 0}
            <div class="search-status">No memories found</div>
          {/if}

          {#each $searchResults.memories as mem}
            <button class="result-item" on:click={() => selectResult(mem.id)}>
              <div class="result-content">{mem.content.slice(0, 100)}{mem.content.length > 100 ? '...' : ''}</div>
              <div class="result-meta">activation: {mem.activation_score.toFixed(3)}</div>
            </button>
          {/each}

          {#if $searchResults.ghost_activations.length > 0}
            <div class="ghost-section-label">Ghost Graph Matches</div>
            {#each $searchResults.ghost_activations as ghost}
              <div class="result-item ghost">
                <div class="result-content">{ghost.title}</div>
                <div class="result-meta">{ghost.source_graph} · score {ghost.activation_score.toFixed(3)}</div>
              </div>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .search-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    padding-top: 15vh;
    z-index: 100;
  }

  .search-palette {
    width: 560px;
    max-height: 500px;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .search-input {
    width: 100%;
    padding: 16px 20px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
    font-size: 16px;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .search-status {
    padding: 12px 20px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .results {
    overflow-y: auto;
    max-height: 380px;
  }

  .result-item {
    width: 100%;
    padding: 12px 20px;
    border: none;
    border-bottom: 1px solid rgba(99, 102, 241, 0.08);
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }

  .result-item:hover {
    background: rgba(99, 102, 241, 0.1);
  }

  .result-item.ghost {
    cursor: default;
    opacity: 0.6;
  }

  .result-content {
    font-size: 13px;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .result-meta {
    font-size: 11px;
    color: var(--text-muted);
  }

  .ghost-section-label {
    padding: 8px 20px 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--accent-cyan);
    opacity: 0.6;
  }
</style>
