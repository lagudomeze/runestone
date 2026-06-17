#!/bin/bash
# SessionEnd — final commit, git sync, clean up
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"
LOG="/tmp/runestone_hook_err.log"

# Claude Code sends hook context as JSON via stdin
INPUT=$(cat 2>/dev/null || echo "")

SESSION_ID=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('session_id',''))" 2>/dev/null || echo "")
[ -z "$SESSION_ID" ] && SESSION_ID=$(cat /tmp/runestone_session_file 2>/dev/null || echo "")

if [ -z "$SESSION_ID" ]; then
    echo "[session_end] no session id" >> "$LOG"
    exit 0
fi

# Final commit
runestone --owner "$OWNER" session --agent "$AGENT" commit --session "$SESSION_ID" 2>>"$LOG" || true

# Sync to remote if configured
if [ -n "${RUNESTONE_REMOTE:-}" ]; then
    runestone --owner "$OWNER" git sync --remote "$RUNESTONE_REMOTE" 2>>"$LOG" || true
    echo "[session_end] synced to $RUNESTONE_REMOTE" >> "$LOG"
fi

# Cleanup
rm -f /tmp/runestone_session_file /tmp/runestone_transcript /tmp/runestone_offset_* 2>/dev/null || true
