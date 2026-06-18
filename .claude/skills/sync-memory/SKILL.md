---
name: sync-memory
description: |
  Sync runestone memory data with a remote git repository.
  Use when the user asks to sync memories, backup, push memory data, or pull from remote.
---

# Sync Memory Data

Sync the owner's memory repository with a remote.

## Prerequisites

Set `RUNESTONE_REMOTE` in `.envrc`:

```bash
export RUNESTONE_REMOTE="git@github.com:user/runestone-memory.git"
```

## Manual sync

```bash
runestone --owner "$RUNESTONE_OWNER" git sync --remote "$RUNESTONE_REMOTE"
```

## Automated sync (SessionEnd hook)

The `session_end.sh` hook automatically syncs after each session.

## First-time setup

1. Create a private repo on GitHub
2. Add `RUNESTONE_REMOTE` to `.envrc`
3. Run manual sync once to push initial data
