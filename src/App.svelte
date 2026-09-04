<script lang="ts">
  import { onMount } from 'svelte';
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { confirm, open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebview } from '@tauri-apps/api/webview';

  import pkg from '../package.json';

  const version = pkg.version;
  const imageExtensions = ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'tif', 'tiff'];

  type OutputMode = 'poster' | 'imposition';

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

  type ImpositionOptions = {
    paperWidthMm: number;
    paperHeightMm: number;
    itemWidthMm: number;
    itemHeightMm: number;
    safetyTopMm: number;
    safetyRightMm: number;
    safetyBottomMm: number;
    safetyLeftMm: number;
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

  type PosterPreview = {
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

  type ImpositionPlacement = {
    cutRect: Rect;
    safeRect: Rect;
    imageRect: Rect;
  };
  type ImpositionPreview = {
    pageWidthPt: number;
    pageHeightPt: number;
    paperWidthCm: number;
    paperHeightCm: number;
    cols: number;
    rows: number;
    copies: number;
    itemRotated: boolean;
    itemWidthPt: number;
    itemHeightPt: number;
    usedWidthPt: number;
    usedHeightPt: number;
    placements: ImpositionPlacement[];
  };

  let outputMode: OutputMode = 'poster';
  let inputPath = '';
  let outputName = '';
  let status = '選圖片後，選模式與尺寸，再查看預覽並產生 PDF。';
  let busy = false;
  let isDraggingFiles = false;

  let grid = '3x2 / 2x3';
  let cols = 3;
  let rows = 2;
  let targetWidthMm = 594;
  let targetHeightMm = 841;
  let overlapMm = 5;
  let marginMm = 1;
  let customCols = 3;
  let customRows = 2;

  let itemPreset = 'A6';
  let itemWidthMm = 105;
  let itemHeightMm = 148;
  let paperWidthMm = 210;
  let paperHeightMm = 297;
  let safetyTopMm = 15;
  let safetyRightMm = 15;
  let safetyBottomMm = 15;
  let safetyLeftMm = 15;

  let posterPreview: PosterPreview | null = null;
  let posterGeometry: PreviewGeometry | null = null;
  let impositionPreview: ImpositionPreview | null = null;
  let posterPreviewSeq = 0;
  let impositionPreviewSeq = 0;

  $: usesCustomSizeMode = grid === 'Custom Size';
  $: resolvedTargetWidthMm = usesCustomSizeMode ? validPositiveMm(targetWidthMm) : null;
  $: resolvedTargetHeightMm = usesCustomSizeMode ? validPositiveMm(targetHeightMm) : null;
  $: usesTargetSize = usesCustomSizeMode && (resolvedTargetWidthMm !== null || resolvedTargetHeightMm !== null);
  $: posterOptions = {
    cols,
    rows,
    targetWidthMm: usesTargetSize ? resolvedTargetWidthMm : null,
    targetHeightMm: usesTargetSize ? resolvedTargetHeightMm : null,
    overlapMm,
    marginMm,
    drawOuterMarks: true,
    drawCutGuides: true,
  } satisfies PosterOptions;
  $: impositionOptions = {
    paperWidthMm,
    paperHeightMm,
    itemWidthMm,
    itemHeightMm,
    safetyTopMm,
    safetyRightMm,
    safetyBottomMm,
    safetyLeftMm,
  } satisfies ImpositionOptions;
  $: impositionValidationMessage = validateImpositionOptions(impositionOptions);
  $: impositionOptionsValid = impositionValidationMessage === null;
  $: previewImageSrc = inputPath ? convertFileSrc(inputPath, 'asset') : '';

  $: if (inputPath && outputMode === 'poster') {
    refreshPosterPreview(posterOptions, inputPath);
  }

  $: if (inputPath && outputMode === 'imposition') {
    if (impositionOptionsValid) {
      refreshImpositionPreview(impositionOptions, inputPath);
    } else {
      impositionPreviewSeq += 1;
      impositionPreview = null;
      status = impositionValidationMessage ?? '請修正拼版設定。';
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (disposed) return;
        const { payload } = event;

        if (payload.type === 'enter' || payload.type === 'over') {
          isDraggingFiles = true;
          return;
        }
        if (payload.type === 'leave') {
          isDraggingFiles = false;
          return;
        }

        isDraggingFiles = false;
        if (payload.paths.length !== 1) {
          status = '請一次拖曳一張圖片。已保留目前來源圖片。';
          return;
        }
        const [path] = payload.paths;
        if (!isSupportedImage(path)) {
          status = '只支援 PNG、JPG、WEBP、BMP 或 TIFF 圖片。已保留目前來源圖片。';
          return;
        }
        loadInputPath(path);
      })
      .then((stopListening) => {
        if (disposed) stopListening();
        else unlisten = stopListening;
      })
      .catch((error) => {
        console.error('Unable to listen for drag and drop events:', error);
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function gridChanged() {
    if (grid === 'Custom') {
      customCols = cols;
      customRows = rows;
      return;
    }
    if (grid === 'Custom Size') return;
    const [nextCols, nextRows] = grid.split('/')[0].trim().split('x').map(Number);
    cols = nextCols;
    rows = nextRows;
  }

  function setCustomGrid(nextCols: number, nextRows: number) {
    customCols = clampGrid(nextCols);
    customRows = clampGrid(nextRows);
    cols = customCols;
    rows = customRows;
    grid = 'Custom';
  }

  function adjustCustom(which: 'cols' | 'rows', delta: number) {
    setCustomGrid(
      customCols + (which === 'cols' ? delta : 0),
      customRows + (which === 'rows' ? delta : 0),
    );
  }

  function itemPresetChanged() {
    const presets: Record<string, [number, number]> = {
      A5: [148, 210],
      A6: [105, 148],
      A7: [74, 105],
    };
    const preset = presets[itemPreset];
    if (preset) {
      [itemWidthMm, itemHeightMm] = preset;
    }
  }

  function useCustomItemSize() {
    itemPreset = 'Custom';
  }

  function clampGrid(value: number) {
    const n = Number(value);
    if (!Number.isFinite(n)) return 1;
    return Math.max(1, Math.min(12, Math.round(n)));
  }

  function validPositiveMm(value: number) {
    const n = Number(value);
    return Number.isFinite(n) && n > 0 ? n : null;
  }

  function validateImpositionOptions(options: ImpositionOptions) {
    const dimensions = [options.paperWidthMm, options.paperHeightMm, options.itemWidthMm, options.itemHeightMm];
    if (!dimensions.every((value) => Number.isFinite(value) && value > 0)) {
      return '紙張與成品尺寸必須大於 0 mm。';
    }
    if (dimensions.some((value) => value > 10_000)) {
      return '紙張與成品尺寸不可超過 10,000 mm。';
    }
    const safety = [options.safetyTopMm, options.safetyRightMm, options.safetyBottomMm, options.safetyLeftMm];
    if (!safety.every((value) => Number.isFinite(value) && value >= 0)) {
      return '安全邊界不可為負數。';
    }
    if (options.safetyLeftMm + options.safetyRightMm >= options.itemWidthMm) {
      return '左、右安全邊界總和必須小於成品寬度。';
    }
    if (options.safetyTopMm + options.safetyBottomMm >= options.itemHeightMm) {
      return '上、下安全邊界總和必須小於成品高度。';
    }
    const unrotatedCopies = Math.floor(options.paperWidthMm / options.itemWidthMm)
      * Math.floor(options.paperHeightMm / options.itemHeightMm);
    const rotatedCopies = Math.floor(options.paperWidthMm / options.itemHeightMm)
      * Math.floor(options.paperHeightMm / options.itemWidthMm);
    const copies = Math.max(unrotatedCopies, rotatedCopies);
    if (copies < 1) return '成品無法放入目前的外部紙張。';
    if (copies > 1_000) return '單張紙最多可排列 1,000 份，請放大成品尺寸。';
    return null;
  }

  function isSupportedImage(path: string) {
    const extension = path.split('.').pop()?.toLowerCase();
    return extension !== undefined && imageExtensions.includes(extension);
  }

  function defaultOutputName(path: string) {
    const file = path.split(/[\\/]/).pop() || 'poster';
    const stem = file.replace(/\.[^.]*$/, '') || 'poster';
    return `${stem}-poster.pdf`;
  }

  function loadInputPath(path: string) {
    inputPath = path;
    outputName = defaultOutputName(path);
    posterPreview = null;
    posterGeometry = null;
    impositionPreview = null;
    status = '正在讀取圖片與預覽…';
  }

  async function pickInput() {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Images', extensions: imageExtensions }],
    });
    if (typeof selected === 'string') loadInputPath(selected);
  }

  async function refreshPosterPreview(options: PosterOptions, path: string) {
    const seq = ++posterPreviewSeq;
    try {
      const [result, geometry] = await Promise.all([
        invoke<PosterPreview>('inspect_image', { path, options }),
        invoke<PreviewGeometry>('preview_geometry', { path, options }),
      ]);
      if (seq !== posterPreviewSeq || outputMode !== 'poster' || path !== inputPath) return;
      posterPreview = result;
      posterGeometry = geometry;
      const layoutLabel = usesTargetSize ? '自動張數' : '最佳輸出';
      status = `${layoutLabel}：${result.cols} × ${result.rows} 張 A4，共 ${result.cols * result.rows} 張，${result.landscape ? '橫向' : '直向'}\n成品圖面：約 ${result.imageWidthCm.toFixed(1)} × ${result.imageHeightCm.toFixed(1)} cm\nA4 總外框：約 ${result.paperWidthCm.toFixed(1)} × ${result.paperHeightCm.toFixed(1)} cm\n重疊 ${overlapMm} mm，邊界 ${marginMm} mm`;
    } catch (error) {
      if (seq !== posterPreviewSeq || outputMode !== 'poster' || path !== inputPath) return;
      posterPreview = null;
      posterGeometry = null;
      status = `預覽失敗：${error}`;
    }
  }

  async function refreshImpositionPreview(options: ImpositionOptions, path: string) {
    const seq = ++impositionPreviewSeq;
    try {
      const result = await invoke<ImpositionPreview>('inspect_imposition', { path, options });
      if (seq !== impositionPreviewSeq || outputMode !== 'imposition' || path !== inputPath) return;
      impositionPreview = result;
      status = impositionSummary(result);
    } catch (error) {
      if (seq !== impositionPreviewSeq || outputMode !== 'imposition' || path !== inputPath) return;
      impositionPreview = null;
      status = `拼版預覽失敗：${error}`;
    }
  }

  function impositionSummary(result: ImpositionPreview) {
    return `最佳拼版：${result.cols} × ${result.rows}，共 ${result.copies} 份\n${result.itemRotated ? '成品已旋轉 90° 排版' : '成品未旋轉'}\n紙張：${result.paperWidthCm.toFixed(1)} × ${result.paperHeightCm.toFixed(1)} cm／成品：${itemWidthMm} × ${itemHeightMm} mm\n安全邊界：上 ${safetyTopMm}、右 ${safetyRightMm}、下 ${safetyBottomMm}、左 ${safetyLeftMm} mm`;
  }

  async function generate() {
    if (!inputPath || !outputName.trim()) {
      status = '請先選來源並填寫輸出 PDF 檔名。';
      return;
    }
    if (outputMode === 'imposition' && !impositionOptionsValid) {
      status = impositionValidationMessage ?? '請先修正拼版尺寸與安全邊界。';
      return;
    }

    busy = true;
    status = '產生中…';
    try {
      const exists = await invoke<boolean>('output_exists', { input: inputPath, outputName });
      if (exists) {
        const overwrite = await confirm(`「${outputName}」已存在，是否覆蓋？`, {
          title: '覆蓋既有 PDF？',
          kind: 'warning',
        });
        if (!overwrite) {
          status = '已取消，未覆蓋既有檔案。';
          return;
        }
      }

      if (outputMode === 'imposition') {
        const result = await invoke<{ copies: number; output: string }>('generate_imposition', {
          input: inputPath,
          outputName,
          overwrite: exists,
          options: impositionOptions,
        });
        status = `完成：${result.copies} 份拼版 → ${result.output}`;
      } else {
        const result = await invoke<{ pages: number; output: string }>('generate_poster', {
          input: inputPath,
          outputName,
          overwrite: exists,
          options: posterOptions,
        });
        status = `完成：${result.pages} 頁 A4 → ${result.output}`;
      }
    } catch (error) {
      status = `產生失敗：${error}`;
    } finally {
      busy = false;
    }
  }

  function posterViewBox(info: PosterPreview) {
    return `0 0 ${info.pageWidthPt * info.cols} ${info.pageHeightPt * info.rows}`;
  }

  function impositionViewBox(info: ImpositionPreview) {
    return `0 0 ${info.pageWidthPt} ${info.pageHeightPt}`;
  }

  function rectWidth(rect: Rect) {
    return rect.x1 - rect.x0;
  }

  function rectHeight(rect: Rect) {
    return rect.y1 - rect.y0;
  }

  function rotatedImageViewport(rect: Rect) {
    const width = rectWidth(rect);
    const height = rectHeight(rect);
    const centerX = (rect.x0 + rect.x1) / 2;
    const centerY = (rect.y0 + rect.y1) / 2;
    return {
      x0: centerX - height / 2,
      y0: centerY - width / 2,
      x1: centerX + height / 2,
      y1: centerY + width / 2,
    };
  }

  function rotateAroundCenter(rect: Rect) {
    const centerX = (rect.x0 + rect.x1) / 2;
    const centerY = (rect.y0 + rect.y1) / 2;
    return `rotate(90 ${centerX} ${centerY})`;
  }
</script>

<div class:dragging={isDraggingFiles} class="app">
  <aside class="panel sidebar" aria-label="輸出設定">
    <div class="sidebar-scroll">
      <div class="header">
      <h1>Poster Maker</h1>
      <span class="version">v{version}</span>
    </div>

    <fieldset class="mode-fieldset">
      <legend>輸出模式</legend>
      <div class="mode-switch">
        <label class:active={outputMode === 'poster'}>
          <input type="radio" name="output-mode" value="poster" bind:group={outputMode} />
          <span>海報分割</span>
        </label>
        <label class:active={outputMode === 'imposition'}>
          <input type="radio" name="output-mode" value="imposition" bind:group={outputMode} />
          <span>一張多份</span>
        </label>
      </div>
    </fieldset>

    <div class="field">
      <label class="label" for="source-image">來源圖片</label>
      <div class="row">
        <input id="source-image" bind:value={inputPath} placeholder="PNG / JPG / WEBP / BMP / TIFF" aria-describedby="source-image-help" />
        <button type="button" on:click={pickInput}>選擇…</button>
      </div>
      <div id="source-image-help" class="muted">可拖曳一張圖片到整個視窗，或使用「選擇」。</div>
    </div>

    <div class="field">
      <label class="label" for="output-name">輸出 PDF 檔名</label>
      <input id="output-name" bind:value={outputName} placeholder="example-poster.pdf" />
      <div class="muted">預設存到來源圖片的同一個資料夾。</div>
    </div>

    {#if outputMode === 'poster'}
      <section class="settings-section" aria-labelledby="poster-settings-title">
        <h2 id="poster-settings-title">海報分割尺寸</h2>
        <div class="field">
          <label class="label" for="poster-grid">輸出尺寸</label>
          <select id="poster-grid" bind:value={grid} on:change={gridChanged}>
            <option>2x1 / 1x2</option>
            <option>2x2</option>
            <option>3x2 / 2x3</option>
            <option>3x3</option>
            <option>4x2 / 2x4</option>
            <option>4x3 / 3x4</option>
            <option>4x4</option>
            <option>Custom</option>
            <option>Custom Size</option>
          </select>
          <div class="muted">選擇 A4 張數，系統會自動選擇最佳擺放。</div>
        </div>

        {#if grid === 'Custom'}
          <div class="custom-grid" aria-label="自訂海報分割張數">
            <div class="stepper">
              <span class="stepper-label">欄</span>
              <button type="button" aria-label="欄數減一" on:click={() => adjustCustom('cols', -1)}>−</button>
              <output aria-live="polite">{customCols}</output>
              <button type="button" aria-label="欄數加一" on:click={() => adjustCustom('cols', 1)}>+</button>
            </div>
            <div class="stepper">
              <span class="stepper-label">列</span>
              <button type="button" aria-label="列數減一" on:click={() => adjustCustom('rows', -1)}>−</button>
              <output aria-live="polite">{customRows}</output>
              <button type="button" aria-label="列數加一" on:click={() => adjustCustom('rows', 1)}>+</button>
            </div>
          </div>
        {/if}

        {#if usesCustomSizeMode}
          <div class="grid2">
            <div class="field compact-field">
              <label for="target-width">寬（mm）</label>
              <input id="target-width" type="number" min="1" step="1" bind:value={targetWidthMm} />
            </div>
            <div class="field compact-field">
              <label for="target-height">高（mm）</label>
              <input id="target-height" type="number" min="1" step="1" bind:value={targetHeightMm} />
            </div>
          </div>
          <div class="muted">依圖片比例輸出；只填一邊時會自動推算另一邊與所需 A4 張數。</div>
        {/if}
      </section>
    {:else}
      <section class="settings-section" aria-labelledby="imposition-settings-title">
        <h2 id="imposition-settings-title">一張多份設定</h2>

        <div class="field">
          <label class="label" for="item-preset">成品尺寸</label>
          <select id="item-preset" bind:value={itemPreset} on:change={itemPresetChanged}>
            <option value="A5">A5 — 148 × 210 mm</option>
            <option value="A6">A6 — 105 × 148 mm</option>
            <option value="A7">A7 — 74 × 105 mm</option>
            <option value="Custom">Custom</option>
          </select>
        </div>

        {#if itemPreset === 'Custom'}
          <div class="grid2">
            <div class="field compact-field">
              <label for="item-width">成品寬（mm）</label>
              <input id="item-width" type="number" min="0.1" max="10000" step="0.1" bind:value={itemWidthMm} on:input={useCustomItemSize} />
            </div>
            <div class="field compact-field">
              <label for="item-height">成品高（mm）</label>
              <input id="item-height" type="number" min="0.1" max="10000" step="0.1" bind:value={itemHeightMm} on:input={useCustomItemSize} />
            </div>
          </div>
        {/if}

        <fieldset class="measure-group">
          <legend>外部紙張</legend>
          <div class="grid2">
            <div class="field compact-field">
              <label for="paper-width">寬（mm）</label>
              <input id="paper-width" type="number" min="0.1" max="10000" step="0.1" bind:value={paperWidthMm} />
            </div>
            <div class="field compact-field">
              <label for="paper-height">高（mm）</label>
              <input id="paper-height" type="number" min="0.1" max="10000" step="0.1" bind:value={paperHeightMm} />
            </div>
          </div>
        </fieldset>

        <fieldset class="measure-group">
          <legend>安全邊界</legend>
          <div class="edge-grid">
            <div class="field compact-field">
              <label for="safety-top">上（mm）</label>
              <input id="safety-top" type="number" min="0" step="0.1" bind:value={safetyTopMm} />
            </div>
            <div class="field compact-field">
              <label for="safety-right">右（mm）</label>
              <input id="safety-right" type="number" min="0" step="0.1" bind:value={safetyRightMm} />
            </div>
            <div class="field compact-field">
              <label for="safety-bottom">下（mm）</label>
              <input id="safety-bottom" type="number" min="0" step="0.1" bind:value={safetyBottomMm} />
            </div>
            <div class="field compact-field">
              <label for="safety-left">左（mm）</label>
              <input id="safety-left" type="number" min="0" step="0.1" bind:value={safetyLeftMm} />
            </div>
          </div>
        </fieldset>

        <p class="explanation">裁切線位於成品尺寸的外框。圖片會完整保留在向內扣除安全邊界的區域；比例不同的部分會留白。</p>
      </section>
    {/if}
    </div>

    <div class="action-area">
      <button class="primary" type="button" disabled={busy || (outputMode === 'imposition' && !impositionOptionsValid)} on:click={generate}>
        {busy ? '產生中…' : outputMode === 'imposition' ? '產生拼版 PDF' : '產生海報 PDF'}
      </button>
      <p class="status" aria-live="polite" aria-atomic="true">{status}</p>
    </div>
  </aside>

  <main class="panel preview-panel" aria-label="預覽">
    <div class="preview-heading">
      <h2>{outputMode === 'imposition' ? '拼版預覽' : '海報分割預覽'}</h2>
      {#if outputMode === 'imposition' && impositionPreview}
        <span>虛線是安全邊界，僅供預覽</span>
      {/if}
    </div>
    <div class="preview-wrap">
      {#if outputMode === 'poster' && posterPreview && posterGeometry}
        <svg class="preview-svg" viewBox={posterViewBox(posterPreview)} preserveAspectRatio="xMidYMid meet" aria-label="海報分割預覽" role="img">
          <defs>
            <pattern id="paper-grid" width="18" height="18" patternUnits="userSpaceOnUse">
              <path d="M 18 0 L 0 0 0 18" fill="none" stroke="#e9edf1" stroke-width="1" />
            </pattern>
          </defs>
          <rect x="0" y="0" width={posterPreview.pageWidthPt * posterPreview.cols} height={posterPreview.pageHeightPt * posterPreview.rows} fill="url(#paper-grid)" />
          {#each posterGeometry.pages as page}
            <rect x={page.col * posterPreview.pageWidthPt} y={page.row * posterPreview.pageHeightPt} width={posterPreview.pageWidthPt} height={posterPreview.pageHeightPt} fill="white" stroke="#151a20" stroke-width="1.2" />
            <svg x={page.col * posterPreview.pageWidthPt + page.destPage.x0} y={page.row * posterPreview.pageHeightPt + page.destPage.y0} width={page.destPage.x1 - page.destPage.x0} height={page.destPage.y1 - page.destPage.y0} viewBox={`${page.clipCanvas.x0} ${page.clipCanvas.y0} ${page.clipCanvas.x1 - page.clipCanvas.x0} ${page.clipCanvas.y1 - page.clipCanvas.y0}`} preserveAspectRatio="none">
              <image href={previewImageSrc} x={posterGeometry.imageCanvas.x0} y={posterGeometry.imageCanvas.y0} width={posterPreview.imageWidthPt} height={posterPreview.imageHeightPt} preserveAspectRatio="none" />
            </svg>
            {#each page.outerLines as line}
              <line x1={page.col * posterPreview.pageWidthPt + line.a.x} y1={page.row * posterPreview.pageHeightPt + line.a.y} x2={page.col * posterPreview.pageWidthPt + line.b.x} y2={page.row * posterPreview.pageHeightPt + line.b.y} class="poster-outer-guide" />
            {/each}
            {#each page.cutLines as line}
              <line x1={page.col * posterPreview.pageWidthPt + line.a.x} y1={page.row * posterPreview.pageHeightPt + line.a.y} x2={page.col * posterPreview.pageWidthPt + line.b.x} y2={page.row * posterPreview.pageHeightPt + line.b.y} class="poster-cut-guide" />
            {/each}
            {#each page.markers as marker}
              <g class="poster-marker">
                <rect x={page.col * posterPreview.pageWidthPt + marker.rect.x0} y={page.row * posterPreview.pageHeightPt + marker.rect.y0} width={marker.rect.x1 - marker.rect.x0} height={marker.rect.y1 - marker.rect.y0} />
                <line x1={page.col * posterPreview.pageWidthPt + marker.rect.x0} y1={page.row * posterPreview.pageHeightPt + marker.rect.y0} x2={page.col * posterPreview.pageWidthPt + marker.rect.x1} y2={page.row * posterPreview.pageHeightPt + marker.rect.y1} />
                <line x1={page.col * posterPreview.pageWidthPt + marker.rect.x0} y1={page.row * posterPreview.pageHeightPt + marker.rect.y1} x2={page.col * posterPreview.pageWidthPt + marker.rect.x1} y2={page.row * posterPreview.pageHeightPt + marker.rect.y0} />
              </g>
            {/each}
          {/each}
        </svg>
      {:else if outputMode === 'imposition' && impositionPreview}
        <svg class="preview-svg imposition-svg" viewBox={impositionViewBox(impositionPreview)} preserveAspectRatio="xMidYMid meet" aria-label={`一張多份拼版預覽，共 ${impositionPreview.copies} 份`} role="img">
          <defs>
            {#each impositionPreview.placements as placement, index}
              <clipPath id={`image-clip-${index}`}>
                <rect x={placement.imageRect.x0} y={placement.imageRect.y0} width={rectWidth(placement.imageRect)} height={rectHeight(placement.imageRect)} />
              </clipPath>
            {/each}
          </defs>
          <rect x="0" y="0" width={impositionPreview.pageWidthPt} height={impositionPreview.pageHeightPt} class="imposition-paper" />
          {#each impositionPreview.placements as placement, index}
            <rect x={placement.imageRect.x0} y={placement.imageRect.y0} width={rectWidth(placement.imageRect)} height={rectHeight(placement.imageRect)} class="imposition-image-bed" />
            <g clip-path={`url(#image-clip-${index})`}>
              {#if impositionPreview.itemRotated}
                {@const viewport = rotatedImageViewport(placement.imageRect)}
                <g transform={rotateAroundCenter(placement.imageRect)}>
                  <image href={previewImageSrc} x={viewport.x0} y={viewport.y0} width={rectWidth(viewport)} height={rectHeight(viewport)} preserveAspectRatio="xMidYMid meet" />
                </g>
              {:else}
                <image href={previewImageSrc} x={placement.imageRect.x0} y={placement.imageRect.y0} width={rectWidth(placement.imageRect)} height={rectHeight(placement.imageRect)} preserveAspectRatio="xMidYMid meet" />
              {/if}
            </g>
            <rect x={placement.cutRect.x0} y={placement.cutRect.y0} width={rectWidth(placement.cutRect)} height={rectHeight(placement.cutRect)} class="imposition-cut-frame" />
            <rect x={placement.safeRect.x0} y={placement.safeRect.y0} width={rectWidth(placement.safeRect)} height={rectHeight(placement.safeRect)} class="imposition-safe-frame" />
          {/each}
        </svg>
      {:else}
        <div class="empty-preview">
          <strong>{inputPath ? '正在建立預覽…' : '從這裡開始'}</strong>
          <span>{inputPath ? '正在讀取目前設定。' : '選擇或拖曳一張圖片，然後選擇輸出模式。'}</span>
        </div>
      {/if}
    </div>
  </main>

  {#if isDraggingFiles}
    <div class="drag-notice" aria-hidden="true">
      <div>放開以載入一張圖片</div>
      <span>PNG、JPG、WEBP、BMP 或 TIFF</span>
    </div>
  {/if}
</div>
