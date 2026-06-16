use std::path::PathBuf;

use crate::{error::Result, memory::MemoryKind};

/// An event or decision record (`memory/events/{title}.md`).
#[derive(Debug, Clone)]
pub struct Event {
    pub title: String,
}

impl MemoryKind for Event {
    type Value = String;
    fn path(&self) -> PathBuf {
        PathBuf::from("memory/events").join(format!("{}.md", self.title))
    }
    fn encode(&self, value: &String) -> String {
        value.clone()
    }
    fn decode(&self, raw: &str) -> Result<String> {
        Ok(raw.to_string())
    }
}
