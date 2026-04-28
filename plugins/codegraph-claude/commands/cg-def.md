---
description: Jump to the definition of a symbol via the codegraph index. Replaces grep-then-Read for "where is X defined?".
argument-hint: "<symbol-name | qualified::name>"
allowed-tools: Bash(codegraph:*)
---

# /cg-def

Find the definition of: `$ARGUMENTS`

## What to do

1. Run:

   ```
   codegraph --text definition $ARGUMENTS
   ```

   The argument can be a bare name (`handle_request`) or qualified (`Server::handle_request`).

2. Present each hit as `file:line  Kind  qualified-name  signature`.

3. **Next step:** to read the body of the matched definition, use `/cg-snippet <file> <symbol>` — do not `Read` the whole file.
