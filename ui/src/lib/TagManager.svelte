<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { NEBULA_COLORS } from "./types";

  interface Tag {
    name: string;
    color: string;
  }

  let tags: Tag[] = [];
  let newTagName = "";
  let selectedColor = NEBULA_COLORS[0];
  let confirmDelete: string | null = null;
  let loading = false;
  let error = "";

  onMount(loadTags);

  async function loadTags() {
    loading = true;
    error = "";
    try {
      tags = await invoke<Tag[]>("get_all_tags");
    } catch (e) {
      error = `Failed to load tags: ${e}`;
    }
    loading = false;
  }

  async function createTag() {
    const name = newTagName.trim();
    if (!name) return;
    error = "";
    try {
      await invoke("ui_create_tag", { name, color: selectedColor });
      newTagName = "";
      selectedColor = NEBULA_COLORS[0];
      await loadTags();
    } catch (e) {
      error = `Failed to create tag: ${e}`;
    }
  }

  async function deleteTag(name: string) {
    error = "";
    try {
      await invoke("ui_delete_tag", { name });
      confirmDelete = null;
      await loadTags();
    } catch (e) {
      error = `Failed to delete tag: ${e}`;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      createTag();
    }
  }
</script>

<div class="tag-manager">
  <h2 class="view-title">Tags</h2>

  {#if error}
    <div class="error-msg">{error}</div>
  {/if}

  <div class="create-section">
    <div class="create-row">
      <input
        class="tag-input"
        type="text"
        placeholder="New tag name..."
        bind:value={newTagName}
        on:keydown={handleKeydown}
      />
      <button class="create-btn" on:click={createTag} disabled={!newTagName.trim()}>
        Create
      </button>
    </div>
    <div class="color-palette">
      {#each NEBULA_COLORS as color}
        <button
          class="color-swatch"
          class:selected={selectedColor === color}
          style="background: {color}"
          on:click={() => selectedColor = color}
          title={color}
        ></button>
      {/each}
    </div>
  </div>

  {#if loading}
    <div class="empty-state">Loading tags...</div>
  {:else if tags.length === 0}
    <div class="empty-state">No tags yet. Create one above.</div>
  {:else}
    <div class="tag-list">
      {#each tags as tag}
        <div class="tag-row">
          <span class="tag-dot" style="background: {tag.color}"></span>
          <span class="tag-name">{tag.name}</span>
          <div class="tag-actions">
            {#if confirmDelete === tag.name}
              <button class="confirm-delete" on:click={() => deleteTag(tag.name)}>Delete?</button>
              <button class="cancel-delete" on:click={() => confirmDelete = null}>Cancel</button>
            {:else}
              <button class="delete-btn" on:click={() => confirmDelete = tag.name}>&times;</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .tag-manager {
    padding: 32px;
    max-width: 480px;
    height: 100%;
    overflow-y: auto;
  }

  .view-title {
    font-family: var(--font-body);
    font-size: 18px;
    font-weight: 400;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .error-msg {
    font-size: 12px;
    color: var(--accent-rose, #C4727F);
    margin-bottom: 12px;
    padding: 8px 12px;
    background: var(--accent-rose-light, rgba(196, 114, 127, 0.1));
    border-radius: var(--radius-sm);
  }

  .create-section {
    margin-bottom: 24px;
    padding-bottom: 20px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .create-row {
    display: flex;
    gap: 8px;
    margin-bottom: 10px;
  }

  .tag-input {
    flex: 1;
    background: var(--bg-panel-solid);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    font-size: 13px;
    color: var(--text-primary);
    font-family: var(--font-body);
    outline: none;
    transition: border-color var(--transition-fast);
  }

  .tag-input:focus {
    border-color: var(--accent-primary);
  }

  .tag-input::placeholder {
    color: var(--text-muted);
  }

  .create-btn {
    background: var(--accent-primary);
    color: #ffffff;
    border: none;
    border-radius: var(--radius-sm);
    padding: 8px 16px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: opacity var(--transition-fast);
  }

  .create-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .create-btn:not(:disabled):hover {
    opacity: 0.85;
  }

  .color-palette {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .color-swatch {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all var(--transition-fast);
    padding: 0;
  }

  .color-swatch:hover {
    transform: scale(1.15);
  }

  .color-swatch.selected {
    border-color: var(--text-primary);
    box-shadow: 0 0 0 2px var(--bg-deep, #06060f);
  }

  .empty-state {
    font-size: 13px;
    color: var(--text-muted);
    text-align: center;
    padding: 32px 0;
  }

  .tag-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .tag-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    transition: background var(--transition-fast);
  }

  .tag-row:hover {
    background: var(--bg-hover);
  }

  .tag-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tag-name {
    flex: 1;
    font-size: 13px;
    color: var(--text-primary);
  }

  .tag-actions {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .delete-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    opacity: 0;
    transition: all var(--transition-fast);
    line-height: 1;
  }

  .tag-row:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    background: var(--accent-rose-light, rgba(196, 114, 127, 0.15));
    color: var(--accent-rose, #C4727F);
  }

  .confirm-delete {
    background: var(--accent-rose, #C4727F);
    color: #ffffff;
    border: none;
    border-radius: var(--radius-sm);
    padding: 3px 10px;
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
  }

  .cancel-delete {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    padding: 3px 10px;
    font-size: 11px;
    cursor: pointer;
  }

  .cancel-delete:hover {
    border-color: var(--text-muted);
    color: var(--text-secondary);
  }
</style>
