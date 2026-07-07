#!/usr/bin/env bash
# recover-victauri.sh — Kill stale Victauri bridge + verify REST API
set -euo pipefail

TEMP_DIR="/c/Users/XHT/AppData/Local/Temp/victauri"
log()  { echo "[$(date +%H:%M:%S)] $*"; }
die()  { log "ERROR: $*"; exit 1; }

# ── 1. Find latest Poiesis PID ──────────────────────────────────
log "Finding latest Poiesis process..."

PID=$(tasklist //fi "IMAGENAME eq poiesis.exe" 2>/dev/null \
  | grep poiesis \
  | awk '{print $2}' \
  | sort -n \
  | tail -1)

if [ -z "$PID" ]; then
  # Try ps -W fallback
  PID=$(ps -W 2>/dev/null | grep poiesis.exe | awk '{print $1}' | sort -n | tail -1)
fi

if [ -z "$PID" ]; then
  die "No poiesis.exe process found. Start Poiesis first."
fi

log "Found Poiesis PID: $PID"

# ── 2. Verify temp dir exists ──────────────────────────────────
if [ ! -d "$TEMP_DIR/$PID" ]; then
  # Fallback: find the newest temp dir
  PID=$(ls -t "$TEMP_DIR" 2>/dev/null | head -1)
  if [ -z "$PID" ]; then
    die "No Victauri temp directory found (not started with Victauri?)"
  fi
  log "Using newest temp dir: PID=$PID"
fi

TOKEN_FILE="$TEMP_DIR/$PID/token"
PORT_FILE="$TEMP_DIR/$PID/port"

if [ ! -f "$TOKEN_FILE" ] || [ ! -f "$PORT_FILE" ]; then
  die "Missing token/port file for PID $PID"
fi

TOKEN=$(cat "$TOKEN_FILE")
PORT=$(cat "$PORT_FILE")

# ── 3. Verify the process still owns the port ───────────────────
OWNER=$(netstat -ano 2>/dev/null | grep ":$PORT" | grep LISTENING | awk '{print $5}' | head -1)
if [ "$OWNER" != "$PID" ]; then
  log "WARNING: Port $PORT owned by PID $OWNER, not $PID (may still work)"
fi

log "Victauri port=$PORT token=${TOKEN:0:8}..."

# ── 4. Test REST API health ────────────────────────────────────
HEALTH=$(curl -s http://127.0.0.1:$PORT/health 2>/dev/null || echo "no-health")
if [ "$HEALTH" != '{"status":"ok"}' ]; then
  die "REST API health check failed. Poiesis may not have Victauri."
fi

# Test actual tool via REST API
RESULT=$(curl -s -X POST "http://127.0.0.1:$PORT/api/tools/eval_js" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code":"navigator.userAgent.substring(0,20)"}' 2>/dev/null)

if echo "$RESULT" | grep -q "result"; then
  log "REST API tool call OK: $RESULT"
else
  log "WARNING: REST API returned: $RESULT"
fi

# ── 5. Kill ALL stale victauri.exe bridge processes ────────────
log "Killing stale Victauri bridge process(es)..."

BRIDGE_COUNT=0
while IFS= read -r BPID; do
  if [ -n "$BPID" ]; then
    taskkill //f //pid "$BPID" 2>/dev/null && BRIDGE_COUNT=$((BRIDGE_COUNT + 1)) || true
  fi
done < <(ps -W 2>/dev/null | grep victauri.exe | awk '{print $1}' || true)

if [ "$BRIDGE_COUNT" -gt 0 ]; then
  log "Killed $BRIDGE_COUNT bridge process(es)"
else
  log "No bridge process to kill"
fi

# ── 6. Final check — port still owned by Poiesis ───────────────
sleep 1
NEW_OWNER=$(netstat -ano 2>/dev/null | grep ":$PORT" | grep LISTENING | awk '{print $5}' | head -1)
log "Port $PORT owner after cleanup: PID=$NEW_OWNER"

echo ""
echo "Victauri recovered"
echo "Token: $TOKEN"
