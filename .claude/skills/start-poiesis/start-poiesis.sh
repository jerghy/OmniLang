#!/usr/bin/env bash
# Start Poiesis Tauri app and wait until it's running.
# Writes logs to /tmp/poiesis.log, final status to stdout.
set -euo pipefail

LOG_FILE="/tmp/poiesis.log"
echo "[$(date '+%H:%M:%S')] Starting Poiesis..." > "$LOG_FILE"

cd /d/space/OmniLang/poiesis

# Kill existing poiesis if any
if tasklist 2>/dev/null | grep -qi poiesis.exe; then
  echo "[$(date '+%H:%M:%S')] Killing existing poiesis.exe..." >> "$LOG_FILE"
  taskkill //F //IM poiesis.exe 2>/dev/null || true
  sleep 2
fi

# Kill stale cargo processes that might hold the build lock
CARGO_COUNT=$(tasklist 2>/dev/null | grep -ci cargo.exe || true)
if [ "$CARGO_COUNT" -gt 2 ]; then
  echo "[$(date '+%H:%M:%S')] Killing stale cargo processes..." >> "$LOG_FILE"
  taskkill //F //IM cargo.exe 2>/dev/null || true
  taskkill //F //IM rustc.exe 2>/dev/null || true
  sleep 2
fi

# Start Tauri dev (this runs Vite + Rust build + app)
npx tauri dev >> "$LOG_FILE" 2>&1 &
TAURI_PID=$!
echo "[$(date '+%H:%M:%S')] tauri dev PID: $TAURI_PID" >> "$LOG_FILE"

# Wait for the app window to appear (max 120s)
for i in $(seq 1 60); do
  sleep 2
  if tasklist 2>/dev/null | grep -qi poiesis.exe; then
    echo "[$(date '+%H:%M:%S')] Poiesis is running!" >> "$LOG_FILE"
    # Sync Victauri port in .mcp.json with the running process
    bash /d/space/OmniLang/.claude/skills/start-poiesis/sync-victauri-port.sh >> "$LOG_FILE" 2>&1 || true
    echo "Poiesis is running"
    exit 0
  fi
done

echo "ERROR: Poiesis did not start within 120s. Check $LOG_FILE"
exit 1
