//! Ollama HTTP client
//!
//! Async client for communicating with the Ollama API server.

use crate::error::OllamaError;
use futures::Stream;
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;
use tokio_stream::StreamExt;

use super::types::*;

/// Ollama API client
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: impl Into<String>, timeout_secs: u64) -> Result<Self, OllamaError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    /// Create a client with default settings (localhost:11434)
    #[allow(dead_code)]
    pub fn default_local() -> Result<Self, OllamaError> {
        Self::new("http://127.0.0.1:11434", 30)
    }

    /// Check if the Ollama server is reachable
    pub async fn health_check(&self) -> Result<bool, OllamaError> {
        let url = format!("{}/", self.base_url);
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// List all available models
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionFailed { url: self.base_url.clone() }
                } else {
                    OllamaError::Request(e)
                }
            })?;

        if !response.status().is_success() {
            return Err(OllamaError::ApiError {
                message: format!("Failed to list models: HTTP {}", response.status()),
            });
        }

        let body: ListModelsResponse = response.json().await?;
        Ok(body.models)
    }

    /// Send a chat request and return a stream of response chunks
    pub async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<impl Stream<Item = Result<ChatResponseChunk, OllamaError>>, OllamaError> {
        let url = format!("{}/api/chat", self.base_url);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionFailed { url: self.base_url.clone() }
                } else {
                    OllamaError::Request(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OllamaError::ApiError {
                message: format!("Chat request failed: HTTP {} - {}", status, body),
            });
        }

        // Convert the response body into a stream of chunks
        let stream = response.bytes_stream();
        
        // Parse each chunk as JSON
        let parsed_stream = stream.map(|result| {
            result
                .map_err(OllamaError::from)
                .and_then(|bytes| {
                    // Ollama returns newline-delimited JSON
                    let text = String::from_utf8_lossy(&bytes);
                    // Handle potential multiple JSON objects in one chunk
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        // Return a placeholder that won't affect the chat
                        return Ok(ChatResponseChunk {
                            model: String::new(),
                            created_at: None,
                            message: None,
                            done: false,
                            total_duration: None,
                            load_duration: None,
                            prompt_eval_count: None,
                            prompt_eval_duration: None,
                            eval_count: None,
                            eval_duration: None,
                            error: None,
                        });
                    }
                    
                    serde_json::from_str::<ChatResponseChunk>(trimmed)
                        .map_err(OllamaError::from)
                })
        });

        Ok(parsed_stream)
    }

    /// Send a chat request and get the full response (non-streaming)
    #[allow(dead_code)]
    pub async fn chat(
        &self,
        request: ChatRequest,
    ) -> Result<ChatResponseChunk, OllamaError> {
        let non_streaming = ChatRequest {
            stream: false,
            ..request
        };

        let url = format!("{}/api/chat", self.base_url);

        let response = self.client
            .post(&url)
            .json(&non_streaming)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    OllamaError::ConnectionFailed { url: self.base_url.clone() }
                } else {
                    OllamaError::Request(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(OllamaError::ApiError {
                message: format!("Chat request failed: HTTP {} - {}", status, body),
            });
        }

        let chunk: ChatResponseChunk = response.json().await?;
        
        if let Some(error) = &chunk.error {
            return Err(OllamaError::ApiError {
                message: error.clone(),
            });
        }

        Ok(chunk)
    }

    /// Get the base URL
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Boxed stream type for easier handling
#[allow(dead_code)]
pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatResponseChunk, OllamaError>> + Send>>;

impl OllamaClient {
    /// Send a chat request and return a boxed stream (easier to store/pass around)
    #[allow(dead_code)]
    pub async fn chat_stream_boxed(
        &self,
        request: ChatRequest,
    ) -> Result<ChatStream, OllamaError> {
        let stream = self.chat_stream(request).await?;
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = OllamaClient::new("http://localhost:11434", 30);
        assert!(client.is_ok());
    }

    #[test]
    fn test_default_client() {
        let client = OllamaClient::default_local();
        assert!(client.is_ok());
        assert_eq!(client.unwrap().base_url(), "http://127.0.0.1:11434");
    }
}
