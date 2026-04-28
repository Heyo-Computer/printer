---
description: Find lexical references to a name across the indexed files. Replaces `grep -rn` for "where is this called?" questions.
argument-hint: "<symbol-name>"
allowed-tools: Bash(codegraph:*)
---

# /cg-refs

Find references to: `$ARGUMENTS`

## What to do

1. Run:

   ```
   codegraph --text references $ARGUMENTS
   ```

2. Group hits by file and present them. Note which look like definitions vs call sites by inspecting the surrounding line text.

3. **Next steps:**
   - To jump to the definition, use `/cg-def $ARGUMENTS`.
   - To inspect one of the hit sites, use `/cg-snippet <file> <symbol-or-lines>`.

## Caveats

`codegraph references` is a lexical word-boundary scan over indexed files — it can include matches inside comments and strings, and it will miss dynamic dispatch (e.g. method calls resolved at runtime). For a structural call-graph you still need to read the language-aware analyser.
