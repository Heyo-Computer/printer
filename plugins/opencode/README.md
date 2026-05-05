# printer-plugin: opencode

ACP agent plugin for [OpenCode](https://opencode.ai). Lets printer drive
opencode in persistent ACP server mode (`opencode acp`) for multi-turn
sessions with conversational memory, instead of the one-shot `opencode
run --prompt …` fallback.

## Prerequisites

- The `opencode` CLI must be on `$PATH`.
- Auth configured via `opencode auth login` or provider env vars
  (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.).

## Install

### From a local checkout

```sh
# Install the shared ACP runtime skill (required for all ACP agents)
printer add-plugin path:plugins/acp-runtime

# Install the opencode agent plugin
printer add-plugin path:plugins/opencode
```

### From the git remote

```sh
printer add-plugin https://github.com/heyo-computer/printer \
    --subdir plugins/acp-runtime
printer add-plugin https://github.com/heyo-computer/printer \
    --subdir plugins/opencode
```

### Optional: codegraph integration

For tree-sitter-backed code navigation and patching inside opencode:

```sh
printer add-plugin path:plugins/codegraph-opencode
```

Also copy the opencode agent/command files into your project (or
`~/.config/opencode/`):

```sh
mkdir -p .opencode
cp -r plugins/codegraph-opencode/agent   .opencode/
cp -r plugins/codegraph-opencode/command .opencode/
```

And merge `plugins/codegraph-opencode/opencode.json` into your project's
`opencode.json`.

## Verify

```sh
printer plugins        # opencode should appear with ROLES=hooks+agent
printer hooks list     # opencode skill listed under before_run
```

## Usage

```sh
# One-shot opencode backend (no ACP, no persistent session)
printer exec spec.md --agent opencode

# ACP-backed opencode (persistent session, conversational memory)
printer exec spec.md --agent acp:opencode-acp
```

## What it does

- **`[[agent]]` block** — registers an ACP agent named `opencode-acp`
  that launches `opencode acp` (opencode's ACP server mode). Use
  `--agent acp:opencode-acp` to select it.
- **`before_run` skill** — registers `skills/opencode/SKILL.md` which
  briefs the agent on opencode-specific behavior (auth, providers,
  config) when running as an ACP server inside printer.

## See also

- `plugins/acp-runtime/` — shared ACP wire contract skill (install alongside)
- `plugins/codegraph-opencode/` — codegraph navigation/patching for opencode
- `printer/README.md` — full printer docs
- <https://agentclientprotocol.com> — ACP wire spec
