#!/bin/bash
# UserPromptSubmit — add user message to session + semantic recall
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"

INPUT=$(cat)

SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
PROMPT=$(echo "$INPUT" | jq -r '.prompt // empty')
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')

if [ -z "$PROMPT" ]; then
    exit 0
fi

# Persist transcript path for stop hook
[ -n "$TRANSCRIPT_PATH" ] && echo "$TRANSCRIPT_PATH" > /tmp/runestone_transcript

# Add user message
runestone --owner "$OWNER" session --agent "$AGENT" add \
    --session "$SESSION_ID" --role user --content "$PROMPT" 2>/dev/null || true

# Semantic recall
echo
echo "<!-- Runestone recall -->"
runestone --owner "$OWNER" memory recall --query "$PROMPT" --limit 3 2>/dev/null || true
echo "<!-- /Runestone recall -->"
echo
