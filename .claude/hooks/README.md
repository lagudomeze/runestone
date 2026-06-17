# Runestone Claude Code Hooks

Copy these scripts to your project's `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "",
        "command": "\"${CLAUDE_PROJECT_DIR}/.claude/hooks/session_start.sh\""
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "",
        "command": "\"${CLAUDE_PROJECT_DIR}/.claude/hooks/user_prompt_submit.sh\" \"${prompt}\""
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "command": "\"${CLAUDE_PROJECT_DIR}/.claude/hooks/stop.sh\""
      }
    ],
    "SessionEnd": [
      {
        "matcher": "",
        "command": "\"${CLAUDE_PROJECT_DIR}/.claude/hooks/session_end.sh\""
      }
    ]
  }
}
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUNESTONE_OWNER` | `whoami` | Your user identity |
| `RUNESTONE_AGENT` | `claude` | Agent name (must match session agent) |
| `RUNESTONE_SESSION` | `default` | Session ID |

Also ensure `OPENAI_API_KEY` is set in `.envrc` for LLM extraction.
