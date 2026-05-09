<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { confirm, open } from '@tauri-apps/plugin-dialog';

  const version = '0.2.0-alpha.1';
  const ptPerMm = 72 / 25.4;

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
          <rect x="0" y="0" width={preview.pageWidthPt * preview.cols} height={preview.pageHeightPt * preview.rows} fill="white" />
          <rect x={(preview.pageWidthPt * preview.cols - preview.imageWidthPt) / 2} y={(preview.pageHeightPt * preview.rows - preview.imageHeightPt) / 2} width={preview.imageWidthPt} height={preview.imageHeightPt} fill="#ececec" />
          {#each Array(preview.cols) as _, c}
            {#each Array(preview.rows) as _, r}
              <rect x={c * preview.pageWidthPt} y={r * preview.pageHeightPt} width={preview.pageWidthPt} height={preview.pageHeightPt} fill="none" stroke="#111" stroke-width="1" />
              <rect x={c * preview.pageWidthPt + marginMm * ptPerMm} y={r * preview.pageHeightPt + marginMm * ptPerMm} width={preview.pageWidthPt - marginMm * 2 * ptPerMm} height={preview.pageHeightPt - marginMm * 2 * ptPerMm} fill="none" stroke="#999" stroke-dasharray="4 4" stroke-width="1" />
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
