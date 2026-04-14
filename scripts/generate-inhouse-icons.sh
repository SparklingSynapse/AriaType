#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ICON_DIR="$ROOT_DIR/apps/desktop/assets/icons/inhouse"
ICONSET_DIR="$ICON_DIR/icon.iconset"
BASE_LOGO="$ROOT_DIR/apps/desktop/assets/logo.png"
BASE_TRAY="$ROOT_DIR/apps/desktop/src-tauri/assets/tray-icon.png"
TRAY_OUT="$ROOT_DIR/apps/desktop/src-tauri/assets/tray-icon-inhouse.png"

rm -rf "$ICON_DIR"
mkdir -p "$ICONSET_DIR"

magick "$BASE_LOGO" \
  -fill "#ede8df" -stroke "#ede8df" -strokewidth 32 -draw "circle 798,798 900,798" \
  -fill "#22c55e" -stroke "#ede8df" -strokewidth 18 -draw "circle 798,798 888,798" \
  "$ICON_DIR/icon-1024.png"

for size in 16 32 64 128 256 512; do
  magick "$ICON_DIR/icon-1024.png" -resize "${size}x${size}" "$ICON_DIR/${size}x${size}.png"
done

cp "$ICON_DIR/16x16.png" "$ICONSET_DIR/icon_16x16.png"
cp "$ICON_DIR/32x32.png" "$ICONSET_DIR/icon_16x16@2x.png"
cp "$ICON_DIR/32x32.png" "$ICONSET_DIR/icon_32x32.png"
cp "$ICON_DIR/64x64.png" "$ICONSET_DIR/icon_32x32@2x.png"
cp "$ICON_DIR/128x128.png" "$ICONSET_DIR/icon_128x128.png"
cp "$ICON_DIR/256x256.png" "$ICONSET_DIR/icon_128x128@2x.png"
cp "$ICON_DIR/256x256.png" "$ICONSET_DIR/icon_256x256.png"
cp "$ICON_DIR/512x512.png" "$ICONSET_DIR/icon_256x256@2x.png"
cp "$ICON_DIR/512x512.png" "$ICONSET_DIR/icon_512x512.png"
cp "$ICON_DIR/icon-1024.png" "$ICONSET_DIR/icon_512x512@2x.png"

iconutil -c icns "$ICONSET_DIR" -o "$ICON_DIR/icon.icns"
magick \
  "$ICON_DIR/16x16.png" \
  "$ICON_DIR/32x32.png" \
  "$ICON_DIR/64x64.png" \
  "$ICON_DIR/128x128.png" \
  "$ICON_DIR/256x256.png" \
  "$ICON_DIR/512x512.png" \
  "$ICON_DIR/icon.ico"
cp "$ICON_DIR/512x512.png" "$ICON_DIR/icon.png"
rm -rf "$ICONSET_DIR"

magick "$BASE_TRAY" -alpha on -background none -fill black -draw "circle 24,24 28,24" "PNG32:$TRAY_OUT"
