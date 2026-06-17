#!/bin/bash
# Stop hook — commit current session to extract memories.
# Usage: copy to .claude/settings.json hooks.Stop
# Runs in background so Claude Code doesn't wait.

set -e

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"
SESSION="${RUNESTONE_SESSION:-default}"

# Run in background — don't block the agent
(
    runestone --owner "$OWNER" session --agent "$AGENT" commit --session "$SESSION" 2>/dev/null
) &
disown
