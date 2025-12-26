//! Ratatalk - Terminal chat client for Ollama
//!
//! A TUI-based chat interface for interacting with locally-running Ollama LLMs.

mod app;
mod config;
mod error;
mod events;
mod ollama;
mod persistence;
mod ui;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

use app::{AppAction, AppEvent, AppState, ChatSession, InputMode, ResponseStats};
use config::Config;
use events::{handle_key_event, process_action, EventHandler};
use ollama::{ChatRequest, OllamaClient};
use ui::{render_help_popup, render_layout, render_model_popup};

/// Terminal type alias
type Term = Terminal<CrosstermBackend<Stdout>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to file (avoid disturbing TUI)
    init_logging()?;
    
    info!("Starting ratatalk...");

    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;
    info!("Configuration loaded from {:?}", Config::config_path());

    // Initialize terminal
    let mut terminal = setup_terminal()?;
    
    // Run the application
    let result = run_app(&mut terminal, config).await;
    
    // Restore terminal
    restore_terminal(&mut terminal)?;
    
    // Handle any errors
    if let Err(ref e) = result {
        error!("Application error: {:?}", e);
        eprintln!("Error: {:#}", e);
    }
    
    info!("ratatalk exited");
    result
}

/// Initialize logging to a file
fn init_logging() -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    
    // Get log directory
    let log_dir = config::Config::config_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    
    std::fs::create_dir_all(&log_dir)?;
    
    let log_file = std::fs::File::create(log_dir.join("ratatalk.log"))?;
    
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            fmt::layer()
                .with_writer(log_file)
                .with_ansi(false)
        );
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    Ok(())
}

/// Set up the terminal for TUI
fn setup_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to normal state
fn restore_terminal(terminal: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Main application loop
async fn run_app(terminal: &mut Term, config: Config) -> Result<()> {
    // Create application state
    let mut state = AppState::new(config.clone());
    
    // Load saved sessions
    match persistence::load_sessions() {
        Ok(sessions) if !sessions.is_empty() => {
            info!("Loaded {} sessions from disk", sessions.len());
            state.sessions = sessions;
        }
        Ok(_) => {
            info!("No saved sessions found, starting fresh");
        }
        Err(e) => {
            warn!("Failed to load sessions: {}", e);
            state.set_status("Could not load saved sessions");
        }
    }
    
    // Create Ollama client
    let client = OllamaClient::new(&config.server.host, config.server.timeout_secs)
        .context("Failed to create Ollama client")?;
    
    // Create event channels
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(100);
    
    // Spawn task to load models
    {
        let client = client.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            match client.list_models().await {
                Ok(models) => {
                    let _ = tx.send(AppEvent::ModelsLoaded(models)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::ModelsError(e.to_string())).await;
                }
            }
        });
    }
    
    // Check server connectivity
    {
        let client = client.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let connected = client.health_check().await.unwrap_or(false);
            let _ = tx.send(AppEvent::ServerStatus(connected)).await;
        });
    }
    
    // Event handler
    let event_handler = EventHandler::new(config.ui.tick_rate_ms);
    
    // Main loop
    loop {
        // Render
        terminal.draw(|frame| {
            render_layout(frame, &state);
            render_model_popup(frame, &state);
            render_help_popup(frame, &state);
        })?;
        
        // Handle terminal events (non-blocking with timeout)
        if let Some(event) = event_handler.poll()? {
            match event {
                Event::Key(key) => {
                    // Special handling for submit in editing mode
                    if state.input_mode == InputMode::Editing 
                        && key.code == KeyCode::Enter 
                        && !state.input.trim().is_empty()
                        && !state.streaming
                    {
                        // Submit message
                        let input = state.take_input();
                        submit_message(&mut state, &client, &event_tx, input).await;
                        state.input_mode = InputMode::Normal;
                    } else if let Some(action) = handle_key_event(key, &state) {
                        process_action(action, &mut state);
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal will be redrawn on next iteration
                }
                _ => {}
            }
        }
        
        // Handle async events (non-blocking)
        while let Ok(event) = event_rx.try_recv() {
            match event {
                AppEvent::ModelsLoaded(models) => {
                    info!("Loaded {} models", models.len());
                    state.models = models;
                    state.loading = false;
                    if !state.models.is_empty() {
                        // Find current model in list
                        let current = state.current_model().to_string();
                        if let Some(idx) = state.models.iter().position(|m| m.name == current) {
                            state.selected_model_idx = idx;
                        }
                    }
                }
                AppEvent::ModelsError(err) => {
                    warn!("Failed to load models: {}", err);
                    state.set_error(format!("Failed to load models: {}", err));
                    state.loading = false;
                }
                AppEvent::StreamChunk(content) => {
                    if let Some(session) = state.active_session_mut() {
                        session.append_to_response(&content);
                    }
                    // Auto-scroll to bottom during streaming
                    state.scroll_to_bottom();
                }
                AppEvent::StreamComplete(stats) => {
                    info!("Stream complete: {} tokens at {:.1} tok/s", 
                        stats.tokens, stats.tokens_per_second);
                    if let Some(session) = state.active_session_mut() {
                        session.finish_response();
                    }
                    state.streaming = false;
                    state.last_response_stats = Some(stats);
                    
                    // Auto-save after response
                    if let Err(e) = persistence::save_sessions(&state.sessions) {
                        warn!("Failed to save sessions: {}", e);
                    }
                }
                AppEvent::StreamError(err) => {
                    error!("Stream error: {}", err);
                    if let Some(session) = state.active_session_mut() {
                        session.finish_response();
                        // Append error to message
                        if let Some(msg) = session.messages.last_mut() {
                            if msg.content.is_empty() {
                                msg.content = format!("[Error: {}]", err);
                            }
                        }
                    }
                    state.streaming = false;
                    state.set_error(err);
                }
                AppEvent::ServerStatus(connected) => {
                    state.server_connected = connected;
                    if !connected {
                        state.set_error("Cannot connect to Ollama server");
                    }
                }
                AppEvent::Quit => {
                    state.should_quit = true;
                }
                _ => {}
            }
        }
        
        // Check for quit
        if state.should_quit {
            // Save sessions before quitting
            if let Err(e) = persistence::save_sessions(&state.sessions) {
                warn!("Failed to save sessions on exit: {}", e);
            }
            break;
        }
    }
    
    Ok(())
}

/// Submit a user message and start streaming response
async fn submit_message(
    state: &mut AppState,
    client: &OllamaClient,
    event_tx: &mpsc::Sender<AppEvent>,
    content: String,
) {
    let content = content.trim().to_string();
    if content.is_empty() {
        return;
    }
    
    // Add user message
    if let Some(session) = state.active_session_mut() {
        session.add_user_message(&content);
        session.start_assistant_response();
    }
    
    state.streaming = true;
    state.scroll_to_bottom();
    
    // Get messages for API call
    let messages = state
        .active_session()
        .map(|s| s.to_chat_messages())
        .unwrap_or_default();
    
    let model = state.current_model().to_string();
    
    // Build request with options from config
    let mut request = ChatRequest::new(model, messages);
    
    // Apply generation options from config
    let opts = ollama::GenerationOptions {
        temperature: Some(state.config.model.temperature),
        top_k: Some(state.config.model.top_k),
        top_p: Some(state.config.model.top_p),
        num_predict: if state.config.model.max_tokens > 0 {
            Some(state.config.model.max_tokens as i32)
        } else {
            None
        },
        num_ctx: if state.config.model.num_ctx > 0 {
            Some(state.config.model.num_ctx)
        } else {
            None
        },
        ..Default::default()
    };
    request = request.with_options(opts);
    
    // Spawn streaming task
    let client = client.clone();
    let tx = event_tx.clone();
    
    tokio::spawn(async move {
        match client.chat_stream(request).await {
            Ok(mut stream) => {
                let mut total_tokens = 0u32;
                let mut tokens_per_sec = 0.0;
                let mut total_duration = 0u64;
                
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(chunk) => {
                            // Check for errors in the chunk
                            if let Some(error) = chunk.error {
                                let _ = tx.send(AppEvent::StreamError(error)).await;
                                return;
                            }
                            
                            // Send content if present
                            if let Some(content) = chunk.content() {
                                if !content.is_empty() {
                                    let _ = tx.send(AppEvent::StreamChunk(content.to_string())).await;
                                }
                            }
                            
                            // Capture final stats
                            if chunk.done {
                                if let Some(count) = chunk.eval_count {
                                    total_tokens = count;
                                }
                                if let Some(tps) = chunk.tokens_per_second() {
                                    tokens_per_sec = tps;
                                }
                                if let Some(duration) = chunk.total_duration {
                                    total_duration = duration / 1_000_000; // ns to ms
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::StreamError(e.to_string())).await;
                            return;
                        }
                    }
                }
                
                // Send completion
                let _ = tx.send(AppEvent::StreamComplete(ResponseStats {
                    tokens: total_tokens,
                    tokens_per_second: tokens_per_sec,
                    total_duration_ms: total_duration,
                })).await;
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError(e.to_string())).await;
            }
        }
    });
}
