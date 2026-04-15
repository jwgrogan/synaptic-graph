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
    window.dispatchEvent(new CustomEvent("navigate-to-node", { detail: { id } }));
    searchOpen.set(false);
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if $searchOpen}
  <div class="search-overlay" on:click={() => { searchOpen.set(false); clearSearch(); }}>
    <div class="search-palette" on:click|stopPropagation>
      <div class="search-input-wrapper">
        <input
          class="search-input"
          placeholder="Search your memory..."
          bind:value={query}
          on:keydown={(e) => e.key === "Enter" && handleSearch()}
          autofocus
        />
      </div>

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
              <div class="result-meta">
                <span class="result-score">{mem.activation_score.toFixed(3)}</span>
              </div>
            </button>
          {/each}

          {#if $searchResults.ghost_activations.length > 0}
            <div class="ghost-section-label">Ghost Graph Matches</div>
            {#each $searchResults.ghost_activations as ghost}
              <div class="result-item ghost">
                <div class="result-content">{ghost.title}</div>
                <div class="result-meta">
                  <span>{ghost.source_graph}</span>
                  <span class="result-score">{ghost.activation_score.toFixed(3)}</span>
                </div>
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
    background: rgba(0, 0, 0, 0.08);
    display: flex;
    justify-content: center;
    padding-top: 15vh;
    z-index: 100;
  }

  .search-palette {
    width: 520px;
    max-height: 480px;
    background: var(--bg-panel);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-panel);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .search-input-wrapper {
    padding: 0 20px;
  }

  .search-input {
    width: 100%;
    padding: 16px 0;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
    font-size: 15px;
    font-family: var(--font-body);
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-faint);
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
    padding: 10px 20px;
    border: none;
    border-bottom: 1px solid var(--border-subtle);
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    transition: background var(--transition-fast);
  }

  .result-item:hover {
    background: var(--bg-hover);
  }

  .result-item.ghost {
    cursor: default;
    opacity: 0.5;
  }

  .result-content {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.4;
    flex: 1;
  }

  .result-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: var(--text-muted);
    flex-shrink: 0;
    align-items: center;
  }

  .result-score {
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 10px;
    color: var(--text-faint);
  }

  .ghost-section-label {
    padding: 10px 20px 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }
</style>
