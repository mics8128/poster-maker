<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';

  const version = '0.2.0-alpha.1';

  type PosterOptions = {
    cols: number;
    rows: number;
    overlapMm: number;
    marginMm: number;
    imageDpi: number;
    landscape: boolean;
    autoLayout: boolean;
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
    score: number;
  };

  let inputPath = '';
  let outputPath = '';
  let grid = '3x2 / 2x3';
  let cols = 3;
  let rows = 2;
  let overlapMm = 5;
  let marginMm = 3;
  let imageDpi = 200;
  let landscape = false;
  let autoLayout = true;
  let drawOuterMarks = true;
  let drawCutGuides = true;
  let preview: PreviewInfo | null = null;
  let status = '選圖片，按產生。v0.2 image-only prototype';
  let busy = false;
  let advancedOpen = false;

  $: options = { cols, rows, overlapMm, marginMm, imageDpi, landscape, autoLayout, drawOuterMarks, drawCutGuides } satisfies PosterOptions;
  $: if (inputPath) refreshPreview(options, inputPath);

  function gridChanged() {
    if (grid === 'Custom') return;
    const first = grid.split('/')[0].trim();
    const [c, r] = first.split('x').map(Number);
    cols = c;
    rows = r;
  }

  async function pickInput() {
    const selected = await open({ multiple: false, filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tif', 'tiff'] }] });
    if (typeof selected === 'string') {
      inputPath = selected;
      if (!outputPath) outputPath = selected.replace(/\.[^.]+$/, '-poster.pdf');
    }
  }

  async function pickOutput() {
    const selected = await save({ filters: [{ name: 'PDF', extensions: ['pdf'] }], defaultPath: outputPath || 'poster.pdf' });
    if (selected) outputPath = selected;
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
    if (!inputPath || !outputPath) {
      status = '請先選來源與輸出 PDF。';
      return;
    }
    busy = true;
    status = '產生中…';
    try {
      const result = await invoke<{ pages: number; output: string }>('generate_poster', { input: inputPath, output: outputPath, options });
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
      <div class="label">輸出 PDF</div>
      <div class="row"><input bind:value={outputPath} /><button on:click={pickOutput}>儲存…</button></div>
    </div>

    <div class="field">
      <div class="label">A4 張數</div>
      <select bind:value={grid} on:change={gridChanged}>
        <option>2x1</option>
        <option>2x2</option>
        <option>3x2 / 2x3</option>
        <option>3x3</option>
        <option>4x3 / 3x4</option>
        <option>4x4</option>
        <option>Custom</option>
      </select>
      <label class="check"><input type="checkbox" bind:checked={autoLayout} />最佳擺放</label>
    </div>

    <details class="advanced" bind:open={advancedOpen}>
      <summary>進階</summary>
      <div class="grid2">
        <label>欄<input type="number" min="1" bind:value={cols} /></label>
        <label>列<input type="number" min="1" bind:value={rows} /></label>
        <label>重疊 mm<input type="number" min="0" bind:value={overlapMm} /></label>
        <label>邊界 mm<input type="number" min="0" bind:value={marginMm} /></label>
        <label>DPI<input type="number" min="72" bind:value={imageDpi} /></label>
      </div>
      <label class="check"><input type="checkbox" bind:checked={landscape} />強制 A4 橫向（關閉最佳擺放時）</label>
      <label class="check"><input type="checkbox" bind:checked={drawOuterMarks} />淡黑外框裁切輔助</label>
      <label class="check"><input type="checkbox" bind:checked={drawCutGuides} />紅色裁切線與 X 框</label>
    </details>

    <button class="primary" disabled={busy} on:click={generate}>產生海報 PDF</button>
    <div class="status">{status}</div>
    <div class="muted">v{version} image-only prototype；PDF 輸入預計第二階段加 PDFium。</div>
  </aside>

  <main class="panel preview-panel">
    <div class="preview-wrap">
      {#if preview}
        <svg viewBox={viewBox(preview)} preserveAspectRatio="xMidYMid meet">
          <rect x="0" y="0" width={preview.pageWidthPt * preview.cols} height={preview.pageHeightPt * preview.rows} fill="white" />
          <rect x={(preview.pageWidthPt * preview.cols - preview.imageWidthPt) / 2} y={(preview.pageHeightPt * preview.rows - preview.imageHeightPt) / 2} width={preview.imageWidthPt} height={preview.imageHeightPt} fill="#ececec" />
          {#each Array(preview.cols) as _, c}
            {#each Array(preview.rows) as _, r}
              <rect x={c * preview.pageWidthPt} y={r * preview.pageHeightPt} width={preview.pageWidthPt} height={preview.pageHeightPt} fill="none" stroke="#111" stroke-width="1" />
              <rect x={c * preview.pageWidthPt + marginMm * 72 / 25.4} y={r * preview.pageHeightPt + marginMm * 72 / 25.4} width={preview.pageWidthPt - marginMm * 2 * 72 / 25.4} height={preview.pageHeightPt - marginMm * 2 * 72 / 25.4} fill="none" stroke="#999" stroke-dasharray="4 4" stroke-width="1" />
            {/each}
          {/each}
          {#each Array(preview.cols - 1) as _, c}
            <line x1={(c + 1) * preview.pageWidthPt} x2={(c + 1) * preview.pageWidthPt} y1="0" y2={preview.pageHeightPt * preview.rows} stroke="red" stroke-dasharray="8 5" stroke-width="2" />
          {/each}
          {#each Array(preview.rows - 1) as _, r}
            <line y1={(r + 1) * preview.pageHeightPt} y2={(r + 1) * preview.pageHeightPt} x1="0" x2={preview.pageWidthPt * preview.cols} stroke="red" stroke-dasharray="8 5" stroke-width="2" />
          {/each}
        </svg>
      {:else}
        <div class="muted">預覽區</div>
      {/if}
    </div>
  </main>
</div>
