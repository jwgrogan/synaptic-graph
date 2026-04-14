<script lang="ts">
  import { onMount } from "svelte";
  import { getMemoryStats } from "./api";
  import type { MemoryStats } from "./types";

  let stats: MemoryStats | null = null;

  onMount(async () => {
    stats = await getMemoryStats();
  });
</script>

<div class="stats-view">
  <h2>Memory Stats</h2>

  {#if stats}
    <div class="stat-grid">
      <div class="stat-card">
        <div class="stat-value">{stats.total_impulses}</div>
        <div class="stat-label">Total Memories</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.confirmed_impulses}</div>
        <div class="stat-label">Confirmed</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.candidate_impulses}</div>
        <div class="stat-label">Candidates</div>
      </div>
      <div class="stat-card">
        <div class="stat-value">{stats.total_connections}</div>
        <div class="stat-label">Connections</div>
      </div>
    </div>
  {:else}
    <p class="loading">Loading stats...</p>
  {/if}
</div>

<style>
  .stats-view {
    padding: 32px;
    max-width: 600px;
  }

  h2 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 20px;
  }

  .stat-value {
    font-size: 32px;
    font-weight: 700;
    color: var(--accent-indigo);
  }

  .stat-label {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .loading {
    color: var(--text-muted);
  }
</style>
