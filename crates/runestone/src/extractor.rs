use std::sync::Arc;

use async_trait::async_trait;
use rig::{
    agent::AgentBuilder,
    completion::{CompletionModel, Prompt},
};
use serde::Deserialize;

use crate::{
    error::{Result, RunestoneError},
    memory::MemoryChange,
    session::Message,
};

// ── Extraction trait ────────────────────────────────────────────────────────

/// Type-erased extraction interface. Implementations can use any LLM backend.
#[async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(&self, messages: &[Message]) -> Result<Vec<MemoryChange>>;
}

// ── rig-backed implementation ────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are a memory extraction system. Analyze the conversation and output ONLY a JSON array of changes.

Valid change types and their required fields:
- {"type": "GlobalProfile", "content": "..."}
- {"type": "GlobalPreference", "key": "...", "value": "..."}
- {"type": "GlobalEntity", "name": "...", "description": "..."}
- {"type": "GlobalEvent", "title": "...", "detail": "..."}
- {"type": "AgentCase", "agent_id": "...", "title": "...", "content": "..."}
- {"type": "AgentPattern", "agent_id": "...", "name": "...", "workflow": "..."}
- {"type": "UpdateAbstract", "session_path": "...", "content": "..."}

Return [] if nothing new. Output raw JSON only, no markdown blocks."#;

/// Memory extractor backed by a rig CompletionModel.
pub struct RigExtractor<M: CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: CompletionModel> RigExtractor<M> {
    pub fn new(model: M) -> Self {
        let agent = AgentBuilder::new(model).preamble(SYSTEM_PROMPT).build();
        Self { agent }
    }
}

#[async_trait]
impl<M> Extractor for RigExtractor<M>
where
    M: CompletionModel + Send + Sync + 'static,
{
    async fn extract(&self, messages: &[Message]) -> Result<Vec<MemoryChange>> {
        if messages.is_empty() {
            return Ok(vec![]);
        }
        let prompt = format_messages(messages);
        let response =
            self.agent.prompt(prompt).await.map_err(|e| RunestoneError::Other(e.to_string()))?;
        Ok(parse_changes(&response))
    }
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

// ── Factory ─────────────────────────────────────────────────────────────────

/// Create a type-erased extractor from any rig CompletionModel.
pub fn from_model<M: CompletionModel + Send + Sync + 'static>(model: M) -> Arc<dyn Extractor> {
    Arc::new(RigExtractor::new(model))
}
