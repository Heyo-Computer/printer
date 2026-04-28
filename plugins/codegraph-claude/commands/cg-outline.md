---
description: Show a hierarchical outline of a file (signatures only, no bodies). Cheap pre-read step before deciding whether to pull a snippet.
argument-hint: "<path/to/file>"
allowed-tools: Bash(codegraph:*)
---

# /cg-outline

Outline: `$ARGUMENTS`

## What to do

1. Run:

   ```
   codegraph --text outline $ARGUMENTS
   ```

2. Present the outline (kind + qualified name + line range) so the user can see the file's shape without reading bodies.

3. **Next step:** if the user wants the source of one symbol from the outline, call `/cg-snippet $ARGUMENTS <symbol-name>` — do not `Read` the file.

## When to use this instead of Read

Always reach for `/cg-outline` first when you (or the user) needs to know what's in a file but does not yet need the bodies. Reading a 1000-line source file just to find one function is wasteful — outline first, snippet second.
