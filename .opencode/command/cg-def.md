---
description: Jump to a symbol's definition via the codegraph index.
agent: codegraph
---

Run `codegraph --text definition $ARGUMENTS`. The argument can be a bare
name or qualified (`Server::handle_request`).

Present each hit as `file:line  Kind  qualified-name  signature`. To read
the body, follow up with `/cg-snippet <file> <symbol>`.
