use std::path::PathBuf;

use runestone::{
    Case, CommitResult, Entity, Event, MemoryKind, NoopExtractor, Preference, Profile, Runestone,
};

type Rs = Runestone<NoopExtractor>;

fn setup(name: &str) -> (Rs, PathBuf) {
    let dir = std::env::temp_dir().join(format!("runestone_int_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    (Runestone::new(dir.clone(), "testuser", NoopExtractor), dir)
}

fn unwrap<T>(r: runestone::Result<T>) -> T {
    r.unwrap_or_else(|e| panic!("{e:?}"))
}

// ── Construction ─────────────────────────────────────────────────────────────

#[test]
fn test_runestone_construction() {
    let (rs, _dir) = setup("construction");
    assert_eq!(rs.owner(), "testuser");
}

#[test]
fn test_agent_creation() {
    let (rs, _dir) = setup("agent_creation");
    let agent = rs.agent("mybot");
    assert_eq!(agent.owner(), "testuser");
    assert_eq!(agent.id(), "mybot");
}

// ── Memory store/load ────────────────────────────────────────────────────────

#[test]
fn test_memory_store_load_profile() {
    let (rs, _dir) = setup("profile");
    rs.memory_store(&Profile, &"Alice, engineer".to_string()).unwrap();
    let got = rs.memory_load(&Profile).unwrap();
    assert_eq!(got, Some("Alice, engineer".to_string()));
}

#[test]
fn test_memory_store_load_preference() {
    let (rs, _dir) = setup("pref");
    let pref = Preference { key: "editor".into() };
    rs.memory_store(&pref, &"vim".to_string()).unwrap();
    assert_eq!(rs.memory_load(&pref).unwrap(), Some("vim".to_string()));
}

#[test]
fn test_memory_load_nonexistent() {
    let (rs, _dir) = setup("nonexistent");
    let pref = Preference { key: "nonexistent".into() };
    assert_eq!(rs.memory_load(&pref).unwrap(), None);
}

#[test]
fn test_memory_store_overwrite() {
    let (rs, _dir) = setup("overwrite");
    let pref = Preference { key: "lang".into() };
    rs.memory_store(&pref, &"Rust".to_string()).unwrap();
    rs.memory_store(&pref, &"Go".to_string()).unwrap();
    assert_eq!(rs.memory_load(&pref).unwrap(), Some("Go".to_string()));
}

#[test]
fn test_memory_store_entity() {
    let (rs, _dir) = setup("entity");
    let e = Entity { name: "rust".into() };
    rs.memory_store(&e, &"A systems language".to_string()).unwrap();
    assert_eq!(rs.memory_load(&e).unwrap(), Some("A systems language".to_string()));
}

#[test]
fn test_memory_store_event() {
    let (rs, _dir) = setup("event");
    let e = Event { title: "chose-redis".into() };
    rs.memory_store(&e, &"Decided to use Redis for caching".to_string()).unwrap();
    assert_eq!(rs.memory_load(&e).unwrap(), Some("Decided to use Redis for caching".to_string()));
}

#[test]
fn test_memory_store_case() {
    let (rs, _dir) = setup("case");
    let c = Case { title: "fix-timeout".into() };
    rs.memory_store(&c, &"Added 30s timeout with retry".to_string()).unwrap();
    assert_eq!(rs.memory_load(&c).unwrap(), Some("Added 30s timeout with retry".to_string()));
}

// ── Memory list ──────────────────────────────────────────────────────────────

#[test]
fn test_memory_list_empty() {
    let (rs, _dir) = setup("list_empty");
    let files = rs.memory_list().unwrap();
    assert!(files.is_empty());
}

#[test]
fn test_memory_list_with_items() {
    let (rs, _dir) = setup("list_items");
    rs.memory_store(&Profile, &"Alice".to_string()).unwrap();
    let pref = Preference { key: "lang".into() };
    rs.memory_store(&pref, &"Rust".to_string()).unwrap();

    let files = rs.memory_list().unwrap();
    assert!(files.iter().any(|f| f.contains("profile.md")));
    assert!(files.iter().any(|f| f.contains("preferences/lang.md")));
    // L0 abstracts are auto-generated
    assert!(files.iter().any(|f| f.contains(".abstract.md")));
}

#[test]
fn test_agent_memory_list() {
    let (rs, _dir) = setup("agent_list");
    let agent = rs.agent("mybot");
    // Use a global kind — Agent::memory_list scans the owner directory
    let e = Entity { name: "agent-test".into() };
    agent.memory_store(&e, &"content".to_string()).unwrap();

    let files = agent.memory_list().unwrap();
    // Note: Agent::memory_list walks only agents/{agent}/memory/ directory
    assert!(files.is_empty()); // global memory goes under memory/, not agents/
}

// ── Session workflow ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_session_create_and_history() {
    let (rs, _dir) = setup("session_create");
    let agent = rs.agent("mybot");
    let s = unwrap(agent.session_open("s1"));
    assert_eq!(s.agent_id, "mybot");
    assert_eq!(s.session_id, "s1");

    let history = unwrap(agent.session_history(&s));
    assert!(history.is_empty());
}

#[tokio::test]
async fn test_session_add_and_commit() {
    let (rs, _dir) = setup("session_commit");
    let agent = rs.agent("mybot");
    let s = unwrap(agent.session_open("s1"));

    unwrap(agent.session_add(&s, "user", "Hello").await);
    unwrap(agent.session_add(&s, "assistant", "Hi!").await);

    let result = unwrap(agent.session_commit(&s).await);
    assert_eq!(result.messages_processed(), 2);
    assert!(result.changes().is_empty()); // no extractor configured

    // No-op recomit
    match agent.session_commit(&s).await {
        Ok(CommitResult::NoNewMessages) => {}
        other => panic!("expected NoNewMessages, got {other:?}"),
    }

    let history = unwrap(agent.session_history(&s));
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn test_session_agent_isolation() {
    let (rs, _dir) = setup("isolation");
    let bot_a = rs.agent("bot-a");
    let bot_b = rs.agent("bot-b");

    let sa = unwrap(bot_a.session_open("s1"));
    unwrap(bot_a.session_add(&sa, "user", "msg-a").await);
    unwrap(bot_a.session_commit(&sa).await);

    let sb = unwrap(bot_b.session_open("s1"));
    unwrap(bot_b.session_add(&sb, "user", "msg-b").await);
    unwrap(bot_b.session_commit(&sb).await);

    // Each agent sees only its own messages
    let ha = unwrap(bot_a.session_history(&sa));
    let hb = unwrap(bot_b.session_history(&sb));
    assert_eq!(ha.len(), 1);
    assert_eq!(hb.len(), 1);
    assert_ne!(ha[0].content, hb[0].content);
}

// ── Memory isolation ────────────────────────────────────────────────────────

#[test]
fn test_agent_memory_list_isolation() {
    let (rs, _dir) = setup("mem_isolation");
    let bot_a = rs.agent("bot-a");
    let bot_b = rs.agent("bot-b");

    // Store global memories — visible to Runestone but not to Agent
    rs.memory_store(&Profile, &"User".to_string()).unwrap();
    let e = Entity { name: "shared-entity".into() };
    rs.memory_store(&e, &"shared-content".to_string()).unwrap();

    // Runestone::memory_list sees everything
    let all = rs.memory_list().unwrap();
    assert!(all.iter().any(|f| f.contains("profile.md")));
    assert!(all.iter().any(|f| f.contains("shared-entity")));

    // Agent::memory_list is scoped to agents/{agent}/memory/
    let a_files = bot_a.memory_list().unwrap();
    assert!(!a_files.iter().any(|f| f.contains("shared-entity")));
}

// ── Memory path contract ────────────────────────────────────────────────────

#[test]
fn test_kind_paths_are_relative_and_markdown() {
    let kinds: [&dyn Fn() -> std::path::PathBuf; 5] = [
        &|| Profile.path(),
        &|| Preference { key: "k".into() }.path(),
        &|| Entity { name: "n".into() }.path(),
        &|| Event { title: "t".into() }.path(),
        &|| Case { title: "t".into() }.path(),
    ];
    for path_fn in &kinds {
        let p = path_fn();
        assert!(!p.is_absolute(), "path should be relative: {p:?}");
        assert!(p.to_string_lossy().ends_with(".md"));
    }
}
