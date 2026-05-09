# Poster Maker

Poster Maker 是一個海報分割工具：選一張圖片，輸出多頁 A4 PDF，列印後裁切、重疊、拼貼成大海報。

目前主線是 **Rust + Tauri image-only alpha**。舊 Python 版已從工作樹移除，需要時可從 git history 取回。

## 下載

最新 alpha：`v0.2.0-alpha.2`

GitHub Releases：

```text
https://github.com/mics8128/poster-maker/releases
```

目前提供：

- macOS Apple Silicon DMG
- macOS Apple Silicon CLI

暫不提供：

- Windows build：等 Windows 本機驗證；之前 MSI 不能吃 `0.2.0-alpha.N` 版本號
- macOS Intel build
- macOS notarization
- PDF 輸入

macOS 若被 Gatekeeper 擋下，請右鍵 → 打開。正式公開版之後再做 Apple notarization。

## 功能

- 輸入圖片：PNG / JPG / JPEG / WEBP / BMP / TIFF
- 輸出：多頁 A4 PDF
- A4 張數：2x1 / 1x2、2x2、3x2 / 2x3、3x3、4x3 / 3x4、4x4、自訂
- 自動最佳擺放：自動判斷欄列反向與 A4 直/橫向
- 預覽：顯示實際切片圖片、A4 頁面、裁切線、X 對齊框
- 預設：重疊 5mm，邊界 3mm
- 輸出檔名只填檔名，預設存在來源圖片同資料夾
- 覆蓋既有 PDF 前會警告

## 開發

安裝依賴：

```bash
pnpm install
```

跑 GUI：

```bash
pnpm tauri dev
```

測試 / build：

```bash
pnpm build
cd src-tauri && cargo test
```

如果 dev server 卡住：

```bash
pkill -f "tauri dev" || true
pkill -f "target/debug/poster-maker" || true
pkill -f "vite --host 127.0.0.1" || true
pnpm tauri dev
```

清理本機產物：

```bash
pnpm clean
```

會刪：

```text
dist/
src-tauri/target/
release/
舊實驗用 cache 路徑
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

## macOS alpha 打包

```bash
VERSION=0.2.0-alpha.2 scripts/build_macos_alpha.sh
```

這個腳本會：

- 同步版本號
- build Apple Silicon Tauri app
- build CLI
- ad-hoc codesign
- 重新打 DMG
- 保留 Applications 捷徑
- 印 SHA256

## 版本號

版本號集中在：

```text
package.json
```

同步到 Tauri / Cargo：

```bash
pnpm sync-version
cargo update --manifest-path src-tauri/Cargo.toml -p poster-maker
```

前端直接讀 `package.json` 版本號。

## 專案結構

```text
src/                         Svelte GUI
src-tauri/src/layout.rs      layout / best fit
src-tauri/src/pdf_output.rs  shared preview/PDF geometry + minimal PDF writer
src-tauri/src/cli.rs         CLI entrypoint
scripts/build_macos_alpha.sh local macOS alpha packaging
scripts/sync-version.mjs     version sync helper
```

## 已知限制 / 下一步

- macOS app 目前 ad-hoc signed，未 notarized。
- Windows release 暫停，需本機驗證 NSIS/zip 或改用 MSI-compatible 版本號。
- macOS Intel 暫不 build。
- PDF 輸入留到第二階段。
- 還需要 fixture-based PDF smoke test：產生 PDF 後驗證頁數與 A4 尺寸。
- 沒有 auto-updater，目前只用 GitHub Releases。
