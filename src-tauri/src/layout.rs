use serde::{Deserialize, Serialize};

// Image pixels are treated as abstract source units for aspect-ratio/layout.
// Physical output size is determined only by A4 layout, margin, and overlap.
const A4_WIDTH_PT: f64 = 595.2755905512;
const A4_HEIGHT_PT: f64 = 841.8897637795;
const MM_TO_PT: f64 = 72.0 / 25.4;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PosterOptions {
    pub cols: u32,
    pub rows: u32,
    pub overlap_mm: f64,
    pub margin_mm: f64,
    pub draw_outer_marks: bool,
    pub draw_cut_guides: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewInfo {
    pub cols: u32,
    pub rows: u32,
    pub landscape: bool,
    pub page_width_pt: f64,
    pub page_height_pt: f64,
    pub base_tile_width_pt: f64,
    pub base_tile_height_pt: f64,
    pub canvas_width_pt: f64,
    pub canvas_height_pt: f64,
    pub image_width_pt: f64,
    pub image_height_pt: f64,
    pub image_width_cm: f64,
    pub image_height_cm: f64,
    pub paper_width_cm: f64,
    pub paper_height_cm: f64,
    pub score: f64,
}

pub fn mm(value: f64) -> f64 {
    value * MM_TO_PT
}

pub fn pt_to_cm(value: f64) -> f64 {
    value / 72.0 * 2.54
}

pub fn page_size(landscape: bool) -> (f64, f64) {
    if landscape { (A4_HEIGHT_PT, A4_WIDTH_PT) } else { (A4_WIDTH_PT, A4_HEIGHT_PT) }
}

fn max_overlap_extra(count: u32, overlap: f64) -> f64 {
    match count {
        0 | 1 => 0.0,
        2 => overlap,
        _ => overlap * 2.0,
    }
}

pub fn poster_canvas_size(page_w: f64, page_h: f64, cols: u32, rows: u32, margin: f64, overlap: f64) -> Result<(f64, f64, f64, f64), String> {
    let printable_w = page_w - margin * 2.0;
    let printable_h = page_h - margin * 2.0;
    let base_w = printable_w - max_overlap_extra(cols, overlap);
    let base_h = printable_h - max_overlap_extra(rows, overlap);
    if base_w <= 0.0 || base_h <= 0.0 {
        return Err("Overlap/margin too large for selected layout".into());
    }
    Ok((base_w, base_h, base_w * cols as f64, base_h * rows as f64))
}

fn candidate(src_w_pt: f64, src_h_pt: f64, cols: u32, rows: u32, landscape: bool, options: &PosterOptions) -> Result<PreviewInfo, String> {
    let (page_w, page_h) = page_size(landscape);
    let margin = mm(options.margin_mm);
    let overlap = mm(options.overlap_mm);
    let (base_w, base_h, canvas_w, canvas_h) = poster_canvas_size(page_w, page_h, cols, rows, margin, overlap)?;
    let scale = (canvas_w / src_w_pt).min(canvas_h / src_h_pt);
    let image_w = src_w_pt * scale;
    let image_h = src_h_pt * scale;
    Ok(PreviewInfo {
        cols,
        rows,
        landscape,
        page_width_pt: page_w,
        page_height_pt: page_h,
        base_tile_width_pt: base_w,
        base_tile_height_pt: base_h,
        canvas_width_pt: canvas_w,
        canvas_height_pt: canvas_h,
        image_width_pt: image_w,
        image_height_pt: image_h,
        image_width_cm: pt_to_cm(image_w),
        image_height_cm: pt_to_cm(image_h),
        paper_width_cm: pt_to_cm(page_w * cols as f64),
        paper_height_cm: pt_to_cm(page_h * rows as f64),
        score: image_w * image_h,
    })
}

pub fn resolve_layout(src_w_px: u32, src_h_px: u32, options: &PosterOptions) -> Result<PreviewInfo, String> {
    if options.cols == 0 || options.rows == 0 {
        return Err("Rows/cols must be >= 1".into());
    }
    if options.overlap_mm < 0.0 || options.margin_mm < 0.0 {
        return Err("Invalid margin/overlap".into());
    }
    let src_w_pt = src_w_px as f64;
    let src_h_pt = src_h_px as f64;
    let src_ratio = src_w_pt / src_h_pt;
    let mut grids = vec![(options.cols, options.rows)];
    if options.cols != options.rows {
        grids.push((options.rows, options.cols));
    }
    let landscapes: Vec<bool> = vec![false, true];
    let mut best: Option<PreviewInfo> = None;
    for (cols, rows) in grids {
        for &landscape in &landscapes {
            let cand = candidate(src_w_pt, src_h_pt, cols, rows, landscape, options)?;
            let replace = match &best {
                None => true,
                Some(prev) if cand.score > prev.score * 1.000001 => true,
                Some(prev) if (cand.score - prev.score).abs() <= prev.score.max(1.0) * 0.000001 => {
                    let cand_ratio = (cand.page_width_pt * cand.cols as f64) / (cand.page_height_pt * cand.rows as f64);
                    let prev_ratio = (prev.page_width_pt * prev.cols as f64) / (prev.page_height_pt * prev.rows as f64);
                    (cand_ratio - src_ratio).abs() < (prev_ratio - src_ratio).abs()
                }
                _ => false,
            };
            if replace {
                best = Some(cand);
            }
        }
    }
    best.ok_or_else(|| "No valid layout".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(cols: u32, rows: u32) -> PosterOptions {
        PosterOptions { cols, rows, overlap_mm: 5.0, margin_mm: 3.0, draw_outer_marks: true, draw_cut_guides: true }
    }

    #[test]
    fn swapped_grids_resolve_same() {
        let a = resolve_layout(1200, 1800, &opts(3, 2)).unwrap();
        let b = resolve_layout(1200, 1800, &opts(2, 3)).unwrap();
        assert_eq!((a.cols, a.rows, a.landscape), (b.cols, b.rows, b.landscape));
    }

    #[test]
    fn base_tile_leaves_room_for_middle_overlap() {
        let o = opts(3, 2);
        let l = resolve_layout(1200, 800, &o).unwrap();
        let overlap = mm(o.overlap_mm);
        assert!(l.base_tile_width_pt + overlap * 2.0 <= l.page_width_pt - mm(o.margin_mm) * 2.0 + 0.01 || l.cols <= 2);
    }
}
