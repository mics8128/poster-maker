# Poster Maker Tauri rewrite notes

Rust + Tauri rewrite is now the main implementation.

## Current scope

- Image input only.
- PDF input deferred to phase 2.
- Old Python implementation retained under `legacy/python-poster-maker/`.

## Important decisions

- Preview and PDF output share Rust-generated geometry from `src-tauri/src/pdf_output.rs`.
- Frontend should render geometry, not reimplement layout math.
- Version source is `package.json`; run `pnpm sync-version` before release commits.
- macOS alpha releases are Apple Silicon only for now.
- Windows release is deferred until local Windows packaging is verified.

## Next steps

1. Add fixture-based PDF smoke test.
2. Add repeatable local macOS DMG packaging script with Applications shortcut and ad-hoc signing.
3. Verify Windows packaging locally; decide NSIS/zip vs MSI-compatible stable version.
4. Add PDFium input in phase 2.
5. Add macOS notarization only when ready for public distribution.
