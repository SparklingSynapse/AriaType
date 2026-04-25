#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOCK_DIR="$SCRIPT_DIR/../tests/e2e/mocks"
SNAPSHOT_DIR="$SCRIPT_DIR/../tests/e2e/snapshots"

echo "=== Exporting Mock Data from Real Tauri Backend ==="
echo "Mock directory: $MOCK_DIR"

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
  echo "✓ settings.json ($(wc -c < "$MOCK_DIR/settings.json") bytes)"
else
  echo "✗ settings.json (missing)"
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

if [ -f "$MOCK_DIR/shortcut-profiles.json" ]; then
  echo "✓ shortcut-profiles.json ($(wc -c < "$MOCK_DIR/shortcut-profiles.json") bytes)"
else
  echo "✗ shortcut-profiles.json (missing)"
fi

echo ""
echo "=== Export complete ==="