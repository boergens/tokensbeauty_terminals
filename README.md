# Terminals

Bubblewrap-sandboxed tmux terminal microservice. Manages a pool of pre-warmed sandboxed shell instances and exposes an HTTP API for interacting with them.

## Running

```bash
cargo run
```

The server starts on `0.0.0.0:3000` by default.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | Bind port |
| `POOL_TARGET_SIZE` | `2` | Number of warm instances to maintain |
| `POOL_MAX_SIZE` | `10` | Maximum total instances |
| `WORKSPACE_BASE` | `/tmp/terminals-workspaces` | Host directory for instance workspaces |

## API

### Health Check

```
GET /health
```

```json
{"status": "ok"}
```

### Pool Status

```
GET /pool/status
```

```json
{"warm": 2, "total": 3, "target_size": 2, "max_size": 10}
```

### Create Instance

Claims a warm instance from the pool, or creates a new one if none are available.

```
POST /instances
```

```json
{
  "id": "a1b2c3d4-...",
  "status": "claimed",
  "workspace": "/tmp/terminals-workspaces/a1b2c3d4-...",
  "uptime_secs": 12
}
```

### List Instances

```
GET /instances
```

Returns an array of instance objects.

### Get Instance

```
GET /instances/:id
```

### Destroy Instance

```
DELETE /instances/:id
```

```json
{"status": "destroyed"}
```

### Send Input

Send text (with Enter appended) or raw keys to the instance's terminal.

```
POST /instances/:id/input
Content-Type: application/json
```

| Field | Type | Description |
|-------|------|-------------|
| `text` | string | Text to type, followed by Enter |
| `keys` | string[] | Raw tmux keys (no Enter appended) |

At least one of `text` or `keys` must be provided. Both can be sent in a single request.

```json
{"text": "echo hello"}
```

```json
{"status": "sent"}
```

### Capture Screen

Returns the current visible content of the terminal pane.

```
GET /instances/:id/screen
```

```json
{"content": "sandbox@host:~$ echo hello\nhello\nsandbox@host:~$ "}
```

### Execute Command

Sends a command and captures the screen after a delay.

```
POST /instances/:id/exec
Content-Type: application/json
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `command` | string | *required* | Command to run |
| `delay_ms` | integer | `500` | Milliseconds to wait before capturing |

```json
{"command": "ls -la", "delay_ms": 1000}
```

```json
{"content": "...screen output after command..."}
```

## Errors

All errors return JSON:

```json
{"error": "instance a1b2c3d4-... not found"}
```

| Status | Meaning |
|--------|---------|
| 400 | Bad request (missing fields, instance limit reached) |
| 404 | Instance not found |
| 500 | Sandbox or internal error |
