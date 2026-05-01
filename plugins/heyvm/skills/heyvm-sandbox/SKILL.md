---
name: heyvm-sandbox
description: Use this skill any time you are running inside a heyvm sandbox (i.e. a `printer run` / `printer exec` turn dispatched through the heyvm sandbox driver). Explains what is mounted, what is ephemeral, and how to inspect the sandbox from inside it. Triggers when the agent needs to know "where am I", "is this state persistent", "what host paths are visible", or wants to check the sandbox's resources.
version: 0.2.0
---

# heyvm-sandbox

You are running inside a **heyvm sandbox** — an isolated VM-backed
environment provisioned by `printer` through the `heyvm` plugin's driver.
Every command you run is executed via
`heyvm exec <handle> --session printer -- <argv>` against this sandbox
(argv is passed straight through to the persistent session shell — no
extra `sh -c` layer).

## What's mounted

- **The host's `printer exec` cwd is bind-mounted at `/workspace` inside
  the sandbox**, and `/workspace` is your starting cwd (set once via the
  driver's `post_create` step and retained by the persistent `printer`
  session). File reads and writes under `/workspace` are live on the
  host's filesystem — that's how the implementation phase's edits become
  real changes on the host repo, with no separate copy step.
- **`~/.claude` is bind-mounted RW** so claude code can read host
  credentials and persist per-session state (conversation logs,
  `session-env/<uuid>` for Bash tool calls). Writes here also land on the
  host, so don't delete or rewrite anything you didn't create yourself.
- **The rest of `$HOME` is read-only** (heyvm's bubblewrap default).
  Reads work; mkdir/touch outside `~/.claude` and `/workspace` will fail
  with `EROFS`. If you need scratch space, use `/tmp` or a fresh dir
  under `/workspace`.
- **Anything outside `/workspace` and `~/.claude` is ephemeral.** When
  the sandbox is destroyed (at the end of `printer exec`, or on early
  failure) installed packages, `~/.cache/`, `/tmp/`, etc. are gone.
  Don't store durable work there.

## When in doubt

- `pwd` — confirm you're in `/workspace`.
- `findmnt /workspace` — confirm the bind mount and see the host source path.
- `heyvm get <handle>` (from outside the sandbox) — show status and
  metadata for the running sandbox.

## Things to avoid

- **Don't `rm -rf` outside `/workspace` "to free space"** — host paths may
  be bind-mounted in unexpected places, and the sandbox itself is cheap to
  recreate.
- **Don't expect long-running background processes to survive** beyond the
  current `printer` invocation. Each `printer exec` provisions a fresh
  sandbox and tears it down at the end.
- **Don't use `sudo` to install global tooling** unless the spec calls for
  it; the sandbox is recreated on every run, so durable setup belongs in
  the base image (`sandbox.base_image` in `~/.printer/config.toml`) instead.

## Skipping the sandbox

If a step legitimately can't run inside the sandbox (for example, it needs
access to host devices), surface that to the user — don't try to escape.
They can re-run with `--no-sandbox` to dispatch on the host directly.
