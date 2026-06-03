use clap::{Parser, Subcommand};
use runestone::error::Result;
use runestone::session::SessionManager;
use std::path::PathBuf;

/// Runestone — a personal AI memory system based on Rust + Git.
#[derive(Parser)]
#[command(name = "runestone", about, version)]
struct Cli {
    /// Data directory for storing owner repositories (default: ./data)
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage conversation sessions
    #[command(alias = "s")]
    Session {
        #[command(subcommand)]
        action: SessionCmd,
    },
    /// Search and list memories (stub)
    #[command(alias = "m")]
    Memory {
        #[command(subcommand)]
        action: MemoryCmd,
    },
    /// Git synchronization (stub)
    #[command(alias = "g")]
    Git {
        #[command(subcommand)]
        action: GitCmd,
    },
    /// Index management (stub)
    #[command(alias = "i")]
    Index {
        #[command(subcommand)]
        action: IndexCmd,
    },
}

#[derive(Subcommand)]
enum SessionCmd {
    /// Create a new session (idempotent)
    Create {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        session: String,
    },
    /// Append a message to a session
    Add {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        session: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        content: String,
    },
    /// Commit unprocessed messages
    Commit {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        session: String,
    },
    /// Print the full message history of a session
    History {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        session: String,
    },
}

#[derive(Subcommand)]
enum MemoryCmd {
    /// Semantic search over memories (not yet implemented)
    Search {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        query: String,
        #[arg(long, default_value = "global")]
        scope: String,
    },
    /// List memory entries (not yet implemented)
    List {
        #[arg(long)]
        owner: String,
        #[arg(long)]
        agent: Option<String>,
    },
}

#[derive(Subcommand)]
enum GitCmd {
    /// Sync owner repo with remote (not yet implemented)
    Sync {
        #[arg(long)]
        owner: String,
    },
}

#[derive(Subcommand)]
enum IndexCmd {
    /// Rebuild the embedding index (not yet implemented)
    Rebuild {
        #[arg(long)]
        owner: String,
    },
}

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

    match cli.command {
        Commands::Session { action } => handle_session(cli.data_dir, action).await,
        Commands::Memory { action } => handle_memory(action),
        Commands::Git { action } => handle_git(action),
        Commands::Index { action } => handle_index(action),
    }
}

async fn handle_session(data_dir: PathBuf, cmd: SessionCmd) -> Result<()> {
    let mgr = SessionManager::new(data_dir);

    match cmd {
        SessionCmd::Create {
            owner,
            agent,
            session,
        } => {
            let s = mgr.get_or_create(&owner, &agent, &session)?;
            println!("Session created: {}/{}/{}", s.owner, s.agent_id, s.session_id);
            println!("Path: {}", s.base_path.display());
        }
        SessionCmd::Add {
            owner,
            agent,
            session,
            role,
            content,
        } => {
            let s = mgr.get_or_create(&owner, &agent, &session)?;
            mgr.add_message(&s, role, content).await?;
            println!("Message appended to {}/{}/{}", owner, agent, session);
        }
        SessionCmd::Commit {
            owner,
            agent,
            session,
        } => {
            let mut s = mgr.get_or_create(&owner, &agent, &session)?;
            match mgr.commit_session(&mut s).await? {
                runestone::session::CommitResult::Committed {
                    messages_processed,
                    ..
                } => {
                    println!(
                        "Commit successful: {} messages processed, offset now {}",
                        messages_processed, s.offset
                    );
                }
                runestone::session::CommitResult::NoNewMessages => {
                    println!("No new messages to commit.");
                }
            }
        }
        SessionCmd::History {
            owner,
            agent,
            session,
        } => {
            let s = mgr.get_or_create(&owner, &agent, &session)?;
            let messages = mgr.read_full_history(&s)?;
            for msg in &messages {
                println!("[{}] {}: {}", msg.timestamp, msg.role, msg.content);
            }
            println!("--- {} messages total ---", messages.len());
        }
    }
    Ok(())
}

fn handle_memory(cmd: MemoryCmd) -> Result<()> {
    match cmd {
        MemoryCmd::Search { .. } => {
            println!("Memory search is not yet implemented.");
        }
        MemoryCmd::List { .. } => {
            println!("Memory list is not yet implemented.");
        }
    }
    Ok(())
}

fn handle_git(cmd: GitCmd) -> Result<()> {
    match cmd {
        GitCmd::Sync { .. } => {
            println!("Git sync is not yet implemented.");
        }
    }
    Ok(())
}

fn handle_index(cmd: IndexCmd) -> Result<()> {
    match cmd {
        IndexCmd::Rebuild { .. } => {
            println!("Index rebuild is not yet implemented.");
        }
    }
    Ok(())
}
