<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import { getGhostSources } from "./api";
  import type { GhostSource } from "./types";

  let sources: GhostSource[] = [];
  let registering = false;
  let registerName = "";
  let registerResult = "";

  onMount(async () => {
    await loadSources();
  });

  async function loadSources() {
    sources = await getGhostSources();
  }

  async function pickAndRegister() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Knowledge Base Folder",
    });

    if (!selected) return;

    const path = selected as string;
    // Auto-generate name from folder name
    const folderName = path.split("/").pop() || "vault";
    registerName = folderName;

    registering = true;
    registerResult = "";

    try {
      const result = await invoke<{ name: string; nodes_scanned: number }>(
        "register_external_graph",
        { name: folderName, rootPath: path, sourceType: "obsidian" }
      );
      registerResult = `Connected "${result.name}" — ${result.nodes_scanned} notes mapped`;
      await loadSources();
    } catch (err) {
      registerResult = `Error: ${err}`;
    }

    registering = false;
  }
</script>

<div class="ghost-list">
  <h2>External Graphs</h2>
  <p class="subtitle">Connect your knowledge bases — Obsidian vaults, repos, or any markdown directory.</p>

  <button class="connect-btn" on:click={pickAndRegister} disabled={registering}>
    {registering ? "Scanning..." : "+ Connect Knowledge Base"}
  </button>

  {#if registerResult}
    <div class="register-result" class:error={registerResult.startsWith("Error")}>
      {registerResult}
    </div>
  {/if}

  {#if sources.length === 0}
    <p class="empty">No external graphs connected yet. Click the button above to select a folder.</p>
  {:else}
    {#each sources as source}
      <div class="ghost-card">
        <div class="ghost-name">{source.name}</div>
        <div class="ghost-meta">
          <span>{source.source_type}</span>
          <span>{source.node_count} nodes</span>
          {#if source.last_scanned_at}
            <span>scanned {new Date(source.last_scanned_at).toLocaleDateString()}</span>
          {/if}
        </div>
        <div class="ghost-path">{source.root_path}</div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .ghost-list {
    padding: 32px;
    max-width: 600px;
  }

  h2 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 13px;
    margin-bottom: 16px;
  }

  .connect-btn {
    background: var(--accent-mauve);
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    margin-bottom: 16px;
  }

  .connect-btn:hover {
    opacity: 0.9;
  }

  .connect-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .register-result {
    font-size: 13px;
    color: var(--accent-sage-deep);
    margin-bottom: 16px;
    padding: 8px 12px;
    background: rgba(168, 181, 160, 0.15);
    border-radius: 6px;
  }

  .register-result.error {
    color: #f87171;
    background: rgba(248, 113, 113, 0.1);
  }

  .empty {
    color: var(--text-muted);
    font-size: 14px;
  }

  .ghost-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
  }

  .ghost-name {
    font-size: 15px;
    font-weight: 600;
    color: var(--accent-sage-deep);
    margin-bottom: 6px;
  }

  .ghost-meta {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .ghost-path {
    font-size: 11px;
    color: var(--text-muted);
    font-family: monospace;
  }
</style>
