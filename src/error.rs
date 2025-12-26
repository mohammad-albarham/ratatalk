//! Error types for ratatalk
//!
//! Uses thiserror for typed errors that can be converted and displayed nicely.

use thiserror::Error;

/// Application-level errors
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Ollama API error: {0}")]
    Ollama(#[from] OllamaError),

    #[error("Persistence error: {0}")]
    Persistence(#[from] PersistenceError),

    #[error("Terminal error: {0}")]
    Terminal(#[from] std::io::Error),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Read(#[source] std::io::Error),

    #[error("Failed to write config file: {0}")]
    Write(#[source] std::io::Error),

    #[error("Failed to parse config: {0}")]
    Parse(#[source] toml::de::Error),

    #[error("Failed to serialize config: {0}")]
    Serialize(#[source] toml::ser::Error),

    #[error("Could not determine config directory")]
    NoConfigDir,

    #[error("Failed to create config directory: {0}")]
    CreateDir(#[source] std::io::Error),
}

/// Ollama API errors
#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("Ollama server not reachable at {url}")]
    ConnectionFailed { url: String },

    #[error("Model not found: {model}")]
    ModelNotFound { model: String },

    #[error("Stream ended unexpectedly")]
    StreamEnded,

    #[error("API error: {message}")]
    ApiError { message: String },
}

/// Persistence errors (session history)
#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Failed to read sessions file: {0}")]
    Read(#[source] std::io::Error),

    #[error("Failed to write sessions file: {0}")]
    Write(#[source] std::io::Error),

    #[error("Failed to parse sessions: {0}")]
    Parse(#[source] serde_json::Error),

    #[error("Failed to serialize sessions: {0}")]
    Serialize(#[source] serde_json::Error),

    #[error("Could not determine data directory")]
    NoDataDir,

    #[error("Failed to create data directory: {0}")]
    CreateDir(#[source] std::io::Error),

    #[error("Session not found: {id}")]
    SessionNotFound { id: String },
}

/// Result type alias using anyhow for convenient error handling
pub type Result<T> = anyhow::Result<T>;
