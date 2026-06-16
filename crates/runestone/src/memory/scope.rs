/// A hit from semantic memory search (Phase 3).
#[derive(Debug, Clone)]
pub struct MemoryHit {
    pub path: String,
    pub snippet: String,
    pub score: f32,
}
