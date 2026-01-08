# Azul Browse

[![CI](https://github.com/0xSero/Azul/actions/workflows/ci.yml/badge.svg)](https://github.com/0xSero/Azul/actions/workflows/ci.yml)
[![Release](https://github.com/0xSero/Azul/actions/workflows/release.yml/badge.svg)](https://github.com/0xSero/Azul/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A beautiful terminal web browser with AI-powered chat, built in Rust.

## Features

- Full TUI web browsing with keyboard navigation
- Calm 2-pane layout (links + content) with focus mode
- AI chat panel (hidden by default) with tool-calling support
- Multi-engine search (DuckDuckGo, Wikipedia, arXiv, PubMed, Google Scholar, OpenLibrary)
- JavaScript rendering via headless Chrome
- Bookmarks and history
- Theme presets with warm defaults
- Tab support
- RAG integration (optional)

## Prerequisites

- **Rust** (1.70+): https://rustup.rs/
- **Chrome/Chromium** (optional): Required for `--js` mode (JavaScript rendering)

## Installation

### From Releases (Recommended)

Download the latest binary for your platform from [Releases](https://github.com/0xSero/Azul/releases):

```bash
# Linux (x86_64)
curl -LO https://github.com/0xSero/Azul/releases/latest/download/azul-linux-x86_64.tar.gz
tar xzf azul-linux-x86_64.tar.gz
sudo mv azul /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/0xSero/Azul/releases/latest/download/azul-darwin-aarch64.tar.gz
tar xzf azul-darwin-aarch64.tar.gz
sudo mv azul /usr/local/bin/

# macOS (Intel)
curl -LO https://github.com/0xSero/Azul/releases/latest/download/azul-darwin-x86_64.tar.gz
tar xzf azul-darwin-x86_64.tar.gz
sudo mv azul /usr/local/bin/
```

### From Source

```bash
# Clone the repo
git clone https://github.com/0xSero/Azul.git
cd Azul

# Quick install (uses Makefile)
make install          # System-wide (/usr/local/bin, requires sudo)
make install-user     # User-only (~/.local/bin)

# Or manually
cargo build --release
./target/release/azul

# Or via cargo
cargo install --path .
```

## Quick Start

```bash
# Start TUI browser
azul

# Search DuckDuckGo
azul -q "rust programming"

# Search Wikipedia
azul -q "w:Rust language"

# Fetch a URL directly
azul -q "https://example.com"

# Fetch with JavaScript rendering
azul --js -q "https://example.com"
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
  "ui": {
    "panes": {
      "sidebar_width": 26,
      "focus_mode": false
    }
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
| `Tab` | Cycle focus (Content -> Sidebar -> URL) |
| `Shift+Tab` | Reverse cycle |
| `/` | Focus URL bar |
| `Escape` | Close panels / exit chat |
| `c` | Open chat panel |
| `z` | Toggle focus mode |
| `1` | Focus links |
| `2` | Focus content |
| `3` | Focus chat |

### Content Navigation
| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `J` | Scroll down 5 lines |
| `K` | Scroll up 5 lines |
| `Ctrl+D` | Half page down |
| `Ctrl+U` | Half page up |
| `Space` / `PageDown` | Page down |
| `g` | Go to top |
| `G` | Go to bottom |
| `H` / `L` | Back / forward |
| `v` | Toggle raw view |
| `f` | Toggle text reflow |

### Chat Panel
| Key | Action |
|-----|--------|
| `Up` / `PageUp` | Scroll up (older messages) |
| `Down` / `PageDown` | Scroll down (newer messages) |
| `Enter` | Send message |
| `Escape` | Close chat |
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

No prefix runs multi-engine search. URLs (containing `.`) go directly to the site.

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
make build            # or: cargo build

# Run with logging
RUST_LOG=debug cargo run

# Run all checks (format, lint, test)
make check

# Individual checks
make test             # Run tests
make lint             # Run clippy
make fmt              # Format code
make fmt-check        # Check formatting

# Clean build artifacts
make clean
```

## Releasing

Releases are automated via GitHub Actions. To create a new release:

```bash
# 1. Update version (choose one)
make bump-patch       # 0.0.7 -> 0.0.8
make bump-minor       # 0.0.7 -> 0.1.0
make bump-major       # 0.0.7 -> 1.0.0

# 2. Update CHANGELOG.md with changes

# 3. Commit changes
git add -A
git commit -m "chore: bump version to $(make version)"

# 4. Create and push tag (triggers release build)
make release-tag
```

The GitHub Action will automatically:
- Build binaries for Linux (x86_64, musl) and macOS (x86_64, ARM64)
- Create a GitHub Release with the binaries
- Generate SHA256 checksums

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
