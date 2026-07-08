#!/bin/bash
set -euo pipefail

echo "==> Building omnilang WASM..."
WASM_PACK_SKIP_VALIDATE=1 wasm-pack build omnilang \
  --out-dir ../poiesis/public/wasm \
  --target web \
  --features "wasm,codegen-omniview"

echo "==> WASM built to poiesis/public/wasm/"
