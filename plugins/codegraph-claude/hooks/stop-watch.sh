#!/usr/bin/env bash
# SessionEnd / Stop hook: SIGTERM the codegraph watch daemon for this project.
# Idempotent — missing pidfile or dead pid are no-ops.

set -euo pipefail

CWD="$PWD"
if command -v jq >/dev/null 2>&1; then
  if INPUT=$(timeout 1 cat 2>/dev/null) && [ -n "$INPUT" ]; then
    parsed=$(printf '%s' "$INPUT" | jq -r '.cwd // empty' 2>/dev/null || true)
    [ -n "$parsed" ] && [ -d "$parsed" ] && CWD="$parsed"
  fi
fi

PIDFILE="$CWD/.codegraph/watch.pid"
[ -f "$PIDFILE" ] || exit 0

pid=$(cat "$PIDFILE" 2>/dev/null || true)
rm -f "$PIDFILE"
[ -n "$pid" ] || exit 0

if kill -0 "$pid" 2>/dev/null; then
  kill "$pid" 2>/dev/null || true
  # Give it 2s to exit cleanly, then SIGKILL if it's still around.
  for _ in 1 2 3 4; do
    sleep 0.5
    kill -0 "$pid" 2>/dev/null || break
  done
  kill -0 "$pid" 2>/dev/null && kill -KILL "$pid" 2>/dev/null || true
  echo "[codegraph plugin] watch daemon stopped (pid $pid)" >&2
fi

exit 0
