# printer-plugin: heyvm

Sandbox driver for [printer] that dispatches every agent turn through a
[heyvm] worktree, plus a `before_run` skill that briefs the agent on the
worktree it lands in.

[printer]: https://github.com/Heyo-Computer/printer
[heyvm]: https://docs.heyo.computer

## Prerequisites

The `heyvm` CLI must be on `$PATH`. The vendor installer puts it at
`~/.local/bin/heyvm` when `/usr/local/bin` is not writable. See
<https://docs.heyo.computer> for full install docs.

## Install

This plugin contributes a `[driver]` block and a skill — there is no
binary to build. You can install it from the public printer repo or from a
local checkout.

### From the printer git remote (no checkout needed)

`--subdir` points at the plugin inside the monorepo; the inferred plugin
name comes from the subdirectory's basename (`heyvm`):

```
printer add-plugin https://github.com/heyo-computer/printer \
    --subdir plugins/heyvm
```

Pin a specific revision with `--rev <branch|tag|sha>`. To pull the latest
`main` over an existing install, append `--force`.

### From a local checkout

`path:` specs are resolved against your **current working directory**, so
either run from the printer repo root or pass an absolute path:

```
# from the printer repo root:
printer add-plugin path:plugins/heyvm

# or from anywhere:
printer add-plugin path:/abs/path/to/printer/plugins/heyvm
```

### Two-step (driver manifest + heyvm CLI)

The registry name (`printer add-plugin heyvm`) currently runs only the
vendor's `curl … | sh` installer to drop the `heyvm` CLI at
`~/.local/bin/heyvm`; it does not yet bundle the `[driver]` manifest. Until
that is wired up, install in two steps if you also need the CLI:

```
printer add-plugin heyvm                                        # heyvm CLI -> ~/.local/bin
printer add-plugin https://github.com/heyo-computer/printer \   # driver + skill manifest
    --subdir plugins/heyvm --force
```

`--force` only clears `~/.printer/plugins/heyvm/`, not `~/.local/bin/heyvm`.

That copies the `[driver]` block and the `heyvm-sandbox` skill into
`~/.printer/plugins/heyvm/`. Verify with:

```
printer plugins                       # heyvm should appear with ROLES=hooks+driver
printer hooks list --event before_run # heyvm-sandbox skill listed
```

If `heyvm` itself is not on `$PATH`, the install still succeeds (the manifest
is the only thing that lands), but the next `printer exec` will fail at
`create` with heyvm's stderr surfaced.

## What it does

- `[driver]` — `printer exec` runs every agent invocation as
  `heyvm worktree exec <handle> -- sh -c '<quoted argv>'`. The worktree is
  created at the top of `exec`, host cwd is pushed in via
  `heyvm worktree push`, and the worktree is destroyed when `exec` finishes
  (or early-returns / panics — `destroy` runs from `Drop`).
- `before_run` skill — registers `skills/heyvm-sandbox/SKILL.md` so the
  implementer agent understands cwd is mounted from the host while
  everything else is ephemeral.

## Lifecycle of one `printer exec`

A single sandbox spans both run and review phases (provisioned in
`exec.rs::acquire_exec_sandbox`). The lifecycle is:

1. **`create`** — `heyvm worktree create --base {base_image} --name printer-{spec_slug}`.
   Stdout is captured as the opaque worktree `{handle}` and reused by every
   later step.
2. **`sync_in`** — `heyvm worktree push {handle} {cwd}`. Host cwd is
   copied **into** the worktree before any agent runs.
3. **Each agent turn** — both the implementer (`run`) and the reviewer
   (`review`) are wrapped via `enter`: `heyvm worktree exec {handle} -- sh -c {child}`.
   File edits happen on the worktree's copy of cwd, not the host's.
4. **`sync_out`** — `heyvm worktree pull {handle} {cwd}`. The worktree's
   cwd is copied **back over** the host cwd. Best-effort: failures are
   logged and swallowed so a flaky pull can't strand a finished run.
5. **`destroy`** — `heyvm worktree destroy {handle}`. Runs from `Drop`,
   so it fires on panic, early return, or Ctrl-C.

### Files are *replaced*, not git-merged

`sync_out` is a `heyvm worktree pull` — a file-tree copy from the worktree
back to the host. There is no `git merge`, no rebase, and no three-way
reconciliation. If the host cwd was modified in parallel while exec was
running, those host changes will be clobbered by whatever the worktree
produced. Treat the host cwd as locked for the duration of `printer exec`.

Only `{cwd}` round-trips. Anything the agent installed outside cwd
(`~/.cache/`, `/tmp/`, system packages) is discarded with the worktree —
durable tooling belongs in `sandbox.base_image`.

To inspect the result without auto-pulling, run with `--keep-sandbox` (when
exposed) or override `sync_out`/`destroy` to no-ops in `[sandbox.commands]`.

## Configuration

Override any driver step in `~/.printer/config.toml`:

```toml
[sandbox]
driver = "heyvm"             # or "auto" if heyvm is the only installed driver
base_image = "heyvm:ubuntu-22.04"

[sandbox.commands]
# create  = "heyvm worktree create --base {base_image} --name printer-{spec_slug}"
# enter   = "heyvm worktree exec {handle} -- sh -c {child}"
# destroy = "heyvm worktree destroy {handle}"
```

See `printer/HOOKS.md` ("Sandbox drivers" + "Global config") for the full
schema and the variables available to driver templates.

## Bypassing the sandbox

```
printer exec spec.md --no-sandbox
```

Equivalent to `sandbox.driver = "off"` for one invocation. Useful for local
debugging when the driver itself is misbehaving.
