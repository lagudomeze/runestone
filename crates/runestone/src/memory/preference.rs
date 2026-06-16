use std::path::PathBuf;

use crate::{error::Result, memory::MemoryKind};

/// A preference (`memory/preferences/{key}.md`).
#[derive(Debug, Clone)]
pub struct Preference {
    pub key: String,
}

impl MemoryKind for Preference {
    type Value = String;
    fn path(&self) -> PathBuf {
        PathBuf::from("memory/preferences").join(format!("{}.md", self.key))
    }
    fn encode(&self, value: &String) -> String {
        value.clone()
    }
    fn decode(&self, raw: &str) -> Result<String> {
        Ok(raw.to_string())
    }
}
