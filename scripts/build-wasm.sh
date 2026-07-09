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
