<script lang="ts">
  import { currentView } from "./stores";

  const views = [
    { id: "galaxy" as const, label: "Synaptic Graph", icon: "\u2726" },
    { id: "ghosts" as const, label: "External Graphs", icon: "\u25CC" },
    { id: "stats" as const, label: "Stats", icon: "\u25A6" },
    { id: "import" as const, label: "Import", icon: "\u2B73" },
  ];
</script>

<nav class="sidebar">
  <div class="logo">MG</div>
  {#each views as view}
    <button
      class="nav-item"
      class:active={$currentView === view.id}
      on:click={() => currentView.set(view.id)}
    >
      <span class="nav-icon">{view.icon}</span>
      <span class="nav-label">{view.label}</span>
    </button>
  {/each}
</nav>

<style>
  .sidebar {
    width: 56px;
    background: var(--bg-surface);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 12px;
    gap: 4px;
    flex-shrink: 0;
  }

  .logo {
    font-size: 14px;
    font-weight: 700;
    color: var(--accent-mauve-deep);
    margin-bottom: 16px;
    opacity: 0.7;
  }

  .nav-item {
    width: 40px;
    height: 40px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    font: inherit;
  }

  .nav-item:hover {
    background: rgba(201, 169, 184, 0.15);
    color: var(--text-secondary);
  }

  .nav-item.active {
    background: rgba(201, 169, 184, 0.2);
    color: var(--accent-mauve);
  }

  .nav-icon {
    font-size: 16px;
  }

  .nav-label {
    font-size: 7px;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
</style>
