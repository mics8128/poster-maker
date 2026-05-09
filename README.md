# Poster Maker

Poster Maker 是一個跨平台海報分割工具。把一張圖片或單頁 PDF 切成多張 A4，列印後可剪裁、重疊、拼貼成大海報。

目前版本：**v0.1.0**

## 下載

到 GitHub Releases 下載：

- macOS：`PosterMaker-0.1.0-macos.dmg`
- Windows：`PosterMaker-0.1.0-windows-installer.exe` 或 `PosterMaker-0.1.0-windows.zip`

> macOS 第一次開啟若被 Gatekeeper 擋下，請在 Finder 對 App 按右鍵 → Open / 打開。

## 功能

- 支援來源：PDF、PNG、JPG、JPEG、WEBP、BMP、TIFF
- 輸出：多頁 A4 PDF
- 常用張數：2x1、2x2、3x2 / 2x3、3x3、4x3 / 3x4、4x4
- 最佳擺放：自動判斷 A4 直向/橫向與欄列方向
  - 例如 `3x2` 與 `2x3` 會視為同一個 6 張 A4 需求，自動選最佳結果
- 預覽：顯示 A4 分頁、接縫、重疊範圍
- 尺寸估算：顯示成品圖面長寬與 A4 總外框，單位公分
- 剪裁/拼貼輔助：
  - 淡黑色外框裁切輔助線
  - 紅色內部裁切線，可蓋在圖片上
  - 裁切線與對齊位置末端有紅色「框框 + X」
  - 預設不輸出文字標籤，避免干擾圖片

## 使用方式

1. 開啟 Poster Maker
2. 選擇來源圖片或 PDF
3. 選擇輸出 PDF 位置
4. 選 A4 張數，例如 `3x2 / 2x3`
5. 保持「最佳擺放」開啟
6. 按「產生海報 PDF」
7. 用一般 PDF 軟體列印，列印時請選：
   - 紙張：A4
   - 縮放：實際大小 / 100%
   - 不要使用「符合頁面」或「縮小到可列印範圍」

## 預設建議參數

一般情況使用預設即可：

- 最佳擺放：開
- 重疊區：10 mm
- 邊界：8 mm
- 圖片 DPI：200
- 淡黑外框裁切輔助：開
- 紅色裁切線與 X 框：開
- 頁面文字標籤：關

進階選項預設隱藏，需要時可展開「進階」。

## CLI

安裝開發版後可使用 CLI：

```bash
poster-maker-cli input.pdf output.pdf --grid 3x2
poster-maker-cli photo.jpg poster.pdf --grid 2x2 --overlap-mm 10 --margin-mm 8
poster-maker-cli input.pdf output.pdf --grid 2x3 --no-auto-layout
poster-maker-cli input.pdf output.pdf --grid 2x2 --labels
```

常用參數：

- `--grid 3x2`：A4 欄列數。預設會自動嘗試反向欄列取得最佳擺放。
- `--no-auto-layout`：不要自動交換欄列。
- `--overlap-mm 10`：重疊區大小。
- `--margin-mm 8`：頁面邊界。
- `--landscape`：關閉最佳擺放時可強制 A4 橫向。
- `--labels`：輸出頁面文字標籤。

## 開發執行

macOS / Linux：

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
poster-maker
```

Windows PowerShell：

```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
poster-maker
```

## 本機打包

macOS DMG：

```bash
VERSION=0.1.0 bash scripts/build_macos_dmg.sh
```

輸出：

```text
release/PosterMaker-0.1.0-macos.dmg
```

Windows ZIP / Installer：

```powershell
$env:VERSION="0.1.0"
.\scripts\build_windows.ps1
```

輸出：

```text
release/PosterMaker-0.1.0-windows.zip
release/PosterMaker-0.1.0-windows-installer.exe
```

Installer 需要 Inno Setup；若沒有 Inno Setup，仍會產生 ZIP。

## GitHub Release / CI

此專案使用 GitHub Actions 在 tag 發布時自動打包 macOS 與 Windows。

發布 v0.1.0：

```bash
git tag v0.1.0
git push origin master --tags
```

CI 會產生並上傳：

- `PosterMaker-0.1.0-macos.dmg`
- `PosterMaker-0.1.0-windows.zip`
- `PosterMaker-0.1.0-windows-installer.exe`

也可在 GitHub Actions 手動執行 `Release` workflow，輸入版本 `v0.1.0`。

## 技術

- Python 3.10+
- PySide6：GUI
- PyMuPDF：PDF 讀寫與裁切
- Pillow：圖片讀取
- PyInstaller：桌面 App 打包
