//! Integration tests that require an OpenAI-compatible API key.
//! Set `OPENAI_API_KEY` to run. Skipped otherwise.
//!
//! For custom endpoints (DeepSeek, local Ollama, etc.), also set:
//! - `OPENAI_API_BASE` — e.g. `https://api.deepseek.com/v1`
//! - `RUNESTONE_MODEL` — e.g. `deepseek-chat`

use std::path::PathBuf;

use runestone::{
    Message, Runestone,
    extractor::{self, Extractor, FileEntry},
};

fn check_env() -> Option<String> {
    let key = std::env::var("OPENAI_API_KEY").ok()?;
    let model = std::env::var("RUNESTONE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let base = std::env::var("OPENAI_API_BASE")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    eprintln!("Using model={model} base={base}");
    assert!(!key.is_empty());
    Some(model)
}

fn setup(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("runestone_ext_int_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn unwrap<T>(r: runestone::Result<T>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

// ── Extraction tests ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_extract_profile_from_messages() {
    let Some(_model) = check_env() else {
        eprintln!("Skipping: set OPENAI_API_KEY to run");
        return;
    };

    let ext = extractor::from_env().unwrap();

    let msgs = vec![
        Message {
            role: "user".into(),
            content: "My name is Alice and I work as a software engineer. I prefer Rust over \
                      Python."
                .into(),
            timestamp: "now".into(),
        },
        Message {
            role: "assistant".into(),
            content: "Nice to meet you Alice! Rust is a great choice.".into(),
            timestamp: "now".into(),
        },
    ];

    let changes = ext.extract(&msgs).await.unwrap();
    eprintln!("Extracted {} changes: {changes:?}", changes.len());

    // Should extract at least profile + preference
    let has_profile =
        changes.iter().any(|c| matches!(c, runestone::MemoryChange::GlobalProfile { .. }));
    assert!(has_profile, "Expected a GlobalProfile change");
}

#[tokio::test]
async fn test_extract_empty_messages() {
    let Some(_model) = check_env() else {
        eprintln!("Skipping: set OPENAI_API_KEY to run");
        return;
    };

    let ext = extractor::from_env().unwrap();
    let changes = ext.extract(&[]).await.unwrap();
    assert!(changes.is_empty());
}

// ── Summary tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_summarize_directory() {
    let Some(_model) = check_env() else {
        eprintln!("Skipping: set OPENAI_API_KEY to run");
        return;
    };

    let ext = extractor::from_env().unwrap();

    let files = vec![
        FileEntry {
            name: "editor.md".into(),
            content: "Preferred editor: vim with Rust plugins".into(),
        },
        FileEntry {
            name: "language.md".into(),
            content: "Primary language: Rust, secondary: Python".into(),
        },
    ];

    let summary = ext.summarize_directory("preferences", None, &files).await.unwrap();
    eprintln!("Summary: {summary}");
    assert!(!summary.is_empty());
    assert!(summary.len() < 500, "L0 summary too long: {}", summary.len());
}

#[tokio::test]
async fn test_generate_overview() {
    let Some(_model) = check_env() else {
        eprintln!("Skipping: set OPENAI_API_KEY to run");
        return;
    };

    let ext = extractor::from_env().unwrap();

    let children = vec![
        FileEntry {
            name: "cases".into(),
            content: "2 cases: fix-timeout (retry logic), auth-errors (token refresh)".into(),
        },
        FileEntry {
            name: "patterns".into(),
            content: "1 pattern: error-handling (exn + thiserror)".into(),
        },
    ];

    let overview = ext.generate_overview("memory", &children).await.unwrap();
    eprintln!("Overview: {overview}");
    assert!(!overview.is_empty());
    assert!(overview.len() < 3000, "L1 overview too long: {}", overview.len());
}

// ── End-to-end commit with extraction ────────────────────────────────────────

#[tokio::test]
async fn test_full_commit_with_extraction() {
    let Some(_model) = check_env() else {
        eprintln!("Skipping: set OPENAI_API_KEY to run");
        return;
    };

    let dir = setup("full_commit");
    let rs = Runestone::new(&dir, "testuser", extractor::from_env().unwrap());
    let agent = rs.agent("testbot");

    let s = unwrap(agent.session_open("s1"));

    unwrap(agent.session_add(&s, "user", "Hi, I'm Bob. I code in Go and prefer vscode.").await);
    unwrap(agent.session_add(&s, "assistant", "Hi Bob! Go is great for backend services.").await);

    let result = unwrap(agent.session_commit(&s).await);
    let count = result.messages_processed();
    let changes = result.changes();
    eprintln!("Processed {count} messages, {} changes: {changes:?}", changes.len());

    assert_eq!(count, 2);
    // With a real LLM, we expect at least one change extracted
    assert!(!changes.is_empty(), "Expected at least one MemoryChange from LLM extraction");

    // Verify L0 files were generated
    let files = rs.memory_list().unwrap();
    eprintln!("Files after commit: {files:?}");
    assert!(files.iter().any(|f| f.contains(".abstract.md")), "No L0 abstracts generated");

    // Verify profile was extracted
    let profile = rs.memory_load(&runestone::Profile).unwrap();
    eprintln!("Profile after extraction: {profile:?}");
    assert!(profile.is_some(), "Expected profile to be extracted and stored");

    let _ = std::fs::remove_dir_all(&dir);
}
