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
cargo run -- --owner alice session create --agent mybot --session s1
cargo run -- --owner alice session add --agent mybot --session s1 --role user --content "hello"
cargo run -- --owner alice session commit --agent mybot --session s1
cargo run -- --owner alice session history --agent mybot --session s1

# Memory operations
cargo run -- --owner alice memory store --kind preference --key lang --content "Rust"
cargo run -- --owner alice memory load --kind preference --key lang
cargo run -- --owner alice memory list --scope all

# Build on CI (stable — do NOT use nightly)
cargo build --verbose && cargo test --verbose
```

## Architecture

**Runestone** is a personal AI memory system: CLI → session manager → git storage.

### Project structure

```
src/
├── lib.rs           # Public API — Runestone facade, re-exports
├── error.rs         # RunestoneError + IntoExn trait + Result alias
├── git_repo.rs      # GitRepo (internal, not exposed publicly)
├── session.rs       # SessionManager (internal), Session/Message/CommitResult (public)
├── memory.rs        # MemoryChange enum
└── bin/
    └── runestone.rs # CLI binary
```

- **lib** provides a single entry point: `Runestone::new(data_dir)`
- **bin** is a thin CLI wrapper around the lib
- `SessionManager` and `GitRepo` are `pub(crate)` — internal, not part of the public API

### Public API

```rust
use runestone::Runestone;

let rs = Runestone::new("./data");
let s = rs.session_open("alice", "mybot", "s1")?;       // create or open
rs.session_add(&s, "user", "hello").await?;              // append message
let result = rs.session_commit(&s).await?;               // commit offset
let history = rs.session_history(&s)?;                    // read all messages
```

`session_commit` takes `&Session` (not `&mut`) because `offset` is stored in a `Cell<usize>`.

### Error handling (critical pattern)

All functions return `runestone::Result<T>` (alias for `exn::Result<T, RunestoneError>`). Foreign errors (`std::io::Error`, `git2::Error`, `serde_json::Error`) must be converted via `.into_exn()?` — the `?` operator alone won't work because `exn::Exn<RunestoneError>` needs explicit wrapping.

```rust
// Correct:
let content = std::fs::read_to_string(path).into_exn()?;

// Wrong — won't compile:
let content = std::fs::read_to_string(path)?;
```

The `IntoExn` trait is blanket-implemented for any `Result<T, E>` where `RunestoneError: From<E>`. New foreign error types need a `From` variant added to `RunestoneError` in `src/error.rs`.

### Data flow

1. CLI (`src/bin/runestone.rs`) parses subcommands via `clap`, creates a `Runestone`, dispatches to handler functions.
2. `Runestone` (`src/lib.rs`) delegates to `SessionManager` which handles directory creation under `./data/{owner}/agents/{agent_id}/sessions/{session_id}/`.
3. `GitRepo` (`src/git_repo.rs`) wraps `git2::Repository`. Each owner gets an independent git repo at `./data/{owner}/`. `open_or_init` creates the repo if it doesn't exist.
4. Commit flow: lock → count lines in `messages.jsonl` → write new offset to `.commit_offset` → `git add` + `git commit`.

### Key design choices

- **git2 not gitoxide**: git2 is mature with a clean API for our simple needs (init, add, commit). gitoxide's API is still evolving and requires 3-5x more code for the same operations.
- **exn not eyre/anyhow**: `exn` provides structured error trees with automatic backtrace. Required `nightly` at v0.1, but v0.3+ works on stable.
- **lib + bin split**: `src/lib.rs` is the library entry point with a clean `Runestone` facade. `src/bin/runestone.rs` is the CLI binary. Internal modules are `pub(crate)`.
- **`.into_exn()?` pattern exists because `exn::Result<T, E>` wraps errors in `Exn<E>`**, so `?` can't auto-convert from `std::io::Error` to `Exn<RunestoneError>` in one hop.

### What's implemented (Phase 1) vs stubbed

- **Implemented**: session create/add/commit/history, offset-based incremental commits, per-owner git repo init.
- **Stubs only**: `memory search`, `memory list`, `git sync`, `index rebuild` — these print "not yet implemented" and return `Ok(())`.
- **`MemoryChange` enum** is defined (`src/memory.rs`) but not yet populated during commit — `commit_session` returns `changes: vec![]`. Phase 2 will wire the LLM extractor to fill this.
