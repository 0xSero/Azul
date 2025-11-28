#!/bin/bash
# Azul Browse - Startup Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$HOME/.config/azul"
CONFIG_FILE="$CONFIG_DIR/config.json"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}Azul Browse${NC} - Terminal Web Browser"
echo ""

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Rust not found. Install from https://rustup.rs/${NC}"
    exit 1
fi

# Create config directory if needed
if [ ! -d "$CONFIG_DIR" ]; then
    echo -e "${GREEN}Creating config directory...${NC}"
    mkdir -p "$CONFIG_DIR"
fi

# Create default config if not exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${GREEN}Creating default config...${NC}"
    cat > "$CONFIG_FILE" << 'EOF'
{
  "theme": {
    "accent": "#00BFFF",
    "border": "#7aa2f7",
    "text": "#c0caf5",
    "background": "#1a1b26"
  },
  "ai": {
    "provider": "openrouter",
    "api_key": "",
    "model": "anthropic/claude-3.5-sonnet",
    "base_url": "https://openrouter.ai/api/v1"
  },
  "refresh_rate_ms": 200
}
EOF
    echo -e "${YELLOW}Config created at $CONFIG_FILE${NC}"
    echo -e "${YELLOW}Add your API key to enable AI chat${NC}"
    echo ""
fi

cd "$PROJECT_DIR"

# Build if needed
if [ ! -f "target/release/azul-browse" ] || [ "$(find src -newer target/release/azul-browse 2>/dev/null | head -1)" ]; then
    echo -e "${GREEN}Building...${NC}"
    cargo build --release
    echo ""
fi

# Run
echo -e "${GREEN}Starting Azul Browse...${NC}"
exec ./target/release/azul-browse "$@"
