use serde::{Deserialize, Serialize};

/// Structured memory change produced by the Memory Extractor.
/// Each variant maps to a file that should be created or updated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryChange {
    /// Update the global profile (`{owner}/memory/profile.md`).
    GlobalProfile { content: String },
    /// Write a preference (`{owner}/memory/preferences/{key}.md`).
    GlobalPreference { key: String, value: String },
    /// Write an entity note (`{owner}/memory/entities/{name}.md`).
    GlobalEntity { name: String, description: String },
    /// Record an event / decision (`{owner}/memory/events/{title}.md`).
    GlobalEvent { title: String, detail: String },
    /// Write an agent-level case
    /// (`{owner}/agents/{agent}/memory/cases/{title}.md`).
    AgentCase { agent_id: String, title: String, content: String },
    /// Write an agent-level pattern.
    AgentPattern { agent_id: String, name: String, workflow: String },
    /// Write agent tool knowledge.
    AgentTool { agent_id: String, name: String, usage: String },
    /// Write agent skill workflow.
    AgentSkill { agent_id: String, name: String, steps: String },
    /// Replace the session abstract (`.abstract.md`).
    UpdateAbstract { session_path: String, content: String },
    /// Replace the session overview (`.overview.md`).
    UpdateOverview { session_path: String, content: String },
}
