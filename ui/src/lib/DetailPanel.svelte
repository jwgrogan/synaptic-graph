<script lang="ts">
  import { selectedNodeId, selectedDetail } from "./stores";
  import { getImpulseDetail } from "./api";
  import type { ImpulseDetail } from "./types";

  let detail: ImpulseDetail | null = null;
  let loading = false;

  $: if ($selectedNodeId) {
    loadDetail($selectedNodeId);
  } else {
    detail = null;
  }

  async function loadDetail(id: string) {
    loading = true;
    try {
      detail = await getImpulseDetail(id);
      selectedDetail.set(detail);
    } catch (e) {
      console.error("Failed to load detail:", e);
      detail = null;
    }
    loading = false;
  }

  function close() {
    selectedNodeId.set(null);
    selectedDetail.set(null);
  }

  function navigateTo(id: string) {
    selectedNodeId.set(id);
  }

  function engagementColor(level: string): string {
    switch (level) {
      case "high": return "#fbbf24";
      case "medium": return "#94a3b8";
      case "low": return "#475569";
      default: return "#64748b";
    }
  }

  function valenceColor(valence: string): string {
    switch (valence) {
      case "positive": return "#4ade80";
      case "negative": return "#f87171";
      case "neutral": return "#94a3b8";
      default: return "#64748b";
    }
  }

  function relativeTime(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const hours = Math.floor(diffMs / 3600000);
    if (hours < 1) return "just now";
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 30) return `${days}d ago`;
    return `${Math.floor(days / 30)}mo ago`;
  }
</script>

{#if $selectedNodeId && detail}
  <div class="detail-panel">
    <button class="close-btn" on:click={close}>x</button>

    <h2 class="content">{detail.impulse.content}</h2>

    <div class="meta-row">
      <div class="meta-item">
        <span class="label">Weight</span>
        <div class="weight-bar">
          <div class="weight-fill" style="width: {detail.impulse.weight * 100}%"></div>
        </div>
        <span class="value">{detail.impulse.weight.toFixed(2)}</span>
      </div>
    </div>

    <div class="meta-row">
      <div class="meta-item">
        <span class="label">Engagement</span>
        <span class="dot" style="background: {engagementColor(detail.impulse.engagement_level)}"></span>
        <span class="value">{detail.impulse.engagement_level}</span>
      </div>
      <div class="meta-item">
        <span class="label">Valence</span>
        <span class="dot" style="background: {valenceColor(detail.impulse.emotional_valence)}"></span>
        <span class="value">{detail.impulse.emotional_valence}</span>
      </div>
    </div>

    <div class="meta-row">
      <div class="meta-item">
        <span class="label">Type</span>
        <span class="value">{detail.impulse.impulse_type}</span>
      </div>
      <div class="meta-item">
        <span class="label">Source</span>
        <span class="value">{detail.impulse.source_type}</span>
      </div>
    </div>

    <div class="meta-row">
      <span class="label">Last accessed</span>
      <span class="value muted">{relativeTime(detail.impulse.last_accessed_at)}</span>
    </div>

    {#if detail.connections.length > 0}
      <div class="connections-section">
        <span class="label">Connections ({detail.connections.length})</span>
        {#each detail.connections as conn}
          <button
            class="connection-card"
            style="opacity: {Math.max(0.4, conn.weight)}"
            on:click={() => navigateTo(conn.other_id)}
          >
            <div class="conn-content">{conn.other_content.slice(0, 80)}{conn.other_content.length > 80 ? '...' : ''}</div>
            <div class="conn-meta">{conn.relationship} · weight {conn.weight.toFixed(2)}</div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .detail-panel {
    position: absolute;
    top: 0;
    right: 0;
    width: 320px;
    height: 100%;
    background: var(--bg-panel);
    border-left: 1px solid var(--border-subtle);
    padding: 20px;
    overflow-y: auto;
    z-index: 10;
  }

  .close-btn {
    position: absolute;
    top: 12px;
    right: 12px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
  }

  .content {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 16px;
    line-height: 1.5;
  }

  .meta-row {
    display: flex;
    gap: 16px;
    margin-bottom: 12px;
  }

  .meta-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .value {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .value.muted {
    color: var(--text-muted);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .weight-bar {
    width: 80px;
    height: 5px;
    background: rgba(99, 102, 241, 0.15);
    border-radius: 3px;
    overflow: hidden;
  }

  .weight-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent-indigo), var(--accent-violet));
    border-radius: 3px;
  }

  .connections-section {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .connection-card {
    background: rgba(99, 102, 241, 0.08);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 10px;
    cursor: pointer;
    text-align: left;
    color: inherit;
    font: inherit;
  }

  .connection-card:hover {
    background: rgba(99, 102, 241, 0.15);
  }

  .conn-content {
    font-size: 12px;
    color: var(--accent-indigo);
    margin-bottom: 4px;
  }

  .conn-meta {
    font-size: 10px;
    color: var(--text-muted);
  }
</style>
