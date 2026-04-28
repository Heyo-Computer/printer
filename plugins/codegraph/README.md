# codegraph plugin

Printer plugin that wires the `codegraph` CLI into the agent's run and
review phases. Once installed, every implementation turn carries skill
references and an instruction telling the agent to prefer `codegraph` for
search/snippet/outline operations and to apply edits via `codegraph patch`
instead of rewriting whole files.

The plugin is agent-agnostic — it works for both Claude and Opencode
because printer injects hooks purely via prompt text.

## Install

From this repo:

```sh
printer add-plugin path:./plugins/codegraph
```

After install:

```sh
printer hooks list                     # see all 5 hook entries
printer hooks list --event before_run
```

## What the plugin does

| Event           | Kind        | Purpose                                         |
|-----------------|-------------|-------------------------------------------------|
| `before_run`    | cli         | `codegraph index` — refresh the index           |
| `before_run`    | agent-cmd   | "Prefer codegraph for navigation and patches"   |
| `before_run`    | agent-skill | `skills/codegraph-search/SKILL.md`              |
| `before_run`    | agent-skill | `skills/codegraph-edit/SKILL.md`                |
| `before_review` | agent-skill | `skills/codegraph-search/SKILL.md`              |

The actual `codegraph` CLI is a separate binary; this plugin only registers
hooks. Install codegraph via `make install-codegraph` from the repo root.

## Authoring note

The plugin's `printer-plugin.toml` declares hooks and a single
`assets = ["skills"]` line. `printer add-plugin` reads this file at install
time, validates the hooks, and copies the `skills/` directory into the
installed plugin dir alongside the binary. See `../../printer/HOOKS.md` for
the schema.
