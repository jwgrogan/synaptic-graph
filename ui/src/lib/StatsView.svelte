<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getMemoryStats } from "./api";
  import type { MemoryStats } from "./types";

  let stats: MemoryStats | null = null;
  let depthAnalysis: any = null;

  function depthColor(score: number): string {
    if (score < 30) return "var(--accent-rose)";
    if (score < 60) return "var(--accent-amber, #D4A843)";
    return "var(--accent-sage)";
  }

  onMount(async () => {
    const [s, a] = await Promise.all([
      getMemoryStats(),
      invoke<any>("analyze_memory_profile").catch(() => null),
    ]);
    stats = s;
    depthAnalysis = a;
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

  {#if depthAnalysis}
    <div class="depth-section">
      <h3>Memory Depth</h3>

      <div class="depth-hero">
        <div class="depth-score" style="color: {depthColor(depthAnalysis.depth_score)}">
          {depthAnalysis.depth_score}
        </div>
        <div class="depth-meta">
          <div class="depth-label">{depthAnalysis.depth_label}</div>
          <div class="depth-counts">
            {depthAnalysis.imported_count} imported &middot; {depthAnalysis.high_weight_count} high-weight
          </div>
        </div>
      </div>

      {#if depthAnalysis.by_type && Object.keys(depthAnalysis.by_type).length > 0}
        <div class="pill-section">
          <div class="pill-label">Type distribution</div>
          <div class="pill-row">
            {#each Object.entries(depthAnalysis.by_type) as [type, count]}
              <span class="pill">{type} <strong>{count}</strong></span>
            {/each}
          </div>
        </div>
      {/if}

      {#if depthAnalysis.by_source && Object.keys(depthAnalysis.by_source).length > 0}
        <div class="pill-section">
          <div class="pill-label">Source distribution</div>
          <div class="pill-row">
            {#each Object.entries(depthAnalysis.by_source) as [source, count]}
              <span class="pill">{source} <strong>{count}</strong></span>
            {/each}
          </div>
        </div>
      {/if}

      {#if depthAnalysis.gaps && depthAnalysis.gaps.length > 0}
        <div class="gaps-list">
          {#each depthAnalysis.gaps as gap}
            <div class="gap-card">{gap}</div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .stats-view {
    padding: 40px;
    max-width: 600px;
  }

  h2 {
    font-family: var(--font-body);
    font-size: 20px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 32px;
  }

  .stat-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }

  .stat-card {
    background: var(--bg-panel-solid);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 28px 24px;
  }

  .stat-value {
    font-family: var(--font-body);
    font-size: 36px;
    font-weight: 300;
    color: var(--accent-primary);
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

  /* Memory Depth section */
  .depth-section {
    margin-top: 36px;
    padding-top: 24px;
    border-top: 1px solid var(--border-subtle);
  }

  .depth-section h3 {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 16px;
  }

  .depth-hero {
    display: flex;
    align-items: center;
    gap: 20px;
    margin-bottom: 20px;
  }

  .depth-score {
    font-family: var(--font-body);
    font-size: 48px;
    font-weight: 300;
    line-height: 1;
  }

  .depth-meta {
    flex: 1;
  }

  .depth-label {
    font-size: 13px;
    color: var(--text-primary);
    font-weight: 500;
    margin-bottom: 4px;
  }

  .depth-counts {
    font-size: 12px;
    color: var(--text-muted);
  }

  .pill-section {
    margin-bottom: 12px;
  }

  .pill-label {
    font-size: 11px;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .pill {
    font-size: 11px;
    color: var(--text-secondary);
    background: var(--bg-panel-solid);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    padding: 3px 10px;
  }

  .pill strong {
    color: var(--text-primary);
    font-weight: 600;
    margin-left: 2px;
  }

  .gaps-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 16px;
  }

  .gap-card {
    font-size: 12px;
    color: var(--text-secondary);
    padding: 10px 14px;
    background: var(--bg-panel-solid);
    border: 1px solid var(--border-subtle);
    border-left: 3px solid var(--accent-rose);
    border-radius: var(--radius-sm);
  }
</style>
