#!/bin/bash
# UserPromptSubmit hook — semantic recall before each prompt.
# Usage: copy to .claude/settings.json hooks.UserPromptSubmit
# Receives {prompt} as stdin or first argument

set -e

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
QUERY="${1:-$(cat)}"

if [ -z "$QUERY" ]; then
    exit 0
fi

echo
echo "<!-- Runestone recall -->"
runestone --owner "$OWNER" memory recall --query "$QUERY" --limit 3 2>/dev/null || true
echo "<!-- /Runestone recall -->"
echo
