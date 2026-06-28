
<!-- VICTAURI:BEGIN (added by `victauri init` — delete this block to opt out) -->
## Victauri (App Inspection & Testing)

This app has **Victauri** integrated — an MCP server embedded inside the Tauri process
that gives full-stack access to the webview DOM, IPC layer, Rust backend, and native
windows. Available when the app is running in debug mode.

**Use Victauri MCP tools for all app inspection and testing tasks.** Victauri runs inside
the app process with sub-ms response times and direct AppHandle access — it sees
everything, not just the webview.

Key Victauri tools (the read-only backend/DB introspection below has no CDP/Playwright
equivalent — and on macOS/Linux CDP can't attach to a Tauri webview at all):
- `invoke_command` — call any registered Tauri command directly (also records timing and
  honours fault injection; an eval-capable tool can also reach `__TAURI_INTERNALS__.invoke`)
- `verify_state` — cross-boundary frontend/backend state verification
- `detect_ghost_commands` — find frontend calls absent from the introspection registry (read the `reliability` field: only a real "no backend handler" bug when the registry mirrors the app's full command set; with no/partial `#[inspectable]` it lists real, uninstrumented commands)
- `check_ipc_integrity` — verify IPC pipeline health
- `introspect` — command timings, IPC contract testing, coverage, startup timing, capabilities
- `fault` — inject IPC faults (delay, error, drop, corrupt) for chaos engineering
- `explain` — natural-language narration of what happened in the app
- `get_memory_stats` — real OS process memory stats
- `audit_accessibility` — WCAG accessibility checks
- `get_performance` — navigation timing, JS heap, resource loading

### Connecting reliably (read this before reaching for CDP)

`.mcp.json` connects through `victauri bridge` — a stdio proxy that **discovers the running
app's port at connect time and re-discovers on restart**, so you are never pinned to a stale
or wrong port. Do **not** replace it with a fixed `"url": "http://127.0.0.1:7373/mcp"`: a
hardcoded port can point at a *different* Victauri app (e.g. a leftover demo) and every call
then fails with `422`/`404`. The bridge avoids that by design.

- **First contact:** call `get_plugin_info` once and check `app.identifier` — confirm you
  reached the intended app, not another Victauri instance.
- **Multiple apps running?** Pin the bridge with `--app <bundle-identifier>` in `.mcp.json`
  (`"args": ["bridge", "--wait", "--app", "com.your.app"]`), or set `VICTAURI_APP`. `init`
  bakes this in automatically when it can read your identifier.
- **If a tool call fails after an app rebuild/restart:** the bridge re-establishes the
  session automatically — just retry. If the MCP path is genuinely wedged, the **sessionless
  REST API is the fallback, NOT CDP**: `POST http://127.0.0.1:<port>/api/tools/<tool>` with
  the Bearer token from `<temp>/victauri/<pid>/token` (same capabilities, no session).

### Awaiting async backend work (don't guess with sleeps)

Many Tauri commands are **fire-and-forget**: they spawn background work and return
immediately (often `null`) while the real work runs. Don't poll by hand or sprinkle fixed
`sleep`s — use `wait_for` to await true completion:

- **Pollable status (no app changes):** `wait_for` with `condition: "expression"` evaluates a
  JS expression every poll until it is truthy (or equals `expected`). It may `await`, so you
  can await a status command directly:
  `wait_for { condition: "expression", value: "(await window.__TAURI_INTERNALS__.invoke('get_status')).running === false" }`.
  Level-triggered and race-free.
- **Completion event:** `wait_for` with `condition: "event"` blocks until a named Tauri event
  fires, with a `since_ms` look-back so an event emitted in the gap after your `invoke_command`
  is still caught: `wait_for { condition: "event", value: "analysis-complete" }`. (Custom events
  must be registered via `VictauriBuilder::listen_events(&["…"])`.)

The robust pattern is `invoke_command(...)` then `wait_for(expression|event, ...)` — never a bare sleep.

### Reading app-specific backend state

If the app registers state probes (`VictauriBuilder::probe("name", || json!({...}))`), call
`app_state` to read domain state directly from the Rust process — no IPC round-trip, no log
grepping. `app_state` with no args lists probe names; `app_state { probe: "name" }` returns its
snapshot. Use this for pipeline/queue/cache internals (version, depth, stats) instead of
reverse-engineering them from `query_db` + logs.

### Driving specific code paths & mutating test state

- To exercise a specific backend code path, call the relevant command via `invoke_command`
  (with args). If the path you need isn't reachable from any command, that's an app gap — add a
  small debug command and drive it.
- `query_db` is intentionally **read-only**. To mutate state for a test, go through the app's
  own commands with `invoke_command` (which respects app invariants) rather than writing the DB.

Prefer Victauri over Playwright or CDP for any Tauri-app task it handles; fall back to
Playwright only for browser-only work unrelated to this app.
<!-- VICTAURI:END -->
