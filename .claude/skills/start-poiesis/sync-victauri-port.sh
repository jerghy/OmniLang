#!/usr/bin/env bash
# Sync the Victauri port in .mcp.json with the running Poiesis process.
# Called automatically by start-poiesis.sh after Poiesis is confirmed running.
# Can also be run standalone: bash sync-victauri-port.sh
set -euo pipefail

MCP_JSON="/d/space/OmniLang/.mcp.json"
TEMP_VICTAURI="$(cygpath -u "$TEMP" 2>/dev/null || echo "$TEMP")/victauri"

# ── Find Poiesis PID ──
POIESIS_PID=$(tasklist 2>/dev/null | grep -i poiesis.exe | awk '{print $2}' | head -1)
if [ -z "$POIESIS_PID" ]; then
  echo "ERROR: poiesis.exe not running"
  exit 1
fi

# ── Read actual port from Victauri temp file ──
PORT_FILE="$TEMP_VICTAURI/$POIESIS_PID/port"
# Handle Windows-style path
PORT_FILE_WIN=$(cmd //c "echo %TEMP%\\victauri\\$POIESIS_PID\\port" 2>/dev/null || echo "")

if [ -f "$PORT_FILE_WIN" ]; then
  ACTUAL_PORT=$(cat "$PORT_FILE_WIN" | tr -d '[:space:]')
elif [ -f "$PORT_FILE" ]; then
  ACTUAL_PORT=$(cat "$PORT_FILE" | tr -d '[:space:]')
else
  echo "ERROR: port file not found at TEMP/victauri/$POIESIS_PID/port"
  exit 1
fi

if [ -z "$ACTUAL_PORT" ]; then
  echo "ERROR: empty port"
  exit 1
fi

# ── Read current port from .mcp.json ──
if [ ! -f "$MCP_JSON" ]; then
  echo "ERROR: $MCP_JSON not found"
  exit 1
fi

CURRENT_PORT=$(grep '"url":' "$MCP_JSON" | grep -o '127\.0\.0\.1:[0-9]*' | sed 's/.*://' | head -1 || echo "")
if [ -z "$CURRENT_PORT" ]; then
  echo "INFO: No existing url in .mcp.json victauri block"
fi

# ── Compare and update if needed ──
if [ "$ACTUAL_PORT" = "$CURRENT_PORT" ]; then
  echo "Victauri port is correct: $ACTUAL_PORT (no change needed)"
  exit 0
fi

echo "Port mismatch: .mcp.json has $CURRENT_PORT, Poiesis running on $ACTUAL_PORT"

# Update .mcp.json — replace the port in the url line
sed -i "s|\"url\": \"http://127\.0\.0\.1:[0-9]*/mcp\"|\"url\": \"http://127.0.0.1:$ACTUAL_PORT/mcp\"|g" "$MCP_JSON"

echo "Updated .mcp.json to port $ACTUAL_PORT"
