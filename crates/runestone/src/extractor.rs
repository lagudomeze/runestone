use std::future::Future;

use rig::{
    agent::AgentBuilder,
    client::CompletionClient,
    completion::{CompletionModel, Prompt},
};
use serde::Deserialize;

use crate::{
    error::{Result, RunestoneError},
    memory::MemoryChange,
    session::Message,
};

// ── Types ────────────────────────────────────────────────────────────────────

/// A file entry passed to the summarizer.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub content: String,
}

/// Prompt templates for summary generation. Use `{dir_name}`, `{files}`,
/// `{existing_abstract}` as placeholders.
#[derive(Debug, Clone)]
pub struct SummaryPrompts {
    /// Template for L0 abstract generation. Placeholders:
    /// - `{dir_name}` — directory name
    /// - `{existing_abstract}` — the previous abstract, or "(none)"
    /// - `{files}` — file listing (auto-generated from FileEntry list)
    pub summarize: String,

    /// Template for L1 overview generation. Placeholders:
    /// - `{dir_name}` — directory name
    /// - `{children}` — child directory summaries (auto-generated)
    pub overview: String,
}

impl Default for SummaryPrompts {
    fn default() -> Self {
        Self {
            summarize: concat!(
                "You are a directory summarizer. ",
                "Existing abstract: {existing_abstract}\n\n",
                "Updated files:\n{files}\n\n",
                "Write ONE concise sentence (max 100 tokens) merging the existing abstract ",
                "with the updated files. Output only the summary text, no markdown, no JSON."
            )
            .to_string(),
            overview: concat!(
                "You are a directory overview generator.\n",
                "Contents:\n{children}\n\n",
                "Write a structured markdown overview for directory '{dir_name}' (max 2k tokens). ",
                "For each entry write one descriptive line. Output only the overview, no preamble."
            )
            .to_string(),
        }
    }
}

// ── Extraction trait ────────────────────────────────────────────────────────

pub trait Extractor {
    fn extract(
        &self,
        messages: &[Message],
    ) -> impl Future<Output = Result<Vec<MemoryChange>>> + Send;

    /// Summarize files in a directory. `existing_abstract` is the previous
    /// L0 content, or `None` on first generation. `files` contains only
    /// the **new or changed** files since last summary.
    fn summarize_directory(
        &self,
        dir_name: &str,
        existing_abstract: Option<&str>,
        files: &[FileEntry],
    ) -> impl Future<Output = Result<String>> + Send;

    fn generate_overview(
        &self,
        dir_name: &str,
        children: &[FileEntry],
    ) -> impl Future<Output = Result<String>> + Send;

    /// Given a query and a list of candidate directories (with their L0
    /// abstracts), return the indices of directories relevant to the query.
    fn route_directories(
        &self,
        query: &str,
        candidates: &[FileEntry],
    ) -> impl Future<Output = Result<Vec<usize>>> + Send;
}

// ── No-op extractor ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct NoopExtractor;

impl Extractor for NoopExtractor {
    async fn extract(&self, _messages: &[Message]) -> Result<Vec<MemoryChange>> {
        Ok(vec![])
    }

    async fn summarize_directory(
        &self,
        dir_name: &str,
        _existing: Option<&str>,
        files: &[FileEntry],
    ) -> Result<String> {
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        Ok(format!("{}: {}", dir_name, names.join(", ")))
    }

    async fn generate_overview(&self, dir_name: &str, children: &[FileEntry]) -> Result<String> {
        let content = children
            .iter()
            .map(|f| format!("- {}: {}", f.name, f.content))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(format!("## {}\n\n{}", dir_name, content))
    }

    async fn route_directories(
        &self,
        _query: &str,
        _candidates: &[FileEntry],
    ) -> Result<Vec<usize>> {
        Ok((0.._candidates.len()).collect())
    }
}

#[cfg(test)]
pub(crate) fn test_extractor() -> impl Extractor {
    NoopExtractor
}

// ── rig-backed implementation ────────────────────────────────────────────────

const EXTRACT_PROMPT: &str = r#"You are a memory extraction system. Analyze the conversation and output ONLY a JSON array of changes.

Valid change types and their required fields:
- {"type": "GlobalProfile", "content": "..."}
- {"type": "GlobalPreference", "key": "...", "value": "..."}
- {"type": "GlobalEntity", "name": "...", "description": "..."}
- {"type": "GlobalEvent", "title": "...", "detail": "..."}
- {"type": "AgentCase", "agent_id": "...", "title": "...", "content": "..."}
- {"type": "AgentPattern", "agent_id": "...", "name": "...", "workflow": "..."}
- {"type": "AgentTool", "agent_id": "...", "name": "...", "usage": "..."}
- {"type": "AgentSkill", "agent_id": "...", "name": "...", "steps": "..."}
- {"type": "UpdateAbstract", "session_path": "...", "content": "..."}
- {"type": "UpdateOverview", "session_path": "...", "content": "..."}

For UpdateAbstract/UpdateOverview: merge new info with existing content if provided.
Return [] if nothing new. Output raw JSON only, no markdown blocks."#;

/// Memory extractor backed by a rig CompletionModel.
#[derive(Clone)]
pub struct RigExtractor<M: CompletionModel> {
    extract_agent: rig::agent::Agent<M>,
    summarize_agent: rig::agent::Agent<M>,
    overview_agent: rig::agent::Agent<M>,
    prompts: SummaryPrompts,
}

impl<M: CompletionModel> RigExtractor<M> {
    pub fn new(model: M) -> Self {
        let m2 = model.clone();
        let m3 = model.clone();
        Self {
            extract_agent: AgentBuilder::new(model).preamble(EXTRACT_PROMPT).build(),
            summarize_agent: AgentBuilder::new(m2)
                .preamble("You are a directory summarizer.")
                .build(),
            overview_agent: AgentBuilder::new(m3)
                .preamble("You are a directory overview generator.")
                .build(),
            prompts: SummaryPrompts::default(),
        }
    }

    /// Customize the summary prompts.
    pub fn with_prompts(mut self, prompts: SummaryPrompts) -> Self {
        self.prompts = prompts;
        self
    }
}

impl<M> Extractor for RigExtractor<M>
where
    M: CompletionModel + Send + Sync + 'static,
{
    async fn extract(&self, messages: &[Message]) -> Result<Vec<MemoryChange>> {
        let prompt = format_messages(messages);
        if prompt == "## Conversation\n\n" {
            return Ok(vec![]);
        }
        let response = self
            .extract_agent
            .prompt(prompt)
            .await
            .map_err(|e| RunestoneError::Other(e.to_string()))?;
        Ok(parse_changes(&response))
    }

    async fn summarize_directory(
        &self,
        dir_name: &str,
        existing_abstract: Option<&str>,
        files: &[FileEntry],
    ) -> Result<String> {
        let files_text = files
            .iter()
            .map(|f| format!("--- {} ---\n{}", f.name, f.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        let existing = existing_abstract.unwrap_or("(none)");

        let prompt = self
            .prompts
            .summarize
            .replace("{dir_name}", dir_name)
            .replace("{existing_abstract}", existing)
            .replace("{files}", &files_text);

        self.summarize_agent
            .prompt(prompt)
            .await
            .map(|r| r.trim().to_string())
            .map_err(|e| RunestoneError::Other(e.to_string()).into())
    }

    async fn route_directories(&self, query: &str, candidates: &[FileEntry]) -> Result<Vec<usize>> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }
        let mut prompt = String::from("Query: ");
        prompt.push_str(query);
        prompt.push_str("\n\nCandidate directories (name: abstract):\n");
        for (i, c) in candidates.iter().enumerate() {
            prompt.push_str(&format!("[{i}] {}: {}\n", c.name, c.content));
        }
        prompt.push_str(
            "\nReturn a JSON array of indices that are relevant to the query. Example: [0, 3]. \
             Output only the array.",
        );

        let response = self
            .extract_agent
            .prompt(prompt)
            .await
            .map_err(|e| RunestoneError::Other(e.to_string()))?;

        Ok(serde_json::from_str::<Vec<usize>>(response.trim())
            .unwrap_or_else(|_| (0..candidates.len()).collect()))
    }

    async fn generate_overview(&self, dir_name: &str, children: &[FileEntry]) -> Result<String> {
        let children_text = children
            .iter()
            .map(|f| format!("- {}: {}", f.name, f.content))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = self
            .prompts
            .overview
            .replace("{dir_name}", dir_name)
            .replace("{children}", &children_text);

        self.overview_agent
            .prompt(prompt)
            .await
            .map(|r| r.trim().to_string())
            .map_err(|e| RunestoneError::Other(e.to_string()).into())
    }
}

/// Convenience: build a RigExtractor from environment variables.
pub fn from_env() -> Option<RigExtractor<rig::providers::openai::CompletionModel>> {
    let api_key = std::env::var("OPENAI_API_KEY").ok()?;
    let model_name = std::env::var("RUNESTONE_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());

    use rig::providers::openai::CompletionsClient;
    let mut builder = CompletionsClient::builder().api_key(&api_key);
    if let Ok(base) = std::env::var("OPENAI_API_BASE") {
        builder = builder.base_url(&base);
    }
    let client = builder.build().ok()?;
    let model = client.completion_model(&model_name);
    Some(RigExtractor::new(model))
}

pub fn has_env_credentials() -> bool {
    std::env::var("OPENAI_API_KEY").is_ok()
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn format_messages(msgs: &[Message]) -> String {
    let mut out = String::from("## Conversation\n\n");
    for m in msgs {
        out.push_str(&format!("**{}**: {}\n\n", m.role, m.content));
    }
    out
}

fn parse_changes(raw: &str) -> Vec<MemoryChange> {
    let json = raw.trim();
    let json = if json.starts_with('[') {
        json.to_string()
    } else if let Some(inner) = extract_code_block(json, "json") {
        inner
    } else {
        return vec![];
    };

    serde_json::from_str::<Vec<RawChange>>(&json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.into_memory_change())
        .collect()
}

fn extract_code_block(text: &str, lang: &str) -> Option<String> {
    let start = text.find(&format!("```{lang}"))?;
    let rest = &text[start + 3 + lang.len()..];
    let end = rest.find("```")?;
    Some(rest[..end].trim().to_string())
}

#[derive(Deserialize)]
struct RawChange {
    #[serde(rename = "type")]
    change_type: String,
    content: Option<String>,
    key: Option<String>,
    value: Option<String>,
    name: Option<String>,
    description: Option<String>,
    title: Option<String>,
    detail: Option<String>,
    agent_id: Option<String>,
    workflow: Option<String>,
    session_path: Option<String>,
}

impl RawChange {
    fn into_memory_change(self) -> Option<MemoryChange> {
        match self.change_type.as_str() {
            "GlobalProfile" => Some(MemoryChange::GlobalProfile { content: self.content? }),
            "GlobalPreference" => {
                Some(MemoryChange::GlobalPreference { key: self.key?, value: self.value? })
            }
            "GlobalEntity" => Some(MemoryChange::GlobalEntity {
                name: self.name?,
                description: self.description?,
            }),
            "GlobalEvent" => {
                Some(MemoryChange::GlobalEvent { title: self.title?, detail: self.detail? })
            }
            "AgentCase" => Some(MemoryChange::AgentCase {
                agent_id: self.agent_id?,
                title: self.title?,
                content: self.content?,
            }),
            "AgentPattern" => Some(MemoryChange::AgentPattern {
                agent_id: self.agent_id?,
                name: self.name?,
                workflow: self.workflow?,
            }),
            "UpdateAbstract" => Some(MemoryChange::UpdateAbstract {
                session_path: self.session_path?,
                content: self.content?,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Message;

    #[test]
    fn test_format_messages() {
        let msgs = vec![
            Message { role: "user".into(), content: "I like Rust".into(), timestamp: "now".into() },
            Message {
                role: "assistant".into(),
                content: "Rust is great!".into(),
                timestamp: "now".into(),
            },
        ];
        let out = format_messages(&msgs);
        assert!(out.contains("**user**: I like Rust"));
        assert!(out.contains("**assistant**: Rust is great!"));
        assert!(out.starts_with("## Conversation"));
    }

    #[test]
    fn test_format_messages_empty() {
        let out = format_messages(&[]);
        assert_eq!(out, "## Conversation\n\n");
    }

    #[test]
    fn test_parse_empty_array() {
        let changes = parse_changes("[]");
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_profile() {
        let json = r#"[{"type": "GlobalProfile", "content": "Alice is an engineer"}]"#;
        let changes = parse_changes(json);
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            MemoryChange::GlobalProfile { content } => assert!(content.contains("Alice")),
            _ => panic!("expected GlobalProfile"),
        }
    }

    #[test]
    fn test_parse_preference() {
        let json = r#"[{"type": "GlobalPreference", "key": "language", "value": "Rust"}]"#;
        let changes = parse_changes(json);
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            MemoryChange::GlobalPreference { key, value } => {
                assert_eq!(key, "language");
                assert_eq!(value, "Rust");
            }
            _ => panic!("expected GlobalPreference"),
        }
    }

    #[test]
    fn test_parse_entity() {
        let json = r#"[{"type": "GlobalEntity", "name": "rust", "description": "A language"}]"#;
        let changes = parse_changes(json);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_parse_event() {
        let json = r#"[{"type": "GlobalEvent", "title": "Decision", "detail": "Use Redis"}]"#;
        let changes = parse_changes(json);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_parse_agent_case() {
        let json = r#"[{"type": "AgentCase", "agent_id": "bot", "title": "Timeout", "content": "Add retry"}]"#;
        let changes = parse_changes(json);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_parse_multiple_changes() {
        let json = r#"[
            {"type": "GlobalProfile", "content": "Bob"},
            {"type": "GlobalPreference", "key": "editor", "value": "vim"}
        ]"#;
        let changes = parse_changes(json);
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn test_parse_from_code_block() {
        let raw = "Some text\n```json\n[{\"type\": \"GlobalProfile\", \"content\": \
                   \"Alice\"}]\n```\nMore text";
        let changes = parse_changes(raw);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_parse_invalid_json_returns_empty() {
        let changes = parse_changes("not json at all");
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_unknown_type_skipped() {
        let json = r#"[{"type": "UnknownType", "content": "nope"}]"#;
        let changes = parse_changes(json);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_partial_missing_fields() {
        let json = r#"[{"type": "GlobalPreference", "key": null, "value": null}]"#;
        let changes = parse_changes(json);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_extract_code_block_found() {
        let text = "Prefix\n```json\n{\"a\": 1}\n```\nSuffix";
        let result = extract_code_block(text, "json");
        assert_eq!(result, Some("{\"a\": 1}".into()));
    }

    #[test]
    fn test_extract_code_block_not_found() {
        assert_eq!(extract_code_block("no blocks here", "json"), None);
    }

    #[tokio::test]
    async fn test_noop_extractor() {
        let ext = NoopExtractor;
        let changes = ext.extract(&[]).await.unwrap();
        assert!(changes.is_empty());
    }
}
