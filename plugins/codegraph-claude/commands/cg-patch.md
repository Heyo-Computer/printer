---
description: Apply a unified diff to a file with `codegraph patch` instead of using Edit/Write. Patches validate context lines and fail loudly on stale context.
argument-hint: "<file> [--check] [--allow-outside]"
allowed-tools: Bash(codegraph:*)
---

# /cg-patch

Apply a patch to: `$ARGUMENTS`

## What to do

1. Build a unified diff for the change. Pull the current source with `/cg-snippet $ARGUMENTS <symbol>` first if you need to confirm the exact bytes in the context lines (whitespace, trailing chars, line endings all matter).

2. **Always dry-run first** when the context might be stale:

   ```
   codegraph patch $ARGUMENTS --check --diff /tmp/cg-edit.patch
   ```

   If `--check` fails, fix the diff (usually re-pulling fresh context with `/cg-snippet`) and re-run before applying for real.

3. Apply for real:

   ```
   codegraph patch $ARGUMENTS --diff /tmp/cg-edit.patch
   ```

   Or stream the diff over stdin to skip the temp file:

   ```
   printf '%s\n' "$DIFF" | codegraph patch $ARGUMENTS
   ```

4. Output is JSON by default (`{ ok, hunks_applied, hunks_total, bytes_written, failure }`). Pass `--text` for a one-line human summary. Exit code is non-zero on failure.

## Why patch instead of Edit / Write

- Patches validate the surrounding context, so a stale view of the file fails loudly instead of silently corrupting code.
- The diff is a self-contained record of the change — easier for the user to review than `Write` blasting a whole file.
- Smaller payload than re-emitting the entire file body via `Write`.

## When `Edit` / `Write` is still the right tool

- Creating a brand-new file (no existing bytes to diff against).
- Non-source artifacts where a diff is awkward (binary blobs, generated output).
- The user explicitly asks for a full-file rewrite.

For everything else: build a diff, `--check`, then apply with `/cg-patch`.
