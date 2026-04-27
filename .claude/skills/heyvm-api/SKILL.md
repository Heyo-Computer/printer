---
name: heyvm-api
description: Use the Heyo Cloud API to manage deployed sandboxes — execute commands, read/write files, start/stop sandboxes, and check status. Use when an agent needs to interact with a running deployed sandbox programmatically.
argument-hint: "[action] [sandbox-id] [args...]"
allowed-tools: Bash, Read, Grep
---

# Heyo Cloud API — Deployed Sandbox Management

You are helping an agent interact with deployed sandboxes on the Heyo cloud platform via its REST API. These endpoints let you execute commands, transfer files, and control sandbox lifecycle without SSH or the `heyvm` CLI.

## Base URL

```
https://server.heyo.computer
```

Override with `HEYO_CLOUD_URL` env var if set.

## Authentication

All API calls require a JWT Bearer token. Extract it from the local token file:

```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
```

If `~/.heyo/token.json` does not exist, the user must log in first (`heyvm login` or the TUI).

Pass the token in every request:
```
Authorization: Bearer $TOKEN
```

## Sandbox Runtime Endpoints

These endpoints operate on deployed sandboxes by ID (e.g., `sb-a1b2c3d4`).

### Execute a command

```bash
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/exec \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": "ls -la /workspace", "args": []}'
```

**Request:**
| Field | Type | Description |
|-------|------|-------------|
| `command` | string | Shell command to execute (required) |
| `args` | string[] | Additional arguments (default: `[]`) |

**Response:**
```json
{
  "stdout": "total 16\ndrwxr-xr-x 3 root root 4096 ...",
  "stderr": "",
  "output": "total 16\ndrwxr-xr-x 3 root root 4096 ...",
  "exit_code": 0
}
```

Timeout: 120 seconds. For long-running commands, consider backgrounding them (`nohup ... &`).

### Write a file

```bash
CONTENT=$(echo -n "file contents here" | base64)
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/write-file \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"mount_path\": \"/workspace\", \"file_path\": \"hello.txt\", \"content\": \"$CONTENT\"}"
```

**Request:**
| Field | Type | Description |
|-------|------|-------------|
| `mount_path` | string | Mount path inside the sandbox (typically `/workspace`) |
| `file_path` | string | Path relative to mount (e.g., `src/main.py`) |
| `content` | string | File content, **base64-encoded** |

**Response:** `201 {"status": "created"}`

### Read a file

```bash
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/read-file \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"mount_path": "/workspace", "file_path": "hello.txt"}'
```

**Request:**
| Field | Type | Description |
|-------|------|-------------|
| `mount_path` | string | Mount path inside the sandbox |
| `file_path` | string | Path relative to mount |

**Response:**
```json
{
  "content": "ZmlsZSBjb250ZW50cyBoZXJl",
  "file_path": "hello.txt",
  "content_type": "application/octet-stream"
}
```

The `content` field is **base64-encoded**. Decode it:
```bash
echo "$CONTENT" | base64 -d
```

### Get sandbox status

```bash
curl -s https://server.heyo.computer/sandbox/$SANDBOX_ID/status \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```json
{
  "id": "sb-a1b2c3d4",
  "name": "my-app",
  "status": "Running",
  "sandbox_type": "Shell",
  "image": "ubuntu:24.04",
  "backend_type": "Libvirt",
  "uptime": {"secs": 3600, "nanos": 0},
  "cpu_usage": 12.5,
  "memory_usage": 134217728
}
```

Status values: `Running`, `Stopped`, `Paused`, `Unknown`.

### Start a sandbox

```bash
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/start \
  -H "Authorization: Bearer $TOKEN"
```

**Response:** SandboxInfo JSON (same schema as status). Timeout: 120 seconds (sandbox boot may take time).

### Stop a sandbox

```bash
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/stop \
  -H "Authorization: Bearer $TOKEN"
```

**Response:** `{"status": "stopped"}`

## Deployment Lifecycle Endpoints

These endpoints manage the deployment lifecycle (create, delete, update, list).

### Deploy a sandbox

First archive code, then deploy:

```bash
# Step 1: Archive
heyvm archive-dir ./my-project --name my-app-v1
# Outputs archive ID like ar-ddcd890d

# Step 2: Deploy
curl -s -X POST https://server.heyo.computer/sandbox-deploy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-app",
    "archive_id": "ar-ddcd890d",
    "region": "US",
    "driver": "libvirt",
    "image": "ubuntu:24.04",
    "start_command": "cd /workspace && npm start",
    "open_ports": [3000],
    "working_directory": "/workspace"
  }'
```

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `name` | string | Sandbox name (required) | — |
| `archive_id` | string | Archive ID (required) | — |
| `region` | string | `US` or `EU` (uppercase) | — |
| `driver` | string | `libvirt` or `firecracker` | — |
| `image` | string | OS image | `ubuntu:24.04` |
| `start_command` | string | Startup command | — |
| `open_ports` | int[] | Ports to expose | `[]` |
| `ttl_seconds` | int | Time-to-live | Plan default |
| `size_class` | string | `micro`/`mini`/`small`/`medium`/`large` | `small` |
| `working_directory` | string | Working directory | `/workspace` |
| `env_vars` | object | Environment variables | `{}` |
| `setup_hooks` | string[] | Post-creation commands | `[]` |

### List deployed sandboxes

```bash
curl -s https://server.heyo.computer/deployed-sandboxes \
  -H "Authorization: Bearer $TOKEN"
```

### Delete a deployed sandbox

```bash
curl -s -X DELETE https://server.heyo.computer/deployed-sandboxes/$SANDBOX_ID \
  -H "Authorization: Bearer $TOKEN"
```

### Restart a deployed sandbox

```bash
curl -s -X POST https://server.heyo.computer/deployed-sandboxes/$SANDBOX_ID/restart \
  -H "Authorization: Bearer $TOKEN"
```

### Update a deployment (replace code)

```bash
heyvm archive-dir ./my-project --name v2
curl -s -X POST https://server.heyo.computer/deployed-sandboxes/$SANDBOX_ID/replace-mount \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"s3_archive_key": "ar-NEWID", "sandbox_path": "/workspace"}'
```

### Expose a port (create proxy endpoint)

```bash
curl -s -X POST https://server.heyo.computer/proxy-endpoints/for-deployed \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"sandbox_id": "sb-a1b2c3d4", "port": 3000}'
```

**Response:** `{"subdomain": "abc123", ...}` — app available at `https://abc123.heyo.computer/`

## Common Patterns

### Run a command and get output
```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
RESULT=$(curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/exec \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": "cat /etc/os-release"}')
echo "$RESULT" | python3 -c "import json,sys; print(json.load(sys.stdin)['stdout'])"
```

### Write a script and execute it
```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
SCRIPT=$(cat <<'PYEOF' | base64
#!/usr/bin/env python3
print("Hello from the sandbox!")
PYEOF
)
# Write
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/write-file \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"mount_path\": \"/workspace\", \"file_path\": \"run.py\", \"content\": \"$SCRIPT\"}"
# Execute
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/exec \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": "python3 /workspace/run.py"}'
```

### Check if a service is running
```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/exec \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": "curl -s -o /dev/null -w \"%{http_code}\" http://localhost:3000/health"}'
```

### Install packages
```bash
curl -s -X POST https://server.heyo.computer/sandbox/$SANDBOX_ID/exec \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"command": "apt-get update && apt-get install -y nodejs npm"}'
```

## Size Classes

| Class | CPU | Memory | Use case |
|-------|-----|--------|----------|
| `micro` | 0.25 | 0.5 GB | Static sites, tiny scripts |
| `mini` | 0.5 | 1 GB | Small apps, APIs |
| `small` | 1 | 2 GB | Standard workloads |
| `medium` | 2 | 4 GB | Build tasks, databases |
| `large` | 4 | 8 GB | Heavy compute, ML |

## Error Handling

| Status | Meaning | Action |
|--------|---------|--------|
| 401 | Token expired or missing | Re-read `~/.heyo/token.json` or ask user to log in |
| 402 | Payment required | User needs an active subscription |
| 403 | Not sandbox owner | Check sandbox ID |
| 404 | Sandbox not found | Verify ID with `GET /deployed-sandboxes` |
| 502 | Backend unavailable | Backend server may be down; retry after a moment |

## Workflow Guidance

When the user provides `$ARGUMENTS`:

- If it looks like a sandbox ID + action (e.g., `sb-abc123 exec ls`), execute the corresponding API call.
- If it's `list`, call `GET /deployed-sandboxes`.
- If it's `status <id>`, call `GET /sandbox/{id}/status`.
- If no arguments, ask the user what they want to do.

Always extract the token from `~/.heyo/token.json` before making API calls. If the file doesn't exist, tell the user to run `heyvm login`.
