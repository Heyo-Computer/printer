---
description: List symbols (functions, classes, structs, …) in a single file with their kind, line range, and signature.
argument-hint: "<path/to/file>"
allowed-tools: Bash(codegraph:*)
---

# /cg-symbols

List symbols in: `$ARGUMENTS`

## What to do

1. Run:

   ```
   codegraph --text symbols $ARGUMENTS
   ```

2. Show the table to the user. Columns are `Kind`, qualified name, line range, signature.

## When to use this vs `/cg-outline`

- `/cg-symbols` is a flat list — best when you want every symbol in the file at once for grepping or piping into a follow-up.
- `/cg-outline` is hierarchical — best when the file has nested structures (impls, classes with methods) and you care about the parent–child relationships.
