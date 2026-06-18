#!/bin/bash
# SessionStart — discover session ID, create Runestone session, inject context
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"

# Claude Code sends hook context as JSON via stdin
INPUT=$(cat 2>/dev/null || echo "")
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty' 2>/dev/null || echo "")

# Fallback: derive from PPID
if [ -z "$SESSION_ID" ] && [ -f "$HOME/.claude/sessions/$PPID.json" ]; then
    SESSION_ID=$(jq -r '.sessionId // empty' "$HOME/.claude/sessions/$PPID.json" 2>/dev/null || echo "")
fi

# Last resort
[ -z "$SESSION_ID" ] && SESSION_ID="s$(date +%Y%m%d%H%M%S)-$RANDOM"

# Create session (idempotent)
runestone --owner "$OWNER" session --agent "$AGENT" create --session "$SESSION_ID" 2>/dev/null || true

# Inject context
echo
echo "<!-- Runestone context -->"
runestone --owner "$OWNER" memory inject --recent 5 2>/dev/null || true
echo "<!-- /Runestone context -->"
echo
