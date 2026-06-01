#!/usr/bin/env bash
#
# Builds a clean EpochServer release containing ONLY the x64 variants.
#
# Usage:
#   ./scripts/build-x64-release.sh
#
set -euo pipefail

echo "=== Building clean x64-only EpochServer release ==="

cargo build --release

echo ""
echo "Cleaning any non-x64 artifacts..."

RELEASE_DIR="target/release"

# Remove known non-x64 names
rm -f \
    "${RELEASE_DIR}/epochserver.dll" \
    "${RELEASE_DIR}/epochserver.so" \
    "${RELEASE_DIR}/libepochserver.so" \
    "${RELEASE_DIR}/epochserver_x86"*.dll 2>/dev/null || true

echo ""
echo "Final x64 artifacts:"
ls -lh "${RELEASE_DIR}"/*epochserver_x64* 2>/dev/null || echo "(no x64 files found)"

echo ""
echo "=== Done. Only x64 variants remain. ==="
echo "Main file: ${RELEASE_DIR}/libepochserver_x64.so (or epochserver_x64.dll on Windows)"