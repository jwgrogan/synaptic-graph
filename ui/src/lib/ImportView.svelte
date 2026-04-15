<script lang="ts">
  import { searchMemories } from "./api";

  let importText = "";
  let parsing = false;
  let imported: { content: string; type: string; status: string }[] = [];
  let error = "";

  const exportPrompt = `Export all of my stored memories and any context you've learned about me from past conversations. Preserve my words verbatim where possible, especially for instructions and preferences.

## Categories (output in this order):

1. **Instructions**: Rules I've explicitly asked you to follow going forward \u2014 tone, format, style, "always do X", "never do Y", and corrections to your behavior. Only include rules from stored memories, not from conversations.

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

        const headerMatch = trimmed.match(/^#+\s*\d*\.?\s*(Instructions|Identity|Career|Projects|Preferences)/i);
        if (headerMatch) {
          const cat = headerMatch[1].toLowerCase();
          currentCategory = categoryMap[cat] || "observation";
          continue;
        }

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
        <button class="outline-btn" on:click={copyPrompt}>
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
        class="outline-btn primary"
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
    padding: 40px;
    max-width: 700px;
    overflow-y: auto;
    height: 100%;
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
    margin-bottom: 28px;
  }

  .step {
    display: flex;
    gap: 16px;
    margin-bottom: 28px;
  }

  .step-number {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1.5px solid var(--border-medium);
    background: transparent;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    font-weight: 500;
    flex-shrink: 0;
  }

  .step-content {
    flex: 1;
  }

  .step-content h3 {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 6px;
  }

  .step-content p {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 10px;
  }

  .prompt-box {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 14px;
  }

  .prompt-box pre {
    font-size: 11px;
    color: var(--text-muted);
    white-space: pre-wrap;
    margin-bottom: 10px;
    font-family: 'SF Mono', 'Fira Code', monospace;
  }

  .outline-btn {
    background: transparent;
    color: var(--accent-primary);
    border: 1px solid var(--accent-primary);
    padding: 8px 18px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    font-family: var(--font-body);
    transition: all var(--transition-fast);
  }

  .outline-btn:hover {
    background: var(--accent-primary-light);
  }

  .outline-btn.primary {
    padding: 10px 22px;
    font-size: 13px;
  }

  .outline-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .import-textarea {
    width: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    padding: 14px;
    color: var(--text-primary);
    font-size: 12px;
    font-family: 'SF Mono', 'Fira Code', monospace;
    resize: vertical;
    transition: border-color var(--transition-fast);
    outline: none;
  }

  .import-textarea:focus {
    border-color: var(--accent-primary);
  }

  .import-textarea::placeholder {
    color: var(--text-faint);
  }

  .error {
    color: var(--accent-rose);
    font-size: 13px;
    margin-top: 12px;
    padding: 8px 12px;
    background: var(--accent-rose-light);
    border-radius: var(--radius-sm);
  }

  .results {
    margin-top: 24px;
  }

  .results h3 {
    font-size: 14px;
    font-weight: 500;
    color: var(--accent-primary);
    margin-bottom: 12px;
  }

  .result-item {
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-subtle);
    font-size: 12px;
  }

  .result-item.failed {
    opacity: 0.4;
  }

  .result-type {
    color: var(--accent-primary);
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
    color: var(--accent-sage);
    font-size: 11px;
  }

  .result-item.failed .result-status {
    color: var(--accent-rose);
  }
</style>
