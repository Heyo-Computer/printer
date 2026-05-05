# printer-plugin: poolside

ACP agent plugin for [printer] that lets `printer run` / `exec` / `review`
dispatch through the [Poolside CLI]'s ACP server mode.

[printer]: https://github.com/Heyo-Computer/printer
[Poolside CLI]: https://docs.poolside.ai/cli/editor-integration#editor-integration-acp

## Prerequisites

The `pool` binary must be on `$PATH`. See
<https://docs.poolside.ai> for install instructions and credential setup
(typically a `POOLSIDE_API_KEY` env var or `poolside login`).

If `pool` is not on `$PATH` at run time, the ACP transport surfaces
the spawn error from poolside's stderr — there is no silent hang.

## Install

Skill + agent manifest plugin; no binary to build.

### From the printer git remote (no checkout needed)

```
printer add-plugin https://github.com/heyo-computer/printer \
    --subdir plugins/poolside
```

Pin a specific revision with `--rev <branch|tag|sha>`. To pull the latest
`main` over an existing install, append `--force`.

### From a local checkout

```
# from the printer repo root:
printer add-plugin path:plugins/poolside

# or from anywhere:
printer add-plugin path:/abs/path/to/printer/plugins/poolside
```

Verify the install:

```
printer plugins                        # poolside should appear with ROLES=hooks+agent
printer hooks list --event before_run  # poolside skill listed
```

## Usage

```
printer run spec.md --agent acp:poolside
printer exec spec.md --agent acp:poolside
printer review --agent acp:poolside
```

Override the launch argv inline if needed:

```
printer run spec.md --agent acp:poolside \
    --acp-bin /custom/path/to/pool \
    --acp-arg --log-level --acp-arg debug
```

`--acp-bin` replaces the manifest's `command`; `--acp-arg` tokens are
appended after the manifest's `args`.

## Sandbox interaction

If a sandbox driver (e.g. heyvm) is active, printer wraps the ACP server
launch through the driver's `enter` template — `pool` runs inside the
sandbox just like a per-turn child would. The sandbox spans the whole
printer invocation, so the long-lived ACP session lives for the lifetime
of the sandbox.

Skip the sandbox with `--no-sandbox` for host-side debugging.

### Writable state requirement

`pool acp` writes runtime state and logs to `~/.local/state/poolside/`
on first use. If your sandbox driver bind-mounts `$HOME` read-only
(bubblewrap-style), `pool` will fail with `read-only file system` on its
log-dir setup before answering any ACP request — printer surfaces the
captured stderr in that case, but the underlying fix is on the driver.

The bundled `heyvm` plugin already exposes `~/.local/state` and `~/.cache`
RW from the host. If you write or use a different driver, ensure those
two paths (or at least `~/.local/state/poolside/`) are writable inside
the sandbox.

### `heyvm exec` streaming (resolved 2026-05-04)

Earlier `heyvm exec` releases buffered child stdout and only flushed at
exit, which deadlocked any persistent ACP server. `heyvm` ≥ v0.27.2
streams stdout in real time, so `printer exec ... --agent acp:poolside`
through a `heyvm` sandbox now works end-to-end. If you hit a hang on
`still waiting for initialize…`, first check your heyvm version:

```sh
heyvm --version
```

and update if it's older than 0.27.2. Set `PRINTER_ACP_TRACE=1` for
byte-level transport traces if the issue persists after upgrading.

## See also

- `printer/HOOKS.md` — schema for `[[agent]]` blocks and the ACP transport.
- T-020 in `.printer/tasks/` — streaming, cancellation, and permission-mode
  follow-ups for the ACP transport (apply to every ACP plugin including
  this one).
