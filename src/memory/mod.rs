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
