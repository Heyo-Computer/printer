# codegraph (OpenCode plugin)

Tree-sitter-backed code navigation and patching for [OpenCode](https://opencode.ai).
Adds a `codegraph` agent and seven `/cg-*` slash commands that wrap the
`codegraph` CLI, with the built-in `read` / `edit` / `write` tools disabled
so the agent has to use codegraph for source files.

## Prerequisites

The `codegraph` binary must be on `PATH`. From the repo root:

```sh
make install-codegraph
# or, manually:
cd codegraph && cargo build --release && \
  install -m 0755 target/release/codegraph "$HOME/.local/bin/codegraph"
```

Run `codegraph index` once at the project root before using the agent —
or run `codegraph watch` in the background to keep the index continuously
fresh while you work. (If you launch this OpenCode session via
`printer run` / `printer exec`, the watch daemon is auto-spawned for you.)

## Install

OpenCode loads agents and commands from two locations:

- **Project-local**: `<repo>/.opencode/agent/`, `<repo>/.opencode/command/`
- **Global**: `~/.config/opencode/agent/`, `~/.config/opencode/command/`

Pick whichever scope you want.

### Project install

```sh
mkdir -p .opencode
cp -r plugins/codegraph-opencode/agent   .opencode/
cp -r plugins/codegraph-opencode/command .opencode/
```

Then merge `plugins/codegraph-opencode/opencode.json` into your project's
`opencode.json` (create it if you do not have one). The shipped snippet
disables the built-in `read` / `edit` / `write` tools so the agent must use
codegraph:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "instructions": [".opencode/agent/codegraph.md"],
  "tools": { "read": false, "edit": false, "write": false }
}
```

### Global install

```sh
mkdir -p ~/.config/opencode
cp -r plugins/codegraph-opencode/agent   ~/.config/opencode/
cp -r plugins/codegraph-opencode/command ~/.config/opencode/
```

Merge the same `tools` block into `~/.config/opencode/opencode.json` if you
want the redirect to apply globally (instead of per project).

## What you get

- A primary agent `codegraph` defined in `agent/codegraph.md`. Its system
  prompt explains the codegraph workflow (outline before reading, snippet
  before reading, patch before writing) and its `tools` map disables
  `read` / `edit` / `write`.
- Seven slash commands routed to that agent:
  `/cg-search`, `/cg-outline`, `/cg-snippet`, `/cg-symbols`,
  `/cg-def`, `/cg-refs`, `/cg-patch`.

## Using it

Pick the codegraph agent, or run the commands directly:

```
> /cg-outline src/server.rs
> /cg-snippet src/server.rs handle_request
> /cg-patch  src/server.rs           # then paste a unified diff
```

The agent will reach for codegraph on its own once it sees the system
prompt; the slash commands are just convenient shortcuts when you want to
drive a specific call yourself.

## Keeping the index fresh

The codegraph plugin in this repo for *Claude Code* launches a `codegraph
watch` daemon at session start. OpenCode does not currently expose an
equivalent session-start hook in the markdown agent format, so for
OpenCode the recommended pattern is one of:

1. **Run `codegraph watch` manually** in a terminal alongside OpenCode.
   The daemon picks up file changes (including ones the agent makes) and
   keeps `.codegraph/index.json` current.

2. **Drive OpenCode via `printer exec` / `printer run`** — printer
   auto-launches the watch daemon for the duration of the operation
   (see `../../printer/README.md`).

3. **Skip the daemon and re-run `codegraph index` manually** between work
   sessions. Indexing is fast (mtime-cached), so this is fine for short
   bursts.

## Files

```
plugins/codegraph-opencode/
├── README.md                (this file)
├── opencode.json            (snippet to merge into your config)
├── agent/
│   └── codegraph.md
└── command/
    ├── cg-search.md
    ├── cg-outline.md
    ├── cg-snippet.md
    ├── cg-symbols.md
    ├── cg-def.md
    ├── cg-refs.md
    └── cg-patch.md
```
