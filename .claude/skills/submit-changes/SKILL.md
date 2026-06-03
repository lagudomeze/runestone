---
name: submit-changes
description: |
  Submit code changes via feature branch + MR workflow.
  Use when the user asks to commit, push, submit code, create MR/PR, merge, or push changes.
  Enforces branch naming convention: feature/xxx, fix/xxx, refactor/xxx, docs/xxx, ci/xxx, chore/xxx.
  NEVER push directly to main or master.
  MUST create a branch, commit, push, and open a PR/MR.
---

# Submit Changes Workflow

## Core Rule

**NEVER push directly to `main` or `master`.** All changes go through a named branch and a Pull Request.

## Branch Naming Convention

Analyze the changes and pick the correct prefix:

| Prefix | When to use |
|--------|-------------|
| `feature/` | New functionality, new module, new command |
| `fix/` | Bug fixes, crash fixes, incorrect behavior |
| `refactor/` | Code restructuring, renaming, no behavior change |
| `docs/` | README, doc files, comments |
| `ci/` | GitHub Actions, CI config, toolchain changes |
| `chore/` | Deps update, .gitignore, cleanups |

Format: `{prefix}/{short-description}` with kebab-case. Example: `feature/llm-extractor`, `fix/commit-lock-race`.

## Workflow

### Step 1: Check prerequisites

```bash
git status          # Are there changes to commit?
git branch --show-current  # Am I already on a feature branch?
```

If no changes at all: report "Nothing to commit" and stop.

If already on main/master: proceed to Step 2 (create feature branch).

If already on a feature branch with uncommitted changes: proceed to Step 3 (pre-commit checks).

### Step 2: Determine branch name and create it

Read `git diff --stat` and `git diff --cached --stat` to understand what changed. Decide the prefix, write a short kebab-case description. **Confirm with the user**, then:

```bash
git checkout -b {branch-name}
```

### Step 3: Pre-commit checks — ALL must pass before committing

Run these checks in order. If any fails, fix the issue and re-run from the failed step. **Do NOT commit until all pass.**

```bash
# 1. Format
cargo fmt --all

# 2. Check (fast — catches type errors without full compilation)
cargo check

# 3. Build
cargo build

# 4. Test
cargo test

# 5. Lint
cargo clippy -- -D warnings
```

If `cargo fmt` changed any files, re-run `cargo check` and `cargo build`.

If `clippy` emits warnings, fix them before proceeding.

### Step 4: Stage and commit

```bash
git add {specific files}   # NEVER git add -A; add only relevant files
git commit -m "{type}: {description}"
```

Use conventional commit format. Keep subject under 72 chars.

### Step 5: Push the branch

```bash
git push -u runestone {branch-name}
```

Default remote is `runestone` (git@github.com:lagudomeze/runestone.git). Fall back to `origin` if `runestone` doesn't exist.

### Step 6: Create a Pull Request

If `gh` is available:
```bash
gh pr create --title "{type}: {description}" --body "$(cat <<'EOF'
## Summary
{1-2 bullet points}

## Test plan
- [x] cargo fmt --all
- [x] cargo check
- [x] cargo build
- [x] cargo test
- [x] cargo clippy -- -D warnings
EOF
)" --base main --head {branch-name}
```

If `gh` is NOT available:
- Print: "Open a PR at https://github.com/lagudomeze/runestone/pull/new/{branch-name}"
- Show the commit message and pre-commit check results as the PR body

### Step 7: Report

After PR creation, output:
- Branch name
- PR URL
- "Do NOT merge locally. Wait for review and merge on GitHub."

## Guardrails

- If the user says "push to main" or "push directly", **refuse**.
- **Never skip pre-commit checks**, even if the user asks.
- Re-run `git status` before the final push to confirm nothing is left behind.
- If the branch is behind `main`, remind the user to `git fetch runestone && git rebase runestone/main` before pushing.
