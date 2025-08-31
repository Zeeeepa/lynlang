#!/bin/bash

# Zen Language Server launcher script
# Starts the Zen LSP server for IDE integration

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if zen compiler is available
if ! command -v "$PROJECT_ROOT/target/debug/zen" &> /dev/null && ! command -v "$PROJECT_ROOT/target/release/zen" &> /dev/null; then
    echo -e "${RED}Error: Zen compiler not found. Please build the project first.${NC}"
    echo "Run: cargo build"
    exit 1
fi

# Determine which binary to use
if [ -f "$PROJECT_ROOT/target/release/zen" ]; then
    ZEN_BIN="$PROJECT_ROOT/target/release/zen"
    echo -e "${GREEN}Using release build${NC}"
else
    ZEN_BIN="$PROJECT_ROOT/target/debug/zen"
    echo -e "${YELLOW}Using debug build${NC}"
fi

# Log file for debugging
LOG_FILE="/tmp/zen-lsp.log"
echo "Starting Zen Language Server at $(date)" > "$LOG_FILE"

# Options for the LSP
MODE="${1:-stdio}"  # Default to stdio mode

case "$MODE" in
    stdio)
        echo -e "${GREEN}Starting Zen LSP in stdio mode...${NC}" >> "$LOG_FILE"
        # For now, use the Rust-based LSP implementation
        exec "$ZEN_BIN" lsp 2>> "$LOG_FILE"
        ;;
    
    tcp)
        PORT="${2:-7658}"
        echo -e "${GREEN}Starting Zen LSP on TCP port $PORT...${NC}"
        echo "TCP mode not yet implemented"
        exit 1
        ;;
    
    pipe)
        echo -e "${GREEN}Starting Zen LSP in named pipe mode...${NC}"
        echo "Named pipe mode not yet implemented"
        exit 1
        ;;
    
    *)
        echo "Usage: $0 [stdio|tcp|pipe] [options]"
        echo "  stdio - Communication via stdin/stdout (default)"
        echo "  tcp [port] - TCP server mode (default port: 7658)"
        echo "  pipe - Named pipe mode"
        exit 1
        ;;
esac