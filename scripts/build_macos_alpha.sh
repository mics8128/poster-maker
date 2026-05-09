#!/usr/bin/env bash
set -euo pipefail

VERSION="${VERSION:-$(node -p "require('./package.json').version")}" 
TARGET="aarch64-apple-darwin"
APP_NAME="Poster Maker"
APP_PATH="src-tauri/target/${TARGET}/release/bundle/macos/${APP_NAME}.app"
DMG_SCRIPT="src-tauri/target/${TARGET}/release/bundle/dmg/bundle_dmg.sh"
STAGING="release/macos-alpha-dmg"
DMG="release/Poster Maker_${VERSION}_aarch64.dmg"
CLI="release/poster-maker-cli-aarch64-apple-darwin-v${VERSION}"

pnpm sync-version
pnpm tauri build --target "${TARGET}"
cargo build --manifest-path src-tauri/Cargo.toml --release --bin poster-maker-cli --target "${TARGET}"

codesign --force --deep --sign - "${APP_PATH}"
codesign --verify --deep --strict --verbose=2 "${APP_PATH}"

rm -rf "${STAGING}" "${DMG}"
mkdir -p "${STAGING}" release
cp -R "${APP_PATH}" "${STAGING}/"

bash "${DMG_SCRIPT}" \
  --volname "${APP_NAME}" \
  --window-size 520 320 \
  --icon-size 96 \
  --icon "${APP_NAME}.app" 145 145 \
  --app-drop-link 375 145 \
  --no-internet-enable \
  "${DMG}" \
  "${STAGING}"

cp "src-tauri/target/${TARGET}/release/poster-maker-cli" "${CLI}"
shasum -a 256 "${DMG}" "${CLI}"
