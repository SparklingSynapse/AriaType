#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOCK_DIR="$SCRIPT_DIR/../tests/e2e/mocks"

echo "=== Exporting Dynamic E2E Mock Data from Real Tauri Backend ==="
echo "Mock directory: $MOCK_DIR"
echo "Static settings fixture: $MOCK_DIR/settings.json"
echo "Note: settings.json is maintained as a checked-in E2E fixture and is not exported."

mkdir -p "$MOCK_DIR"

# Convert to absolute path
ABS_MOCK_DIR="$(cd "$MOCK_DIR" && pwd)"

echo "Running export-mocks binary..."

cd src-tauri
cargo run --bin export-mocks -- "$ABS_MOCK_DIR" 2>&1
cd ..

echo ""
echo "Checking exported files..."
ls -la "$MOCK_DIR/" 2>&1

if [ -f "$MOCK_DIR/settings.json" ]; then
  echo "• settings.json preserved as static fixture ($(wc -c < "$MOCK_DIR/settings.json") bytes)"
else
  echo "• settings.json fixture missing (expected checked-in fixture)"
fi

if [ -f "$MOCK_DIR/models.json" ]; then
  echo "✓ models.json ($(wc -c < "$MOCK_DIR/models.json") bytes)"
else
  echo "✗ models.json (missing)"
fi

if [ -f "$MOCK_DIR/history.json" ]; then
  echo "✓ history.json ($(wc -c < "$MOCK_DIR/history.json") bytes)"
else
  echo "✗ history.json (missing)"
fi

if [ -f "$MOCK_DIR/dashboard-stats.json" ]; then
  echo "✓ dashboard-stats.json ($(wc -c < "$MOCK_DIR/dashboard-stats.json") bytes)"
else
  echo "✗ dashboard-stats.json (missing)"
fi

echo ""
echo "=== Dynamic mock export complete ==="
