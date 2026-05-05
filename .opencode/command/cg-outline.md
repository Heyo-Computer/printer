---
description: Show a hierarchical outline of a file (signatures only, no bodies).
agent: codegraph
---

Run `codegraph --text outline $ARGUMENTS`. Present the result so the user
sees the file's shape (kind + qualified name + line range) without reading
bodies. If they want one symbol's source, follow up with
`/cg-snippet $ARGUMENTS <symbol>`.
