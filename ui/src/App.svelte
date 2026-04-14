<script lang="ts">
  import Galaxy from "./lib/Galaxy.svelte";
  import { currentView } from "./lib/stores";
</script>

<div class="app-layout">
  <div class="galaxy-container">
    {#if $currentView === "galaxy"}
      <Galaxy />
    {:else}
      <p style="color: var(--text-muted); text-align: center; margin-top: 40vh;">
        {$currentView} view — coming soon
      </p>
    {/if}
  </div>
</div>

<style>
  .app-layout {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .galaxy-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }
</style>
