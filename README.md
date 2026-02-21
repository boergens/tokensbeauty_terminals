# Terminals

Bubblewrap-sandboxed tmux terminal microservice. Manages a pool of pre-warmed sandboxed shell instances running [Claude Code](https://docs.anthropic.com/en/docs/claude-code), each with a [ttyd](https://github.com/tsl0922/ttyd) web terminal for browser access.

## Quick Start

```bash
cargo run
```

Open the dashboard at [http://localhost:3000/dashboard](http://localhost:3000/dashboard).

Create an instance:

```bash
curl -X POST http://localhost:3000/instances
```

The response includes a `ttyd_port` — open `http://localhost:<ttyd_port>` in your browser to access the Claude Code terminal.

## How It Works

Each instance is a bwrap (bubblewrap) sandbox running inside a tmux session. The sandbox isolates the process with its own PID namespace while sharing the network so Claude can reach the API. A read-only ttyd web terminal is attached to each tmux session for browser access.

```
Browser ──► ttyd (read-only) ──► tmux session ──► bwrap sandbox ──► Claude Code
```

The service maintains a warm pool of pre-created instances so new terminals can be claimed instantly.

### Responding from the sandbox

Each sandbox has a `respond` binary on PATH. Claude (or any process inside the sandbox) can call it to send structured responses back to the server:

```
Sandbox                        Server                         Consumer
  respond "answer"  ──POST──►  /instances/{id}/response  ──►  broadcast
                                                                  │
                               /instances/{id}/events  ◄──────────┘
                               (SSE stream held open)    pushes SSE event
```

The consumer holds open an SSE connection to `/instances/{id}/events` and receives each response as a `data:` frame in real time.

## Prerequisites

- [Rust](https://rustup.rs/) (build)
- [tmux](https://github.com/tmux/tmux) (terminal multiplexing)
- [bubblewrap](https://github.com/containers/bubblewrap) (sandboxing)
- [ttyd](https://github.com/tsl0922/ttyd) (web terminal)
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) CLI installed at `~/.local/bin/claude`

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | Bind port |
| `POOL_TARGET_SIZE` | `2` | Warm instances to maintain |
| `POOL_MAX_SIZE` | `10` | Maximum total instances |
| `WORKSPACE_BASE` | `/tmp/terminals-workspaces` | Host directory for workspaces |
| `CLAUDE_MD_TEMPLATE` | see [config.rs](src/config.rs) | CLAUDE.md template copied into each workspace |

## API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/dashboard` | Live HTML dashboard with ttyd links |
| `GET` | `/health` | Health check |
| `GET` | `/pool/status` | Pool statistics |
| `POST` | `/instances` | Create/claim an instance |
| `GET` | `/instances` | List all instances |
| `GET` | `/instances/:id` | Get instance details |
| `DELETE` | `/instances/:id` | Destroy an instance |
| `POST` | `/instances/:id/prompt` | Send a prompt to Claude |
| `POST` | `/instances/:id/keys` | Send raw keys to terminal |
| `GET` | `/instances/:id/screen` | Capture terminal content |
| `POST` | `/instances/:id/response` | Receive a response from the sandbox |
| `GET` | `/instances/:id/events` | SSE stream of sandbox responses |

### SSE event stream

Hold open a connection to receive responses as they arrive:

```bash
curl -N http://localhost:3000/instances/<id>/events
```

Events are plain `data:` frames containing the message string. A keep-alive comment is sent every 15 seconds.

### Posting a response (from inside the sandbox)

The `respond` binary is available on PATH inside every sandbox:

```bash
respond "The answer is 42"
```

This POSTs `{"message": "The answer is 42"}` to the server, which broadcasts it to all SSE listeners for that instance. The binary reads `TERMINAL_SERVER_URL` and `INSTANCE_ID` from environment variables set automatically by the sandbox.

You can also post responses from outside via curl:

```bash
curl -X POST http://localhost:3000/instances/<id>/response \
  -H 'Content-Type: application/json' \
  -d '{"message": "hello"}'
```

## Architecture

```
terminals/
  src/
    main.rs          -- axum server, startup cleanup, graceful shutdown
    config.rs        -- env-based configuration
    instance.rs      -- Instance struct and status types
    error.rs         -- error types with JSON responses
    sandbox.rs       -- bwrap command builder + tmux wrappers
    manager.rs       -- instance lifecycle, ttyd, pool, cleanup
    pool.rs          -- background warm pool replenishment
    handlers.rs      -- HTTP handlers
    routes.rs        -- route definitions
    bin/respond.rs   -- respond CLI binary (copied into each sandbox)
```

## CLAUDE.md Template

Each sandbox gets a `CLAUDE.md` copied from the path configured in `CLAUDE_MD_TEMPLATE`. This file instructs Claude how to behave inside the sandbox — what task to perform, what format to respond in, and how to use `respond` to deliver results.

See [CLAUDE.md.example](CLAUDE.md.example) for a sample template.

## License

Private.
