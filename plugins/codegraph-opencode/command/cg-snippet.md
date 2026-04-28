---
description: Print one symbol's source (or a line range) instead of reading the whole file.
agent: codegraph
---

Run `codegraph --text snippet $ARGUMENTS`. Argument shape is
`<file> <symbol>` (qualified `Foo::bar` or bare `bar`) or
`<file> --lines <start>:<end>`.

Show the snippet with its file/line header. Re-run before building a patch
so the diff context lines match the current bytes exactly.
