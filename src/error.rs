use thiserror::Error;

/// Structured error type for Runestone.
/// Used with `exn::Result<T, RunestoneError>` for ergonomic error handling
/// with automatic backtrace and error chaining.
#[derive(Error, Debug)]
pub enum RunestoneError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("session not found: {owner}/{agent_id}/{session_id}")]
    SessionNotFound { owner: String, agent_id: String, session_id: String },

    #[error("{0}")]
    Other(String),
}

/// Convenience type alias for functions returning `exn`-wrapped errors.
pub type Result<T> = exn::Result<T, RunestoneError>;

/// Extension trait to convert foreign `Result` types into our `Result`.
///
/// Usage: `std::fs::read_to_string("file").into_exn()?`
pub trait IntoExn<T> {
    fn into_exn(self) -> Result<T>;
}

impl<T, E> IntoExn<T> for std::result::Result<T, E>
where
    RunestoneError: From<E>,
{
    fn into_exn(self) -> Result<T> {
        self.map_err(|e| exn::Exn::from(RunestoneError::from(e)))
    }
}
