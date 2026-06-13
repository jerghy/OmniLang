#!/bin/bash
# CI Smoketest — verifies the editor starts and the critical API path works.
# Run: bash scripts/ci_smoke.sh
set -e

PORT=5199
PROJECT_DIR="build/smoketest"
COUNTER_SRC="$PROJECT_DIR/src/Counter.omni"
PASS=0
FAIL=0

echo "=== OmniLang Editor CI Smoketest ==="
echo ""

# Setup: ensure test project exists
mkdir -p "$PROJECT_DIR/src"
if [ ! -f "$COUNTER_SRC" ]; then
    cp tests/frameworks/behavior/01_counter.omni "$COUNTER_SRC"
fi

# Start server in background
echo "--- Starting server ---"
python editor/server/main.py --dir "$PROJECT_DIR" --port $PORT &
SERVER_PID=$!
sleep 3

# Check server responded
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "[FAIL] Server failed to start"
    exit 1
fi

cleanup() {
    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

check() {
    local name="$1"
    local url="$2"
    local data="$3"
    local method="${4:-POST}"

    if [ "$method" = "GET" ]; then
        RESP=$(curl -s "http://127.0.0.1:$PORT$url" 2>/dev/null)
    else
        RESP=$(curl -s -X POST "http://127.0.0.1:$PORT$url" \
            -H "Content-Type: application/json" \
            -d "$data" 2>/dev/null)
    fi

    if echo "$RESP" | python -c "$5" 2>/dev/null; then
        echo "[PASS] $name"
        PASS=$((PASS + 1))
    else
        echo "[FAIL] $name — response: $(echo $RESP | head -c 200)"
        FAIL=$((FAIL + 1))
    fi
}

# 1. Project info
check "Project info" "/api/project" "" "GET" \
    "import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get('mainFile') else 1)"

# 2. File list
check "File list" "/api/fs/list" '{"dirPath":"."}' \
    "import sys,json; d=json.load(sys.stdin); sys.exit(0 if len(d.get('entries',[]))>0 else 1)"

# 3. Read file
SOURCE=$(curl -s -X POST "http://127.0.0.1:$PORT/api/fs/read" \
    -H "Content-Type: application/json" \
    -d '{"filePath":"src/Counter.omni"}' | python -c "import sys,json; print(json.load(sys.stdin).get('content',''))")
if [ ${#SOURCE} -gt 10 ]; then
    echo "[PASS] Read file (${#SOURCE} chars)"
    PASS=$((PASS + 1))
else
    echo "[FAIL] Read file"
    FAIL=$((FAIL + 1))
fi

# 4. Compile
check "Compile" "/api/compile" "{\"source\":$(echo "$SOURCE" | python -c "import sys,json; print(json.dumps(sys.stdin.read()))"),\"target\":\"react\"}" \
    "import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get('success') and 'Counter.tsx' in d.get('files',{}) else 1)"

# 5. Export .omni.json
EXPORT_RESP=$(curl -s -X POST "http://127.0.0.1:$PORT/api/omni-json/export" \
    -H "Content-Type: application/json" \
    -d "{\"source\":$(echo "$SOURCE" | python -c "import sys,json; print(json.dumps(sys.stdin.read()))"),\"filepath\":\"Counter.omni\",\"target\":\"react\"}")
echo "$EXPORT_RESP" | python -c "
import sys,json
d=json.load(sys.stdin)
if d.get('success') and len(d.get('document',{}).get('components',[]))>0:
    print('[PASS] Export .omni.json')
else:
    print('[FAIL] Export .omni.json')
    sys.exit(1)
" && PASS=$((PASS + 1)) || FAIL=$((FAIL + 1))

# 6. Save .omni.json
echo "$EXPORT_RESP" | python -c "
import sys,json,urllib.request
d=json.load(sys.stdin)['document']
req=urllib.request.Request('http://127.0.0.1:$PORT/api/omni-json/save',
    data=json.dumps({'filePath':'project.omni.json','document':d}).encode(),
    headers={'Content-Type':'application/json'})
with urllib.request.urlopen(req, timeout=10) as r:
    d2=json.loads(r.read())
    if d2.get('ok'):
        print('[PASS] Save .omni.json')
    else:
        print('[FAIL] Save .omni.json')
" && PASS=$((PASS + 1)) || FAIL=$((FAIL + 1))

# 7. Load .omni.json + 8. Import round-trip
LOADED=$(curl -s -X POST "http://127.0.0.1:$PORT/api/omni-json/load" \
    -H "Content-Type: application/json" \
    -d '{"filePath":"project.omni.json"}')
echo "$LOADED" | python -c "
import sys,json,urllib.request
d=json.load(sys.stdin)
if not d.get('success'):
    print('[FAIL] Load .omni.json')
    sys.exit(1)
print('[PASS] Load .omni.json')
# Import
req=urllib.request.Request('http://127.0.0.1:$PORT/api/omni-json/import',
    data=json.dumps({'document':d['document']}).encode(),
    headers={'Content-Type':'application/json'})
with urllib.request.urlopen(req, timeout=10) as r:
    imp=json.loads(r.read())
    if imp.get('success') and len(imp.get('source',''))>10:
        print('[PASS] Import .omni.json→.omni')
        # Verify reconstructed compiles
        req2=urllib.request.Request('http://127.0.0.1:$PORT/api/compile',
            data=json.dumps({'source':imp['source'],'target':'react'}).encode(),
            headers={'Content-Type':'application/json'})
        with urllib.request.urlopen(req2, timeout=10) as r2:
            c=json.loads(r2.read())
            if c.get('success'):
                print('[PASS] Round-trip compiles')
            else:
                print('[FAIL] Round-trip compiles')
                sys.exit(1)
    else:
        print('[FAIL] Import .omni.json→.omni')
" && PASS=$((PASS + 3)) || FAIL=$((FAIL + 1))

# 9. Hand-written detection
check "Hand-written detection" "/api/omni-json/detect-handwritten" \
    '{"source":"// my code\ncomponent Foo { view { <Text>hi</Text> } }"}' \
    "import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get('isHandWritten') else 1)"

# 10. @native ports
check "@native ports" "/api/parse-native-ports" \
    '{"source":"// @in: videoUrl (String)\n// @out: isPlaying (bool)\n@native(react) {\n  const ref = useRef(null);\n}"}' \
    "import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get('success') and len(d.get('ports',[]))>0 else 1)"

echo ""
echo "=== Result: $PASS passed, $FAIL failed ==="
if [ $FAIL -gt 0 ]; then
    echo "SMOKETEST FAILED"
    exit 1
else
    echo "SMOKETEST PASSED"
    exit 0
fi
