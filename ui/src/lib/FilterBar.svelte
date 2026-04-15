<script lang="ts">
  import { onMount } from "svelte";
  import { getAllTags } from "./api";
  import { activeTagFilters } from "./stores";

  let tags: { name: string; color: string }[] = [];

  onMount(async () => {
    try {
      tags = await getAllTags();
    } catch (e) {
      console.error("Failed to load tags:", e);
    }
  });

  function toggleTag(name: string) {
    activeTagFilters.update((s) => {
      const next = new Set(s);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }
</script>

{#if tags.length > 0}
  <div class="filter-bar">
    <span class="filter-label">Tags</span>
    {#each tags as tag}
      <button
        class="filter-pill"
        class:active={$activeTagFilters.has(tag.name)}
        on:click={() => toggleTag(tag.name)}
      >
        <span class="pill-dot" style="background: {tag.color || '#8E99A4'}"></span>
        {tag.name}
      </button>
    {/each}
  </div>
{/if}

<style>
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--bg-panel, rgba(20, 20, 24, 0.7));
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.06));
    overflow-x: auto;
    flex-shrink: 0;
  }

  .filter-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted, #6b7280);
    margin-right: 4px;
    flex-shrink: 0;
  }

  .filter-pill {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: 12px;
    border: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.08));
    background: transparent;
    color: var(--text-secondary, #9ca3af);
    font-size: 11px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .filter-pill:hover {
    background: var(--bg-hover, rgba(255, 255, 255, 0.04));
    color: var(--text-primary, #e5e7eb);
  }

  .filter-pill.active {
    background: var(--bg-hover, rgba(255, 255, 255, 0.08));
    border-color: var(--accent-primary, #5C6BC0);
    color: var(--text-primary, #e5e7eb);
  }

  .pill-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
</style>
