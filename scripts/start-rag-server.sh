#!/bin/bash
# Start the simple RAG server for azul-browse
# This runs in the background on port 8766

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PID_FILE="$HOME/.azul-rag-server.pid"
LOG_FILE="$HOME/.azul-rag-server.log"

# Check if already running
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        echo "RAG server already running (PID: $PID)"
        exit 0
    fi
fi

# Start server
echo "Starting RAG server on http://127.0.0.1:8766..."
nohup python3 "$SCRIPT_DIR/simple-rag-server.py" > "$LOG_FILE" 2>&1 &
PID=$!
echo $PID > "$PID_FILE"

# Wait a moment and check if it started
sleep 1
if ps -p "$PID" > /dev/null 2>&1; then
    echo "RAG server started successfully (PID: $PID)"
    echo "Logs: $LOG_FILE"
else
    echo "Failed to start RAG server"
    rm -f "$PID_FILE"
    exit 1
fi
