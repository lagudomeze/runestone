# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run all tests
cargo test

# Run a single test
cargo test --lib session::tests::test_create_and_add_message

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --all

# End-to-end CLI smoke test
cargo run -- session create --owner alice --agent mybot --session s1
cargo run -- session add --owner alice --agent mybot --session s1 --role user --content "hello"
cargo run -- session commit --owner alice --agent mybot --session s1
cargo run -- session history --owner alice --agent mybot --session s1

# Build on CI (stable — do NOT use nightly)
cargo build --verbose && cargo test --verbose
```

## Architecture

**Runestone** is a personal AI memory system: CLI → session manager → git storage.

### Error handling (critical pattern)

All functions return `runestone::error::Result<T>` (alias for `exn::Result<T, RunestoneError>`). Foreign errors (`std::io::Error`, `git2::Error`, `serde_json::Error`) must be converted via `.into_exn()?` — the `?` operator alone won't work because `exn::Exn<RunestoneError>` needs explicit wrapping.

```rust
// Correct:
let content = std::fs::read_to_string(path).into_exn()?;

// Wrong — won't compile:
let content = std::fs::read_to_string(path)?;
```

The `IntoExn` trait is blanket-implemented for any `Result<T, E>` where `RunestoneError: From<E>`. New foreign error types need a `From` variant added to `RunestoneError` in `src/error.rs`.

### Data flow

1. CLI (`src/main.rs`) parses subcommands via `clap`, creates a `SessionManager`, dispatches to handler functions.
2. `SessionManager` (`src/session.rs`) owns a `data_dir: PathBuf`. Each call to `get_or_create` creates directories under `./data/{owner}/agents/{agent_id}/sessions/{session_id}/`.
3. `GitRepo` (`src/git_repo.rs`) wraps `git2::Repository`. Each owner gets an independent git repo at `./data/{owner}/`. `open_or_init` creates the repo if it doesn't exist.
4. Commit flow: lock → count lines in `messages.jsonl` → write new offset to `.commit_offset` → `git add` + `git commit`.

### Key design choices

- **git2 not gitoxide**: git2 is mature with a clean API for our simple needs (init, add, commit). gitoxide's API is still evolving and requires 3-5x more code for the same operations.
- **exn not eyre/anyhow**: `exn` provides structured error trees with automatic backtrace. Required `nightly` at v0.1, but v0.3+ works on stable.
- **Single binary**: `src/main.rs` is the only binary target. Library modules are re-exported through `src/lib.rs`.
- **`.into_exn()?` pattern exists because `exn::Result<T, E>` wraps errors in `Exn<E>`**, so `?` can't auto-convert from `std::io::Error` to `Exn<RunestoneError>` in one hop.

### What's implemented (Phase 1) vs stubbed

- **Implemented**: session create/add/commit/history, offset-based incremental commits, per-owner git repo init.
- **Stubs only**: `memory search`, `memory list`, `git sync`, `index rebuild` — these print "not yet implemented" and return `Ok(())`.
- **`MemoryChange` enum** is defined (`src/memory.rs`) but not yet populated during commit — `commit_session` returns `changes: vec![]`. Phase 2 will wire the LLM extractor to fill this.
