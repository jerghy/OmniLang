#!/bin/bash
set -euo pipefail

# Resolve script dir to find the repo root (one level up from scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building omnilang-wasm WASM..."
cd "$REPO_ROOT"
WASM_PACK_SKIP_VALIDATE=1 wasm-pack build omnilang-wasm \
  --out-dir ../poiesis/public/wasm \
  --target web

echo "==> WASM built to poiesis/public/wasm/"

# Clean old files from src/wasm/ (they have different names now)
rm -f "$REPO_ROOT/poiesis/src/wasm/omnilang.js"
rm -f "$REPO_ROOT/poiesis/src/wasm/omnilang_bg.wasm"
rm -f "$REPO_ROOT/poiesis/src/wasm/omnilang.d.ts"
rm -f "$REPO_ROOT/poiesis/src/wasm/omnilang_bg.wasm.d.ts"
rm -f "$REPO_ROOT/poiesis/src/wasm/omnilang_bg.js"
rm -f "$REPO_ROOT/poiesis/src/wasm/package.json"

# Also sync to src/wasm/ for Vite module resolution (import '../wasm/omnilang_wasm.js' resolves here)
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_wasm_bg.wasm" "$REPO_ROOT/poiesis/src/wasm/omnilang_wasm_bg.wasm"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_wasm.js" "$REPO_ROOT/poiesis/src/wasm/omnilang_wasm.js"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_wasm.d.ts" "$REPO_ROOT/poiesis/src/wasm/omnilang_wasm.d.ts"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_wasm_bg.wasm.d.ts" "$REPO_ROOT/poiesis/src/wasm/omnilang_wasm_bg.wasm.d.ts"
echo "==> Synced to poiesis/src/wasm/"
