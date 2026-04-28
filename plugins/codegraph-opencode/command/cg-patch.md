---
description: Apply a unified diff with `codegraph patch` instead of write/edit.
agent: codegraph
---

Apply a patch to the file passed as `$ARGUMENTS`.

1. If you are not certain the surrounding context is fresh, re-pull it with
   `/cg-snippet $ARGUMENTS <symbol>` first. Whitespace and trailing
   characters in the diff context lines must match the file exactly.

2. Always dry-run first when in doubt:

   ```
   codegraph patch $ARGUMENTS --check --diff /tmp/cg-edit.patch
   ```

   If `--check` fails, re-pull the snippet, rebuild the diff, and re-run.

3. Apply for real:

   ```
   codegraph patch $ARGUMENTS --diff /tmp/cg-edit.patch
   ```

   Or stream the diff over stdin to skip the temp file:

   ```
   printf '%s\n' "$DIFF" | codegraph patch $ARGUMENTS
   ```

Default output is JSON; pass `--text` for a one-line summary. Exit code is
non-zero on failure.

Never fall back to a full-file `bash` rewrite to "just overwrite the file"
— that loses concurrent edits and defeats the whole point of patches. Fix
the diff instead.
