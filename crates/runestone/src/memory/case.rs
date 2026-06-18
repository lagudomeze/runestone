use std::path::PathBuf;

use crate::{error::Result, memory::MemoryKind};

/// A personal case / lesson learned (`memory/cases/{title}.md`).
#[derive(Debug, Clone)]
pub struct Case {
    pub title: String,
}

impl MemoryKind for Case {
    type Value = String;
    fn path(&self) -> PathBuf {
        PathBuf::from("memory").join("cases").join(format!("{}.md", self.title))
    }
    fn encode(&self, value: &String) -> String {
        value.clone()
    }
    fn decode(&self, raw: &str) -> Result<String> {
        Ok(raw.to_string())
    }
}
