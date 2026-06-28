#!/bin/bash
set -euo pipefail

echo "==> Building omnilang WASM..."
wasm-pack build omnilang \
  --out-dir ../poiesis/public/wasm \
  --target web \
  --features wasm

echo "==> WASM built to poiesis/public/wasm/"
