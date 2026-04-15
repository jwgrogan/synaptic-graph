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

  let exporting = false;
  let exportResult = "";

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
    const folderName = path.split("/").pop() || "vault";
    registerName = folderName;

    registering = true;
    registerResult = "";

    try {
      const result = await invoke<{ name: string; nodes_scanned: number }>(
        "register_external_graph",
        { name: folderName, rootPath: path, sourceType: "obsidian" }
      );
      registerResult = `Connected "${result.name}" \u2014 ${result.nodes_scanned} notes mapped`;
      await loadSources();
    } catch (err) {
      registerResult = `Error: ${err}`;
    }

    registering = false;
  }

  async function pickAndExport() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Choose Export Folder",
    });

    if (!selected) return;

    const path = selected as string;
    exporting = true;
    exportResult = "";

    try {
      const result = await invoke<{ files_written: number; output_dir: string }>(
        "export_to_obsidian",
        { outputDir: path }
      );
      exportResult = `Exported ${result.files_written} files to ${result.output_dir}`;
    } catch (err) {
      exportResult = `Error: ${err}`;
    }

    exporting = false;
  }

  function sourceAccent(type: string): string {
    switch (type) {
      case "obsidian": return "var(--accent-primary)";
      case "repo": return "var(--accent-sage)";
      default: return "var(--accent-warm)";
    }
  }
</script>

<div class="ghost-list">
  <h2>External Graphs</h2>
  <p class="subtitle">Connect your knowledge bases -- Obsidian vaults, repos, or any markdown directory.</p>

  <button class="connect-btn" on:click={pickAndRegister} disabled={registering}>
    {registering ? "Scanning..." : "Connect Knowledge Base"}
  </button>

  {#if registerResult}
    <div class="register-result" class:error={registerResult.startsWith("Error")}>
      {registerResult}
    </div>
  {/if}

  {#if sources.length === 0}
    <p class="empty">No external graphs connected yet.</p>
  {:else}
    {#each sources as source}
      <div class="ghost-card" style="border-left-color: {sourceAccent(source.source_type)}">
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

<div class="export-section">
  <h2>Export to Obsidian</h2>
  <p class="subtitle">Export your memory graph as linked markdown files that work as an Obsidian vault.</p>

  <button class="connect-btn export-btn" on:click={pickAndExport} disabled={exporting}>
    {exporting ? "Exporting..." : "Choose Export Folder"}
  </button>

  {#if exportResult}
    <div class="register-result" class:error={exportResult.startsWith("Error")}>
      {exportResult}
    </div>
  {/if}
</div>

<style>
  .ghost-list {
    padding: 40px;
    max-width: 600px;
  }

  h2 {
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 400;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 13px;
    margin-bottom: 20px;
  }

  .connect-btn {
    background: transparent;
    color: var(--accent-primary);
    border: 1px solid var(--accent-primary);
    padding: 8px 20px;
    border-radius: var(--radius-sm);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    margin-bottom: 16px;
    transition: all var(--transition-fast);
    font-family: var(--font-body);
  }

  .connect-btn:hover {
    background: var(--accent-primary-light);
  }

  .connect-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .register-result {
    font-size: 13px;
    color: var(--accent-sage);
    margin-bottom: 16px;
    padding: 8px 12px;
    background: var(--accent-sage-light);
    border-radius: var(--radius-sm);
  }

  .register-result.error {
    color: var(--accent-rose);
    background: var(--accent-rose-light);
  }

  .empty {
    color: var(--text-muted);
    font-size: 13px;
  }

  .ghost-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-left: 3px solid var(--accent-primary);
    border-radius: var(--radius-sm);
    padding: 16px;
    margin-bottom: 10px;
  }

  .ghost-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
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

  .export-section {
    padding: 40px;
    max-width: 600px;
    border-top: 1px solid var(--border-subtle);
    margin-top: 8px;
  }

  .export-btn {
    border-color: var(--accent-sage);
    color: var(--accent-sage);
  }

  .export-btn:hover {
    background: var(--accent-sage-light);
  }
</style>
