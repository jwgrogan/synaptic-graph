<script lang="ts">
  import { selectedNodeId, selectedDetail } from "./stores";
  import { getImpulseDetail } from "./api";
  import type { ImpulseDetail } from "./types";

  let detail: ImpulseDetail | null = null;
  let loading = false;
  let visible = false;

  $: if ($selectedNodeId) {
    loadDetail($selectedNodeId);
  } else {
    detail = null;
    visible = false;
  }

  async function loadDetail(id: string) {
    loading = true;
    try {
      detail = await getImpulseDetail(id);
      selectedDetail.set(detail);
      // Trigger slide-in after data loads
      requestAnimationFrame(() => { visible = true; });
    } catch (e) {
      console.error("Failed to load detail:", e);
      detail = null;
    }
    loading = false;
  }

  function close() {
    visible = false;
    setTimeout(() => {
      selectedNodeId.set(null);
      selectedDetail.set(null);
    }, 200);
  }

  function navigateTo(id: string) {
    selectedNodeId.set(id);
  }

  function engagementLabel(level: string): { text: string; color: string; bg: string } {
    switch (level) {
      case "high": return { text: "High", color: "var(--accent-warm)", bg: "var(--accent-warm-light)" };
      case "medium": return { text: "Medium", color: "var(--accent-primary)", bg: "var(--accent-primary-light)" };
      case "low": return { text: "Low", color: "var(--text-muted)", bg: "var(--bg-hover)" };
      default: return { text: level, color: "var(--text-muted)", bg: "var(--bg-hover)" };
    }
  }

  function valenceLabel(valence: string): { text: string; color: string; bg: string } {
    switch (valence) {
      case "positive": return { text: "Positive", color: "var(--accent-sage)", bg: "var(--accent-sage-light)" };
      case "negative": return { text: "Negative", color: "var(--accent-rose)", bg: "var(--accent-rose-light)" };
      case "neutral": return { text: "Neutral", color: "var(--text-muted)", bg: "var(--bg-hover)" };
      default: return { text: valence, color: "var(--text-muted)", bg: "var(--bg-hover)" };
    }
  }

  function relationshipColor(rel: string): string {
    switch (rel) {
      case "strengthens": return "var(--accent-sage)";
      case "contradicts": return "var(--accent-rose)";
      case "extends": return "var(--accent-primary)";
      case "contextualizes": return "var(--accent-warm)";
      default: return "var(--border-medium)";
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
  <div class="detail-panel" class:visible>
    <button class="close-btn" on:click={close}>&times;</button>

    <h2 class="content-title">{detail.impulse.content}</h2>

    <div class="weight-section">
      <div class="weight-header">
        <span class="label">Weight</span>
        <span class="weight-value">{detail.impulse.weight.toFixed(2)}</span>
      </div>
      <div class="weight-bar">
        <div class="weight-fill" style="width: {detail.impulse.weight * 100}%"></div>
      </div>
    </div>

    <div class="pills-row">
      <span class="pill" style="color: {engagementLabel(detail.impulse.engagement_level).color}; background: {engagementLabel(detail.impulse.engagement_level).bg}">{engagementLabel(detail.impulse.engagement_level).text}</span>
      <span class="pill" style="color: {valenceLabel(detail.impulse.emotional_valence).color}; background: {valenceLabel(detail.impulse.emotional_valence).bg}">{valenceLabel(detail.impulse.emotional_valence).text}</span>
    </div>

    <div class="meta-grid">
      <div class="meta-cell">
        <span class="label">Type</span>
        <span class="meta-value">{detail.impulse.impulse_type}</span>
      </div>
      <div class="meta-cell">
        <span class="label">Source</span>
        <span class="meta-value">{detail.impulse.source_type}</span>
      </div>
      <div class="meta-cell">
        <span class="label">Last accessed</span>
        <span class="meta-value muted">{relativeTime(detail.impulse.last_accessed_at)}</span>
      </div>
    </div>

    {#if detail.connections.length > 0}
      <div class="connections-section">
        <span class="label">Connections ({detail.connections.length})</span>
        <div class="connections-list">
          {#each detail.connections as conn}
            <button
              class="connection-item"
              on:click={() => navigateTo(conn.other_id)}
            >
              <div class="conn-border" style="background: {relationshipColor(conn.relationship)}"></div>
              <div class="conn-body">
                <div class="conn-content">{conn.other_content.slice(0, 80)}{conn.other_content.length > 80 ? '...' : ''}</div>
                <div class="conn-meta">
                  <span class="conn-rel">{conn.relationship}</span>
                  <span class="conn-weight">{conn.weight.toFixed(2)}</span>
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
  .detail-panel {
    position: absolute;
    top: 0;
    right: 0;
    width: 340px;
    height: 100%;
    background: var(--bg-panel);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    box-shadow: var(--shadow-panel);
    padding: 24px 20px;
    overflow-y: auto;
    z-index: 10;
    transform: translateX(100%);
    transition: transform var(--transition-medium);
  }

  .detail-panel.visible {
    transform: translateX(0);
  }

  .close-btn {
    position: absolute;
    top: 16px;
    right: 16px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 18px;
    cursor: pointer;
    line-height: 1;
    padding: 4px;
    transition: color var(--transition-fast);
  }

  .close-btn:hover {
    color: var(--text-primary);
  }

  .content-title {
    font-family: var(--font-display);
    font-size: 16px;
    font-weight: 400;
    color: var(--text-primary);
    margin-bottom: 20px;
    line-height: 1.5;
    padding-right: 24px;
  }

  .weight-section {
    margin-bottom: 16px;
  }

  .weight-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
  }

  .weight-value {
    font-size: 12px;
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .weight-bar {
    width: 100%;
    height: 3px;
    background: var(--border-subtle);
    border-radius: 2px;
    overflow: hidden;
  }

  .weight-fill {
    height: 100%;
    background: var(--accent-primary);
    border-radius: 2px;
    transition: width var(--transition-medium);
  }

  .pills-row {
    display: flex;
    gap: 6px;
    margin-bottom: 16px;
  }

  .pill {
    font-size: 11px;
    font-weight: 500;
    padding: 3px 10px;
    border-radius: 20px;
    letter-spacing: 0.2px;
  }

  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .meta-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-bottom: 20px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .meta-cell {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .meta-value {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .meta-value.muted {
    color: var(--text-muted);
  }

  .connections-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .connections-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .connection-item {
    display: flex;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    color: inherit;
    font: inherit;
    padding: 0;
    overflow: hidden;
    transition: background var(--transition-fast);
  }

  .connection-item:hover {
    background: var(--bg-hover);
  }

  .conn-border {
    width: 3px;
    flex-shrink: 0;
    border-radius: 2px;
  }

  .conn-body {
    padding: 8px 10px;
    flex: 1;
    min-width: 0;
  }

  .conn-content {
    font-size: 12px;
    color: var(--text-primary);
    margin-bottom: 4px;
    line-height: 1.4;
  }

  .conn-meta {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    color: var(--text-muted);
  }

  .conn-rel {
    text-transform: lowercase;
  }

  .conn-weight {
    font-variant-numeric: tabular-nums;
  }
</style>
