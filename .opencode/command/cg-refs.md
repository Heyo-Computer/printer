---
description: Find lexical references to a name across the indexed files.
agent: codegraph
---

Run `codegraph --text references $ARGUMENTS`. Group hits by file and call
out which lines look like definitions vs call sites.

Caveat: this is a word-boundary scan over indexed files — comments and
strings can match, dynamic dispatch will be missed. For the canonical
definition, use `/cg-def`.
