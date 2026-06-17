#!/bin/bash
# SessionStart hook — inject recent context before each Claude Code session.
# Usage: copy to .claude/settings.json hooks.SessionStart

set -e

OWNER="${RUNESTONE_OWNER:-$(whoami)}"

echo
echo "<!-- Runestone session start -->"
runestone --owner "$OWNER" memory inject --recent 5 2>/dev/null || true
echo "<!-- /Runestone session start -->"
echo
