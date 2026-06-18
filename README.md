# Runestone

Personal AI memory system — cross-project, Git-backed, vendor-neutral.

Claude Code remembers things per-project. Runestone remembers things about **you**,
across all projects. Store preferences, patterns, lessons learned, and concepts in
a Git repository you control.

## Install

```bash
git clone git@github.com:lagudomeze/runestone.git
cd runestone
cargo install --path crates/runestone-cli
```

Requires Rust 1.96+. The binary is a single static executable.

## Quick start

```bash
# Configure
cp .envrc.example .envrc   # Add OPENAI_API_KEY, RUNESTONE_OWNER, RUNESTONE_REMOTE
direnv allow

# Store a preference
runestone --owner yubiao memory store \
  --kind preference --key "commit-style" --content "Use conventional commits (feat/fix/refactor)"

# Search memories
runestone --owner yubiao memory search --query "commit"

# Sync to GitHub
runestone --owner yubiao git sync --remote git@github.com:you/memory.git
```

## Claude Code integration — `/rune`

Drop the skill into any project:

```bash
cp -r skills/rune /your-project/.claude/skills/
```

Then use `/rune` in Claude Code:

```
/rune remember "git2 credential callback dead loop on empty remote"
/rune recall "git branch naming"
/rune list
/rune sync
/rune clean
```

Memories persist across all your projects — teach Claude once, it knows everywhere.

## Memory model

| Kind | Example | Path |
|------|---------|------|
| `profile` | "Backend developer, Rust primary" | `memory/profile.md` |
| `preference` | "kebab-case git branches" | `memory/preferences/git-branch-naming.md` |
| `entity` | "OpenViking context database" | `memory/entities/openviking.md` |
| `case` | "git2 hangs on empty remote fetch" | `memory/cases/git2-empty-remote.md` |

All stored as Markdown files in `./data/{owner}/memory/`. Git-versioned, human-readable, portable.

## Architecture

```
crates/
├── runestone/         # Library: Runestone, Agent, SessionManager, GitRepo, Extractor
└── runestone-cli/     # Binary: clap CLI

data/{owner}/
├── memory/            # Personal memories (preferences, entities, cases, events, profile)
└── agents/{agent}/    # Agent-scoped sessions (legacy, being phased out)
```

Key decisions: Rust 2024, git2 (not gitoxide), rig for LLM extraction, `tracing` for logging.

## Commands

```
runestone --owner <name> memory store   --kind <k> --key <k> --content "..."
runestone --owner <name> memory load    --kind <k> --key <k>
runestone --owner <name> memory list
runestone --owner <name> memory search  --query "..."
runestone --owner <name> memory clean   --dry-run

runestone --owner <name> git init   --remote <url>
runestone --owner <name> git sync   --remote <url>

runestone --owner <name> session --agent <a> create --session <s>
runestone --owner <name> session --agent <a> add --session <s> --role user --content "..."
runestone --owner <name> session --agent <a> commit --session <s>
runestone --owner <name> session --agent <a> history --session <s>
```

## License

MIT
