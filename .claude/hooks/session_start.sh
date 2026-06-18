#!/bin/bash
# SessionStart — discover session ID, create Runestone session, inject context
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"

# Claude Code sends hook context as JSON via stdin
INPUT=$(cat 2>/dev/null || echo "")

SESSION_ID=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('session_id',''))" 2>/dev/null || echo "")

# Fallback: derive from PPID
if [ -z "$SESSION_ID" ] && [ -f "$HOME/.claude/sessions/$PPID.json" ]; then
    SESSION_ID=$(python3 -c "import json; print(json.load(open('$HOME/.claude/sessions/$PPID.json'))['sessionId'])" 2>/dev/null || echo "")
fi

# Last resort
[ -z "$SESSION_ID" ] && SESSION_ID="s$(date +%Y%m%d%H%M%S)-$RANDOM"

export RUNESTONE_SESSION="$SESSION_ID"
echo "$SESSION_ID" > /tmp/runestone_session_file

# Clean stale files
find /tmp -name 'runestone_session_*' -mtime +1 -delete 2>/dev/null || true
find /tmp -name 'runestone_offset_*' -mtime +1 -delete 2>/dev/null || true

# Create session (idempotent)
runestone --owner "$OWNER" session --agent "$AGENT" create --session "$SESSION_ID" 2>/dev/null || true

# Inject context
echo
echo "<!-- Runestone context -->"
runestone --owner "$OWNER" memory inject --recent 5 2>/dev/null || true
echo "<!-- /Runestone context -->"
echo
