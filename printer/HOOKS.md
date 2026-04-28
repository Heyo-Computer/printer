# Plugin hooks

Plugins are no longer just "an extra binary on `$PATH`-via-printer". They can
register **hooks** that fire at specific points in the `printer init` /
`printer exec` lifecycle. A single plugin can ship as many hooks as it wants;
the existing `printer <plugin-name> <args>` dispatch still works alongside.

There are two hook **types**:

- **CLI hooks** — printer runs an arbitrary shell command, with context
  variables interpolated. Use these to notify, log, post artifacts,
  precondition the workspace, etc.
- **Agent hooks** — printer enriches the prompt or available skills for
  the agent session that's about to run. Use these to register a Claude
  skill, force a slash-command-like instruction, or inject phase-specific
  guidance.

## Where hooks come from

Hooks live in installed plugin manifests at:

```
~/.printer/plugins/<plugin-name>/plugin.toml
```

Any plugin manifest may include `[[hooks]]` entries (see schema below). When
you run `printer init` or `printer exec`, every installed plugin's manifest is
read and its hooks are fired at the matching events.

## Hook events

Events fire in this order. `before_*` events run *before* the phase begins;
`after_*` events run *after* it ends, with `{exit_status}` set to either
`ok` or `err`.

### `before_init` / `after_init`

Fires around `printer init`. The spec file has just been written (or is about
to be); `.printer/tasks/` and the codegraph index get bootstrapped between
the two. Useful for:

- CLI: dropping additional config files into the new project, kicking off a
  one-off bootstrap.
- Agent: N/A (no agent runs during init).

### `before_exec` / `after_exec`

Outermost wrapping of `printer exec`. Fires once per invocation, before any
phase decision is made and after the whole pipeline finishes (success **or**
failure). Useful for:

- CLI: notify-on-start / notify-on-finish, push artifacts up to a server.
- Agent: N/A (no single agent session spans exec).

### `before_run` / `after_run`

Wraps the implementation phase (the `run::run` loop). `before_run` is the
right place to **augment every nudge prompt** in the run loop. Agent
hook contributions to `before_run` are folded into the per-turn nudge prompt
in `printer/src/prompts.rs::nudge_prompt`, so they apply for every turn until
the phase ends. Useful for:

- CLI: pre-/post-flight checks (lint, tests, branch state).
- Agent: pin coding-style guidance for the run loop, register a skill the
  implementer should consult, or inject "always run /lint after edits".

### `before_review` / `after_review`

Wraps the review phase (single agent turn). Agent hook contributions to
`before_review` are appended to the review prompt and skill list. Useful for:

- CLI: copy the produced report into a docs site, post the verdict to chat.
- Agent: load a custom security-review skill, force the reviewer to grade
  against an extra rubric.

## Hook schema

Inside a plugin manifest:

```toml
# Existing fields:
name = "..."
version = "..."
binary = "bin/..."
installed_at = "..."

[source]
type = "git" | "path" | "shell"
# ...

# NEW: hooks the plugin registers.

[[hooks]]
type = "cli"
event = "after_review"
command = "notify-slack '#releases' 'review for {spec} finished: {exit_status}'"
# Optional. How to react to a non-zero exit from the hook:
#   "fail"   — abort the printer run (default for before_* events)
#   "warn"   — log a warning, keep going (default for after_* events)
#   "ignore" — silent
on_failure = "warn"

[[hooks]]
type = "agent"
event = "before_review"
# Inject a slash-command-style instruction into the review prompt.
command = "Run /security-review and incorporate its findings into the review."

[[hooks]]
type = "agent"
event = "before_run"
# Path to a SKILL.md (or skill directory containing one). Resolved relative
# to the plugin's directory. The skill is exposed to the agent in the same
# way `printer review --skill` exposes review-time skills.
skill = "skills/our-coding-style/"
```

A single `[[hooks]]` entry must specify exactly one of `command` (CLI or
agent) or `skill` (agent only).

## Available `{variables}` for interpolation

Substituted into CLI `command` strings and agent `command` text:

| Variable          | Available in                          | Meaning                                   |
|-------------------|---------------------------------------|-------------------------------------------|
| `{cwd}`           | all events                            | Working directory printer is operating in |
| `{spec}`          | all events except `before_init`       | Absolute path to the spec file            |
| `{event}`         | all events                            | The event name (`before_run`, …)          |
| `{phase}`         | run / review events                   | `run` or `review`                         |
| `{exit_status}`   | `after_*` events                      | `ok` or `err`                             |
| `{base_ref}`      | review events                        | Git ref used for the review diff          |
| `{report_path}`   | `after_review` (when `--out` set)    | Path where the review report was written  |

Unknown `{vars}` are left in place (so a hook command that uses `{name}` for
its own templating won't be mangled).

## CLI environment

CLI hooks are spawned via `sh -c "<command>"` in the working directory. They
inherit printer's environment plus the following injected vars (mirrors the
interpolation table above; useful when you'd rather read env than template):

```
PRINTER_HOOK_EVENT=after_review
PRINTER_HOOK_PHASE=review
PRINTER_HOOK_CWD=/abs/path
PRINTER_HOOK_SPEC=/abs/path/spec.md
PRINTER_HOOK_EXIT_STATUS=ok
PRINTER_HOOK_BASE_REF=main
PRINTER_HOOK_REPORT_PATH=/abs/path/review.md     # if --out was set
PRINTER_PLUGIN=my-plugin                          # which plugin owns this hook
```

## Authoring a plugin

A plugin's source crate may include an optional `printer-plugin.toml` at
its root. When `printer add-plugin` installs the plugin, this file is read
and its contents are merged into the installed `plugin.toml` and copied
into the install directory. Without this file, plugins install with no
hooks and no extra assets — the historical behaviour.

```toml
# plugins/<name>/printer-plugin.toml

# Files or directories (relative paths only, no `..`) to copy verbatim
# from the source root into the installed plugin dir alongside the
# binary. Hook fields like `skill = "skills/foo/SKILL.md"` resolve
# against the install dir, so anything they reference must be listed here.
assets = ["skills"]

# Hooks have the same schema as `[[hooks]]` in the installed manifest;
# they are validated at install time and refused if malformed.

[[hooks]]
type = "cli"
event = "before_run"
command = "codegraph index"
on_failure = "warn"

[[hooks]]
type = "agent"
event = "before_run"
skill = "skills/codegraph-search/SKILL.md"

[[hooks]]
type = "agent"
event = "before_run"
command = "Prefer `codegraph` over grep + Read for navigation."
```

Notes:

- **Source dir requirement** — `printer-plugin.toml` is only read for
  plugins installed via `path:` or a git URL (cargo crates with a real
  source tree). Shell-installer plugins (`--install-cmd`) have no source
  dir under printer's control, so they cannot ship hooks via this
  mechanism; their hooks must be hand-edited into `plugin.toml`.
- **`cargo install` discards non-binary files** — the only reason your
  `skills/` directory survives the install is the `assets` list. Forget
  to include it and the hook's `skill = "..."` paths will dangle.
- **Validation is strict** — install fails if a hook references an
  unknown event or violates the type/command/skill constraints. Fix the
  source manifest and re-run with `--force`.
- **Path safety** — assets must be relative paths without `..`
  components. Symlinks inside an asset directory are refused. The install
  refuses to clobber pre-existing files in the install dir.

## Listing what's wired up

```
printer hooks list             # all hooks across all installed plugins
printer hooks list --event after_review
```

Shows event, plugin name, kind, and resolved command/skill for each hook.

## Backwards compatibility

- `printer add-plugin <spec>` works exactly as before. A plugin without
  `[[hooks]]` in its manifest contributes no hooks (and dispatches via
  `printer <name> <args>` as it always has).
- `printer plugins` keeps listing installed plugins.
- The new hook system is purely additive: removing every `[[hooks]]` entry
  recovers the old behaviour.
