# Poster Maker alpha review

## Current state

- Tauri/Svelte/Rust image-only alpha.
- PDF input intentionally deferred.
- GUI and CLI exist.
- Preview and PDF output now share Rust-generated geometry, so tile/guide positions have a single source of truth.
- Release CI currently builds macOS Apple Silicon only.

## Known limitations / risks

1. **macOS app is ad-hoc signed, not notarized**
   - Users may still need right-click Open or remove quarantine.
   - Proper public release needs Apple Developer ID signing + notarization.

2. **Windows release deferred**
   - GitHub Windows MSI build failed because MSI does not accept `0.2.0-alpha.N` style prerelease versions.
   - Need local Windows verification and either MSI-compatible versioning or NSIS/zip-only Windows release.

3. **macOS Intel not built**
   - Intentionally skipped for alpha. Apple Silicon only.

4. **PDF writer is custom/minimal**
   - Produces valid image tiled PDFs but still needs more smoke tests around page count, page dimensions, and PDF readers.

5. **Image preview uses browser SVG/image rendering**
   - Geometry is shared with Rust, but browser image rendering/cropping can still differ slightly from PDF JPEG crop behavior.

6. **No automated end-to-end artifact test yet**
   - Need command-line smoke test that generates PDF from a fixture image and validates page count / dimensions.

7. **No auto-updater**
   - GitHub releases only.

## Release process notes

- Single source version is `package.json`.
- Run `pnpm sync-version` to sync:
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Frontend reads version from `package.json`.

## Recommended next fixes

1. Add fixture-based PDF smoke test.
2. Add local DMG packaging script that preserves Applications shortcut and re-signs ad-hoc correctly.
3. Decide Windows packaging strategy: NSIS/zip vs MSI-compatible stable version.
4. Add notarization later when ready for general users.
