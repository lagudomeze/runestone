#![deny(unused_crate_dependencies)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use runestone::{Case, Entity, Event, Preference, Profile, Result, Runestone, RunestoneError};

/// Runestone — a personal AI memory system based on Rust + Git.
#[derive(Parser)]
#[command(name = "runestone", about, version)]
struct Cli {
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,

    #[arg(long, short = 'o')]
    owner: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "s")]
    Session {
        #[arg(long)]
        agent: String,
        #[command(subcommand)]
        action: SessionCmd,
    },
    #[command(alias = "m")]
    Memory {
        #[command(subcommand)]
        action: MemoryCmd,
    },
    #[command(alias = "g")]
    Git {
        #[command(subcommand)]
        action: GitCmd,
    },
    #[command(alias = "i")]
    Index {
        #[command(subcommand)]
        action: IndexCmd,
    },
}

#[derive(Subcommand)]
enum SessionCmd {
    Create {
        #[arg(long)]
        session: String,
    },
    Add {
        #[arg(long)]
        session: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        content: String,
    },
    Commit {
        #[arg(long)]
        session: String,
    },
    History {
        #[arg(long)]
        session: String,
    },
}

#[derive(Subcommand)]
enum MemoryCmd {
    Search {
        #[arg(long)]
        query: String,
        #[arg(long, default_value = "5")]
        limit: usize,
    },
    /// List all memory files
    List {
        /// Optional: restrict to a specific agent
        #[arg(long)]
        agent: Option<String>,
    },
    Store {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        content: String,
    },
    Load {
        #[arg(long)]
        kind: String,
        #[arg(long)]
        key: Option<String>,
        #[arg(long)]
        agent: Option<String>,
    },
}

#[derive(Subcommand)]
enum GitCmd {
    Sync {},
}

#[derive(Subcommand)]
enum IndexCmd {
    Rebuild {},
}

// ── Entry ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = run().await {
        eprintln!("Error: {e:#?}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let rs = Runestone::new(cli.data_dir, cli.owner);

    match cli.command {
        Commands::Session { agent, action } => {
            let a = rs.agent(&agent);
            handle_session(&a, action).await
        }
        Commands::Memory { action } => handle_memory(&rs, action),
        Commands::Git { .. } => handle_git(),
        Commands::Index { .. } => handle_index(),
    }
}

// ── Session
// ───────────────────────────────────────────────────────────────────

async fn handle_session(agent: &runestone::Agent, cmd: SessionCmd) -> Result<()> {
    match cmd {
        SessionCmd::Create { session } => {
            let s = agent.session_open(&session)?;
            println!("Session created: {}/{}/{}", agent.owner(), agent.id(), s.session_id);
            println!("Path: {}", s.base_path.display());
        }
        SessionCmd::Add { session, role, content } => {
            let s = agent.session_open(&session)?;
            agent.session_add(&s, &role, &content).await?;
            println!("Message appended to {}/{}/{}", agent.owner(), agent.id(), session);
        }
        SessionCmd::Commit { session } => {
            let s = agent.session_open(&session)?;
            match agent.session_commit(&s).await? {
                runestone::CommitResult::Committed { messages_processed, .. } => {
                    println!(
                        "Commit successful: {} messages processed, offset now {}",
                        messages_processed,
                        s.offset()
                    );
                }
                runestone::CommitResult::NoNewMessages => {
                    println!("No new messages to commit.");
                }
            }
        }
        SessionCmd::History { session } => {
            let s = agent.session_open(&session)?;
            let messages = agent.session_history(&s)?;
            for msg in &messages {
                println!("[{}] {}: {}", msg.timestamp, msg.role, msg.content);
            }
            println!("--- {} messages total ---", messages.len());
        }
    }
    Ok(())
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn handle_memory(rs: &Runestone, cmd: MemoryCmd) -> Result<()> {
    match cmd {
        MemoryCmd::Search { query, limit } => match rs.memory_search(&query, limit) {
            Ok(hits) => {
                for hit in &hits {
                    println!("[{:.2}] {} — {}", hit.score, hit.path, hit.snippet);
                }
                if hits.is_empty() {
                    println!("No results.");
                }
            }
            Err(e) => println!("{e}"),
        },
        MemoryCmd::List { agent } => {
            let files =
                if let Some(id) = agent { rs.agent(&id).memory_list()? } else { rs.memory_list()? };
            if files.is_empty() {
                println!("No memory files found.");
            } else {
                for f in &files {
                    println!("{f}");
                }
            }
        }
        MemoryCmd::Store { kind, key, agent, content } => {
            dispatch_store(rs, &kind, key, agent, &content)?;
            println!("Stored.");
        }
        MemoryCmd::Load { kind, key, agent } => {
            let val = dispatch_load(rs, &kind, key, agent)?;
            match val {
                Some(v) => println!("{v}"),
                None => println!("(not found)"),
            }
        }
    }
    Ok(())
}

// ── Dispatch helpers
// ──────────────────────────────────────────────────────────

fn dispatch_store(
    rs: &Runestone,
    kind: &str,
    key: Option<String>,
    agent: Option<String>,
    content: &str,
) -> Result<()> {
    let v = content.to_string();
    match kind {
        "profile" => rs.memory_store(&Profile, &v),
        "preference" => {
            let k = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_store(&Preference { key: k }, &v)
        }
        "entity" => {
            let n = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_store(&Entity { name: n }, &v)
        }
        "event" => {
            let t = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_store(&Event { title: t }, &v)
        }
        "case" => {
            let a = agent.ok_or_else(|| RunestoneError::Other("--agent required".into()))?;
            let t = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_store(&Case { agent: a, title: t }, &v)
        }
        _ => Err(RunestoneError::Other(format!(
            "unknown kind '{kind}'. Valid: profile, preference, entity, event, case"
        ))
        .into()),
    }
}

fn dispatch_load(
    rs: &Runestone,
    kind: &str,
    key: Option<String>,
    agent: Option<String>,
) -> Result<Option<String>> {
    match kind {
        "profile" => rs.memory_load(&Profile),
        "preference" => {
            let k = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_load(&Preference { key: k })
        }
        "entity" => {
            let n = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_load(&Entity { name: n })
        }
        "event" => {
            let t = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_load(&Event { title: t })
        }
        "case" => {
            let a = agent.ok_or_else(|| RunestoneError::Other("--agent required".into()))?;
            let t = key.ok_or_else(|| RunestoneError::Other("--key required".into()))?;
            rs.memory_load(&Case { agent: a, title: t })
        }
        _ => Err(RunestoneError::Other(format!(
            "unknown kind '{kind}'. Valid: profile, preference, entity, event, case"
        ))
        .into()),
    }
}

// ── Stubs ─────────────────────────────────────────────────────────────────────

fn handle_git() -> Result<()> {
    println!("Git sync is not yet implemented.");
    Ok(())
}

fn handle_index() -> Result<()> {
    println!("Index rebuild is not yet implemented.");
    Ok(())
}
