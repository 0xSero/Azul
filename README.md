# Azul Browse

[![CI](https://github.com/0xSero/Azul/actions/workflows/ci.yml/badge.svg)](https://github.com/0xSero/Azul/actions/workflows/ci.yml)
[![Release](https://github.com/0xSero/Azul/actions/workflows/release.yml/badge.svg)](https://github.com/0xSero/Azul/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A beautiful terminal web browser with AI-powered chat, built in Rust.

## Features

- **Native Windows Support:** Fixed input lag, double-typing issues, and system "open" command compatibility.
- **MiniMax-M2.1 Integration:** Built-in support for MiniMax AI with reasoning capabilities.
- **Persistent Chat:** AI conversations are automatically saved and loaded between sessions.
- **Zen Mode:** Press `z` to toggle fullscreen focus mode for reading papers and articles.
- **Local File Access:** AI tools to read and analyze local files on your machine.
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

### From Releases (Recommended)

Download the latest binary for your platform from [Releases](https://github.com/nice-bills/Azul/releases):

```powershell
# Windows (x86_64) - PowerShell
Invoke-WebRequest -Uri "https://github.com/nice-bills/Azul/releases/latest/download/azul-windows-x86_64.zip" -OutFile "azul.zip"
Expand-Archive -Path azul.zip -DestinationPath .
# Move azul.exe to a folder in your PATH (e.g. C:\Windows\system32 or a custom bin folder)
```

```bash
# Linux (x86_64)
curl -LO https://github.com/nice-bills/Azul/releases/latest/download/azul-linux-x86_64.tar.gz
tar xzf azul-linux-x86_64.tar.gz
sudo mv azul /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/nice-bills/Azul/releases/latest/download/azul-darwin-aarch64.tar.gz
tar xzf azul-darwin-aarch64.tar.gz
sudo mv azul /usr/local/bin/

# macOS (Intel)
curl -LO https://github.com/nice-bills/Azul/releases/latest/download/azul-darwin-x86_64.tar.gz
tar xzf azul-darwin-x86_64.tar.gz
sudo mv azul /usr/local/bin/
```

### From Source

```bash
# Clone the repo
git clone https://github.com/nice-bills/Azul.git
cd Azul

# Quick install (Windows)
cargo build --release
copy target\release\azul.exe %USERPROFILE%\.cargo\bin\azul.exe

# Quick install (Unix)
make install-user     # (~/.local/bin)
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
    "provider": "minimax",
    "api_key": "YOUR_API_KEY",
    "model": "MiniMax-M2.1",
    "base_url": "https://api.minimax.io/v1"
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
| MiniMax | `https://api.minimax.io/v1` |
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
| `z` | Toggle Zen Mode (Fullscreen content) |
| `Escape` | Cancel/unfocus |
| `Ctrl+[` | Alternative Escape (useful for broken keys) |

### Content Navigation
| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Tab` (in content) | Next link |
| `Enter` | Follow selected link |
| `o` | Open current URL in default system browser |
| `1-9` | Follow link by number |

### Chat Panel
| Key | Action |
|-----|--------|
| `Up` / `PageUp` | Scroll up (older messages) |
| `Down` / `PageDown` | Scroll down (newer messages) |
| `Enter` | Send message |
| `/clear` | Type this in chat to wipe history |
| `Ctrl+[` | Exit chat focus |
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
- **Read local files** (e.g. "read C:\docs\notes.md")
- **Save reports** (e.g. "save a summary to report.md")

Example prompts:
- "Go to wikipedia.org"
- "Click on the first link"
- "Scroll down to see more"
- "Summarize this page"
- "Research X and save a report to research.md"

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
