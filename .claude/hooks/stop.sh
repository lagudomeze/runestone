#!/bin/bash
# Stop hook — parse stdin JSON, capture new assistant messages, commit (background)
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"
LOG="/tmp/runestone_hook_err.log"

# Claude Code sends hook context as JSON via stdin
INPUT=$(cat)

SESSION_ID=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('session_id',''))" 2>/dev/null)
TRANSCRIPT=$(echo "$INPUT" | python3 -c "import json,sys; print(json.load(sys.stdin).get('transcript_path',''))" 2>/dev/null)

# Fallback: read from files written by UserPromptSubmit
[ -z "$SESSION_ID" ] && SESSION_ID=$(cat /tmp/runestone_session_file 2>/dev/null || echo "")
[ -z "$TRANSCRIPT" ] && TRANSCRIPT=$(cat /tmp/runestone_transcript 2>/dev/null || echo "")

if [ -z "$SESSION_ID" ] || [ -z "$TRANSCRIPT" ] || [ ! -f "$TRANSCRIPT" ]; then
    echo "[stop] missing session_id or transcript" >> "$LOG"
    exit 0
fi

OFFSET_FILE="/tmp/runestone_offset_${SESSION_ID}"
OFFSET=$(cat "$OFFSET_FILE" 2>/dev/null || echo "0")

# Extract new assistant messages since offset
RESULT=$(python3 -c "
import json, sys
seen = 0
captured = 0
content = ''
with open('$TRANSCRIPT') as f:
    for line in f:
        try:
            msg = json.loads(line)
            if msg.get('role') == 'assistant':
                seen += 1
                if seen > $OFFSET:
                    captured = seen
                    for part in msg.get('content', []):
                        if isinstance(part, dict) and part.get('type') == 'text':
                            content += part.get('text', '')
        except json.JSONDecodeError:
            continue
print(captured)
print(content, end='')
" 2>>"$LOG")

NEW_OFFSET=$(echo "$RESULT" | head -1)
ASSISTANT_CONTENT=$(echo "$RESULT" | tail -n +2)

if [ -n "$ASSISTANT_CONTENT" ] && [ -n "$NEW_OFFSET" ] && [ "$NEW_OFFSET" != "$OFFSET" ]; then
    echo "$NEW_OFFSET" > "$OFFSET_FILE"
    runestone --owner "$OWNER" session --agent "$AGENT" add \
        --session "$SESSION_ID" --role assistant --content "$ASSISTANT_CONTENT" 2>>"$LOG" || true

    (
        runestone --owner "$OWNER" session --agent "$AGENT" commit --session "$SESSION_ID" 2>>"$LOG" || true
        echo "[stop] commit done for $SESSION_ID ($NEW_OFFSET msgs)" >> "$LOG"
    ) &
    disown
fi
