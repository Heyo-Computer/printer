---
name: acp-runtime
description: Use this skill any time you are running as an ACP (Agent Client Protocol) server inside printer — that means poolside, opencode, claude-code-acp, or any other vendor with `kind = "acp"` in its plugin manifest. Explains the wire contract printer expects (one turn = one `session/prompt`, persistent session, cwd-is-live-filesystem) plus the project-conventions discipline that applies regardless of which model is driving. Skip vendor-specific quirks — those live in the per-vendor skill that ships alongside this one.
version: 0.1.0
---

# acp-runtime

You are running as an **ACP server** spawned by `printer`. Printer
launched your CLI in ACP server mode and is talking to you via JSON-RPC
2.0 over stdio. This skill describes the runtime contract that holds for
any ACP-driven printer turn, regardless of vendor; your vendor-specific
skill (e.g. `poolside`, `opencode`) covers binary location, auth, and
quirks.

See <https://agentclientprotocol.com> for the wire spec.

## Runtime contract

- **One printer turn = one `session/prompt` request.** Stream content
  blocks back via `session/update` notifications as you work; finish the
  turn by resolving the request with a `stopReason`. Don't hold open
  state across turns expecting printer to send something else first —
  the next thing it sends is the next prompt (or a teardown).
- **Session lifetime spans the whole printer invocation.** Printer sends
  `session/new` once at startup and reuses the returned `sessionId` for
  every subsequent `session/prompt`. Don't assume a fresh session per
  turn — long-lived caches and conversational memory are expected.
- **The cwd printer hands you in `session/new` is the live host repo
  (or its sandbox bind mount).** Edits land directly on the user's
  filesystem. Treat reads as authoritative for current state and writes
  as production changes; there is no separate "stage and commit" step
  unless the spec calls for one.
- **Permission requests must resolve.** Printer auto-allows tool calls
  by default (it's running non-interactively against the
  `bypassPermissions` mode), but the wire contract still expects you to
  issue `session/request_permission` for any tool you'd ordinarily gate
  on. Don't try to bypass the prompt — printer will respond promptly.

## Project conventions over vendor defaults

This is somebody else's project, not your scratchpad. Before proposing
changes:

- Read what's already in the repo (`AGENTS.md`, `CLAUDE.md`,
  `README.md`, the immediate file's neighbors). If the repo encodes a
  style or workflow, follow it.
- Match the surrounding code's patterns — error handling, naming, test
  style — instead of rewriting things into your preferred shape.
- Use the build / test commands the repo documents. Don't introduce new
  tooling unless the spec asks for it.

## When to surface uncertainty

If a step requires a decision that isn't pinned down by the spec or
repo conventions (which API to add, which library to pull in, breaking
change vs. additive), flag it in your reply rather than picking
silently. Printer's review pass will catch silent guesses, but flagging
up front saves a round-trip.

## When something is wrong

If you can't make progress (auth failed, a required path is read-only,
a tool you need isn't available in this sandbox), surface the specific
error and stop. Printer cannot tell the difference between "thinking
hard" and "stuck on something fixable" without a textual signal, so
the round-trip is much shorter when you say so explicitly.
