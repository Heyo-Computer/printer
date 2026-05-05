---
name: poolside
description: Use this skill when you are running as the Poolside ACP agent inside printer (i.e. printer launched the `pool` CLI in `acp` server mode). Covers poolside-specific behavior — auth via `~/.config/poolside/credentials.json`, log/state under `~/.local/state/poolside/`, the `.poolside/` per-project config dir written into cwd — that the generic `acp-runtime` skill doesn't cover. The runtime contract (one turn = one `session/prompt`, session lifetime, cwd handling) lives in `acp-runtime`; install that skill alongside this one.
version: 0.2.0
---

# poolside

You are running as the **Poolside model**, dispatched by `printer` over
the [Agent Client Protocol](https://agentclientprotocol.com). Printer
launched the `pool` CLI in `acp` server mode and is talking to you via
JSON-RPC over stdio.

The generic ACP wire contract — one printer turn = one
`session/prompt`, persistent session, cwd-is-live-filesystem, permission
RPC discipline — lives in the `acp-runtime` skill. Read that one for
runtime fundamentals; this skill covers poolside-specific quirks.

## Auth

- Credentials are loaded from `~/.config/poolside/credentials.json`.
  `~/.config/` is read-only inside printer's heyvm sandbox (host bind),
  so you can read but not write that file from inside the sandbox.
  Refreshes that need to write should happen on the host before the
  printer run.
- `POOLSIDE_API_KEY` is also honored if you'd rather inject the key
  via environment.

## State and logs

- Runtime state and trajectories: `~/.local/state/poolside/`.
- Per-project config dir: `pool acp` writes `<cwd>/.poolside/` on first
  use. Inside printer's heyvm sandbox, cwd is `/workspace`, which is
  RW-bound to the host repo — so the directory persists across runs.
- ACP debug logs: `~/.local/state/poolside/pool/logs/`. Useful when
  printer surfaces a transport error and you want to see what poolside
  actually thought was happening.

## Conventions

Defer to the host repo's conventions over your defaults — the
`acp-runtime` skill has the full discipline. The short version: read
`AGENTS.md`/`CLAUDE.md`/`README.md`, match surrounding style, use the
build/test commands the repo documents.
