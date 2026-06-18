use std::path::PathBuf;

use crate::error::Result;

mod case;
mod change;
mod entity;
mod event;
mod preference;
mod profile;
mod scope;

pub use case::Case;
pub use change::MemoryChange;
pub use entity::Entity;
pub use event::Event;
pub use preference::Preference;
pub use profile::Profile;
pub use scope::MemoryHit;

/// Trait for memory item kinds.
///
/// Each kind knows where it lives on disk (`path`) and the type of value it
/// stores (`Value`). Currently all kinds use `Value = String`; individual
/// kinds can switch to structured types later without changing the API.
pub trait MemoryKind {
    /// The shape of data stored for this memory item.
    type Value;

    /// Relative path within the owner's data directory.
    fn path(&self) -> PathBuf;

    /// Serialize the value to a string for storage.
    fn encode(&self, value: &Self::Value) -> String;

    /// Deserialize from stored text.
    fn decode(&self, raw: &str) -> Result<Self::Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_path() {
        assert_eq!(Profile.path().to_string_lossy(), "memory/profile.md");
    }

    #[test]
    fn profile_encode_decode() {
        let v = "Alice, engineer".to_string();
        let encoded = Profile.encode(&v);
        assert_eq!(encoded, v);
        let decoded = Profile.decode(&encoded).unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn preference_path() {
        let p = Preference { key: "language".into() };
        assert_eq!(p.path().to_string_lossy(), "memory/preferences/language.md");
    }

    #[test]
    fn preference_encode_decode() {
        let p = Preference { key: "editor".into() };
        let v = "vim".to_string();
        let encoded = p.encode(&v);
        assert_eq!(encoded, v);
        let decoded = p.decode(&encoded).unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn entity_path() {
        let e = Entity { name: "rust".into() };
        assert_eq!(e.path().to_string_lossy(), "memory/entities/rust.md");
    }

    #[test]
    fn entity_encode_decode() {
        let e = Entity { name: "rust".into() };
        let v = "A systems programming language".to_string();
        let encoded = e.encode(&v);
        assert_eq!(encoded, v);
        let decoded = e.decode(&encoded).unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn event_path() {
        let e = Event { title: "decided-on-redis".into() };
        assert_eq!(e.path().to_string_lossy(), "memory/events/decided-on-redis.md");
    }

    #[test]
    fn event_encode_decode() {
        let e = Event { title: "something".into() };
        let v = "Decided to use Redis for caching".to_string();
        let encoded = e.encode(&v);
        assert_eq!(encoded, v);
        let decoded = e.decode(&encoded).unwrap();
        assert_eq!(decoded, v);
    }

    #[test]
    fn case_path() {
        let c = Case { title: "fix-timeout".into() };
        assert_eq!(c.path().to_string_lossy(), "memory/cases/fix-timeout.md");
    }

    #[test]
    fn case_encode_decode() {
        let c = Case { title: "t1".into() };
        let v = "Use timeout + retry".to_string();
        let encoded = c.encode(&v);
        assert_eq!(encoded, v);
        let decoded = c.decode(&encoded).unwrap();
        assert_eq!(decoded, v);
    }
}
