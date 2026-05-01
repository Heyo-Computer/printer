---
name: heyvm-sandbox
description: Use this skill any time you are running inside a heyvm worktree (i.e. a `printer run` / `printer exec` turn dispatched through the heyvm sandbox driver). Explains what is mounted, what is ephemeral, and how to inspect the worktree from inside it. Triggers when the agent needs to know "where am I", "is this state persistent", "what host paths are visible", or wants to check the worktree's resources.
version: 0.1.0
---

# heyvm-sandbox

You are running inside a **heyvm worktree** — an isolated VM-backed sandbox
provisioned by `printer` through the `heyvm` plugin's driver. Every shell
command you run is executed via `heyvm worktree exec <handle> -- sh -c …`
against this worktree.

## What's mounted

- **`{cwd}` from the host is the working directory inside the worktree.**
  Files you read or write under cwd round-trip back to the host: that's how
  the implementation phase's edits become real changes on the host repo.
- **Anything outside cwd is ephemeral.** When the worktree is destroyed (at
  the end of `printer exec`, or on early failure) installed packages,
  `~/.cache/`, `/tmp/`, etc. are gone. Don't store work there.

## When in doubt

- `pwd` — confirm you're in the mounted cwd.
- `heyvm worktree status` — print resource usage and lifecycle state for the
  current worktree, if you need to check disk / memory / network.
- `mount` or `findmnt {cwd}` — confirm what's actually shared from the host.

## Things to avoid

- **Don't `rm -rf` outside cwd "to free space"** — host paths may be bind-
  mounted in unexpected places, and the worktree itself is cheap to recreate.
- **Don't expect long-running background processes to survive** beyond the
  current `printer` invocation. Each `printer exec` provisions a fresh
  worktree and tears it down at the end.
- **Don't use `sudo` to install global tooling** unless the spec calls for
  it; the worktree is recreated on every run, so durable setup belongs in
  the base image (`sandbox.base_image` in `~/.printer/config.toml`) instead.

## Skipping the sandbox

If a step legitimately can't run inside the worktree (for example, it needs
access to host devices), surface that to the user — don't try to escape.
They can re-run with `--no-sandbox` to dispatch on the host directly.
