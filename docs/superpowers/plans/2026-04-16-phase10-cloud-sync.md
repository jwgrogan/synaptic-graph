# Phase 10: Cloud Sync via Turso — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add optional cloud sync via Turso (libSQL) so memories sync across devices. Fully open source — users can self-host Turso or use a hosted option. Local-first workflow is unchanged when sync is not configured.

**Architecture:** Feature flag `cloud-sync` swaps rusqlite for libsql's embedded replica mode. When a Turso URL is configured, reads are local (fast), writes propagate to cloud (eventual consistency). When not configured, pure local SQLite — zero difference from current behavior.

**Tech Stack:** libsql (Turso's Rust client), existing synaptic-graph crate

**Depends on:** All current phases complete.

---

## Design Decisions

- **Feature flag, not separate repo.** Cloud sync code lives in the main repo behind `--features cloud-sync`. Default build is pure local.
- **Turso, not custom sync.** Turso/libSQL is SQLite-compatible, open source, has embedded replicas with local-first reads. No custom sync protocol needed.
- **Self-host or pay.** Users can run their own Turso instance (open source) or use hosted Turso. Future: offer a managed option at ~$5/mo for convenience.
- **No account system.** Sync is configured via URL + auth token. No user accounts, no sign-up flow. Keep it simple.

---

### Task 1: Abstract Database Backend

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/db.rs`

Add libsql as optional dependency:
```toml
[features]
default = []
cloud-sync = ["libsql"]

[dependencies]
libsql = { version = "0.6", optional = true }
```

Create a thin abstraction in `src/db.rs` that works with either backend:
- When `cloud-sync` is not enabled: use rusqlite as-is (current behavior, zero changes)
- When `cloud-sync` is enabled: use libsql's `Database::open_with_remote_sync()` for embedded replica mode

The key insight: libsql's Rust API is nearly identical to rusqlite. Most queries work unchanged. The abstraction is thin — mainly the `Database::open()` path changes.

**Tests:** All 148 existing tests must pass with both `cargo test` and `cargo test --features cloud-sync`.

---

### Task 2: Sync Configuration

**Files:**
- Create: `src/sync_config.rs`
- Modify: `src/main.rs`
- Modify: `ui/src-tauri/src/commands.rs`

Configuration via environment variables:
- `SYNAPTIC_SYNC_URL` — Turso database URL (e.g., `libsql://your-db-name.turso.io`)
- `SYNAPTIC_SYNC_TOKEN` — Auth token

When both are set and `cloud-sync` feature is enabled, use embedded replica mode. When not set, use local SQLite.

Add CLI commands:
```bash
synaptic-graph sync-status     # show if sync is configured, last sync time
synaptic-graph sync-enable     # prompt for URL + token, save to config file
synaptic-graph sync-disable    # remove sync config, revert to local-only
```

Store sync config in `~/.local/share/synaptic-graph/sync.toml`:
```toml
[sync]
url = "libsql://your-db.turso.io"
token = "your-auth-token"
enabled = true
```

---

### Task 3: MCP Sync Tools

**Files:**
- Modify: `src/server.rs`

Add MCP tools:
- `sync_enable(url, token)` — configure and enable sync
- `sync_disable` — disable sync, revert to local
- `sync_status` — return sync state, URL (masked), last sync time

These let the AI configure sync on behalf of the user:
"Set up cloud sync for me" → calls sync_enable with the user's Turso credentials.

---

### Task 4: UI Sync Settings

**Files:**
- Create: `ui/src/lib/SyncSettings.svelte`
- Modify: `ui/src/lib/Sidebar.svelte`
- Modify: `ui/src/lib/App.svelte`
- Modify: `ui/src-tauri/src/commands.rs`

Add a "Sync" section to settings:
- Status indicator: "Local only" or "Synced to cloud"
- Enable: input fields for Turso URL + token, "Connect" button
- Disable: "Disconnect" button
- Device list: which devices have synced (from Turso metadata)
- Last synced: timestamp

Two-path UX:
1. **Self-host:** "Enter your Turso URL and token"
2. **Managed:** "Sign up at [link] for cloud sync ($5/mo)" → gives them a URL + token

---

### Task 5: Install Script Update

**Files:**
- Modify: `install.sh`

Add option 6: "Configure cloud sync"
- Prompts: "Do you want to enable cross-device sync?"
- If yes: "Enter Turso URL:" and "Enter auth token:"
- Saves to sync.toml
- If no: skips (local-only, current behavior)

Also add build flag detection:
```bash
# Build with sync support
cargo build --release --features cloud-sync
```

---

### Task 6: Documentation

**Files:**
- Modify: `README.md`
- Create: `docs/cloud-sync.md`

Add to README:
```markdown
### Cloud Sync (optional)

Sync your memories across devices using Turso (SQLite with cloud sync):

```bash
# Build with sync support
cargo build --release --features cloud-sync

# Configure
synaptic-graph sync-enable
# Enter your Turso URL and auth token

# Or self-host Turso:
# https://docs.turso.tech/self-hosting
```
```

Create docs/cloud-sync.md with:
- How it works (embedded replicas, local-first reads)
- Self-hosting guide
- Managed option
- Privacy: your data goes to your Turso instance, not ours
- Troubleshooting

---

## Summary

| Task | What | Effort |
|------|------|--------|
| 1 | Abstract DB backend with feature flag | Medium |
| 2 | Sync configuration (env vars, CLI, config file) | Small |
| 3 | MCP tools for sync management | Small |
| 4 | UI sync settings | Medium |
| 5 | Install script update | Small |
| 6 | Documentation | Small |

Total: ~1 day of focused work. The hardest part is Task 1 (ensuring libsql is a true drop-in for rusqlite across all queries). The rest is plumbing.
