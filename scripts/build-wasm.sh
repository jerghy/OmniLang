#!/bin/bash
set -euo pipefail

# Resolve script dir to find the repo root (one level up from scripts/)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building omnilang WASM..."
cd "$REPO_ROOT"
WASM_PACK_SKIP_VALIDATE=1 wasm-pack build omnilang \
  --out-dir ../poiesis/public/wasm \
  --target web \
  --features "wasm,codegen-omniview"

echo "==> WASM built to poiesis/public/wasm/"

# Also sync to src/wasm/ for Vite module resolution (import '../wasm/omnilang.js' resolves here)
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_bg.wasm" "$REPO_ROOT/poiesis/src/wasm/omnilang_bg.wasm"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang.js" "$REPO_ROOT/poiesis/src/wasm/omnilang.js"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang.d.ts" "$REPO_ROOT/poiesis/src/wasm/omnilang.d.ts"
cp -f "$REPO_ROOT/poiesis/public/wasm/omnilang_bg.wasm.d.ts" "$REPO_ROOT/poiesis/src/wasm/omnilang_bg.wasm.d.ts"
echo "==> Synced to poiesis/src/wasm/"
