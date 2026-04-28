---
description: List symbols (functions, classes, structs, …) in a single file.
agent: codegraph
---

Run `codegraph --text symbols $ARGUMENTS`. Show the table to the user;
columns are `Kind`, qualified name, line range, signature.

Use `/cg-symbols` when you want the flat list. Use `/cg-outline` instead
when the file has nested structures (impls, classes with methods) and the
parent–child relationships matter.
