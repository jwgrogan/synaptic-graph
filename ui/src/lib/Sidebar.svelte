<script lang="ts">
  import { currentView } from "./stores";

  const views = [
    { id: "galaxy" as const, label: "Graph", icon: "\u2B21" },
    { id: "fading" as const, label: "Fading", icon: "\u25D0" },
    { id: "tags" as const, label: "Tags", icon: "\u25C9" },
    { id: "ghosts" as const, label: "External", icon: "\u25C7" },
    { id: "stats" as const, label: "Stats", icon: "\u25A6" },
    { id: "import" as const, label: "Import", icon: "\u2193" },
  ];
</script>

<nav class="sidebar">
  <div class="logo" title="Synaptic Graph">S</div>
  {#each views as view}
    <button
      class="nav-item"
      class:active={$currentView === view.id}
      on:click={() => currentView.set(view.id)}
      title={view.label}
    >
      {view.icon}
    </button>
  {/each}
</nav>

<style>
  .sidebar {
    width: 48px;
    background: var(--bg-surface);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 16px 0;
    gap: 2px;
    flex-shrink: 0;
  }

  .logo {
    font-family: var(--font-body);
    font-size: 20px;
    font-weight: 600;
    color: var(--accent-primary);
    margin-bottom: 20px;
    opacity: 0.9;
    cursor: default;
  }

  .nav-item {
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    transition: all var(--transition-fast);
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  .nav-item.active {
    background: var(--accent-primary-light);
    color: var(--accent-primary);
  }
</style>
