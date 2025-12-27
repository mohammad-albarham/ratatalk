//! Ollama module
//!
//! HTTP client and types for the Ollama API.

mod client;
mod types;

pub use client::OllamaClient;
pub use types::*;
