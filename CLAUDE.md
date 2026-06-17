# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Workflow rule

Before starting any new development, **always**:
1. Review the current state of the codebase and `doc/ROADMAP.md`
2. Update `doc/ROADMAP.md` to reflect actual progress
3. Present a summary: what's done, what's next, and the recommended next step

Do NOT start coding until the user confirms the direction.

## Commands

```bash
# Build (workspace root)
cargo build

# Run all tests
cargo test

# Run a single test
cargo test -p runestone --lib session::tests::test_create_and_add_message

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt --all

# End-to-end CLI smoke test
cargo run -- --owner alice session --agent mybot create --session s1
cargo run -- --owner alice session --agent mybot add --session s1 --role user --content "hello"
cargo run -- --owner alice session --agent mybot commit --session s1
cargo run -- --owner alice session --agent mybot history --session s1

# Memory operations
cargo run -- --owner alice memory store --kind preference --key lang --content "Rust"
cargo run -- --owner alice memory load --kind preference --key lang
cargo run -- --owner alice memory list

# Build on CI (stable)
cargo build --verbose && cargo test --verbose
```

## Architecture

**Runestone** is a personal AI memory system: CLI → Agent → session manager → git storage.

### Project structure (workspace)

```
crates/
├── runestone/           # Library crate (name = "runestone")
│   └── src/
│       ├── lib.rs       # #![deny(unused_crate_dependencies)], Runestone + Agent facade
│       ├── error.rs     # RunestoneError + IntoExn + Result alias
│       ├── git_repo.rs  # GitRepo (pub(crate) — internal)
│       ├── session.rs   # SessionManager (pub(crate)), Session/Message/CommitResult (pub)
│       ├── extractor.rs # Extractor trait + RigExtractor + from_model()
│       └── memory/      # MemoryKind trait + per-kind files
└── runestone-cli/       # Binary crate (name = "runestone-cli")
    └── src/main.rs      # #![deny(unused_crate_dependencies)], CLI binary
```

### Dependency management

All versions and internal paths are centralized in root `Cargo.toml` under `[workspace.dependencies]`. Each crate uses `.workspace = true` to reference them. Use `cargo add -p <crate> <dep>` to add new deps.

### Public API

```rust
use runestone::Runestone;

let rs = Runestone::new("./data", "alice");
let agent = rs.agent("mybot");

// Sessions are scoped to the agent
let s = agent.session_open("s1")?;
agent.session_add(&s, "user", "hello").await?;
let result = agent.session_commit(&s).await?;   // Phase 2: LLM extraction

// Global memory on Runestone; agent memory on Agent
rs.memory_store(&Profile, &"Alice".to_string())?;
agent.memory_store(&Preference { key: "lang".into() }, &"Rust".to_string())?;
```

### Error handling (critical pattern)

All functions return `runestone::Result<T>` (alias for `exn::Result<T, RunestoneError>`). Foreign errors must be converted via `.into_exn()?` — bare `?` won't compile.

```rust
// Correct:
let content = std::fs::read_to_string(path).into_exn()?;

// Wrong:
let content = std::fs::read_to_string(path)?;
```

### Data flow

1. CLI parses subcommands via `clap`, creates a `Runestone`, dispatches.
2. `Runestone` holds owner + data_dir. `Agent` holds owner + agent_id + cloned `SessionManager`.
3. `SessionManager` handles `./data/{owner}/agents/{agent_id}/sessions/{session_id}/`.
4. `GitRepo` wraps `git2::Repository` — one per owner at `./data/{owner}/`.
5. Commit flow: lock → count lines → read new messages → LLM extract (if configured) → update offset → `git add` + `git commit`.

### Key design choices

- **git2 not gitoxide**: mature, stable API. gitoxide requires 3-5x more code for same operations.
- **exn not eyre/anyhow**: structured error trees with automatic backtrace. v0.3+ works on stable.
- **Workspace**: `crates/runestone` (lib) + `crates/runestone-cli` (bin). Dep versions centralized in root.
- **`MemoryKind` trait with associated `Value` type**: each kind (Profile, Preference, etc.) knows its path and value shape. Currently all `Value = String`; can switch to structs later.
- **`Agent` is cheap to clone**: `SessionManager` uses `Arc<Mutex<()>>` for shared locking + `Option<Arc<dyn Extractor>>` for optional LLM extraction.
- **rig for LLM**: `Extractor` trait + `RigExtractor<M>` + `from_model()` factory. Any `CompletionModel` can be type-erased into `Arc<dyn Extractor>`.

### What's implemented vs stubbed

- **Phase 1 (done)**: session CRUD, offset-based commit, per-owner git, Agent abstraction, MemoryKind trait, memory_store/load/list, lib+bin split.
- **Phase 2 (done)**: `Extractor` trait, rig-backed `RigExtractor`, `session_commit` wired to extraction. `resource_add` is stubbed.
- **Stubs**: `memory_search`, `git sync`, `index rebuild` — print "not yet implemented" and return `Ok(())`.
