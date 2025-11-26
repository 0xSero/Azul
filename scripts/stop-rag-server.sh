#!/bin/bash
# Stop the simple RAG server

PID_FILE="$HOME/.azul-rag-server.pid"

if [ ! -f "$PID_FILE" ]; then
    echo "RAG server is not running (no PID file)"
    exit 0
fi

PID=$(cat "$PID_FILE")

if ps -p "$PID" > /dev/null 2>&1; then
    echo "Stopping RAG server (PID: $PID)..."
    kill "$PID"
    rm -f "$PID_FILE"
    echo "RAG server stopped"
else
    echo "RAG server is not running (stale PID file)"
    rm -f "$PID_FILE"
fi
