---
name: heyvm-deploy
description: Deploy apps to Heyo cloud sandboxes — archive code, deploy to production, bind ports, set up custom domains, and manage deployed sandboxes. Use when the user wants to deploy, update, or manage a running app.
argument-hint: "[action] [args...]"
allowed-tools: Bash, Read, Grep
---

# Deploy — App Deployment with Heyo Sandboxes

You are helping the user deploy applications to Heyo's cloud sandbox infrastructure. **Always prefer the `heyvm` CLI over direct API calls.** The CLI handles authentication, error handling, and retries more robustly than raw `curl` commands. Only fall back to the API when a specific operation has no CLI equivalent.

## Authentication

The `heyvm` CLI handles authentication automatically using the token stored at `~/.heyo/token.json`. If the token file does not exist or is expired, tell the user to run `heyvm login` first.

For the rare cases where you must use the API directly (see "API-Only Operations" below), extract the token with:

```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
```

## Deployment Workflow

### Step 1: Deploy in one step (preferred)

The `heyvm deploy` command handles archiving, creating the sandbox, binding ports, and waiting for readiness — all in one step:

```bash
heyvm deploy ./my-project \
  --name my-app \
  --start-command "cd /workspace && npm start" \
  --port 8080
```

This is the preferred method for most deployments. It archives the directory, creates a cloud sandbox, binds the specified ports, and waits for the sandbox to be ready.

**`heyvm deploy` options:**

| Option | Description | Default |
|--------|-------------|---------|
| `[PATH]` | Directory to deploy | `.` (current dir) |
| `--name <NAME>` | Sandbox name | Auto-generated |
| `--start-command <CMD>` | Shell command to run on startup | — |
| `--port <PORT>` | Port to expose publicly (repeatable) | — |
| `--image <IMAGE>` | Base image (built-in like `ubuntu:24.04`, or a public image name — see "Public Images") | `ubuntu:24.04` |
| `--region <REGION>` | `US` or `EU` | `US` |
| `--driver <DRIVER>` | `libvirt` or `firecracker` | `libvirt` |
| `--size-class <CLASS>` | `micro`, `mini`, `small`, `medium`, `large` | `small` |
| `--working-directory <PATH>` | Working directory inside sandbox | `/workspace` |
| `--mount-path <PATH>` | Mount path prefix in the archive | `/workspace` |
| `--env KEY=VALUE` | Environment variable (repeatable) | — |
| `--setup-hook <CMD>` | Setup hook command (repeatable) | — |
| `--ttl-seconds <N>` | Time-to-live in seconds | Plan default |
| `--health-path <PATH>` | Health check path (e.g. `/health`); waits for 2xx | — |
| `--health-timeout <DUR>` | Health check timeout | `120s` |
| `--private` | Make bound ports private (account members only) | — |
| `--format <FMT>` | Output format: `json` or `text` | `text` |

### Step 2 (alternative): Archive + deploy separately

If you need more control (e.g., reusing an archive across multiple deployments), you can archive and deploy in separate steps.

#### Archive the code

**Archive a local directory:**
```bash
heyvm archive-dir ./my-project --name my-app-v1
```

**Archive from an existing sandbox mount:**
```bash
heyvm archive <sandbox-id> --name my-app-v1
```

Both commands output the archive ID (e.g. `ar-ddcd890d`). Save this for the deploy step.

**`archive-dir` options:**

| Option | Description | Default |
|--------|-------------|---------|
| `[PATH]` | Directory to archive | `.` (current dir) |
| `--name <NAME>` | Archive name | — |
| `--mount-path <PATH>` | Mount path prefix in the archive | `/workspace` |
| `--no-ignore` | Include build assets (`node_modules`, `target`, `dist`, etc.) | — |
| `--format <FMT>` | Output format: `json` or `text` | `text` |

#### Deploy with the archive (API-only — no CLI equivalent for archive-based deploy)

```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
curl -s -X POST https://server.heyo.computer/sandbox-deploy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "<app-name>",
    "archive_id": "<archive-id>",
    "region": "US",
    "driver": "libvirt",
    "image": "ubuntu:24.04",
    "start_command": "<startup command>",
    "open_ports": [<port>],
    "working_directory": "/workspace"
  }'
```

Then bind ports with the CLI (see "Bind a port" below).

## Public Images

Public images are pre-built sandbox images shared across the platform. Use them as the `--image` for `heyvm deploy` to start from an environment with tools, runtimes, or data already baked in (instead of installing them via `--setup-hook` on every deploy).

### Discover public images

```bash
heyvm images list                    # All public images
heyvm images list --libvirt          # Only libvirt-compatible
heyvm images list --firecracker      # Only firecracker-compatible
heyvm images list --local            # Local Docker images instead
```

The list shows each image's ID (`im-...`), name, and supported drivers. Match the `--driver` you plan to deploy with.

### Add a public image to your account

Before deploying with a public image, register it locally with a name:

```bash
heyvm images add <IMAGE_ID>                   # Use the image's registered name
heyvm images add <IMAGE_ID> --name my-base    # Give it a custom local name
```

### Deploy with a public image

Pass the image name (the one shown by `images list`, or your custom `--name` from `images add`) as `--image`:

```bash
heyvm deploy ./my-app \
  --name my-app \
  --image my-base \
  --start-command "cd /workspace && npm start" \
  --port 3000
```

Make sure `--driver` is compatible with the image (check `heyvm images list --libvirt` / `--firecracker`).

### Publish your own public image

Snapshot a configured sandbox and submit it for review:

```bash
heyvm images publish <sandbox-id> --name my-image --description "Node 22 + Postgres"
heyvm images publish <sandbox-id> --name my-image --sysprep   # libvirt: strip machine-id, ssh keys, logs
```

Published images go through review before becoming available to others.

## Managing Deployed Sandboxes

All management commands below work for both local and deployed sandboxes. The CLI resolves deployed sandboxes via the cloud API automatically.

### Bind a port (expose publicly)

```bash
heyvm bind <id-or-name> <port>
heyvm bind <id-or-name> <port> --private    # Account members only
heyvm bind <id-or-name> <port> --format json # JSON output
```

The app becomes accessible at `https://<subdomain>.heyo.computer/`.

### List sandboxes

```bash
heyvm list                    # Running sandboxes (local + deployed)
heyvm list-inactive           # Stopped sandboxes
```

### Execute commands

```bash
heyvm exec <id-or-name> -- <command>
heyvm sh <id-or-name>                       # Interactive shell
```

### Mount workspace locally

```bash
heyvm mount <id-or-name>                    # Mount and wait (Ctrl+C to unmount)
heyvm mount <id-or-name> -- code .          # Mount and open in editor
heyvm mount <id-or-name> --mount-path /app  # Mount a specific path
```

### Update a deployment

```bash
heyvm archive-dir ./my-project --name my-app-v2
heyvm update <id-or-name> --archive <new-archive-id>
```

### Resize

```bash
heyvm resize <id-or-name> --size-class <CLASS>
```

Size classes: `micro` (0.25 CPU, 0.5 GB), `mini` (0.5 CPU, 1 GB), `small` (1 CPU, 2 GB), `medium` (2 CPU, 4 GB), `large` (4 CPU, 8 GB).

### Edit TTL

```bash
heyvm edit-ttl <id-or-name> --ttl-seconds <N>   # 0 for unlimited (if plan allows)
```

### Wait for readiness

```bash
heyvm wait-for <id-or-name> <port>                           # Wait for port to be ready
heyvm wait-for <id-or-name> <port> --path /health            # Wait for HTTP 2xx on /health
heyvm wait-for <id-or-name> <port> --timeout 60s             # Custom timeout
heyvm wait-for --url https://slug.heyo.computer/health       # Poll external URL directly
```

### Port forwarding

```bash
heyvm port-forward <id-or-name> <sandbox-port>               # Forward same port locally
heyvm port-forward <id-or-name> <sandbox-port> -p <host-port> # Forward to different local port
```

### Snapshot (create reusable image)

```bash
heyvm snapshot <id-or-name> --name my-snapshot
heyvm snapshot <id-or-name> --name my-snapshot --no-restart   # Don't restart after snapshot
```

### SSH access

```bash
heyvm share <id-or-name> --name my-app
heyvm ssh my-app              # From another machine
```

### Manage archives

```bash
heyvm list-archives           # List all archives
heyvm delete-archive <id>     # Delete an archive
```

## API-Only Operations

Use the API directly only when no CLI command exists:

### Deploy with a pre-existing archive ID

When deploying from a previously created archive (not from a local directory), use the API:

```bash
TOKEN=$(python3 -c "import json; print(json.load(open('$HOME/.heyo/token.json'))['access_token'])")
curl -s -X POST https://server.heyo.computer/sandbox-deploy \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "<app-name>",
    "archive_id": "<archive-id>",
    "region": "US",
    "driver": "libvirt",
    "image": "ubuntu:24.04",
    "start_command": "<startup command>",
    "open_ports": [<port>],
    "working_directory": "/workspace"
  }'
```

**Deploy request fields:**

| Field | Description | Default |
|-------|-------------|---------|
| `name` | Sandbox name (required) | — |
| `archive_id` | Archive ID (required) | — |
| `region` | `US` or `EU` (**must be uppercase**) | — |
| `driver` | `libvirt` or `firecracker` | — |
| `image` | `ubuntu:24.04` or `alpine:3.23` | — |
| `start_command` | Shell command to run on startup | — |
| `open_ports` | Array of port numbers to expose | `[]` |
| `ttl_seconds` | Time-to-live in seconds | Plan default |
| `disk_size_gb` | Disk size (max 250 GB) | — |
| `working_directory` | Working directory inside sandbox | `/workspace` |
| `env_vars` | Environment variables map | `{}` |
| `setup_hooks` | Shell commands to run after creation | `[]` |
| `size_class` | `micro`, `mini`, `small`, `medium`, or `large` | `small` |

### Custom domains

Custom domains are configured via the cloud API (`/custom-domains` endpoint). The domain must have a CNAME record pointing to `heyo.computer`. SSL certificates are provisioned automatically.

## Typical Deployment Examples

### Deploy a static site (one command)
```bash
heyvm deploy ./public \
  --name my-site \
  --start-command "cd /workspace && python3 -m http.server 8080" \
  --port 8080
```

### Deploy a Node.js app
```bash
heyvm deploy ./my-node-app \
  --name my-app \
  --start-command "cd /workspace && npm install && npm start" \
  --port 3000 \
  --size-class medium
```

### Deploy with health check
```bash
heyvm deploy ./my-api \
  --name my-api \
  --start-command "cd /workspace && npm start" \
  --port 8080 \
  --health-path /health \
  --health-timeout 60s
```

### Deploy with environment variables
```bash
heyvm deploy ./my-app \
  --name my-app \
  --start-command "cd /workspace && node server.js" \
  --port 3000 \
  --env NODE_ENV=production \
  --env DATABASE_URL=postgres://...
```

### Update with new code
```bash
heyvm archive-dir ./my-app --name my-app-v2
heyvm update my-app --archive <v2-archive-id>
```

### Deploy from a local sandbox
```bash
# Develop in a local sandbox
heyvm create --name staging --type node --mount ./app:/workspace
heyvm exec staging -- npm install
heyvm exec staging -- npm run build

# Archive the sandbox mounts and deploy via API (archive-based deploy)
heyvm archive staging --name production-v1
# Then deploy via curl with the archive ID (see API-Only Operations)
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HEYO_ARCHIVE_TOKEN` | JWT token for archive authentication |
| `HEYO_CLOUD_URL` | Cloud server URL (default: `https://server.heyo.computer`) |
| `API_HOSTNAME` | Required for `heyvm bind` on **local** sandboxes only |

## Workflow Guidance

When the user asks to deploy:

1. **Deploy**: Use `heyvm deploy` for the simplest path — it handles archiving, creating, binding, and waiting in one step.
2. **Verify**: Check the output URL or use `heyvm wait-for` with `--path` for health checks.
3. **Scale**: Use `heyvm resize` to adjust compute resources if needed.
4. **Update**: Use `heyvm archive-dir` + `heyvm update` for subsequent code pushes.

When the user provides `$ARGUMENTS`, interpret them as deployment-related actions. For example, `/deploy ./my-app` should deploy the directory using `heyvm deploy`.

If `$ARGUMENTS` is empty, ask the user what they want to deploy or manage.
