---
name: opencode
description: Use this skill when you are running as the opencode ACP agent inside printer (i.e. printer launched the `opencode` CLI in `acp` server mode via `--agent acp:opencode-acp`). Covers opencode-specific behavior — provider/auth via `opencode auth`, multi-provider support, configuration via `opencode.json` and `~/.config/opencode/` — that the generic `acp-runtime` skill doesn't cover. The runtime contract (one turn = one `session/prompt`, etc.) lives in `acp-runtime`; install that skill alongside this one.
version: 0.1.0
---

# opencode

You are running as the **opencode** agent, dispatched by `printer` over
the [Agent Client Protocol](https://agentclientprotocol.com). Printer
launched the `opencode` CLI in `acp` server mode (`opencode acp`) and is
talking to you via JSON-RPC over stdio.

The generic ACP wire contract — one printer turn = one
`session/prompt`, persistent session, cwd-is-live-filesystem, permission
RPC discipline — lives in the `acp-runtime` skill. Read that one for
runtime fundamentals; this skill covers opencode-specific quirks.

## Path matters: built-in vs. ACP

Printer has *two* ways to reach opencode:

- `--agent opencode` — printer's built-in one-shot path. Spawns
  `opencode run --prompt …` once per turn, captures stdout, exits. No
  ACP handshake; you wouldn't be reading this skill.
- `--agent acp:opencode-acp` — this plugin. Long-lived `opencode acp`
  server speaking ACP over stdio. You're here.

The two paths share a binary but have different runtime contracts.
This skill applies only when you're driving the persistent ACP path.

## Auth and providers

Opencode supports multiple providers (Anthropic, OpenAI, etc.).
Credentials are configured via:

- `opencode auth login` (interactive) — writes to opencode's auth store.
- Per-provider environment variables (`ANTHROPIC_API_KEY`,
  `OPENAI_API_KEY`, etc.).

In printer's heyvm sandbox, opencode's auth store under
`~/.config/opencode/` is RO-bound from the host. Set up
authentication on the host before the printer run; refreshes that
need to write back will fail inside the sandbox.

## Configuration

- Per-project: `opencode.json` in the workspace root, if present, is
  honored. Printer doesn't write or modify it; treat it as user-owned.
- Global: `~/.config/opencode/`.

If a step would change `opencode.json` (adding a custom command, a new
agent definition, etc.), call that out in your reply — it's a config
change the user may want to review separately from code changes.

## State and logs

Use `opencode --print-logs --log-level DEBUG` from the host (not from
inside this ACP turn) to see opencode's own diagnostic logs when
investigating a transport failure. From inside the turn, prefer
surfacing the failure via `session/update` content blocks rather than
stuffing it into log files printer would have to fish out.

## Conventions

Defer to the host repo's conventions over your defaults — the
`acp-runtime` skill has the full discipline. The short version: read
`AGENTS.md`/`CLAUDE.md`/`README.md`, match surrounding style, use the
build/test commands the repo documents.
