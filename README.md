# Poster Maker

Poster Maker 是一個海報分割工具：選一張圖片，輸出多頁 A4 PDF，列印後裁切、重疊、拼貼成大海報。

目前主線是 **Rust + Tauri image-only alpha**。舊 Python 版已從工作樹移除，需要時可從 git history 取回。

## 目前版本

- 最新 alpha：`v0.2.0-alpha.2`
- 下載：https://github.com/mics8128/poster-maker/releases

目前 release 只提供：

- macOS Apple Silicon DMG
- macOS Apple Silicon CLI

暫不包含：

- Windows build（等 Windows 本機驗證）
- macOS Intel build
- macOS notarization
- PDF 輸入

## 功能

- 輸入圖片：PNG / JPG / JPEG / WEBP / BMP / TIFF
- 輸出：多頁 A4 PDF
- A4 張數：2x1 / 1x2、2x2、3x2 / 2x3、3x3、4x3 / 3x4、4x4、自訂
- 自動最佳擺放：自動判斷欄列反向與 A4 直/橫向
- 預覽：顯示實際切片圖片、A4 頁面、裁切線、X 對齊框
- 預設：重疊 5mm，邊界 3mm
- 輸出檔名只填檔名，預設存在來源圖片同資料夾
- 覆蓋既有 PDF 前會警告

## GUI 開發執行

```bash
pnpm install
pnpm tauri dev
```

如果 dev server 卡住：

```bash
pkill -f "tauri dev" || true
pkill -f "target/debug/poster-maker" || true
pkill -f "vite --host 127.0.0.1" || true
pnpm tauri dev
```

## CLI

```bash
cd src-tauri
cargo run --bin poster-maker-cli -- /path/to/image.jpg
```

指定張數：

```bash
cargo run --bin poster-maker-cli -- /path/to/image.jpg --grid 3x2
```

指定輸出檔名並覆蓋：

```bash
cargo run --bin poster-maker-cli -- /path/to/image.jpg --grid 3x2 -o output.pdf --overwrite
```

CLI 預設輸出到來源圖片同一個資料夾。

## Build

```bash
pnpm build
cd src-tauri && cargo test
pnpm tauri build --target aarch64-apple-darwin
```

## Version / release notes

版本號集中在 `package.json`。

同步到 Tauri / Cargo：

```bash
pnpm sync-version
cargo update --manifest-path src-tauri/Cargo.toml -p poster-maker
```

目前 macOS alpha DMG 需要本機重包以保留 Applications 捷徑與 ad-hoc signing。正式公開版需要 Apple notarization。

## Repo structure

```text
src/                         Svelte GUI
src-tauri/src/layout.rs      layout / best fit
src-tauri/src/pdf_output.rs  shared preview/PDF geometry + minimal PDF writer
src-tauri/src/cli.rs         CLI entrypoint
REVIEW.md                    current limitations / next steps
```

## Known limitations

見 `REVIEW.md`。
