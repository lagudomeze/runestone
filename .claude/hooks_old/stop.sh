#!/bin/bash
# Stop hook — capture last assistant message + tool calls/results, commit (background)
set -euo pipefail
export PATH="$HOME/.cargo/bin:$PATH"

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"
LOG="/tmp/runestone_hook_err.log"

INPUT=$(cat)
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id // empty')
TRANSCRIPT=$(echo "$INPUT" | jq -r '.transcript_path // empty')

if [ -z "$SESSION_ID" ] || [ -z "$TRANSCRIPT" ] || [ ! -f "$TRANSCRIPT" ]; then
    echo "[stop] missing session_id or transcript" >> "$LOG"
    exit 0
fi

# Step 1: last assistant message ID (chronological — last in file)
LAST_ID=$(jq -s -r '
  [ .[] | select(.type == "assistant" and .message.id != null) ] | last | .message.id
' "$TRANSCRIPT" 2>/dev/null)

if [ -z "$LAST_ID" ] || [ "$LAST_ID" = "null" ]; then
    exit 0
fi

# Dedup by message UUID
LAST_FILE="/tmp/runestone_last_${SESSION_ID}"
PREV_ID=$(cat "$LAST_FILE" 2>/dev/null || echo "")

if [ "$LAST_ID" = "$PREV_ID" ]; then
    exit 0
fi
echo "$LAST_ID" > "$LAST_FILE"

# Step 2: collect all content for this message ID
# Lines with type=text → assistant text response
# Lines with type=tool_use → [tool: name(args)]
# Lines with type=tool_result → [tool result: ...]
CONTENT=$(jq -s -r --arg id "$LAST_ID" '
  def msgs: [ .[] | select(.type == "assistant" and .message.id == $id) ];
  ( [ msgs[] | .message.content[]? | select(.type == "text") | .text ] | add ) as $text
  | ( [ msgs[] | .message.content[]? | select(.type == "tool_use")
        | "[tool: \(.name)(\(.input | tojson | .[:200]))]" ] | join("\n") ) as $tools
  | ( [ .[] | select(.type == "user")
        | .message.content[]? | select(.type == "tool_result")
        | "[tool result: \(.content | if type == "array" then .[0].text? // .[]? | strings else . end | .[:300])]" ]
      | join("\n") ) as $results
  | [ $text, $tools, $results ] | map(select(length > 0)) | join("\n\n")
' "$TRANSCRIPT" 2>>"$LOG")

if [ -z "$CONTENT" ]; then
    exit 0
fi

# Add to session
runestone --owner "$OWNER" session --agent "$AGENT" add \
    --session "$SESSION_ID" --role assistant --content "$CONTENT" 2>>"$LOG" || true

# Commit in background
(
    runestone --owner "$OWNER" session --agent "$AGENT" commit --session "$SESSION_ID" 2>>"$LOG" || true
    echo "[stop] commit done for $SESSION_ID ($LAST_ID)" >> "$LOG"
) &
disown
