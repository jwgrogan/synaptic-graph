<script lang="ts">
  import { onMount } from "svelte";
  import { getAllImpulses, getAllConnections } from "./api";
  import type { Impulse, Connection } from "./types";

  interface FadingMemory {
    impulse: Impulse;
    effectiveWeight: number;
    hoursSinceAccess: number;
    connectionCount: number;
  }

  let fading: FadingMemory[] = [];
  let loading = true;

  // Decay constants (must match Rust)
  const DECAY_SEMANTIC = 0.0005;
  const DECAY_EPISODIC = 0.005;

  function decayRate(type: string): number {
    return type === "observation" ? DECAY_EPISODIC : DECAY_SEMANTIC;
  }

  function effectiveWeight(weight: number, hours: number, lambda: number): number {
    return Math.max(0.001, weight * Math.exp(-lambda * hours));
  }

  onMount(async () => {
    try {
      const impulses = await getAllImpulses();
      const connections = await getAllConnections();

      const now = Date.now();
      const connCounts = new Map<string, number>();
      for (const c of connections) {
        connCounts.set(c.source_id, (connCounts.get(c.source_id) || 0) + 1);
        connCounts.set(c.target_id, (connCounts.get(c.target_id) || 0) + 1);
      }

      const scored: FadingMemory[] = impulses.map((imp) => {
        const lastAccessed = new Date(imp.last_accessed_at).getTime();
        const hoursSince = (now - lastAccessed) / 3600000;
        const lambda = decayRate(imp.impulse_type);
        const ew = effectiveWeight(imp.weight, hoursSince, lambda);
        return {
          impulse: imp,
          effectiveWeight: ew,
          hoursSinceAccess: hoursSince,
          connectionCount: connCounts.get(imp.id) || 0,
        };
      });

      scored.sort((a, b) => a.effectiveWeight - b.effectiveWeight);
      fading = scored.filter((m) => m.effectiveWeight < m.impulse.weight * 0.8);
    } catch (err) {
      console.error("Failed to load fading memories:", err);
    }
    loading = false;
  });

  function formatTime(hours: number): string {
    if (hours < 1) return "just now";
    if (hours < 24) return `${Math.round(hours)}h ago`;
    const days = Math.round(hours / 24);
    if (days < 30) return `${days}d ago`;
    return `${Math.round(days / 30)}mo ago`;
  }

  function fadingLevel(ratio: number): string {
    if (ratio < 0.3) return "critical";
    if (ratio < 0.5) return "significant";
    if (ratio < 0.7) return "moderate";
    return "slight";
  }

  function borderColor(ratio: number): string {
    if (ratio < 0.3) return "var(--accent-rose)";
    if (ratio < 0.7) return "var(--accent-warm)";
    return "var(--accent-sage)";
  }

  function barColor(ratio: number): string {
    if (ratio < 0.3) return "var(--accent-rose)";
    if (ratio < 0.7) return "var(--accent-warm)";
    return "var(--accent-sage)";
  }
</script>

<div class="fading-view">
  <h2>Fading Memories</h2>
  <p class="subtitle">Knowledge weakening from disuse. Re-engage to strengthen these connections.</p>

  {#if loading}
    <p class="loading">Analyzing memory decay...</p>
  {:else if fading.length === 0}
    <div class="all-good">
      <p>All memories are well-maintained. Nothing is significantly fading.</p>
    </div>
  {:else}
    <p class="count">{fading.length} memories are fading</p>

    {#each fading as mem}
      {@const ratio = mem.effectiveWeight / mem.impulse.weight}
      <div class="fading-card" style="border-left-color: {borderColor(ratio)}">
        <div class="fading-header">
          <span class="fading-type">{mem.impulse.impulse_type}</span>
          <span class="fading-time">{formatTime(mem.hoursSinceAccess)}</span>
        </div>
        <div class="fading-content">{mem.impulse.content}</div>
        <div class="fading-meta">
          <div class="decay-bar-container">
            <div class="decay-label">
              <span>Strength</span>
              <span class="decay-status" style="color: {barColor(ratio)}">{fadingLevel(ratio)}</span>
            </div>
            <div class="decay-bar">
              <div
                class="decay-fill"
                style="width: {ratio * 100}%; background: {barColor(ratio)}"
              ></div>
            </div>
            <div class="decay-numbers">
              <span>{mem.effectiveWeight.toFixed(3)}</span>
              <span class="decay-original">/ {mem.impulse.weight.toFixed(2)}</span>
            </div>
          </div>
          {#if mem.connectionCount > 0}
            <span class="connections-badge">{mem.connectionCount} connections</span>
          {:else}
            <span class="connections-badge orphan">orphan</span>
          {/if}
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .fading-view {
    padding: 40px;
    max-width: 650px;
    overflow-y: auto;
    height: 100%;
  }

  h2 {
    font-family: var(--font-body);
    font-size: 20px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 13px;
    margin-bottom: 24px;
  }

  .loading {
    color: var(--text-muted);
    font-style: italic;
  }

  .all-good {
    text-align: center;
    padding: 40px 20px;
    color: var(--accent-sage);
  }

  .count {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 16px;
  }

  .fading-card {
    background: var(--bg-panel-solid);
    border: 1px solid var(--border-subtle);
    border-left: 3px solid var(--accent-warm);
    border-radius: var(--radius-sm);
    padding: 14px 16px;
    margin-bottom: 8px;
  }

  .fading-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 6px;
  }

  .fading-type {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .fading-time {
    font-size: 11px;
    color: var(--text-muted);
  }

  .fading-content {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.5;
    margin-bottom: 12px;
  }

  .fading-meta {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }

  .decay-bar-container {
    flex: 1;
  }

  .decay-label {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .decay-status {
    font-weight: 500;
  }

  .decay-bar {
    height: 3px;
    background: var(--border-subtle);
    border-radius: 2px;
    overflow: hidden;
  }

  .decay-fill {
    height: 100%;
    border-radius: 2px;
    transition: width var(--transition-medium);
  }

  .decay-numbers {
    display: flex;
    gap: 4px;
    font-size: 10px;
    color: var(--text-muted);
    margin-top: 3px;
    font-variant-numeric: tabular-nums;
  }

  .decay-original {
    opacity: 0.5;
  }

  .connections-badge {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .connections-badge.orphan {
    color: var(--accent-rose);
  }
</style>
