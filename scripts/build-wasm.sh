#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════
# build-wasm.sh — Build the Rust omni-compiler to WASM for the editor
# ═══════════════════════════════════════════════════════════════
# This script compiles omni-compiler (Rust) → wasm32-unknown-unknown
# and places the output in editor/public/wasm/ for the Vite dev server.
#
# Prerequisites:
#   wasm-pack  (install: cargo install wasm-pack)
#   rustup target add wasm32-unknown-unknown
#
# Notes:
#   - wasm-pack 0.13.x defaults to release profile already (the --release
#     flag is implicit).  Passing --release to this version causes cargo
#     to receive it twice, so we never pass it explicitly.
#   - wasm-opt is disabled (Cargo.toml: wasm-opt = false) due to
#     bulk-memory incompatibility with the current toolchain.
#
# Usage:
#   bash scripts/build-wasm.sh          # release build (default)
#   bash scripts/build-wasm.sh --debug  # debug build (faster iteration)
# ═══════════════════════════════════════════════════════════════

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

COMPILER_DIR="$PROJECT_DIR/omni-compiler"
OUT_DIR="$PROJECT_DIR/editor/public/wasm"
PROFILE="${1:-release}"

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║     OmniLang — WASM Build                                ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "  Source:  $COMPILER_DIR"
echo "  Output:  $OUT_DIR"
echo "  Profile: $PROFILE"
echo ""

# ── Ensure wasm-pack is available ──
if ! command -v wasm-pack &>/dev/null; then
    echo "ERROR: wasm-pack not found."
    echo "  Install with: cargo install wasm-pack"
    echo "  Also: rustup target add wasm32-unknown-unknown"
    exit 1
fi
echo "[OK] wasm-pack $(wasm-pack --version)"

# ── Ensure wasm32 target is installed ──
if ! rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    echo "[INFO] Adding wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# ── Clean output dir (wasm-pack appends, we want clean state) ──
rm -rf "$OUT_DIR"/*.wasm "$OUT_DIR"/*.js "$OUT_DIR"/*.ts "$OUT_DIR"/*.json

# ── Build ──
echo ""
echo "── Building ──"

WASM_FLAGS=()
if [ "$PROFILE" = "debug" ]; then
    WASM_FLAGS+=(--dev)
    echo "  Mode: debug"
else
    # wasm-pack 0.13.x defaults to release already; no --release needed.
    echo "  Mode: release (LTO, stripped, optimized for size)"
fi

wasm-pack build "$COMPILER_DIR" \
    --out-dir "$OUT_DIR" \
    --target web \
    --features wasm \
    "${WASM_FLAGS[@]}"

# ── Post-build: remove generated .gitignore ──
rm -f "$OUT_DIR/.gitignore"

# ── Verify ──
echo ""
echo "── Verification ──"
WASM_FILE=$(ls "$OUT_DIR"/*.wasm 2>/dev/null | head -1)
if [ -n "$WASM_FILE" ]; then
    SIZE=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE" 2>/dev/null)
    echo "  [OK] $(basename "$WASM_FILE")  ($(( SIZE / 1000 )) KB)"
    echo "  Files:"
    ls -lh "$OUT_DIR"/
else
    echo "  [FAIL] No .wasm file produced!"
    exit 1
fi

echo ""
echo "✅ WASM build complete."
echo "   The editor will pick up the new compiler on next vite restart."
