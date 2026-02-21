use std::io;

#[derive(Debug, thiserror::Error)]
pub enum ClawHubError {
    #[error("Skill '{name}' not found in {location}")]
    SkillNotFound { name: String, location: String },

    #[error("Invalid format in {file}: {reason}")]
    InvalidFormat { file: String, reason: String },

    #[error("Conversion error from {from} to {to}: {reason}")]
    ConversionError {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Missing dependency: {dependency}")]
    DependencyMissing { dependency: String },

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Path traversal detected: {path}")]
    PathTraversal { path: String },
}

pub type Result<T> = std::result::Result<T, ClawHubError>;
