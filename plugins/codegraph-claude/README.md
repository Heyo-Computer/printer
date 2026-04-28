# codegraph (Claude Code plugin)

Tree-sitter-backed code navigation and patching for Claude Code. Adds:

- **Slash commands** that wrap the `codegraph` CLI:
  `/cg-search`, `/cg-outline`, `/cg-snippet`, `/cg-symbols`, `/cg-def`,
  `/cg-refs`, `/cg-patch`.
- **Skills** (`codegraph-search`, `codegraph-edit`) that nudge the model to
  prefer `codegraph` over `Read` / `Grep` / `Edit` / `Write` for source files.
- **SessionStart / SessionEnd hooks** that launch a `codegraph watch`
  daemon for the project and tear it down when the session ends, so the
  on-disk index stays current as files change.

## Prerequisites

The `codegraph` binary must be on `PATH`. Build and install it from
[`codegraph/`](../../codegraph) at the repo root:

```sh
cd codegraph
cargo build --release
install -m 0755 target/release/codegraph "$HOME/.local/bin/codegraph"
# or, from the project root:
make install-codegraph
```

## Install

This plugin lives at `plugins/codegraph-claude/` in the
[printer](https://github.com/sarocu/printer) repo. Either:

1. **From a marketplace** (recommended once published):

   ```
   /plugin install codegraph@<your-marketplace>
   ```

2. **From a local checkout** — start Claude Code with the plugin path:

   ```sh
   claude --plugin-dir /absolute/path/to/printer/plugins/codegraph-claude
   ```

The plugin manifest lives at
`plugins/codegraph-claude/.claude-plugin/plugin.json`.

## What you get out of the box

- `codegraph watch` is launched at session start (PID written to
  `<project>/.codegraph/watch.pid`, logs to `<project>/.codegraph/watch.log`).
  The daemon is killed on `SessionEnd` / `Stop`. Re-attaches gracefully if a
  daemon is already running for the project.
- All `/cg-*` slash commands are available immediately.
- The two skills are auto-loaded; they instruct the agent to reach for
  codegraph instead of `Read` / `Grep` / `Edit` / `Write` whenever a file is
  in a supported language (Rust, Python, JavaScript, TypeScript).

## Opt-in: hard-enforce the redirect

Skills only nudge — the agent can still call `Read` / `Edit` / `Write` if it
decides to. To **block** those tools entirely (so the agent has no choice but
to use the codegraph commands), add a `PreToolUse` hook to your user
settings at `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read|Edit|Write|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": "jq -n '{hookSpecificOutput: {hookEventName: \"PreToolUse\", permissionDecision: \"deny\", permissionDecisionReason: \"Use codegraph: /cg-snippet to read source, /cg-patch to edit. Read/Edit/Write are blocked for tracked source files in this project.\"}}'"
          }
        ]
      }
    ]
  }
}
```

This denies the call and shows the reason to the model, which then chooses a
codegraph command. To scope the deny to a single project, put the same JSON
in `<project>/.claude/settings.local.json` instead.

If you want a softer variant that warns rather than denies, swap the
`permissionDecision` for `"ask"` (Claude Code will prompt you each time) or
emit nothing on stdout and just log a hint to stderr.

## Troubleshooting

- **Daemon didn't start.** Check `<project>/.codegraph/watch.log`. Common
  causes: `codegraph` not on `PATH` (the hook prints a one-line warning to
  stderr and skips), or another process already holding the pidfile (the
  hook is idempotent — kill the stale process or delete `watch.pid`).
- **Index is empty.** The daemon walks the tree on startup using `ignore`
  crate semantics — anything in `.gitignore` and the standard exclusion
  list (`target`, `node_modules`, `dist`, `build`, `.next`, `.venv`,
  `__pycache__`, `.cache`, …) is skipped. Make sure your source files are
  in supported languages: Rust, Python, JavaScript, TypeScript.
- **`/cg-*` runs but says "no index". `codegraph index` from the project
  root once. The watch daemon should keep it fresh after that.

## Files

```
plugins/codegraph-claude/
├── .claude-plugin/plugin.json
├── README.md                          (this file)
├── commands/
│   ├── cg-search.md
│   ├── cg-outline.md
│   ├── cg-snippet.md
│   ├── cg-symbols.md
│   ├── cg-def.md
│   ├── cg-refs.md
│   └── cg-patch.md
├── hooks/
│   ├── hooks.json
│   ├── start-watch.sh
│   └── stop-watch.sh
└── skills/
    ├── codegraph-search/SKILL.md
    └── codegraph-edit/SKILL.md
```
