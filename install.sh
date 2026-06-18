#!/bin/bash
# Install Runestone for Claude Code
# Usage: curl -fsSL https://raw.githubusercontent.com/lagudomeze/runestone/main/install.sh | bash

set -euo pipefail

REPO="lagudomeze/runestone"
INSTALL_DIR="${HOME}/.local/bin"
SKILL_DIR="${HOME}/.claude/skills/rune"

# Determine latest release tag
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
    echo "Error: could not find latest release"
    exit 1
fi

echo "Installing Runestone ${LATEST}..."

# Download and extract
TMP=$(mktemp -d)
curl -fsSL "https://github.com/${REPO}/releases/download/${LATEST}/runestone-${LATEST}.tar.gz" -o "${TMP}/runestone.tar.gz"
tar xzf "${TMP}/runestone.tar.gz" -C "${TMP}"

# Install binary
mkdir -p "${INSTALL_DIR}"
cp "${TMP}/runestone" "${INSTALL_DIR}/runestone"
chmod +x "${INSTALL_DIR}/runestone"

# Install skill
if [ -d "${TMP}/skill-rune" ]; then
    mkdir -p "${SKILL_DIR}"
    cp -r "${TMP}/skill-rune/"* "${SKILL_DIR}/"
fi

rm -rf "${TMP}"

echo ""
echo "Runestone ${LATEST} installed."
echo ""
echo "  Binary: ${INSTALL_DIR}/runestone"
echo "  Skill:  ${SKILL_DIR}"
echo ""
echo "Make sure ${INSTALL_DIR} is on your PATH."
echo "Then configure your environment:"
echo "  export RUNESTONE_OWNER=\"\$(whoami)\""
echo "  export RUNESTONE_REMOTE=\"git@github.com:you/memory.git\""
echo ""
echo "To use /rune in any project:"
echo "  cp -r ${SKILL_DIR} /your-project/.claude/skills/"
echo ""
echo "Or add globally via ~/.claude/settings.json skills configuration."
