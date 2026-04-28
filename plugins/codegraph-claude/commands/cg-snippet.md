---
description: Print the source of a single symbol (or line range) from a file — instead of reading the whole file with Read.
argument-hint: "<file> <symbol> | <file> --lines start:end"
allowed-tools: Bash(codegraph:*)
---

# /cg-snippet

Pull source for: `$ARGUMENTS`

## What to do

1. Invoke one of:

   ```
   codegraph --text snippet $ARGUMENTS              # by symbol
   ```

   The argument list is `<file> <symbol>` (qualified `Foo::bar` or bare `bar`), or `<file> --lines <start>:<end>`.

2. Show the snippet to the user with its file/line header.

## When to use this instead of Read

- Pulling one function out of a large source file.
- Re-fetching the exact current bytes of a region you are about to patch (so the diff context lines match).

`Read` is only justified when:
- the file is not in a supported language (codegraph supports Rust, Python, JavaScript, TypeScript), or
- the user really does want to see the entire file end-to-end.

For source-file edits, follow up with `/cg-patch <file>` and the unified diff, not `Edit` or `Write`.
