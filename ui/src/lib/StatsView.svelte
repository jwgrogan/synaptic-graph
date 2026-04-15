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
  <h2>Statistics</h2>

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
    padding: 40px;
    max-width: 600px;
  }

  h2 {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 400;
    color: var(--text-primary);
    margin-bottom: 32px;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 28px 24px;
  }

  .stat-value {
    font-family: var(--font-display);
    font-size: 36px;
    font-weight: 300;
    color: var(--text-primary);
    line-height: 1;
  }

  .stat-label {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .loading {
    color: var(--text-muted);
  }
</style>
