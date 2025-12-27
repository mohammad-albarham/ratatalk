//! Ollama API types
//!
//! Request and response structures for the Ollama HTTP API.
//! Based on https://github.com/ollama/ollama/blob/main/docs/api.md

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Model Types
// ============================================================================

/// Model information returned by /api/tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub model: String,
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub digest: String,
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

/// Detailed model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDetails {
    #[serde(default)]
    pub parent_model: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub families: Vec<String>,
    #[serde(default)]
    pub parameter_size: String,
    #[serde(default)]
    pub quantization_level: String,
}

/// Response from /api/tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfo>,
}

// ============================================================================
// Chat Types
// ============================================================================

/// Role in a chat conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// A single message in a chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            images: None,
        }
    }

    #[allow(dead_code)]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            images: None,
        }
    }

    #[allow(dead_code)]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            images: None,
        }
    }
}

/// Options for model generation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<i32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
}

/// Request body for /api/chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default = "default_true")]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerationOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

fn default_true() -> bool {
    true
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            stream: true,
            options: None,
            keep_alive: None,
        }
    }

    pub fn with_options(mut self, options: GenerationOptions) -> Self {
        self.options = Some(options);
        self
    }

    #[allow(dead_code)]
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

/// Streamed response chunk from /api/chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponseChunk {
    pub model: String,
    pub created_at: Option<DateTime<Utc>>,
    pub message: Option<ChatMessage>,
    pub done: bool,
    
    // Final response fields (only present when done=true)
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,
    #[serde(default)]
    pub prompt_eval_duration: Option<u64>,
    #[serde(default)]
    pub eval_count: Option<u32>,
    #[serde(default)]
    pub eval_duration: Option<u64>,
    
    // Error field
    #[serde(default)]
    pub error: Option<String>,
}

impl ChatResponseChunk {
    /// Get the content from this chunk if present
    pub fn content(&self) -> Option<&str> {
        self.message.as_ref().map(|m| m.content.as_str())
    }

    /// Check if this chunk contains an error
    #[allow(dead_code)]
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
    
    /// Calculate tokens per second from the final chunk
    pub fn tokens_per_second(&self) -> Option<f64> {
        match (self.eval_count, self.eval_duration) {
            (Some(count), Some(duration)) if duration > 0 => {
                Some(count as f64 / (duration as f64 / 1_000_000_000.0))
            }
            _ => None,
        }
    }
}

// ============================================================================
// Health/Status Types
// ============================================================================

/// Response from GET / (health check)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct HealthResponse {
    #[serde(default)]
    pub status: String,
}

// ============================================================================
// Generate Types (alternative to chat)
// ============================================================================

/// Request body for /api/generate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default = "default_true")]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<GenerationOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<u64>>,
}

/// Response chunk from /api/generate
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GenerateResponseChunk {
    pub model: String,
    pub created_at: Option<DateTime<Utc>>,
    pub response: String,
    pub done: bool,
    #[serde(default)]
    pub context: Option<Vec<u64>>,
    #[serde(default)]
    pub total_duration: Option<u64>,
    #[serde(default)]
    pub load_duration: Option<u64>,
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,
    #[serde(default)]
    pub eval_count: Option<u32>,
    #[serde(default)]
    pub eval_duration: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage::user("Hello!");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello!");
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest::new("llama3.2", vec![ChatMessage::user("Hi")]);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("llama3.2"));
        assert!(json.contains("\"stream\":true"));
    }

    #[test]
    fn test_response_chunk_parsing() {
        let json = r#"{"model":"llama3.2","created_at":"2024-01-01T00:00:00Z","message":{"role":"assistant","content":"Hello"},"done":false}"#;
        let chunk: ChatResponseChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.content(), Some("Hello"));
        assert!(!chunk.done);
    }
}
