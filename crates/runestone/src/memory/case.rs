use std::path::PathBuf;

use crate::{error::Result, memory::MemoryKind};

/// An agent-level case (`agents/{agent}/memory/cases/{title}.md`).
#[derive(Debug, Clone)]
pub struct Case {
    pub agent: String,
    pub title: String,
}

impl MemoryKind for Case {
    type Value = String;
    fn path(&self) -> PathBuf {
        PathBuf::from("agents")
            .join(&self.agent)
            .join("memory")
            .join("cases")
            .join(format!("{}.md", self.title))
    }
    fn encode(&self, value: &String) -> String {
        value.clone()
    }
    fn decode(&self, raw: &str) -> Result<String> {
        Ok(raw.to_string())
    }
}
