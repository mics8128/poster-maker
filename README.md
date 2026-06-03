# Poster Maker

Poster Maker 是一個海報分割工具：選一張圖片，輸出多頁 A4 PDF，列印後裁切、重疊、拼貼成大海報。

目前主線是 **Rust + Tauri image-only**。舊 Python 版已從工作樹移除，需要時可從 git history 取回。

## 下載

最新版本：`v0.2.4`

GitHub Releases：

```text
https://github.com/mics8128/poster-maker/releases
```

目前提供：

- macOS Apple Silicon DMG
- Windows NSIS installer

暫不提供：

- CLI release artifacts
- Windows MSI installer
- macOS Intel build
- macOS notarization
- PDF 輸入

### macOS 開啟說明

app 是 ad-hoc signed、未經 Apple notarization，所以從網路下載後會被 Gatekeeper 標記為 quarantine。第一次開啟請任選一種方式：

- 右鍵點 app → 打開 → 在警告視窗按「打開」。
- 或在終端機移除 quarantine 屬性：

  ```bash
  xattr -dr com.apple.quarantine "/Applications/Poster Maker.app"
  ```

> 若出現「Poster Maker 已損毀，無法打開」(damaged)，通常是下載的 app 沒有有效簽章造成的；v0.2.4 起 CI release 已對 app 做 ad-hoc 簽章修正此問題。若仍遇到，執行上面的 `xattr` 指令即可。

正式公開版之後再做 Apple notarization。

## 功能

- 輸入圖片：PNG / JPG / JPEG / WEBP / BMP / TIFF
- 輸出：多頁 A4 PDF
- 輸出尺寸：2x1 / 1x2、2x2、3x2 / 2x3、3x3、4x3 / 3x4、4x4、自訂張數、指定 mm 尺寸自動計算張數
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

## macOS 打包

```bash
VERSION=0.2.4 scripts/build_macos_alpha.sh
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
scripts/build_macos_alpha.sh local macOS packaging
scripts/sync-version.mjs     version sync helper
```

## 已知限制 / 下一步

- macOS app 目前 ad-hoc signed，未 notarized。
- Windows prerelease tag 若恢復使用，MSI 需要設定 `bundle.windows.wix.version` 成數字版號並重新檢查升級語意。
- macOS Intel 暫不 build。
- PDF 輸入留到第二階段。
- 還需要 fixture-based PDF smoke test：產生 PDF 後驗證頁數與 A4 尺寸。
- 沒有 auto-updater，目前只用 GitHub Releases。
