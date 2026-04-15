<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import DetailPanel from "./lib/DetailPanel.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import StatsView from "./lib/StatsView.svelte";
  import GhostList from "./lib/GhostList.svelte";
  import ImportView from "./lib/ImportView.svelte";
  import FadingView from "./lib/FadingView.svelte";
  import SearchPalette from "./lib/SearchPalette.svelte";
  import FilterBar from "./lib/FilterBar.svelte";
  import TagManager from "./lib/TagManager.svelte";
  import Toasts from "./lib/Toasts.svelte";
  import { currentView, selectedNodeId, searchOpen } from "./lib/stores";

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      selectedNodeId.set(null);
      searchOpen.set(false);
    }
    if (e.key === "/" && !isInputFocused()) {
      e.preventDefault();
      searchOpen.set(true);
    }
  }

  function isInputFocused(): boolean {
    const el = document.activeElement;
    return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement;
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="app-layout">
  <Sidebar />

  <div class="main-content">
    {#if $currentView === "galaxy"}
      <FilterBar />
      <Galaxy />
      <DetailPanel />
    {:else if $currentView === "stats"}
      <StatsView />
    {:else if $currentView === "ghosts"}
      <GhostList />
    {:else if $currentView === "fading"}
      <FadingView />
    {:else if $currentView === "tags"}
      <TagManager />
    {:else if $currentView === "import"}
      <ImportView />
    {/if}
  </div>
</div>

<SearchPalette />
<Toasts />

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .main-content {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
</style>
