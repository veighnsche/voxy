use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("audio fixture not found: {0}")]
    FixtureNotFound(PathBuf),
    #[error("invalid fixture name: {0}")]
    InvalidFixtureName(String),
    #[error("failed to open fixture '{path}': {source}")]
    FixtureOpen { path: PathBuf, source: io::Error },
    #[error("failed to decode fixture '{path}': {reason}")]
    FixtureDecode { path: PathBuf, reason: String },
    #[error("audio lock poisoned: {0}")]
    LockPoisoned(&'static str),
}
