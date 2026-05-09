<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { confirm, open } from '@tauri-apps/plugin-dialog';

  const version = '0.2.0-alpha.1';
  const ptPerMm = 72 / 25.4;
  const markerSizePt = 12;
  const markerGapPt = 2;

  type PosterOptions = {
    cols: number;
    rows: number;
    overlapMm: number;
    marginMm: number;
    drawOuterMarks: boolean;
    drawCutGuides: boolean;
  };

  type PreviewInfo = {
    cols: number;
    rows: number;
    landscape: boolean;
    pageWidthPt: number;
    pageHeightPt: number;
    baseTileWidthPt: number;
    baseTileHeightPt: number;
    canvasWidthPt: number;
    canvasHeightPt: number;
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
  let overlapMm = 5;
  let marginMm = 3;
  let preview: PreviewInfo | null = null;
  let status = '選圖片，按產生。';
  let busy = false;

  $: options = { cols, rows, overlapMm, marginMm, drawOuterMarks: true, drawCutGuides: true } satisfies PosterOptions;
  $: if (inputPath) refreshPreview(options, inputPath);

  let customCols = 3;
  let customRows = 2;

  function gridChanged() {
    if (grid === 'Custom') {
      customCols = cols;
      customRows = rows;
      return;
    }
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
      const result = await invoke<PreviewInfo>('inspect_image', { path, options: opts });
      if (seq !== previewSeq) return;
      preview = result;
      status = `最佳輸出：${result.cols}x${result.rows} A4，${result.landscape ? '橫向' : '直向'}\n成品圖面：約 ${result.imageWidthCm.toFixed(1)} × ${result.imageHeightCm.toFixed(1)} cm\nA4總外框：約 ${result.paperWidthCm.toFixed(1)} × ${result.paperHeightCm.toFixed(1)} cm\n重疊 ${overlapMm}mm，邊界 ${marginMm}mm`;
    } catch (error) {
      preview = null;
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

  function imageCanvas(info: PreviewInfo) {
    const x = (info.canvasWidthPt - info.imageWidthPt) / 2;
    const y = (info.canvasHeightPt - info.imageHeightPt) / 2;
    return { x, y, w: info.imageWidthPt, h: info.imageHeightPt };
  }

  function pageTile(info: PreviewInfo, row: number, col: number) {
    const overlap = overlapMm * ptPerMm;
    const img = imageCanvas(info);
    const base = {
      x0: col * info.baseTileWidthPt,
      y0: row * info.baseTileHeightPt,
      x1: (col + 1) * info.baseTileWidthPt,
      y1: (row + 1) * info.baseTileHeightPt,
    };
    const clip = {
      x0: Math.max(base.x0 - (col > 0 ? overlap : 0), img.x),
      y0: Math.max(base.y0 - (row > 0 ? overlap : 0), img.y),
      x1: Math.min(base.x1 + (col < info.cols - 1 ? overlap : 0), img.x + img.w),
      y1: Math.min(base.y1 + (row < info.rows - 1 ? overlap : 0), img.y + img.h),
    };
    const w = clip.x1 - clip.x0;
    const h = clip.y1 - clip.y0;
    const pageX = col * info.pageWidthPt;
    const pageY = row * info.pageHeightPt;
    const dest = {
      x0: pageX + (info.pageWidthPt - w) / 2,
      y0: pageY + (info.pageHeightPt - h) / 2,
      x1: pageX + (info.pageWidthPt + w) / 2,
      y1: pageY + (info.pageHeightPt + h) / 2,
    };
    const sx = (dest.x1 - dest.x0) / w;
    const sy = (dest.y1 - dest.y0) / h;
    const guides = {
      leftX: dest.x0 + (base.x0 - clip.x0) * sx,
      rightX: dest.x0 + (base.x1 - clip.x0) * sx,
      topY: dest.y0 + (base.y0 - clip.y0) * sy,
      bottomY: dest.y0 + (base.y1 - clip.y0) * sy,
    };
    return { dest, guides };
  }

  function markerCenters(a: { x: number; y: number }, b: { x: number; y: number }) {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const len = Math.max(Math.hypot(dx, dy), 1);
    const ux = dx / len;
    const uy = dy / len;
    const offset = markerSizePt / 2 + markerGapPt;
    return [
      { x: a.x - ux * offset, y: a.y - uy * offset },
      { x: b.x + ux * offset, y: b.y + uy * offset },
    ];
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
      <div class="label">A4 張數</div>
      <select bind:value={grid} on:change={gridChanged}>
        <option>2x1 / 1x2</option>
        <option>2x2</option>
        <option>3x2 / 2x3</option>
        <option>3x3</option>
        <option>4x3 / 3x4</option>
        <option>4x4</option>
        <option>Custom</option>
      </select>
      <div class="muted">自動使用最佳擺放</div>
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
    </div>

    <button class="primary" disabled={busy} on:click={generate}>產生海報 PDF</button>
    <div class="status">{status}</div>
    <div class="muted">image-only prototype；PDF 輸入預計第二階段加 PDFium。</div>
  </aside>

  <main class="panel preview-panel">
    <div class="preview-wrap">
      {#if preview}
        <svg viewBox={viewBox(preview)} preserveAspectRatio="xMidYMid meet">
          <defs>
            <pattern id="paperGrid" width="18" height="18" patternUnits="userSpaceOnUse">
              <path d="M 18 0 L 0 0 0 18" fill="none" stroke="#f1f3f5" stroke-width="1" />
            </pattern>
          </defs>
          <rect x="0" y="0" width={preview.pageWidthPt * preview.cols} height={preview.pageHeightPt * preview.rows} fill="url(#paperGrid)" />
          {#each Array(preview.rows) as _, r}
            {#each Array(preview.cols) as _, c}
              {@const tile = pageTile(preview, r, c)}
              <rect x={c * preview.pageWidthPt} y={r * preview.pageHeightPt} width={preview.pageWidthPt} height={preview.pageHeightPt} fill="white" stroke="#111" stroke-width="1.2" />
              <rect x={tile.dest.x0} y={tile.dest.y0} width={tile.dest.x1 - tile.dest.x0} height={tile.dest.y1 - tile.dest.y0} fill="#e9ecef" />
              <line x1={c * preview.pageWidthPt} x2={(c + 1) * preview.pageWidthPt} y1={tile.dest.y0} y2={tile.dest.y0} stroke="black" stroke-opacity="0.5" stroke-dasharray="3 3" stroke-width="0.5" />
              <line x1={tile.dest.x1} x2={tile.dest.x1} y1={r * preview.pageHeightPt} y2={(r + 1) * preview.pageHeightPt} stroke="black" stroke-opacity="0.5" stroke-dasharray="3 3" stroke-width="0.5" />
              <line x1={(c + 1) * preview.pageWidthPt} x2={c * preview.pageWidthPt} y1={tile.dest.y1} y2={tile.dest.y1} stroke="black" stroke-opacity="0.5" stroke-dasharray="3 3" stroke-width="0.5" />
              <line x1={tile.dest.x0} x2={tile.dest.x0} y1={(r + 1) * preview.pageHeightPt} y2={r * preview.pageHeightPt} stroke="black" stroke-opacity="0.5" stroke-dasharray="3 3" stroke-width="0.5" />
              {#if c > 0}
                <line x1={tile.guides.leftX} x2={tile.guides.leftX} y1={tile.dest.y0} y2={tile.dest.y1} stroke="white" stroke-dasharray="7 3" stroke-width="2" />
                <line x1={tile.guides.leftX} x2={tile.guides.leftX} y1={tile.dest.y0} y2={tile.dest.y1} stroke="black" stroke-opacity="0.6" stroke-dasharray="7 3" stroke-width="1" />
                {#each markerCenters({ x: tile.guides.leftX, y: tile.dest.y0 }, { x: tile.guides.leftX, y: tile.dest.y1 }) as marker}
                  <g transform={`translate(${marker.x} ${marker.y})`} stroke="black" stroke-width="1.1" fill="none">
                    <rect x={-markerSizePt / 2} y={-markerSizePt / 2} width={markerSizePt} height={markerSizePt} />
                    <line x1={-markerSizePt / 2} y1={-markerSizePt / 2} x2={markerSizePt / 2} y2={markerSizePt / 2} />
                    <line x1={-markerSizePt / 2} y1={markerSizePt / 2} x2={markerSizePt / 2} y2={-markerSizePt / 2} />
                  </g>
                {/each}
              {/if}
              {#if r > 0}
                <line x1={tile.dest.x0} x2={tile.dest.x1} y1={tile.guides.topY} y2={tile.guides.topY} stroke="white" stroke-dasharray="7 3" stroke-width="2" />
                <line x1={tile.dest.x0} x2={tile.dest.x1} y1={tile.guides.topY} y2={tile.guides.topY} stroke="black" stroke-opacity="0.6" stroke-dasharray="7 3" stroke-width="1" />
                {#each markerCenters({ x: tile.dest.x0, y: tile.guides.topY }, { x: tile.dest.x1, y: tile.guides.topY }) as marker}
                  <g transform={`translate(${marker.x} ${marker.y})`} stroke="black" stroke-width="1.1" fill="none">
                    <rect x={-markerSizePt / 2} y={-markerSizePt / 2} width={markerSizePt} height={markerSizePt} />
                    <line x1={-markerSizePt / 2} y1={-markerSizePt / 2} x2={markerSizePt / 2} y2={markerSizePt / 2} />
                    <line x1={-markerSizePt / 2} y1={markerSizePt / 2} x2={markerSizePt / 2} y2={-markerSizePt / 2} />
                  </g>
                {/each}
              {/if}
              {#if c < preview.cols - 1}
                {#each markerCenters({ x: tile.guides.rightX, y: tile.dest.y0 }, { x: tile.guides.rightX, y: tile.dest.y1 }) as marker}
                  <g transform={`translate(${marker.x} ${marker.y})`} stroke="black" stroke-width="1.1" fill="none">
                    <rect x={-markerSizePt / 2} y={-markerSizePt / 2} width={markerSizePt} height={markerSizePt} />
                    <line x1={-markerSizePt / 2} y1={-markerSizePt / 2} x2={markerSizePt / 2} y2={markerSizePt / 2} />
                    <line x1={-markerSizePt / 2} y1={markerSizePt / 2} x2={markerSizePt / 2} y2={-markerSizePt / 2} />
                  </g>
                {/each}
              {/if}
              {#if r < preview.rows - 1}
                {#each markerCenters({ x: tile.dest.x0, y: tile.guides.bottomY }, { x: tile.dest.x1, y: tile.guides.bottomY }) as marker}
                  <g transform={`translate(${marker.x} ${marker.y})`} stroke="black" stroke-width="1.1" fill="none">
                    <rect x={-markerSizePt / 2} y={-markerSizePt / 2} width={markerSizePt} height={markerSizePt} />
                    <line x1={-markerSizePt / 2} y1={-markerSizePt / 2} x2={markerSizePt / 2} y2={markerSizePt / 2} />
                    <line x1={-markerSizePt / 2} y1={markerSizePt / 2} x2={markerSizePt / 2} y2={-markerSizePt / 2} />
                  </g>
                {/each}
              {/if}
            {/each}
          {/each}
        </svg>
      {:else}
        <div class="muted">預覽區</div>
      {/if}
    </div>
  </main>
</div>
