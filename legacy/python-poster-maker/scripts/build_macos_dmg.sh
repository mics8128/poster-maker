#!/usr/bin/env bash
set -euo pipefail

APP_NAME="PosterMaker"
VERSION="${VERSION:-0.1.0}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

python -m pip install --upgrade pip
python -m pip install -e . pyinstaller
pyinstaller --noconfirm --clean poster_maker.spec
python scripts/prune_pyinstaller_bundle.py

mkdir -p release
DMG_DIR="build/dmg"
rm -rf "$DMG_DIR"
mkdir -p "$DMG_DIR"
cp -R "dist/${APP_NAME}.app" "$DMG_DIR/"
ln -s /Applications "$DMG_DIR/Applications"
hdiutil create -volname "Poster Maker ${VERSION}" \
  -srcfolder "$DMG_DIR" \
  -ov -format UDZO \
  "release/PosterMaker-${VERSION}-macos.dmg"
