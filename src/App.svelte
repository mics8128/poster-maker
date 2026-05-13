<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { confirm, open } from '@tauri-apps/plugin-dialog';

  import pkg from '../package.json';

  const version = pkg.version;


  type PosterOptions = {
    cols: number;
    rows: number;
    targetWidthMm?: number | null;
    targetHeightMm?: number | null;
    overlapMm: number;
    marginMm: number;
    drawOuterMarks: boolean;
    drawCutGuides: boolean;
  };

  type Rect = { x0: number; y0: number; x1: number; y1: number };
  type Point = { x: number; y: number };
  type LineGeometry = { a: Point; b: Point };
  type MarkerGeometry = { rect: Rect };
  type PreviewPageGeometry = {
    row: number;
    col: number;
    clipCanvas: Rect;
    destPage: Rect;
    outerLines: LineGeometry[];
    cutLines: LineGeometry[];
    markers: MarkerGeometry[];
  };
  type PreviewGeometry = {
    imageCanvas: Rect;
    pages: PreviewPageGeometry[];
  };

  type PreviewInfo = {
    cols: number;
    rows: number;
    landscape: boolean;
    pageWidthPt: number;
    pageHeightPt: number;
    imageWidthPt: number;
    imageHeightPt: number;
    imageWidthCm: number;
    imageHeightCm: number;
    paperWidthCm: number;
    paperHeightCm: number;
  };

  let inputPath = '';
  let outputName = '';
  let grid = '3x2 / 2x3';
  let cols = 3;
  let rows = 2;
  let targetWidthMm = 594;
  let targetHeightMm = 841;
  let overlapMm = 5;
  let marginMm = 1;
  let preview: PreviewInfo | null = null;
  let previewGeometry: PreviewGeometry | null = null;
  let status = '選圖片，按產生。';
  let busy = false;

  $: usesCustomSizeMode = grid === 'Custom Size';
  $: resolvedTargetWidthMm = usesCustomSizeMode ? validTargetMm(targetWidthMm) : null;
  $: resolvedTargetHeightMm = usesCustomSizeMode ? validTargetMm(targetHeightMm) : null;
  $: usesTargetSize = usesCustomSizeMode && (resolvedTargetWidthMm !== null || resolvedTargetHeightMm !== null);
  $: options = {
    cols,
    rows,
    targetWidthMm: usesTargetSize ? resolvedTargetWidthMm : null,
    targetHeightMm: usesTargetSize ? resolvedTargetHeightMm : null,
    overlapMm,
    marginMm,
    drawOuterMarks: true,
    drawCutGuides: true,
  } satisfies PosterOptions;
  $: previewImageSrc = inputPath ? convertFileSrc(inputPath, 'asset') : '';
  $: if (inputPath) refreshPreview(options, inputPath);

  let customCols = 3;
  let customRows = 2;

  function gridChanged() {
    if (grid === 'Custom') {
      customCols = cols;
      customRows = rows;
      return;
    }
    if (grid === 'Custom Size') return;
    const [c, r] = grid.split('/')[0].trim().split('x').map(Number);
    cols = c;
    rows = r;
  }

  function setCustomGrid(nextCols: number, nextRows: number) {
    customCols = clampGrid(nextCols);
    customRows = clampGrid(nextRows);
    cols = customCols;
    rows = customRows;
    grid = 'Custom';
  }

  function adjustCustom(which: 'cols' | 'rows', delta: number) {
    setCustomGrid(customCols + (which === 'cols' ? delta : 0), customRows + (which === 'rows' ? delta : 0));
  }

  function clampGrid(value: number) {
    const n = Number(value);
    if (!Number.isFinite(n)) return 1;
    return Math.max(1, Math.min(12, Math.round(n)));
  }

  function validTargetMm(value: number) {
    const n = Number(value);
    return Number.isFinite(n) && n > 0 ? n : null;
  }

  function defaultOutputName(path: string) {
    const file = path.split(/[\\/]/).pop() || 'poster';
    const stem = file.replace(/\.[^.]*$/, '') || 'poster';
    return `${stem}-poster.pdf`;
  }

  async function pickInput() {
    const selected = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tif', 'tiff'] }] });
    if (typeof selected === 'string') {
      inputPath = selected;
      outputName = defaultOutputName(selected);
    }
  }

  let previewSeq = 0;
  async function refreshPreview(opts: PosterOptions, path: string) {
    const seq = ++previewSeq;
    try {
      const [result, geometry] = await Promise.all([
        invoke<PreviewInfo>('inspect_image', { path, options: opts }),
        invoke<PreviewGeometry>('preview_geometry', { path, options: opts }),
      ]);
      if (seq !== previewSeq) return;
      preview = result;
      previewGeometry = geometry;
      const layoutLabel = usesTargetSize ? '自動張數' : '最佳輸出';
      status = `${layoutLabel}：${result.cols}x${result.rows} A4，共 ${result.cols * result.rows} 張，${result.landscape ? '橫向' : '直向'}\n成品圖面：約 ${result.imageWidthCm.toFixed(1)} × ${result.imageHeightCm.toFixed(1)} cm\nA4總外框：約 ${result.paperWidthCm.toFixed(1)} × ${result.paperHeightCm.toFixed(1)} cm\n重疊 ${overlapMm}mm，邊界 ${marginMm}mm`;
    } catch (error) {
      preview = null;
      previewGeometry = null;
      status = `預覽失敗：${error}`;
    }
  }

  async function generate() {
    if (!inputPath || !outputName.trim()) {
      status = '請先選來源並填輸出 PDF 檔名。';
      return;
    }
    busy = true;
    status = '產生中…';
    try {
      const exists = await invoke<boolean>('output_exists', { input: inputPath, outputName });
      if (exists) {
        const overwrite = await confirm(`「${outputName}」已存在，是否覆蓋？`, { title: '覆蓋既有 PDF？', kind: 'warning' });
        if (!overwrite) {
          status = '已取消，未覆蓋既有檔案。';
          return;
        }
      }
      const result = await invoke<{ pages: number; output: string }>('generate_poster', { input: inputPath, outputName, overwrite: exists, options });
      status = `完成：${result.pages} 頁 A4 → ${result.output}`;
    } catch (error) {
      status = `產生失敗：${error}`;
    } finally {
      busy = false;
    }
  }

  function viewBox(info: PreviewInfo) {
    return `0 0 ${info.pageWidthPt * info.cols} ${info.pageHeightPt * info.rows}`;
  }

</script>

<div class="app">
  <aside class="panel sidebar">
    <div class="header"><h1>Poster Maker</h1><span class="version">v{version}</span></div>

    <div class="field">
      <div class="label">來源圖片</div>
      <div class="row"><input bind:value={inputPath} placeholder="PNG / JPG / WEBP / BMP / TIFF" /><button on:click={pickInput}>選擇…</button></div>
    </div>

    <div class="field">
      <div class="label">輸出 PDF 檔名</div>
      <input bind:value={outputName} placeholder="example-poster.pdf" />
      <div class="muted">預設存到來源圖片同一個資料夾</div>
    </div>

    <div class="field">
      <div class="label">輸出尺寸</div>
      <select bind:value={grid} on:change={gridChanged}>
        <option>2x1 / 1x2</option>
        <option>2x2</option>
        <option>3x2 / 2x3</option>
        <option>3x3</option>
        <option>4x3 / 3x4</option>
        <option>4x4</option>
        <option>Custom</option>
        <option>Custom Size</option>
      </select>
      <div class="muted">{usesCustomSizeMode ? '依 mm 尺寸自動計算 A4 張數' : '選擇 A4 張數並自動使用最佳擺放'}</div>
      {#if grid === 'Custom'}
        <div class="custom-grid">
          <div class="stepper">
            <div class="stepper-label">欄</div>
            <button type="button" on:click={() => adjustCustom('cols', -1)}>−</button>
            <div class="stepper-value">{customCols}</div>
            <button type="button" on:click={() => adjustCustom('cols', 1)}>+</button>
          </div>
          <div class="stepper">
            <div class="stepper-label">列</div>
            <button type="button" on:click={() => adjustCustom('rows', -1)}>−</button>
            <div class="stepper-value">{customRows}</div>
            <button type="button" on:click={() => adjustCustom('rows', 1)}>+</button>
          </div>
        </div>
      {/if}
      {#if usesCustomSizeMode}
        <div class="grid2">
          <label class="field">
            <span class="stepper-label">寬 mm</span>
            <input type="number" min="1" step="1" bind:value={targetWidthMm} />
          </label>
          <label class="field">
            <span class="stepper-label">高 mm</span>
            <input type="number" min="1" step="1" bind:value={targetHeightMm} />
          </label>
        </div>
        <div class="muted">依圖片比例輸出；填單邊時會自動推算另一邊，並反推需要的 A4 張數。</div>
      {/if}
    </div>

    <button class="primary" disabled={busy} on:click={generate}>產生海報 PDF</button>
    <div class="status">{status}</div>
    <div class="muted">image-only prototype；PDF 輸入預計第二階段加 PDFium。</div>
  </aside>

  <main class="panel preview-panel">
    <div class="preview-wrap">
      {#if preview && previewGeometry}
        <svg viewBox={viewBox(preview)} preserveAspectRatio="xMidYMid meet">
          <defs>
            <pattern id="paperGrid" width="18" height="18" patternUnits="userSpaceOnUse">
              <path d="M 18 0 L 0 0 0 18" fill="none" stroke="#f1f3f5" stroke-width="1" />
            </pattern>
          </defs>
          <rect x="0" y="0" width={preview.pageWidthPt * preview.cols} height={preview.pageHeightPt * preview.rows} fill="url(#paperGrid)" />
          {#each previewGeometry.pages as page}
            <rect x={page.col * preview.pageWidthPt} y={page.row * preview.pageHeightPt} width={preview.pageWidthPt} height={preview.pageHeightPt} fill="white" stroke="#111" stroke-width="1.2" />
            <svg x={page.col * preview.pageWidthPt + page.destPage.x0} y={page.row * preview.pageHeightPt + page.destPage.y0} width={page.destPage.x1 - page.destPage.x0} height={page.destPage.y1 - page.destPage.y0} viewBox={`${page.clipCanvas.x0} ${page.clipCanvas.y0} ${page.clipCanvas.x1 - page.clipCanvas.x0} ${page.clipCanvas.y1 - page.clipCanvas.y0}`} preserveAspectRatio="none">
              <image href={previewImageSrc} x={previewGeometry.imageCanvas.x0} y={previewGeometry.imageCanvas.y0} width={preview.imageWidthPt} height={preview.imageHeightPt} preserveAspectRatio="none" />
            </svg>
            {#each page.outerLines as line}
              <line x1={page.col * preview.pageWidthPt + line.a.x} y1={page.row * preview.pageHeightPt + line.a.y} x2={page.col * preview.pageWidthPt + line.b.x} y2={page.row * preview.pageHeightPt + line.b.y} stroke="#8c8c8c" stroke-opacity="0.5" stroke-dasharray="3 3" stroke-width="0.5" />
            {/each}
            {#each page.cutLines as line}
              <line x1={page.col * preview.pageWidthPt + line.a.x} y1={page.row * preview.pageHeightPt + line.a.y} x2={page.col * preview.pageWidthPt + line.b.x} y2={page.row * preview.pageHeightPt + line.b.y} stroke="#737373" stroke-opacity="0.65" stroke-dasharray="7 3" stroke-width="0.9" />
            {/each}
            {#each page.markers as marker}
              <g stroke="#737373" stroke-opacity="0.65" stroke-width="0.8" fill="none">
                <rect x={page.col * preview.pageWidthPt + marker.rect.x0} y={page.row * preview.pageHeightPt + marker.rect.y0} width={marker.rect.x1 - marker.rect.x0} height={marker.rect.y1 - marker.rect.y0} />
                <line x1={page.col * preview.pageWidthPt + marker.rect.x0} y1={page.row * preview.pageHeightPt + marker.rect.y0} x2={page.col * preview.pageWidthPt + marker.rect.x1} y2={page.row * preview.pageHeightPt + marker.rect.y1} />
                <line x1={page.col * preview.pageWidthPt + marker.rect.x0} y1={page.row * preview.pageHeightPt + marker.rect.y1} x2={page.col * preview.pageWidthPt + marker.rect.x1} y2={page.row * preview.pageHeightPt + marker.rect.y0} />
              </g>
            {/each}
          {/each}
        </svg>
      {:else}
        <div class="muted">預覽區</div>
      {/if}
    </div>
  </main>
</div>
