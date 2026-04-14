<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import DetailPanel from "./lib/DetailPanel.svelte";
  import Sidebar from "./lib/Sidebar.svelte";
  import StatsView from "./lib/StatsView.svelte";
  import GhostList from "./lib/GhostList.svelte";
  import SearchPalette from "./lib/SearchPalette.svelte";
  import { currentView } from "./lib/stores";
</script>

<div class="app-layout">
  <Sidebar />

  <div class="main-content">
    {#if $currentView === "galaxy"}
      <Galaxy />
      <DetailPanel />
    {:else if $currentView === "stats"}
      <StatsView />
    {:else if $currentView === "ghosts"}
      <GhostList />
    {/if}
  </div>
</div>

<SearchPalette />

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
