---
description: Search the codebase for a symbol or signature substring using `codegraph search`.
agent: codegraph
---

Run `codegraph --text search $ARGUMENTS` against the on-disk index. Append
`--limit 50` if `$ARGUMENTS` does not contain `--limit` and the query looks
broad. Use `--name` to restrict matching to symbol names; filter with
`--kind function|method|struct|class|enum|trait|interface|module|type|constant|variable`
when the user is hunting for one shape.

Show the most relevant hits as `file:line  Kind  qualified-name  signature`.
For any hit the user wants to inspect, follow up with `/cg-snippet`.
