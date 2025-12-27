//! Configuration management for ratatalk
//!
//! Handles loading and saving config from `~/.config/ratatalk/config.toml`

use crate::error::ConfigError;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Ollama server settings
    #[serde(default)]
    pub server: ServerConfig,

    /// Default model settings
    #[serde(default)]
    pub model: ModelConfig,

    /// UI settings
    #[serde(default)]
    pub ui: UiConfig,

    /// Keybinding overrides (future use)
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
}

/// Ollama server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Ollama server URL
    #[serde(default = "default_host")]
    pub host: String,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_host() -> String {
    "http://127.0.0.1:11434".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            timeout_secs: default_timeout(),
        }
    }
}

/// Model configuration defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Default model to use
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Temperature (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Top-K sampling
    #[serde(default = "default_top_k")]
    pub top_k: u32,

    /// Top-P (nucleus) sampling
    #[serde(default = "default_top_p")]
    pub top_p: f32,

    /// Maximum tokens to generate (0 = unlimited)
    #[serde(default)]
    pub max_tokens: u32,

    /// Context window size (0 = model default)
    #[serde(default)]
    pub num_ctx: u32,
}

fn default_model() -> String {
    "llama3.2:latest".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_top_k() -> u32 {
    40
}

fn default_top_p() -> f32 {
    0.9
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_model: default_model(),
            temperature: default_temperature(),
            top_k: default_top_k(),
            top_p: default_top_p(),
            max_tokens: 0,
            num_ctx: 0,
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Show timestamps in chat
    #[serde(default = "default_true")]
    pub show_timestamps: bool,

    /// Show token count in status bar
    #[serde(default = "default_true")]
    pub show_token_count: bool,

    /// Sidebar width in characters
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u16,

    /// Enable mouse support
    #[serde(default = "default_true")]
    pub mouse_support: bool,

    /// Tick rate in milliseconds
    #[serde(default = "default_tick_rate")]
    pub tick_rate_ms: u64,
}

fn default_true() -> bool {
    true
}

fn default_sidebar_width() -> u16 {
    30
}

fn default_tick_rate() -> u64 {
    100
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_timestamps: true,
            show_token_count: true,
            sidebar_width: default_sidebar_width(),
            mouse_support: true,
            tick_rate_ms: default_tick_rate(),
        }
    }
}

/// Keybindings configuration (extensible for future use)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeybindingsConfig {
    /// Vim-mode enabled
    #[serde(default)]
    pub vim_mode: bool,
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let proj_dirs = ProjectDirs::from("com", "ratatalk", "ratatalk")
            .ok_or(ConfigError::NoConfigDir)?;
        
        let config_dir = proj_dirs.config_dir();
        Ok(config_dir.join("config.toml"))
    }

    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf, ConfigError> {
        let proj_dirs = ProjectDirs::from("com", "ratatalk", "ratatalk")
            .ok_or(ConfigError::NoConfigDir)?;
        
        Ok(proj_dirs.config_dir().to_path_buf())
    }

    /// Load config from disk, or create default if not exists
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        
        if !path.exists() {
            // Create default config
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let contents = std::fs::read_to_string(&path)
            .map_err(ConfigError::Read)?;
        
        let config: Config = toml::from_str(&contents)
            .map_err(ConfigError::Parse)?;
        
        Ok(config)
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path()?;
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(ConfigError::CreateDir)?;
        }

        let contents = toml::to_string_pretty(self)
            .map_err(ConfigError::Serialize)?;
        
        std::fs::write(&path, contents)
            .map_err(ConfigError::Write)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_serializes() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("host"));
        assert!(toml_str.contains("127.0.0.1:11434"));
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.server.host, parsed.server.host);
    }
}
