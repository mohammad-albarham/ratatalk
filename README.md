# Ratatalk 🐀💬

A terminal chat client for [Ollama](https://ollama.com), built with Rust and [ratatui](https://ratatui.rs).

![Ratatalk Screenshot](docs/screenshot.png)

## Features

- 🚀 **Fast & Responsive**: Non-blocking UI with streamed responses
- 💬 **Multi-Session**: Manage multiple named chat conversations
- 💾 **Persistent**: Sessions auto-save across restarts
- ⌨️ **Keyboard-Driven**: Vim-inspired keybindings
- 🎨 **Beautiful TUI**: Clean, modern terminal interface
- ⚙️ **Configurable**: TOML-based configuration

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Ollama](https://ollama.com/) running locally

### Build from source

```bash
git clone https://github.com/yourusername/ratatalk.git
cd ratatalk
cargo build --release
```

The binary will be at `target/release/ratatalk`.

### Install with cargo

```bash
cargo install --path .
```

## Usage

1. Make sure Ollama is running:
   ```bash
   ollama serve
   ```

2. Start ratatalk:
   ```bash
   ratatalk
   ```

## Keybindings

### General
| Key | Action |
|-----|--------|
| `q` / `Ctrl+c` | Quit |
| `?` | Toggle help |
| `Ctrl+r` | Refresh models |

### Navigation
| Key | Action |
|-----|--------|
| `Tab` | Next session |
| `Shift+Tab` | Previous session |
| `Ctrl+n` | New session |
| `Ctrl+w` | Delete session |
| `m` | Select model |

### Chat
| Key | Action |
|-----|--------|
| `i` / `Enter` | Start typing |
| `Esc` | Stop typing |
| `Enter` | Send message (while typing) |

### Scrolling
| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Ctrl+d` | Page down |
| `Ctrl+u` | Page up |
| `g` | Scroll to top |
| `G` | Scroll to bottom |

### Input Editing
| Key | Action |
|-----|--------|
| `Ctrl+a` | Move to start of line |
| `Ctrl+e` | Move to end of line |
| `Ctrl+u` | Clear input |
| `Ctrl+w` | Delete word |

## Configuration

Configuration is stored at `~/.config/ratatalk/config.toml`:

```toml
[server]
host = "http://127.0.0.1:11434"
timeout_secs = 30

[model]
default_model = "llama3.2:latest"
temperature = 0.7
top_k = 40
top_p = 0.9
max_tokens = 0  # 0 = unlimited

[ui]
show_timestamps = true
show_token_count = true
sidebar_width = 30
mouse_support = true
tick_rate_ms = 100

[keybindings]
vim_mode = false
```

## Data Storage

- **Config**: `~/.config/ratatalk/config.toml`
- **Sessions**: `~/.local/share/ratatalk/sessions.json`
- **Logs**: `~/.config/ratatalk/ratatalk.log`

## Architecture

```
src/
├── main.rs           # Entry point, terminal setup, main loop
├── app.rs            # Application state, events, actions
├── config.rs         # Configuration management
├── error.rs          # Error types
├── events.rs         # Input handling, keybindings
├── persistence.rs    # Session save/load
├── ollama/
│   ├── mod.rs        # Module exports
│   ├── client.rs     # HTTP client
│   └── types.rs      # API types
└── ui/
    ├── mod.rs        # UI module, colors, styles
    ├── layout.rs     # Screen layout
    ├── chat.rs       # Chat area rendering
    ├── input.rs      # Input box rendering
    ├── sidebar.rs    # Session/model sidebar
    └── popup.rs      # Modal dialogs
```

## Roadmap

### MVP ✅
- [x] Connect to local Ollama
- [x] List available models
- [x] Chat with streaming responses
- [x] Multiple chat sessions
- [x] Persistent config & sessions
- [x] Basic keybindings

### Future
- [ ] System prompts per session
- [ ] Adjustable model parameters per session
- [ ] SQLite backend for history
- [ ] Export chat to Markdown
- [ ] Token/latency statistics
- [ ] Vim-style keybindings
- [ ] Search within chat
- [ ] RAG support with local files
- [ ] Image/multimodal support

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Ollama](https://ollama.com/) for the excellent local LLM server
- [ratatui](https://ratatui.rs/) for the amazing TUI framework
- [oterm](https://github.com/ggozad/oterm) for inspiration
