# Poster Maker notes

## Current state

- Tauri/Svelte/Rust image-only alpha.
- GUI and CLI exist.
- Preview and PDF output share Rust-generated geometry; frontend should not reimplement layout math.
- macOS Apple Silicon alpha release exists.
- Old Python version was removed from the working tree; retrieve from git history if needed.

## Known limitations

1. **macOS app is ad-hoc signed, not notarized**
   - Alpha users may need right-click Open or remove quarantine.
   - Public release needs Apple Developer ID signing + notarization.

2. **Windows release deferred**
   - GitHub Windows MSI build failed because MSI does not accept `0.2.0-alpha.N` prerelease versions.
   - Verify on Windows locally and choose NSIS/zip or MSI-compatible stable versioning.

3. **macOS Intel not built**
   - Apple Silicon only for alpha.

4. **Need automated artifact smoke test**
   - Generate a PDF from a fixture image.
   - Validate page count and A4 dimensions.

5. **No auto-updater**
   - GitHub releases only.

## Release notes

- Version source: `package.json`.
- Sync version: `pnpm sync-version`.
- Local macOS alpha package: `VERSION=0.2.0-alpha.N scripts/build_macos_alpha.sh`.
