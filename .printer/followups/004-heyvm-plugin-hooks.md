# Follow-ups for /home/sarocu/Projects/printer/specs/004-heyvm-plugin-hooks.md

Generated: 2026-04-30T16:37:57.781243629+00:00
Verdict: PASS

## Suggested follow-ups

- Wire a `--keep-sandbox` flag (mentioned in the `Drop` log) to `ActiveSandbox::set_keep`.
- Teach the registry installer to also drop the `[driver]` manifest so `printer add-plugin heyvm` is one-step instead of two.
- Add an integration test that exercises an `ActiveSandbox` end-to-end with a stub driver (`create=echo handle-123`, `enter=sh -c {child}`) so the lifecycle isn't only covered by unit tests of the pure helpers.
- Clean up the new `collapsible_if` / `useless format!` clippy hits in `drivers.rs` and `config.rs`.
