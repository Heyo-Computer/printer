---
description: Search the codebase for a symbol or signature substring using `codegraph search`. Far cheaper than grep + Read for locating named code.
argument-hint: "<query> [--name] [--kind <kind>] [--limit N]"
allowed-tools: Bash(codegraph:*)
---

# /cg-search

Run a codegraph search against the on-disk index for: `$ARGUMENTS`

## What to do

1. Invoke:

   ```
   codegraph --text search $ARGUMENTS
   ```

   Append `--limit 50` if the user did not supply one and the query looks broad. Use `--name` to restrict matching to symbol names. Filter with `--kind function|method|struct|class|enum|trait|interface|module|type|constant|variable` when the user is clearly hunting for one shape.

2. Show the user the most relevant hits (file:line, kind, qualified name, signature). If there are dozens, group by file or summarise.

3. **Next step:** for any hit they want to read, use `/cg-snippet <file> <symbol>` — do **not** `Read` the whole file.

## When to use this instead of Grep / Read

- Locating a function, struct, class, enum, trait, or constant by name.
- Listing all signatures matching a pattern (e.g. `"fn handle_"`).
- Any time you would otherwise grep then open a file to confirm context.

If the query is for *occurrences* of a name (not its definition), prefer `/cg-refs` instead.
