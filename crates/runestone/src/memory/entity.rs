use std::path::PathBuf;

use crate::{error::Result, memory::MemoryKind};

/// An entity reference (`memory/entities/{name}.md`).
#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
}

impl MemoryKind for Entity {
    type Value = String;
    fn path(&self) -> PathBuf {
        PathBuf::from("memory/entities").join(format!("{}.md", self.name))
    }
    fn encode(&self, value: &String) -> String {
        value.clone()
    }
    fn decode(&self, raw: &str) -> Result<String> {
        Ok(raw.to_string())
    }
}
