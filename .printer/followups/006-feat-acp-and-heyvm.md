# Follow-ups for /home/sarocu/Projects/printer/specs/006-feat-acp-and-heyvm.md

Generated: 2026-05-02T22:01:24.236927878+00:00
Verdict: PASS

## Suggested follow-ups

- Implement ACP token-usage extraction so compaction-by-rotation works for long ACP sessions (called out in `HOOKS.md` and T-020 notes).
- Add a stub-ACP-server integration test exercising cancel/transport-error paths end-to-end (T-020 notes acknowledge this was deferred).
- Either flip the spec checkboxes to `[x]` or close the spec; the three items are functionally complete but the file still shows them unchecked.
- Consider scheduling a follow-up to remove the `_printerPermissionMode` advisory key once a real ACP permission-policy field is standardized upstream.
