# Poster Maker Rust + Tauri 重寫計劃書

## 目標

把 Poster Maker 從目前的 Python 桌面工具，重做成 Rust + Tauri 版本。

第一階段先做 **image-only**，不支援 PDF 輸入；PDFium 放第二階段。這樣可先驗證：

- 打包大小是否明顯降低
- GUI 操作體驗是否比 Tk/PySide 更好
- Rust 版輸出 PDF 品質是否足夠
- macOS / Windows CI 發布流程是否穩定

目前 Python 版會保留，不直接刪除。建議移到 `legacy/python-poster-maker/` 作為參考與 fallback。

## 背景

目前 v0.1.1 Python 版已完成：

- 圖片/PDF 輸入
- A4 poster tiling
- 最佳擺放
- 預覽
- 成品尺寸估算
- 裁切線與 X 框
- macOS / Windows GitHub Release

但有幾個限制：

- Python standalone app 仍需內含 Python runtime
- PyMuPDF / MuPDF 體積較大
- GUI 從 Qt 改成 Tk 後體積下降，但 UI 質感普通
- 若長期產品化，Rust/Tauri 會更適合維護、測試與打包

目前 v0.1.1 release 大小約：

- macOS DMG：約 37 MB
- Windows installer：約 24 MB
- Windows ZIP：約 30 MB

Rust/Tauri image-only 目標：

- macOS DMG：目標 10–20 MB
- Windows installer：目標 8–20 MB
- 啟動速度比 Python 版快

> 注意：若第二階段加入 PDFium，大小可能回到 20–45 MB，取決於 PDFium 打包方式。

## 範圍

### 第一階段：Rust + Tauri image-only MVP

支援：

- 輸入圖片：PNG / JPG / JPEG / WEBP / BMP / TIFF（視 `image` crate 支援情況確認）
- 輸出：多頁 A4 PDF
- A4 張數：2x1、2x2、3x2 / 2x3、3x3、4x3 / 3x4、4x4、自訂
- 最佳擺放：
  - 自動嘗試欄列反向
  - 自動嘗試 A4 直向/橫向
  - 以最大圖面使用面積為主，比例接近為 tie-breaker
- 預覽：
  - A4 分頁框
  - 圖面範圍
  - 重疊範圍
  - 裁切線位置
- 尺寸估算：
  - 成品圖面，單位公分
  - A4 總外框，單位公分
- 裁切輔助：
  - 淡黑色外框裁切輔助線
  - 紅色內部裁切線
  - 裁切線端點與對齊位置的 X 框
  - 無文字標籤預設
- GitHub Actions：
  - macOS DMG
  - Windows installer / zip
  - GitHub Release

不支援：

- PDF 輸入
- 多頁 PDF 批次處理
- 保留 PDF vector content
- 自動更新
- code signing / notarization（除非後續需要）

### 第二階段：PDFium PDF 輸入

支援：

- PDF 輸入
- PDF 頁碼選擇
- PDF 頁面 render 成 bitmap 後進入相同 tiling pipeline
- render DPI 設定

不保證：

- 保留 PDF vector quality

第二階段方案偏向：

```text
PDFium render -> bitmap -> tile -> output PDF
```

而不是：

```text
PDF vector crop -> output PDF
```

後者難度高很多。

## 建議 repo 結構

重寫時建議改成：

```text
poster-maker/
  legacy/
    python-poster-maker/
      README.md
      pyproject.toml
      src/
      scripts/
      installer/
      poster_maker.spec
  src-tauri/
    Cargo.toml
    tauri.conf.json
    src/
      main.rs
      commands.rs
      layout.rs
      image_io.rs
      pdf_output.rs
      markers.rs
      error.rs
  src/
    App.svelte 或 React App
    components/
    lib/
    styles/
  package.json
  README.md
  TAURI_REWRITE_PLAN.md
  .github/workflows/release.yml
```

也可保留 Python 歷史在 git，不移動完整資料夾；但實務上建議移到 `legacy/python-poster-maker/`，避免新舊打包設定混在一起。

## 技術選型

### Frontend

建議：**Svelte + Tailwind**

原因：

- bundle 小
- 寫表單/狀態簡單
- UI 比 Tk 好做
- 互動預覽畫 canvas 很直覺

可替代：React + Tailwind。若未來團隊較熟 React，也可改 React。

### Desktop Shell

- Tauri v2

### Rust backend crates

第一階段：

```toml
tauri
serde
serde_json
thiserror
anyhow
image
printpdf
rfd
dirs
```

可能需要：

```toml
resvg / tiny-skia  # 若未來支援 SVG 或更複雜繪圖
rayon              # 大圖處理平行化，先不加
```

第二階段：

```toml
pdfium-render
```

## 核心資料模型

### PosterOptions

```rust
struct PosterOptions {
    cols: u32,
    rows: u32,
    overlap_mm: f64,
    margin_mm: f64,
    image_dpi: f64,
    landscape: bool,
    auto_layout: bool,
    draw_outer_marks: bool,
    draw_cut_guides: bool,
}
```

### ResolvedLayout

```rust
struct ResolvedLayout {
    cols: u32,
    rows: u32,
    landscape: bool,
    score: f64,
    page_width_pt: f64,
    page_height_pt: f64,
    image_width_cm: f64,
    image_height_cm: f64,
    paper_width_cm: f64,
    paper_height_cm: f64,
}
```

### TilePlan

```rust
struct TilePlan {
    row: u32,
    col: u32,
    page_rect_pt: Rect,
    content_rect_pt: Rect,
    source_crop_px: Rect,
    cut_lines: Vec<Line>,
    marker_boxes: Vec<XBox>,
}
```

## 重要演算法

### 固定 print scale

必須保留 v0.1.1 修正後的邏輯：

> 所有頁面使用同一個 1:1 print scale，不可因為中間頁 overlap 比較多而縮小。

計算方式：

```text
max_extra_x = if cols <= 1 then 0 else if cols == 2 then overlap else overlap * 2
max_extra_y = if rows <= 1 then 0 else if rows == 2 then overlap else overlap * 2
base_tile_w = printable_w - max_extra_x
base_tile_h = printable_h - max_extra_y
total_canvas_w = base_tile_w * cols
total_canvas_h = base_tile_h * rows
```

每頁 tile 的 clip 是 base tile 加上相鄰 overlap，但輸出時不再各自 fit 到 A4，而是用固定 print unit 放置。

### 最佳擺放

候選：

```text
(cols, rows, portrait)
(cols, rows, landscape)
(rows, cols, portrait)
(rows, cols, landscape)
```

score：

```text
scale = min(total_canvas_w / src_w, total_canvas_h / src_h)
score = src_w * scale * src_h * scale
```

tie-breaker：選 poster aspect ratio 更接近 source aspect ratio 的候選。

### 裁切策略

延續目前規則：

- 拼貼從左上往右、往下
- 有左邊鄰居：畫左側紅色裁切線
- 有上方鄰居：畫上方紅色裁切線
- 有右邊鄰居：對應側只畫 X 框，不畫線
- 有下方鄰居：對應側只畫 X 框，不畫線
- 外框裁切輔助為淡黑色
- 不輸出文字標籤

## UI 設計

主畫面只留常用項：

- 來源圖片
- 輸出 PDF
- A4 張數
- 最佳擺放 toggle
- 產生按鈕
- 預覽
- 成品尺寸顯示

進階收合：

- 自訂欄列
- 重疊區 mm
- 邊界 mm
- 圖片 DPI
- 強制 A4 橫向
- 淡黑外框裁切輔助
- 紅色裁切線與 X 框

預覽 canvas：

- 白底 A4 紙張
- 灰色圖片範圍
- 紅色接縫 / 裁切位置
- 青色重疊範圍（可選）

## Tauri commands

建議 commands：

```rust
#[tauri::command]
fn inspect_image(path: String, options: PosterOptions) -> Result<PreviewInfo, AppError>

#[tauri::command]
fn generate_poster(input: String, output: String, options: PosterOptions) -> Result<GenerateResult, AppError>
```

Frontend 可直接算 preview，也可呼叫 Rust 算。建議 layout 計算由 Rust 回傳，避免前後端邏輯分叉。

## 測試計劃

Rust unit tests：

- `3x2` 和 `2x3` 對同一張圖 resolve 結果一致
- 中間 tile 和邊緣 tile 使用相同 print scale
- overlap 過大時回傳錯誤
- 成品尺寸 cm 計算正確
- tile 數量正確

Golden smoke tests：

- 用固定測試圖片產生 2x2 PDF
- 用固定測試圖片產生 3x2 PDF
- 檢查頁數
- 檢查所有頁面尺寸為 A4

CI：

- Rust fmt
- Rust clippy
- Rust test
- frontend build
- tauri build mac/windows

## 發布策略

建議不要覆蓋 Python v0.1.x。

版本線：

```text
v0.1.x = Python legacy
v0.2.0-alpha.1 = Tauri image-only prototype
v0.2.0 = Tauri image-only stable
v0.3.0 = Tauri + PDFium PDF input
```

或如果想更清楚：

```text
legacy-python-v0.1.1
v0.2.0-alpha.1
```

## 遷移步驟

### Step 1：保存 legacy

```bash
mkdir -p legacy/python-poster-maker
# 移動目前 Python 相關檔案
```

建議移動：

```text
pyproject.toml
poster_maker.spec
src/poster_maker/
scripts/
installer/
```

保留 root：

```text
README.md
TAURI_REWRITE_PLAN.md
.gitignore
.github/
```

### Step 2：建立 Tauri app

```bash
pnpm create tauri-app
```

或手動初始化，選 Svelte + TypeScript。

### Step 3：建立 Rust core modules

```text
src-tauri/src/layout.rs
src-tauri/src/pdf_output.rs
src-tauri/src/image_io.rs
src-tauri/src/markers.rs
```

先把 Python `core.py` 的演算法移植成 Rust unit-testable functions。

### Step 4：做 frontend MVP

- 表單
- file picker
- preview canvas
- generate button

### Step 5：產生 PDF

用 `printpdf` 或其他 PDF writer 寫入：

- A4 page
- image crop
- outer marks
- crop lines
- X boxes

### Step 6：CI 打包

使用 Tauri 官方 GitHub Action：

- macOS runner
- Windows runner
- upload artifacts
- GitHub Release

## 風險

### PDF 輸出 crate 能力

`printpdf` 是否足夠處理高解析圖片裁切與多頁輸出，需要 prototype 驗證。

備案：

- `lopdf` 手寫 image XObject
- `pdf-writer`
- 先輸出每頁 raster image，再包 PDF

### 圖片格式支援

Rust `image` crate 對 TIFF/WEBP 支援可行，但細節需確認 feature flags。

### 大圖記憶體

巨大圖片切 3x3 / 4x4 可能吃記憶體。需要：

- 限制 preview size
- output 時避免不必要複製
- 必要時分 tile 處理

### PDFium 第二階段大小

加入 PDFium 後，bundle size 會上升。需實測。

## 預估工期

第一階段 image-only：

- Project setup：0.5 天
- Rust layout core + tests：1 天
- PDF output：1–2 天
- Tauri UI：1–2 天
- CI release：0.5–1 天
- 調整與測試：1 天

合計：**4–7 天**

第二階段 PDFium：

- PDFium 整合：1–2 天
- PDF page render / DPI / page select：1–2 天
- CI native library packaging：1–2 天
- 測試：1 天

合計：**4–7 天**

## Go / No-Go 檢查

第一階段完成後，若符合以下條件，才建議正式取代 Python 版：

- macOS + Windows artifacts 都能穩定打包
- image-only bundle 明顯小於 Python 版
- 3x2 / 2x3 layout 一致
- 中間 tile 無縮放差異
- 輸出 PDF 列印品質 OK
- GUI 體驗比 Tk 版好
- 加 PDFium 的估計大小仍可接受

## 建議下一步

先開分支：

```bash
git checkout -b tauri-prototype
```

然後：

1. 移動 Python 版到 `legacy/python-poster-maker/`
2. 建立 Tauri + Svelte 專案
3. 先移植 layout 計算與測試
4. 再做 image-only PDF output
5. 實測 bundle size
