#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PKG_DIR="$SCRIPT_DIR/pkg"

echo "Building swatchthis WASM package..."
wasm-pack build "$PROJECT_ROOT" \
    --target web \
    --out-dir "$PKG_DIR" \
    --features wasm

# Run system wasm-opt since wasm-pack's bundled version is often outdated
WASM_FILE="$PKG_DIR/swatchthis_bg.wasm"
if command -v wasm-opt &> /dev/null; then
    echo "Optimising with wasm-opt $(wasm-opt --version)..."
    wasm-opt "$WASM_FILE" -O3 --enable-bulk-memory --enable-nontrapping-float-to-int -o "$WASM_FILE"
else
    echo "Warning: wasm-opt not found on PATH, skipping optimisation"
fi

echo ""
echo "Build complete! Serve the demo with:"
echo "  cd $SCRIPT_DIR && python3 -m http.server 8080"
echo "  Then open http://localhost:8080"
