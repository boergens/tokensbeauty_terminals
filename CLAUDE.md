# Terminal Server - Architecture & Knowledge

## Overview

This is a Rust/axum microservice that manages bubblewrap-sandboxed tmux sessions running Claude Code. Each instance is fully isolated via bwrap and accessible through a ttyd web terminal.

## Key Architecture: tmux on host, bwrap inside tmux

- tmux runs on the host with per-instance sockets (`tmux -L inst-<uuid>`)
- tmux launches `bwrap ... /bin/bash` as its shell command
- All tmux commands (send-keys, capture-pane) work normally from the host
- `--die-with-parent` ensures killing tmux kills the bwrap sandbox
- ttyd attaches read-only to the tmux session for browser access

## Source Layout

```
src/
  main.rs        -- axum server, startup cleanup, graceful shutdown
  config.rs      -- env-based configuration
  instance.rs    -- Instance struct, InstanceStatus enum, InstanceInfo
  error.rs       -- AppError/SandboxError with IntoResponse, 500s are logged
  sandbox.rs     -- bwrap command builder + tmux command wrappers
  manager.rs     -- InstanceManager: create/destroy/send/capture/pool/ttyd/cleanup
  pool.rs        -- background pool replenishment task (every 5s)
  handlers.rs    -- HTTP handlers + request/response types
  routes.rs      -- axum Router construction
```

## Bwrap Sandbox Details

The sandbox mounts:
- `/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/etc` as read-only
- `/home/kevin/.local` read-only (contains the `claude` binary)
- `/dev`, `/proc` as device/proc filesystems
- `/tmp`, `/run` as tmpfs
- The workspace directory as `/home/sandbox` (read-write)

Environment inside sandbox:
- `HOME=/home/sandbox`
- `PATH` includes `/home/kevin/.local/bin` for the claude binary
- `CLAUDECODE` is unset (prevents "nested session" detection)
- PID namespace is unshared, network is shared (claude needs API access)

## Instance Boot Sequence

1. Create workspace directory
2. Copy `~/.claude/` directory (credentials, settings)
3. Copy `~/.claude.json` with injected trust for `/home/sandbox`
4. Copy `CLAUDE.md.template` as `CLAUDE.md`
5. Start tmux session with bwrap command
6. Wait 500ms for bash to start
7. Send `claude --dangerously-skip-permissions`
8. Wait 1000ms for the safety prompt
9. Send Down arrow (200ms pause) then Enter to confirm
10. Start ttyd on a random available port

## Important Gotchas

- **Port allocation**: Uses OS-assigned ephemeral ports (bind to port 0, read assigned port, close, use for ttyd). There's a tiny race window but works in practice.
- **tmux sockets**: Each instance has its own socket at `/tmp/tmux-<uid>/inst-<uuid>`. Plain `tmux ls` won't show them — must use `tmux -L inst-<uuid>`.
- **Stale cleanup**: On startup, the server kills all `inst-*` tmux servers, ttyd processes, and workspace dirs. This handles unclean shutdowns.
- **`.claude.json` vs `~/.claude/`**: Claude Code uses TWO config locations. `~/.claude.json` has onboarding state, per-project trust, tips history. `~/.claude/` has credentials (`.credentials.json`), settings, session data.
- **Workspace trust**: Claude Code prompts "Do you trust this folder?" per-project. We inject `{"hasTrustDialogAccepted": true}` for `/home/sandbox` in the copied `.claude.json`.
- **`--dangerously-skip-permissions`**: Skips all tool permission prompts (bash, file edits, web search). Requires interactive confirmation: Down arrow to select "Yes", then Enter.
- **State management**: `Arc<Mutex<HashMap<Uuid, Instance>>>` — lock held only for fast in-memory ops, all subprocess work (tmux, bwrap) happens on `spawn_blocking` threads outside the lock.
- **500 errors**: All 500 responses are logged server-side with request context (instance ID, input text, etc.) via handler-level `map_err` logging.

## Dependencies

- `axum 0.8` — HTTP framework
- `tokio` — async runtime
- `serde`/`serde_json` — serialization
- `uuid` — instance IDs
- `tracing`/`tracing-subscriber` — structured logging

External tools (must be on PATH):
- `tmux` — terminal multiplexer
- `bwrap` (bubblewrap) — unprivileged sandboxing
- `ttyd` — web-based terminal
- `claude` — Claude Code CLI (at `/home/kevin/.local/bin/claude`)
