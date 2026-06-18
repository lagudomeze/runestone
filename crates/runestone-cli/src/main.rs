#![deny(unused_crate_dependencies)]

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use runestone::{
    Case, Entity, Event, Preference, Profile, Result, Runestone, RunestoneError,
    extractor::Extractor,
};

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
    /// Semantic recall — for Claude Code hooks (outputs <runestone-recall>)
    Recall {
        #[arg(long)]
        query: String,
        #[arg(long, default_value = "5")]
        limit: usize,
    },
    /// Context injection — for Claude Code SessionStart hook (outputs
    /// <runestone-context>)
    Inject {
        #[arg(long, default_value = "5")]
        recent: usize,
    },
    List {
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
    /// Clone a remote memory repository into the data directory
    Init {
        #[arg(long)]
        remote: String,
    },
    /// Commit pending changes, pull rebase, and push
    Sync {
        #[arg(long)]
        remote: String,
    },
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

    let ext = runestone::extractor::from_env().ok_or_else(|| {
        RunestoneError::Other(
            "OPENAI_API_KEY is not set. Please configure your API key.\nCopy .envrc.example to \
             .envrc, fill in your key, and run: direnv allow"
                .into(),
        )
    })?;

    let rs = Runestone::new(cli.data_dir, cli.owner, ext);
    dispatch(cli.command, &rs).await
}

async fn dispatch<E: Extractor + Clone>(cmd: Commands, rs: &Runestone<E>) -> Result<()> {
    match cmd {
        Commands::Session { agent, action } => {
            let a = rs.agent(&agent);
            handle_session(&a, action).await
        }
        Commands::Memory { action } => handle_memory(rs, action).await,
        Commands::Git { action } => handle_git(rs, action),
        Commands::Index { .. } => handle_index(rs).await,
    }
}

// ── Session
// ───────────────────────────────────────────────────────────────────

async fn handle_session<E: Extractor>(agent: &runestone::Agent<E>, cmd: SessionCmd) -> Result<()> {
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
                runestone::CommitResult::Committed { messages_processed, changes } => {
                    println!(
                        "Commit successful: {} messages processed, {} changes extracted, offset \
                         now {}",
                        messages_processed,
                        changes.len(),
                        s.offset()
                    );
                    for c in &changes {
                        println!("  → {c:?}");
                    }
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

async fn handle_memory<E: Extractor + Clone>(rs: &Runestone<E>, cmd: MemoryCmd) -> Result<()> {
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
        MemoryCmd::Recall { query, limit } => {
            let hits = rs.memory_search_deep(&query, limit).await.unwrap_or_default();
            println!("<runestone-recall>");
            if hits.is_empty() {
                println!("(no relevant memories found)");
            } else {
                for hit in &hits {
                    println!("## {}\n{}\n", hit.path, hit.snippet);
                }
            }
            println!("</runestone-recall>");
        }
        MemoryCmd::Inject { recent } => {
            println!("<runestone-context>");
            if let Ok(files) = rs.memory_list() {
                let abstracts: Vec<&String> =
                    files.iter().filter(|f| f.contains(".abstract.md")).collect();
                for f in abstracts.iter().take(recent) {
                    // memory_list returns paths relative to ./data/
                    let path = std::path::PathBuf::from("./data").join(f);
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let short =
                            f.replace("/.abstract.md", "").replace(&format!("{}/", rs.owner()), "");
                        println!("- **{short}**: {}", content.trim());
                    }
                }
                if abstracts.is_empty() {
                    println!("(no context yet)");
                }
            }
            println!("</runestone-context>");
        }
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
        MemoryCmd::Load { kind, key, agent } => match dispatch_load(rs, &kind, key, agent)? {
            Some(v) => println!("{v}"),
            None => println!("(not found)"),
        },
    }
    Ok(())
}

fn dispatch_store<E: Extractor>(
    rs: &Runestone<E>,
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

fn dispatch_load<E: Extractor>(
    rs: &Runestone<E>,
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

fn handle_git<E: Extractor>(rs: &Runestone<E>, cmd: GitCmd) -> Result<()> {
    match cmd {
        GitCmd::Init { remote } => {
            rs.git_init(&remote)?;
            println!("Initialized from {remote}");
        }
        GitCmd::Sync { remote } => {
            rs.sync(&remote)?;
            println!("Synced with {remote}");
        }
    }
    Ok(())
}

async fn handle_index(rs: &Runestone<impl Extractor>) -> Result<()> {
    rs.index_rebuild().await?;
    println!("Index rebuilt successfully.");
    Ok(())
}
