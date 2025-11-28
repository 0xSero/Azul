# Azul Browse

A beautiful terminal web browser with AI-powered chat, built in Rust.

## Features

- Full TUI web browsing with keyboard navigation
- AI chat panel with tool-calling support (navigate, scroll, follow links)
- Multi-engine search (DuckDuckGo, Wikipedia, arXiv, PubMed, Google Scholar, OpenLibrary)
- JavaScript rendering via headless Chrome
- Bookmarks and history
- Cosmic dark theme
- Tab support
- RAG integration (optional)

## Prerequisites

- **Rust** (1.70+): https://rustup.rs/
- **Chrome/Chromium** (optional): Required for `--js` mode (JavaScript rendering)

## Installation

```bash
# Clone the repo
git clone https://github.com/youruser/azul-browse-rust.git
cd azul-browse-rust

# Build release
cargo build --release

# Run
./target/release/azul-browse

# Or install to PATH
cargo install --path .
```

## Quick Start

```bash
# Start TUI browser
azul-browse

# Search DuckDuckGo
azul-browse -q "rust programming"

# Search Wikipedia
azul-browse -q "w:Rust language"

# Fetch a URL directly
azul-browse -q "https://example.com"

# Fetch with JavaScript rendering
azul-browse --js -q "https://example.com"
```

## Configuration

Config file location: `~/.config/azul/config.json`

```json
{
  "theme": {
    "accent": "#00BFFF",
    "border": "#7aa2f7",
    "text": "#c0caf5",
    "background": "#1a1b26"
  },
  "ai": {
    "provider": "openrouter",
    "api_key": "YOUR_API_KEY",
    "model": "anthropic/claude-3.5-sonnet",
    "base_url": "https://openrouter.ai/api/v1"
  },
  "refresh_rate_ms": 200,
  "rag_base_url": "http://127.0.0.1:3002",
  "memory_scope": "azul-browse"
}
```

### AI Providers

The browser supports OpenAI-compatible APIs:

| Provider | base_url |
|----------|----------|
| OpenRouter | `https://openrouter.ai/api/v1` |
| OpenAI | `https://api.openai.com/v1` |
| Local (Ollama) | `http://localhost:11434/v1` |
| Custom | Any OpenAI-compatible endpoint |

## Keybindings

### Global
| Key | Action |
|-----|--------|
| `q`, `Ctrl+C` | Quit |
| `?` | Toggle help |
| `Tab` | Cycle focus (Content -> Chat -> URL) |
| `Shift+Tab` | Reverse cycle |
| `/` | Focus URL bar |
| `Escape` | Cancel/unfocus |

### Content Navigation
| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Tab` (in content) | Next link |
| `Enter` | Follow selected link |
| `1-9` | Follow link by number |

### Chat Panel
| Key | Action |
|-----|--------|
| `Up` / `PageUp` | Scroll up (older messages) |
| `Down` / `PageDown` | Scroll down (newer messages) |
| `Enter` | Send message |
| Type | Input text |

### URL Bar
| Key | Action |
|-----|--------|
| `Enter` | Navigate/search |
| `Escape` | Cancel |

## Search Prefixes

| Prefix | Engine |
|--------|--------|
| `w:` | Wikipedia |
| `a:` | arXiv (academic papers) |
| `s:` | Google Scholar |
| `d:` | DuckDuckGo |
| `g:` | General web search |
| `p:` | PubMed (medical) |
| `ol:` | OpenLibrary (books) |

No prefix defaults to DuckDuckGo. URLs (containing `.`) go directly to the site.

## AI Chat

The chat panel supports tool calls - the AI can:
- **Navigate** to URLs
- **Follow links** by number
- **Scroll** the page

Example prompts:
- "Go to wikipedia.org"
- "Click on the first link"
- "Scroll down to see more"
- "Summarize this page"

## Development

```bash
# Build debug
cargo build

# Run with logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Lint
cargo clippy
```

## Project Structure

```
src/
├── main.rs          # Entry point, CLI/TUI setup
├── app.rs           # Application state and input handling
├── browser/         # HTTP fetching, page parsing
├── chat/            # Chat session management
├── config.rs        # Configuration loading
├── ai/              # AI provider integration
├── ui/              # Ratatui UI rendering
├── search/          # Multi-engine search
├── storage/         # SQLite bookmarks/history
├── tabs/            # Tab management
├── rag/             # RAG integration
└── memory/          # Memory/context persistence
```

## License

MIT
