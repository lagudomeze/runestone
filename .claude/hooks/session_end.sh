#!/bin/bash
# SessionEnd hook — final commit and CLAUDE.md update.
# Usage: copy to .claude/settings.json hooks.SessionEnd

set -e

OWNER="${RUNESTONE_OWNER:-$(whoami)}"
AGENT="${RUNESTONE_AGENT:-claude}"
SESSION="${RUNESTONE_SESSION:-default}"

# Final commit
runestone --owner "$OWNER" session --agent "$AGENT" commit --session "$SESSION" 2>/dev/null || true

# Update CLAUDE.md with recent context summary
CONTEXT=$(runestone --owner "$OWNER" memory inject --recent 3 2>/dev/null || true)

if [ -n "$CONTEXT" ] && [ -f CLAUDE.md ]; then
    # Replace or append Runestone context block in CLAUDE.md
    if grep -q "<!-- RUNESTONE:START -->" CLAUDE.md; then
        sed -i '/<!-- RUNESTONE:START -->/,/<!-- RUNESTONE:END -->/c\<!-- RUNESTONE:START -->\n'"$CONTEXT"'\n<!-- RUNESTONE:END -->' CLAUDE.md
    else
        echo -e "\n<!-- RUNESTONE:START -->\n$CONTEXT\n<!-- RUNESTONE:END -->" >> CLAUDE.md
    fi
fi
