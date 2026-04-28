#!/usr/bin/env bash
# SessionStart hook: launch (or re-attach to) a `codegraph watch` daemon for
# the project, so the index stays fresh while the agent works. Idempotent —
# if a daemon is already running for this cwd, do nothing.
#
# Reads:  JSON on stdin (uses .cwd if present, falls back to $PWD).
# Writes: nothing on stdout (silent on success).
# Exit:   always 0 — never block a session over a missing optional tool.

set -euo pipefail

# Best-effort cwd extraction. If jq is missing or stdin is empty, fall back.
CWD="$PWD"
if command -v jq >/dev/null 2>&1; then
  if INPUT=$(timeout 1 cat 2>/dev/null) && [ -n "$INPUT" ]; then
    parsed=$(printf '%s' "$INPUT" | jq -r '.cwd // empty' 2>/dev/null || true)
    [ -n "$parsed" ] && [ -d "$parsed" ] && CWD="$parsed"
  fi
fi

if ! command -v codegraph >/dev/null 2>&1; then
  # Soft-fail — surface a one-liner to the transcript via stderr but do not
  # block the session.
  echo "[codegraph plugin] codegraph not on PATH; skipping watch daemon" >&2
  exit 0
fi

PIDDIR="$CWD/.codegraph"
PIDFILE="$PIDDIR/watch.pid"
LOG="$PIDDIR/watch.log"
mkdir -p "$PIDDIR"

# Already-running check.
if [ -f "$PIDFILE" ]; then
  prev=$(cat "$PIDFILE" 2>/dev/null || true)
  if [ -n "$prev" ] && kill -0 "$prev" 2>/dev/null; then
    echo "[codegraph plugin] watch daemon already running (pid $prev)" >&2
    exit 0
  fi
  rm -f "$PIDFILE"
fi

# Detach: setsid + nohup so the daemon outlives the hook process.
( setsid nohup codegraph watch "$CWD" >>"$LOG" 2>&1 < /dev/null & echo $! >"$PIDFILE" ) >/dev/null 2>&1
disown 2>/dev/null || true

new=$(cat "$PIDFILE" 2>/dev/null || true)
if [ -n "$new" ]; then
  echo "[codegraph plugin] watch daemon started (pid $new); logs → $LOG" >&2
fi

exit 0
