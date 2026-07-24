#!/usr/bin/env bash
# Start Poiesis Storybook and wait until it's ready.
# Writes logs to /tmp/storybook.log, final status to stdout.
set -euo pipefail

PROJECT_ROOT="/d/space/OmniLang"
LOG_FILE="/tmp/storybook.log"
PORT=6006

echo "[$(date '+%H:%M:%S')] Starting Poiesis Storybook..." > "$LOG_FILE"

cd "$PROJECT_ROOT"

# Kill any existing process on port 6006
EXISTING_PID=$(netstat -ano 2>/dev/null | grep ":$PORT " | grep LISTENING | awk '{print $NF}' | head -1 || true)
if [ -n "${EXISTING_PID:-}" ]; then
  echo "[$(date '+%H:%M:%S')] Killing existing process on port $PORT (PID: $EXISTING_PID)..." >> "$LOG_FILE"
  taskkill //F //PID "$EXISTING_PID" 2>/dev/null || true
  sleep 1
fi

# Also try to kill any stale node processes named storybook
STALE_PIDS=$(tasklist 2>/dev/null | grep -i "node" | awk '{print $2}' || true)
if [ -n "${STALE_PIDS:-}" ]; then
  for pid in $STALE_PIDS; do
    CMD=$(tasklist //FI "PID eq $pid" //FO CSV 2>/dev/null | grep -i "storybook" || true)
    if [ -n "${CMD:-}" ]; then
      echo "[$(date '+%H:%M:%S')] Killing stale storybook node process (PID: $pid)..." >> "$LOG_FILE"
      taskkill //F //PID "$pid" 2>/dev/null || true
    fi
  done
fi

# Start Storybook in background
npx storybook dev -p "$PORT" --no-open >> "$LOG_FILE" 2>&1 &
SB_PID=$!
echo "[$(date '+%H:%M:%S')] storybook PID: $SB_PID" >> "$LOG_FILE"

# Wait for port 6006 to be ready (max 60s)
for i in $(seq 1 30); do
  sleep 2
  if curl -s -o /dev/null -w "%{http_code}" "http://localhost:$PORT" 2>/dev/null | grep -q "200"; then
    echo "[$(date '+%H:%M:%S')] Storybook is ready on http://localhost:$PORT" >> "$LOG_FILE"
    echo "Storybook is running on http://localhost:$PORT"
    exit 0
  fi
done

echo "ERROR: Storybook did not start within 60s. Check $LOG_FILE"
exit 1
