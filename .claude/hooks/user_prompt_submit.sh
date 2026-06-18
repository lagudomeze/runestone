#!/bin/bash
# UserPromptSubmit — parse stdin JSON, add user message to session, run semantic recall
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"

# Claude Code sends hook context as JSON via stdin
INPUT=$(cat)

SESSION_ID=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('session_id',''))" 2>/dev/null)
PROMPT=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('prompt',''))" 2>/dev/null)
TRANSCRIPT=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('transcript_path',''))" 2>/dev/null)

# Persist for other hooks
[ -n "$SESSION_ID" ] && echo "$SESSION_ID" > /tmp/runestone_session_file
[ -n "$TRANSCRIPT" ] && echo "$TRANSCRIPT" > /tmp/runestone_transcript

if [ -z "$PROMPT" ]; then
    exit 0
fi

# Add user message to session
runestone --owner "$OWNER" session --agent "$AGENT" add \
    --session "$SESSION_ID" --role user --content "$PROMPT" 2>/dev/null || true

# Semantic recall
echo
echo "<!-- Runestone recall -->"
runestone --owner "$OWNER" memory recall --query "$PROMPT" --limit 3 2>/dev/null || true
echo "<!-- /Runestone recall -->"
echo
