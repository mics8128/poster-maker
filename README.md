# Poster Maker

PDF/圖片海報分割工具。輸入單頁 PDF 或圖片，輸出多頁 A4 PDF，可選 2x1、2x2、2x3、3x3 等組合，含裁切輔助線、重疊區、標籤。

## 開發執行

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .
poster-maker
```

Windows:

```powershell
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
poster-maker
```

## CLI

```bash
poster-maker-cli input.pdf output.pdf --grid 2x3 --overlap-mm 10 --margin-mm 8 --auto-orientation
poster-maker-cli photo.jpg poster.pdf --grid 3x3 --landscape
poster-maker-cli input.pdf output.pdf --grid 2x2 --no-alignment-guides
poster-maker-cli input.pdf output.pdf --grid 2x2 --labels  # 可選：加頁面標籤文字
```

## 打包桌面 App

```bash
pip install pyinstaller
pyinstaller --noconfirm --windowed --name PosterMaker src/poster_maker/app.py
```

輸出在 `dist/PosterMaker`。

mac 可再包成 `.dmg`，Windows 可用 Inno Setup / NSIS 包 installer。

## 功能

- 來源：PDF、PNG、JPG、WEBP、BMP、TIFF
- 輸出：多頁 A4 PDF
- 組合：預設 2x1、2x2、2x3、3x2、3x3、4x3、4x4，也可自訂 1–12
- 預覽：GUI 右側顯示 A4 分頁、接縫、重疊範圍
- 自動偵測：可自動選 A4 直向/橫向，讓圖用掉最大面積
- 裁切：紅色虛線框 + 角落裁切線
- 標記極簡：預設無任何文字
- 裁切線：紅色，可蓋在圖片上，線末端有紅色「框框 + X」
- 對齊線：藍色，只畫在圖片外，不蓋圖片，線末端有藍色「框框 + X」
- 四邊剪裁線：外側邊界會補齊裁切線，方便最後修邊
- 重疊：每張紙包含相鄰區塊重疊內容，方便黏貼與剪裁
- GUI：PySide6，跨 macOS / Windows / Linux
