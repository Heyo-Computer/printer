# printer-plugin: acp-runtime

Shared `before_run` skill that briefs ACP-driven agents on the wire
contract printer expects. Co-installed with any vendor ACP plugin
(`poolside`, `opencode`, future `claude-code-acp`) so the common bits
— "one turn = one `session/prompt`", session lifetime, cwd is live
filesystem, permission RPC discipline, project-conventions-over-vendor-
defaults — live in one place rather than getting duplicated across
vendors.

This plugin contributes no `[[agent]]`. It's purely a skill. Install it
alongside whichever vendor plugin you're driving:

## Install

```sh
# alongside poolside
printer add-plugin path:plugins/acp-runtime
printer add-plugin path:plugins/poolside

# alongside opencode
printer add-plugin path:plugins/acp-runtime
printer add-plugin path:plugins/opencode
```

Or pull both from a git remote with `--subdir`:

```sh
printer add-plugin https://github.com/heyo-computer/printer --subdir plugins/acp-runtime
printer add-plugin https://github.com/heyo-computer/printer --subdir plugins/opencode
```

Verify:

```
printer plugins                        # acp-runtime should appear with ROLES=hooks
printer hooks list --event before_run  # acp-runtime skill listed
```

## What it briefs

- **Runtime contract** — one printer turn = one `session/prompt`;
  session id is reused across the whole printer invocation; cwd is
  live; permission requests resolve.
- **Project conventions** — read `AGENTS.md` / `CLAUDE.md` / repo
  files; match surrounding style; use the repo's build/test commands.
- **Surface uncertainty** — flag silent decisions early instead of
  letting them surface in review.
- **Surface failure** — bail with a specific error rather than
  appearing to "think hard" when actually stuck.

## See also

- `printer/HOOKS.md` — schema for `[[agent]]` blocks and the ACP transport.
- `plugins/poolside/`, `plugins/opencode/` — vendor plugins that
  layer their specifics on top of this shared briefing.
