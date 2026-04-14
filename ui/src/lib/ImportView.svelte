<script lang="ts">
  import { searchMemories } from "./api";

  let importText = "";
  let parsing = false;
  let imported: { content: string; type: string; status: string }[] = [];
  let error = "";

  const exportPrompt = `Export all of my stored memories and any context you've learned about me from past conversations. Preserve my words verbatim where possible, especially for instructions and preferences.

## Categories (output in this order):

1. **Instructions**: Rules I've explicitly asked you to follow going forward — tone, format, style, "always do X", "never do Y", and corrections to your behavior. Only include rules from stored memories, not from conversations.

2. **Identity**: Name, age, location, education, family, relationships, languages, and personal interests.

3. **Career**: Current and past roles, companies, and general skill areas.

4. **Projects**: Projects I meaningfully built or committed to. Ideally ONE entry per project. Include what it does, current status, and any key decisions. Use the project name or a short descriptor as the first words of the entry.

5. **Preferences**: Opinions, tastes, and working-style preferences that apply broadly.

## Format:

Use section headers for each category. Within each category, list one entry per line, sorted by oldest date first. Format each line as:

[YYYY-MM-DD] - Entry content here.

If no date is known, use [unknown] instead.

## Output:
- Wrap the entire export in a single code block for easy copying.
- After the code block, state whether this is the complete set or if more remain.`;

  let copied = false;

  function copyPrompt() {
    navigator.clipboard.writeText(exportPrompt);
    copied = true;
    setTimeout(() => { copied = false; }, 2000);
  }

  async function parseAndImport() {
    if (!importText.trim()) {
      error = "Paste your exported memories first";
      return;
    }

    parsing = true;
    error = "";
    imported = [];

    try {
      // Parse the export format: lines starting with [date] -
      const lines = importText.split("\n");
      let currentCategory = "observation";
      const entries: { content: string; type: string }[] = [];

      const categoryMap: Record<string, string> = {
        "instructions": "preference",
        "identity": "observation",
        "career": "observation",
        "projects": "pattern",
        "preferences": "preference",
      };

      for (const line of lines) {
        const trimmed = line.trim();

        // Detect category headers
        const headerMatch = trimmed.match(/^#+\s*\d*\.?\s*(Instructions|Identity|Career|Projects|Preferences)/i);
        if (headerMatch) {
          const cat = headerMatch[1].toLowerCase();
          currentCategory = categoryMap[cat] || "observation";
          continue;
        }

        // Detect entry lines: [date] - content
        const entryMatch = trimmed.match(/^\[[\w-]+\]\s*-\s*(.+)$/);
        if (entryMatch) {
          entries.push({
            content: entryMatch[1].trim(),
            type: currentCategory,
          });
        }
      }

      if (entries.length === 0) {
        error = "No entries found. Make sure the format uses [YYYY-MM-DD] - Entry lines.";
        parsing = false;
        return;
      }

      // Import each entry via the Tauri API
      const { invoke } = await import("@tauri-apps/api/core");

      for (const entry of entries) {
        try {
          await invoke("quick_save", {
            params: {
              content: entry.content,
              impulse_type: entry.type,
              emotional_valence: "neutral",
              engagement_level: "medium",
              source_ref: "import",
            },
          });
          imported.push({ ...entry, status: "saved" });
        } catch (err) {
          imported.push({ ...entry, status: `failed: ${err}` });
        }
      }
    } catch (err) {
      error = `Parse error: ${err}`;
    }

    parsing = false;
  }
</script>

<div class="import-view">
  <h2>Import Memories</h2>
  <p class="subtitle">Bring your memories from other AI providers into synaptic-graph.</p>

  <div class="step">
    <div class="step-number">1</div>
    <div class="step-content">
      <h3>Copy the export prompt</h3>
      <p>Send this prompt to ChatGPT, Claude, Gemini, or any AI that has your memories:</p>
      <div class="prompt-box">
        <pre>{exportPrompt.slice(0, 200)}...</pre>
        <button class="copy-btn" on:click={copyPrompt}>
          {copied ? "Copied!" : "Copy Full Prompt"}
        </button>
      </div>
    </div>
  </div>

  <div class="step">
    <div class="step-number">2</div>
    <div class="step-content">
      <h3>Paste the response</h3>
      <p>Paste the AI's exported memories below:</p>
      <textarea
        class="import-textarea"
        placeholder="Paste the exported memories here..."
        bind:value={importText}
        rows="12"
      ></textarea>
    </div>
  </div>

  <div class="step">
    <div class="step-number">3</div>
    <div class="step-content">
      <h3>Import</h3>
      <button
        class="import-btn"
        on:click={parseAndImport}
        disabled={parsing || !importText.trim()}
      >
        {parsing ? "Importing..." : "Parse & Import Memories"}
      </button>
    </div>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if imported.length > 0}
    <div class="results">
      <h3>Imported {imported.filter(i => i.status === "saved").length} / {imported.length} memories</h3>
      {#each imported as entry}
        <div class="result-item" class:failed={entry.status !== "saved"}>
          <span class="result-type">{entry.type}</span>
          <span class="result-content">{entry.content.slice(0, 80)}{entry.content.length > 80 ? "..." : ""}</span>
          <span class="result-status">{entry.status}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .import-view {
    padding: 32px;
    max-width: 700px;
    overflow-y: auto;
    height: 100%;
  }

  h2 {
    font-size: 18px;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .subtitle {
    color: var(--text-muted);
    font-size: 13px;
    margin-bottom: 24px;
  }

  .step {
    display: flex;
    gap: 16px;
    margin-bottom: 24px;
  }

  .step-number {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--accent-indigo);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    flex-shrink: 0;
  }

  .step-content {
    flex: 1;
  }

  .step-content h3 {
    font-size: 14px;
    color: var(--text-primary);
    margin-bottom: 6px;
  }

  .step-content p {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .prompt-box {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 12px;
    position: relative;
  }

  .prompt-box pre {
    font-size: 11px;
    color: var(--text-muted);
    white-space: pre-wrap;
    margin-bottom: 8px;
    font-family: monospace;
  }

  .copy-btn {
    background: var(--accent-indigo);
    color: white;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 12px;
    cursor: pointer;
    font-weight: 600;
  }

  .copy-btn:hover {
    opacity: 0.9;
  }

  .import-textarea {
    width: 100%;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 12px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: monospace;
    resize: vertical;
  }

  .import-textarea::placeholder {
    color: var(--text-muted);
  }

  .import-btn {
    background: var(--accent-indigo);
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .import-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error {
    color: #f87171;
    font-size: 13px;
    margin-top: 12px;
    padding: 8px 12px;
    background: rgba(248, 113, 113, 0.1);
    border-radius: 6px;
  }

  .results {
    margin-top: 20px;
  }

  .results h3 {
    font-size: 14px;
    color: var(--accent-indigo);
    margin-bottom: 12px;
  }

  .result-item {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid rgba(99, 102, 241, 0.08);
    font-size: 12px;
  }

  .result-item.failed {
    opacity: 0.5;
  }

  .result-type {
    color: var(--accent-violet);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    width: 70px;
    flex-shrink: 0;
  }

  .result-content {
    color: var(--text-secondary);
    flex: 1;
  }

  .result-status {
    color: var(--accent-teal);
    font-size: 11px;
  }

  .result-item.failed .result-status {
    color: #f87171;
  }
</style>
