use std::path::PathBuf;

use crate::{error::Result, memory::MemoryKind};

/// User profile (`memory/profile.md`).
#[derive(Debug, Clone)]
pub struct Profile;

impl MemoryKind for Profile {
    type Value = String;
    fn path(&self) -> PathBuf {
        PathBuf::from("memory/profile.md")
    }
    fn encode(&self, value: &String) -> String {
        value.clone()
    }
    fn decode(&self, raw: &str) -> Result<String> {
        Ok(raw.to_string())
    }
}
