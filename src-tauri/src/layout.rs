use serde::{Deserialize, Serialize};

// Image pixels are treated as abstract source units for aspect-ratio/layout.
// Physical output size is determined only by A4 layout, margin, and overlap.
const A4_WIDTH_PT: f64 = 595.2755905512;
const A4_HEIGHT_PT: f64 = 841.8897637795;
const MM_TO_PT: f64 = 72.0 / 25.4;
const MARKER_SIZE_PT: f64 = 10.0;
const MARKER_GAP_PT: f64 = 2.0;
const MARKER_STROKE_PT: f64 = 1.1;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PosterOptions {
    pub cols: u32,
    pub rows: u32,
    pub target_width_mm: Option<f64>,
    pub target_height_mm: Option<f64>,
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
    if landscape {
        (A4_HEIGHT_PT, A4_WIDTH_PT)
    } else {
        (A4_WIDTH_PT, A4_HEIGHT_PT)
    }
}

fn max_overlap_extra(count: u32, overlap: f64) -> f64 {
    match count {
        0 | 1 => 0.0,
        2 => overlap,
        _ => overlap * 2.0,
    }
}

pub fn poster_canvas_size(
    page_w: f64,
    page_h: f64,
    cols: u32,
    rows: u32,
    margin: f64,
    overlap: f64,
) -> Result<(f64, f64, f64, f64), String> {
    let printable_w = page_w - margin * 2.0;
    let printable_h = page_h - margin * 2.0;
    let base_w = printable_w - max_overlap_extra(cols, overlap);
    let base_h = printable_h - max_overlap_extra(rows, overlap);
    if base_w <= 0.0 || base_h <= 0.0 {
        return Err("Overlap/margin too large for selected layout".into());
    }
    Ok((base_w, base_h, base_w * cols as f64, base_h * rows as f64))
}

fn candidate(
    src_w_pt: f64,
    src_h_pt: f64,
    cols: u32,
    rows: u32,
    landscape: bool,
    options: &PosterOptions,
) -> Result<PreviewInfo, String> {
    let (page_w, page_h) = page_size(landscape);
    let margin = reserved_margin(options);
    let overlap = mm(options.overlap_mm);
    let (max_base_w, max_base_h, _, _) =
        poster_canvas_size(page_w, page_h, cols, rows, margin, overlap)?;
    let (base_w, base_h) =
        fit_base_tile_to_source_ratio(max_base_w, max_base_h, cols, rows, src_w_pt / src_h_pt);
    let canvas_w = base_w * cols as f64;
    let canvas_h = base_h * rows as f64;
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
        image_width_pt: canvas_w,
        image_height_pt: canvas_h,
        image_width_cm: pt_to_cm(canvas_w),
        image_height_cm: pt_to_cm(canvas_h),
        paper_width_cm: pt_to_cm(page_w * cols as f64),
        paper_height_cm: pt_to_cm(page_h * rows as f64),
        score: canvas_w * canvas_h,
    })
}

fn fit_base_tile_to_source_ratio(
    max_base_w: f64,
    max_base_h: f64,
    cols: u32,
    rows: u32,
    src_ratio: f64,
) -> (f64, f64) {
    let target_base_ratio = src_ratio * rows as f64 / cols as f64;
    let base_h_for_max_w = max_base_w / target_base_ratio;
    if base_h_for_max_w <= max_base_h {
        (max_base_w, base_h_for_max_w)
    } else {
        (max_base_h * target_base_ratio, max_base_h)
    }
}

pub fn default_options(cols: u32, rows: u32) -> PosterOptions {
    PosterOptions {
        cols,
        rows,
        target_width_mm: None,
        target_height_mm: None,
        overlap_mm: 5.0,
        margin_mm: 1.0,
        draw_outer_marks: true,
        draw_cut_guides: true,
    }
}

pub fn resolve_layout(
    src_w_px: u32,
    src_h_px: u32,
    options: &PosterOptions,
) -> Result<PreviewInfo, String> {
    if options.cols == 0 || options.rows == 0 {
        return Err("Rows/cols must be >= 1".into());
    }
    if options.overlap_mm < 0.0 || options.margin_mm < 0.0 {
        return Err("Invalid margin/overlap".into());
    }
    let src_w_pt = src_w_px as f64;
    let src_h_pt = src_h_px as f64;
    if let Some((target_w, target_h)) = target_canvas_size(src_w_pt, src_h_pt, options)? {
        return resolve_target_layout(target_w, target_h, options);
    }
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
                    let cand_ratio = (cand.page_width_pt * cand.cols as f64)
                        / (cand.page_height_pt * cand.rows as f64);
                    let prev_ratio = (prev.page_width_pt * prev.cols as f64)
                        / (prev.page_height_pt * prev.rows as f64);
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

fn target_canvas_size(
    src_w_pt: f64,
    src_h_pt: f64,
    options: &PosterOptions,
) -> Result<Option<(f64, f64)>, String> {
    let target_w = checked_target_mm(options.target_width_mm, "width")?;
    let target_h = checked_target_mm(options.target_height_mm, "height")?;
    let ratio = src_w_pt / src_h_pt;
    Ok(match (target_w, target_h) {
        (None, None) => None,
        (Some(w), None) => Some((w, w / ratio)),
        (None, Some(h)) => Some((h * ratio, h)),
        (Some(w), Some(h)) => {
            let scale = (w / src_w_pt).min(h / src_h_pt);
            Some((src_w_pt * scale, src_h_pt * scale))
        }
    })
}

fn checked_target_mm(value: Option<f64>, name: &str) -> Result<Option<f64>, String> {
    match value {
        None => Ok(None),
        Some(v) if v.is_finite() && v > 0.0 => Ok(Some(mm(v))),
        Some(_) => Err(format!("Target {name} must be greater than 0mm")),
    }
}

fn resolve_target_layout(
    target_w: f64,
    target_h: f64,
    options: &PosterOptions,
) -> Result<PreviewInfo, String> {
    let mut best: Option<PreviewInfo> = None;
    for cols in 1..=12 {
        for rows in 1..=12 {
            for &landscape in &[false, true] {
                let Some(cand) =
                    target_candidate(target_w, target_h, cols, rows, landscape, options)?
                else {
                    continue;
                };
                let replace = match &best {
                    None => true,
                    Some(prev) if cand.cols * cand.rows < prev.cols * prev.rows => true,
                    Some(prev)
                        if cand.cols * cand.rows == prev.cols * prev.rows
                            && cand.score < prev.score =>
                    {
                        true
                    }
                    _ => false,
                };
                if replace {
                    best = Some(cand);
                }
            }
        }
    }
    best.ok_or_else(|| "Target size is too large for the 12x12 A4 limit".into())
}

fn target_candidate(
    target_w: f64,
    target_h: f64,
    cols: u32,
    rows: u32,
    landscape: bool,
    options: &PosterOptions,
) -> Result<Option<PreviewInfo>, String> {
    let (page_w, page_h) = page_size(landscape);
    let margin = reserved_margin(options);
    let overlap = mm(options.overlap_mm);
    let printable_w = page_w - margin * 2.0;
    let printable_h = page_h - margin * 2.0;
    let required_base_w = target_w / cols as f64;
    let required_base_h = target_h / rows as f64;
    let max_base_w = printable_w - max_overlap_extra(cols, overlap);
    let max_base_h = printable_h - max_overlap_extra(rows, overlap);
    if max_base_w <= 0.0 || max_base_h <= 0.0 {
        return Err("Overlap/margin too large for A4 paper".into());
    }
    if required_base_w > max_base_w + 0.001 || required_base_h > max_base_h + 0.001 {
        return Ok(None);
    }
    let capacity_w = max_base_w * cols as f64;
    let capacity_h = max_base_h * rows as f64;
    Ok(Some(PreviewInfo {
        cols,
        rows,
        landscape,
        page_width_pt: page_w,
        page_height_pt: page_h,
        base_tile_width_pt: required_base_w,
        base_tile_height_pt: required_base_h,
        canvas_width_pt: target_w,
        canvas_height_pt: target_h,
        image_width_pt: target_w,
        image_height_pt: target_h,
        image_width_cm: pt_to_cm(target_w),
        image_height_cm: pt_to_cm(target_h),
        paper_width_cm: pt_to_cm(page_w * cols as f64),
        paper_height_cm: pt_to_cm(page_h * rows as f64),
        score: (capacity_w * capacity_h) - (target_w * target_h),
    }))
}

fn reserved_margin(options: &PosterOptions) -> f64 {
    let margin = mm(options.margin_mm);
    let marker_clearance = if options.draw_cut_guides {
        MARKER_SIZE_PT + MARKER_GAP_PT + MARKER_STROKE_PT / 2.0
    } else {
        0.0
    };
    margin + marker_clearance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(cols: u32, rows: u32) -> PosterOptions {
        default_options(cols, rows)
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
        assert!(
            l.base_tile_width_pt + overlap * 2.0 <= l.page_width_pt - mm(o.margin_mm) * 2.0 + 0.01
                || l.cols <= 2
        );
    }

    #[test]
    fn fixed_grid_fills_one_axis_without_letterboxing() {
        let o = opts(3, 3);
        let l = resolve_layout(1055, 1491, &o).unwrap();
        let source_ratio = 1055.0 / 1491.0;
        let canvas_ratio = l.canvas_width_pt / l.canvas_height_pt;

        assert!((canvas_ratio - source_ratio).abs() < 0.000001);
        assert!((l.image_width_pt - l.canvas_width_pt).abs() < 0.001);
        assert!((l.image_height_pt - l.canvas_height_pt).abs() < 0.001);
    }

    #[test]
    fn target_size_resolves_minimum_grid() {
        let mut o = opts(3, 2);
        o.target_width_mm = Some(600.0);
        o.target_height_mm = Some(400.0);
        let l = resolve_layout(1200, 800, &o).unwrap();
        assert_eq!(l.cols * l.rows, 8);
        assert!((l.image_width_cm - 60.0).abs() < 0.01);
        assert!((l.image_height_cm - 40.0).abs() < 0.01);
    }

    #[test]
    fn target_single_side_uses_source_ratio() {
        let mut o = opts(3, 2);
        o.target_width_mm = Some(500.0);
        let l = resolve_layout(1200, 800, &o).unwrap();
        assert!((l.image_width_cm - 50.0).abs() < 0.01);
        assert!((l.image_height_cm - 33.333).abs() < 0.01);
    }

    #[test]
    fn target_box_does_not_distort_source_ratio() {
        let mut o = opts(3, 2);
        o.target_width_mm = Some(500.0);
        o.target_height_mm = Some(500.0);
        let l = resolve_layout(1200, 800, &o).unwrap();
        assert!((l.image_width_cm - 50.0).abs() < 0.01);
        assert!((l.image_height_cm - 33.333).abs() < 0.01);
    }
}
