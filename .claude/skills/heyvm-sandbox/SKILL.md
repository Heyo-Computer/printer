---
name: heyvm-sandbox
description: Create, start, stop, restart, list, and exec commands in heyvm sandboxes. Use when the user wants to manage sandbox lifecycle, run commands in isolated environments, or configure sandbox settings like mounts, ports, and backend types.
argument-hint: "[subcommand] [args...]"
allowed-tools: Bash, Read, Grep
---

# heyvm CLI — Sandbox Manager

You are interacting with the `heyvm` CLI, the sandbox management tool for the Heyo platform. Use it to create and manage isolated sandbox environments backed by multiple runtimes.

## Binary Location

The binary is `heyvm`. If it is not on PATH, build it from `mvm-ctrl/`:

```bash
cargo build --release -p heyvm
```

## Global Options

| Flag | Description |
|------|-------------|
| `--api` | Run only the HTTP API server (no TUI). Default port 3000 |
| `--port <PORT>` | API server port when using `--api` (default: 3000). TUI always uses 34099 |
| `--msb-host <HOST>` | Microsandbox server host (default: localhost) |
| `--dev` | Development mode: uses localhost for auth |
| `--cloud-url <URL>` | Cloud server URL |
| `--auth-url <URL>` | Auth server URL |
| `--debug` | Enable verbose logging |
| `--upgrade` | Self-upgrade to the latest version |

## Available Subcommands

### Sandbox Lifecycle

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm create` | Create a new sandbox | `heyvm create --name my-sandbox --type shell` |
| `heyvm start <id>` | Start an inactive sandbox | `heyvm start my-sandbox` |
| `heyvm stop <id>` | Stop a running sandbox | `heyvm stop my-sandbox` |
| `heyvm restart <id>` | Restart a sandbox | `heyvm restart my-sandbox` |

### Execution

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm exec <id> <cmd...>` | Run a command in a sandbox | `heyvm exec my-sandbox -- python -c "print('hello')"` |
| `heyvm sh <id>` | Open interactive shell | `heyvm sh my-sandbox` |
| `heyvm run-host <id> <cmd...>` | Run a host command in the sandbox mount dir | `heyvm run-host my-sandbox -- npm install` |

### Compound Commands

When running compound commands with `heyvm exec`, wrap them in `sh -c` with a quoted string:

```bash
# Compound commands via sh -c
heyvm exec my-sandbox -- sh -c 'echo hello && echo world'

# Pipes
heyvm exec my-sandbox -- sh -c 'cat /etc/os-release | head -5'

# Redirects
heyvm exec my-sandbox -- sh -c 'echo data > /tmp/file.txt'

# Simple commands work directly
heyvm exec my-sandbox -- python -c "print('hello')"
```

The `--` separator is required before the command to prevent flag parsing.

### Listing

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm list` | List all sandboxes (local + deployed) | `heyvm list` |
| `heyvm list-inactive` | List stopped sandboxes | `heyvm list-inactive --count 20` |
| `heyvm images-list` | List pulled docker images | `heyvm images-list` |

### Mounts & Storage

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm mount-add` | Add mount to a sandbox | `heyvm mount-add -i my-sandbox --host-path /tmp/data --sandbox-path /data` |
| `heyvm archive <id>` | Archive sandbox mounts to S3 | `heyvm archive my-sandbox --name backup` |
| `heyvm archive-dir [path]` | Archive a local directory | `heyvm archive-dir ./my-project --name v1` |

### Networking & Sharing

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm bind <id> <port>` | Proxy a **local** sandbox port publicly (requires `API_HOSTNAME` env var) | `API_HOSTNAME=heyo.computer heyvm bind my-sandbox 8080` |
| `heyvm proxy start <port>` | Expose local port over iroh P2P | `heyvm proxy start 3000` |
| `heyvm proxy list` | List saved proxy endpoints | `heyvm proxy list` |
| `heyvm proxy sync` | Sync proxy endpoints with cloud | `heyvm proxy sync` |
| `heyvm proxy add <name> <url>` | Add a saved endpoint by shortname | `heyvm proxy add my-server heyo://...` |
| `heyvm connect <ticket>` | Connect to remote proxy | `heyvm connect heyo://...` |
| `heyvm share <id>` | Share sandbox shell over P2P | `heyvm share my-sandbox --name pair-session` |
| `heyvm ssh <ticket>` | SSH into a shared sandbox | `heyvm ssh my-shared-sandbox` |

**Important:** `heyvm bind` only works for **local** sandboxes. For deployed (cloud) sandboxes, use the cloud API — see the `/deploy` skill.

### Deployed Sandbox Management

These commands work for both local and cloud-deployed sandboxes. The CLI resolves deployed sandboxes by name, slug, or ID via the cloud API.

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm mount <id>` | Mount deployed sandbox workspace locally via SSHFS | `heyvm mount my-deployed-app` |
| `heyvm update <id>` | Replace deployed sandbox mount from archive | `heyvm update my-app --archive abc123` |
| `heyvm resize <id>` | Resize deployed sandbox compute resources | `heyvm resize my-app --size-class medium` |
| `heyvm exec <id> -- <cmd>` | Run a command in a deployed sandbox | `heyvm exec my-app -- ls /workspace` |
| `heyvm sh <id>` | Interactive shell into a deployed sandbox | `heyvm sh my-app` |
| `heyvm list-archives` | List available archives | `heyvm list-archives` |
| `heyvm delete-archive <id>` | Delete an archive | `heyvm delete-archive ar-abc123` |
| `heyvm checkpoint <id>` | Save VM state for fast resume | `heyvm checkpoint my-app` |

**Note:** There is no `heyvm deploy` subcommand. Cloud deployment is done via the cloud API — see the `/deploy` skill.

### Git & Development

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm wt <branch>` | Create git worktree sandbox | `heyvm wt feat/cool-feature -b` |
| `heyvm pull <image>` | Pull a docker image | `heyvm pull ubuntu:24.04` |

### Skills & Setup

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm install-skills` | Download and install Claude Code skills | `heyvm install-skills` |

### Maintenance

| Command | Description | Example |
|---------|-------------|---------|
| `heyvm normalize-wasix-images` | Normalize WASIX image values | `heyvm normalize-wasix-images --dry-run` |
| `heyvm test-proxy` | End-to-end proxy test | `heyvm test-proxy --keep` |

## Create Options

```
--name <NAME>              Sandbox name (required)
--image <IMAGE>            Container image (default from settings)
--slug <SLUG>              URL-safe slug (default: slugified name)
--type <TYPE>              shell | python | node
--mount <HOST:SANDBOX>     Mount path (repeatable)
--ttl-seconds <SECS>       Time-to-live
--start-command <CMD>      Custom start command
--backend-type <TYPE>      msb | wasix | wasip2 | docker | apple_container | apple_virt | sandbox_exec | bubblewrap | libvirt | firecracker
--env <KEY=VALUE>          Environment variable (repeatable)
--setup-hook <CMD>         Shell command to run after creation or mount replacement (repeatable)
```

## Connect Options

```
<TICKET_URL>               heyo:// connection URL or shortname
-p, --port <PORT>          Local port to listen on (default: random)
-r, --relay <URL>          Relay server URL for short ticket lookup
--save <NAME>              Save connection under a shortname
--shell                    Open interactive SSH shell after connecting
--run-host                 Mount remote workspace via SSHFS and run a host command
--mount-path <PATH>        Sandbox path to mount (default: /workspace, used with --run-host)
```

## Archive-dir Options

```
[PATH]                     Local directory to archive (default: current directory)
--name <NAME>              Optional archive name
--mount-path <PATH>        Mount path prefix in the archive (default: /workspace)
--token <TOKEN>            JWT token (or set HEYO_ARCHIVE_TOKEN env var)
--no-ignore                Include build assets (node_modules, target, dist, etc.)
```

## Mount Options

```
<ID>                       Deployed sandbox ID, slug, or name
--mount-path <PATH>        Remote path to mount (default: /workspace)
--local-path <PATH>        Local directory to mount into (default: auto-generated)
[COMMAND]                  Host command to run after mounting (use -- before command)
```

## Update Options

```
<ID>                       Deployed sandbox ID, slug, or name
--archive <ARCHIVE_ID>     Archive ID to replace the mount with (required)
--mount-path <PATH>        Mount path to replace (default: /workspace)
```

## Backend Types

- **msb** — Microsandbox (default)
- **wasix** — WASIX WebAssembly
- **wasip2** — WASI Preview 2
- **docker** — Docker container
- **apple_container** — Apple Container (macOS only, uses Apple's `container` CLI)
- **apple_virt** — Apple Virtualization native VM (macOS only, uses Virtualization.framework directly)
- **sandbox_exec** — macOS sandbox (macOS only)
- **bubblewrap** — Linux namespaces (Linux only)
- **libvirt** — QEMU/KVM via libvirt (Linux only)
- **firecracker** — Firecracker microVMs (Linux only)

## Workflow

When the user asks to interact with sandboxes:

1. **First, check what's running**: `heyvm list` to see active sandboxes.
2. **Create if needed**: Use `heyvm create` with appropriate `--type` and `--backend-type`.
3. **Execute commands**: Use `heyvm exec <id-or-slug> -- <command>` for non-interactive work.
4. **Use slug or ID**: All commands that take `<id>` also accept the sandbox slug.

When the user provides `$ARGUMENTS`, interpret them as a subcommand and arguments to pass directly to `heyvm`. For example, `/heyvm list` should run `heyvm list`.

If `$ARGUMENTS` is empty, ask the user what they want to do with their sandboxes.
