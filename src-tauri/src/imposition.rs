//! Pure-Rust layout model for placing many identical finished items on one sheet.
//!
//! All geometry returned by this module uses PDF points and a top-left origin. This
//! keeps the geometry directly usable by an SVG preview; the PDF renderer converts
//! the y-axis only when it writes drawing commands.

use crate::layout::mm;
use serde::{Deserialize, Serialize};

const MAX_DIMENSION_MM: f64 = 10_000.0;
const MAX_COPIES: u32 = 1_000;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpositionOptions {
    pub paper_width_mm: f64,
    pub paper_height_mm: f64,
    pub item_width_mm: f64,
    pub item_height_mm: f64,
    pub safety_top_mm: f64,
    pub safety_right_mm: f64,
    pub safety_bottom_mm: f64,
    pub safety_left_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImpositionRect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl ImpositionRect {
    pub fn width(self) -> f64 {
        self.x1 - self.x0
    }

    pub fn height(self) -> f64 {
        self.y1 - self.y0
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpositionPlacement {
    /// Finished item boundary. This is the only printed cutting guide.
    pub cut_rect: ImpositionRect,
    /// The image containment box after applying the item's safety margins.
    pub safe_rect: ImpositionRect,
    /// The source image fitted with `contain` inside `safe_rect`.
    pub image_rect: ImpositionRect,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpositionPreview {
    pub page_width_pt: f64,
    pub page_height_pt: f64,
    pub paper_width_cm: f64,
    pub paper_height_cm: f64,
    pub cols: u32,
    pub rows: u32,
    pub copies: u32,
    pub item_rotated: bool,
    /// Item dimensions in their physical orientation on the sheet.
    pub item_width_pt: f64,
    pub item_height_pt: f64,
    pub used_width_pt: f64,
    pub used_height_pt: f64,
    pub placements: Vec<ImpositionPlacement>,
}

#[derive(Debug, Clone, Copy)]
struct Arrangement {
    cols: u32,
    rows: u32,
    copies: u32,
    item_rotated: bool,
    item_width_mm: f64,
    item_height_mm: f64,
}

impl ImpositionOptions {
    /// Validate dimensions before attempting a physical layout.
    pub fn validate(&self) -> Result<(), String> {
        validate_dimension(self.paper_width_mm, "Paper width")?;
        validate_dimension(self.paper_height_mm, "Paper height")?;
        validate_dimension(self.item_width_mm, "Item width")?;
        validate_dimension(self.item_height_mm, "Item height")?;

        for (name, value) in [
            ("Safety top", self.safety_top_mm),
            ("Safety right", self.safety_right_mm),
            ("Safety bottom", self.safety_bottom_mm),
            ("Safety left", self.safety_left_mm),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!(
                    "{name} must be a finite value greater than or equal to 0mm"
                ));
            }
        }

        if self.safety_left_mm + self.safety_right_mm >= self.item_width_mm {
            return Err("Safety left and right must total less than the item width".into());
        }
        if self.safety_top_mm + self.safety_bottom_mm >= self.item_height_mm {
            return Err("Safety top and bottom must total less than the item height".into());
        }
        Ok(())
    }
}

/// Determine the maximum-count, single-orientation arrangement for an item.
///
/// The two permitted candidates are the original orientation and a clockwise
/// quarter turn. On an equal copy count, the original orientation wins.
pub fn best_arrangement(options: &ImpositionOptions) -> Result<(u32, u32, bool), String> {
    let arrangement = resolve_arrangement(options)?;
    Ok((arrangement.cols, arrangement.rows, arrangement.item_rotated))
}

/// Build all preview geometry for a source image of the supplied pixel size.
///
/// Pixel dimensions are only used for aspect-ratio containment. They never set
/// the printed physical size.
pub fn resolve_imposition(
    image_width_px: u32,
    image_height_px: u32,
    options: &ImpositionOptions,
) -> Result<ImpositionPreview, String> {
    if image_width_px == 0 || image_height_px == 0 {
        return Err("Image dimensions must be greater than zero".into());
    }

    let arrangement = resolve_arrangement(options)?;
    let page_width_pt = mm(options.paper_width_mm);
    let page_height_pt = mm(options.paper_height_mm);
    let item_width_pt = mm(arrangement.item_width_mm);
    let item_height_pt = mm(arrangement.item_height_mm);
    let used_width_pt = item_width_pt * arrangement.cols as f64;
    let used_height_pt = item_height_pt * arrangement.rows as f64;
    let start_x = (page_width_pt - used_width_pt) / 2.0;
    let start_y = (page_height_pt - used_height_pt) / 2.0;

    let (source_width, source_height) = if arrangement.item_rotated {
        // The PDF writer embeds one clockwise-rotated raster. Its aspect ratio
        // must match the preview image rectangles.
        (image_height_px as f64, image_width_px as f64)
    } else {
        (image_width_px as f64, image_height_px as f64)
    };

    let mut placements = Vec::with_capacity(arrangement.copies as usize);
    for row in 0..arrangement.rows {
        for col in 0..arrangement.cols {
            let cut_rect = ImpositionRect {
                x0: start_x + col as f64 * item_width_pt,
                y0: start_y + row as f64 * item_height_pt,
                x1: start_x + (col + 1) as f64 * item_width_pt,
                y1: start_y + (row + 1) as f64 * item_height_pt,
            };
            let safe_rect = safe_rect_for_cut(cut_rect, options, arrangement.item_rotated);
            let image_rect = contain_rect(source_width, source_height, safe_rect);
            placements.push(ImpositionPlacement {
                cut_rect,
                safe_rect,
                image_rect,
            });
        }
    }

    Ok(ImpositionPreview {
        page_width_pt,
        page_height_pt,
        paper_width_cm: options.paper_width_mm / 10.0,
        paper_height_cm: options.paper_height_mm / 10.0,
        cols: arrangement.cols,
        rows: arrangement.rows,
        copies: arrangement.copies,
        item_rotated: arrangement.item_rotated,
        item_width_pt,
        item_height_pt,
        used_width_pt,
        used_height_pt,
        placements,
    })
}

fn resolve_arrangement(options: &ImpositionOptions) -> Result<Arrangement, String> {
    options.validate()?;

    let unrotated = arrangement_candidate(options, false)?;
    let rotated = arrangement_candidate(options, true)?;
    match (unrotated, rotated) {
        (None, None) => Err("Item does not fit on paper in either orientation".into()),
        (Some(candidate), None) | (None, Some(candidate)) => Ok(candidate),
        (Some(unrotated), Some(rotated)) => {
            if rotated.copies > unrotated.copies {
                Ok(rotated)
            } else {
                // This also deliberately resolves equal copy counts to no rotation.
                Ok(unrotated)
            }
        }
    }
}

fn arrangement_candidate(
    options: &ImpositionOptions,
    item_rotated: bool,
) -> Result<Option<Arrangement>, String> {
    let (item_width_mm, item_height_mm) = if item_rotated {
        (options.item_height_mm, options.item_width_mm)
    } else {
        (options.item_width_mm, options.item_height_mm)
    };
    let cols = fit_count(options.paper_width_mm, item_width_mm)?;
    let rows = fit_count(options.paper_height_mm, item_height_mm)?;
    if cols == 0 || rows == 0 {
        return Ok(None);
    }
    let copies = cols
        .checked_mul(rows)
        .ok_or_else(|| "Too many copies fit on one paper sheet".to_string())?;
    if copies > MAX_COPIES {
        return Err(format!(
            "At most {MAX_COPIES} copies can be placed on one paper sheet"
        ));
    }
    Ok(Some(Arrangement {
        cols,
        rows,
        copies,
        item_rotated,
        item_width_mm,
        item_height_mm,
    }))
}

fn fit_count(available_mm: f64, item_mm: f64) -> Result<u32, String> {
    let count = (available_mm / item_mm).floor();
    if count > u32::MAX as f64 {
        return Err("Too many copies fit on one paper sheet".into());
    }
    Ok(count as u32)
}

fn safe_rect_for_cut(
    cut: ImpositionRect,
    options: &ImpositionOptions,
    item_rotated: bool,
) -> ImpositionRect {
    let (top, right, bottom, left) = if item_rotated {
        // Clockwise rotation maps original (top, right, bottom, left) margins
        // to physical (left, top, right, bottom) margins respectively.
        (
            options.safety_left_mm,
            options.safety_top_mm,
            options.safety_right_mm,
            options.safety_bottom_mm,
        )
    } else {
        (
            options.safety_top_mm,
            options.safety_right_mm,
            options.safety_bottom_mm,
            options.safety_left_mm,
        )
    };
    ImpositionRect {
        x0: cut.x0 + mm(left),
        y0: cut.y0 + mm(top),
        x1: cut.x1 - mm(right),
        y1: cut.y1 - mm(bottom),
    }
}

fn contain_rect(
    source_width: f64,
    source_height: f64,
    container: ImpositionRect,
) -> ImpositionRect {
    let scale = (container.width() / source_width).min(container.height() / source_height);
    let width = source_width * scale;
    let height = source_height * scale;
    ImpositionRect {
        x0: container.x0 + (container.width() - width) / 2.0,
        y0: container.y0 + (container.height() - height) / 2.0,
        x1: container.x0 + (container.width() + width) / 2.0,
        y1: container.y0 + (container.height() + height) / 2.0,
    }
}

fn validate_dimension(value: f64, name: &str) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be a finite value greater than 0mm"));
    }
    if value > MAX_DIMENSION_MM {
        return Err(format!("{name} must not exceed {MAX_DIMENSION_MM}mm"));
    }
    let points = mm(value);
    if !points.is_finite() || points <= 0.0 {
        return Err(format!("{name} cannot be represented as PDF points"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(item_width_mm: f64, item_height_mm: f64) -> ImpositionOptions {
        ImpositionOptions {
            paper_width_mm: 210.0,
            paper_height_mm: 297.0,
            item_width_mm,
            item_height_mm,
            safety_top_mm: 0.0,
            safety_right_mm: 0.0,
            safety_bottom_mm: 0.0,
            safety_left_mm: 0.0,
        }
    }

    #[test]
    fn a6_on_a4_is_a_centered_unrotated_two_by_two_grid() {
        let preview = resolve_imposition(1200, 800, &options(105.0, 148.0)).unwrap();
        assert_eq!((preview.cols, preview.rows, preview.copies), (2, 2, 4));
        assert!(!preview.item_rotated);
        assert_close(preview.used_width_pt, mm(210.0));
        assert_close(preview.used_height_pt, mm(296.0));
        assert_close(preview.placements[0].cut_rect.x0, 0.0);
        assert_close(preview.placements[0].cut_rect.y0, mm(0.5));
    }

    #[test]
    fn a7_on_a4_rotates_for_an_eight_copy_two_by_four_grid() {
        let preview = resolve_imposition(1200, 800, &options(74.0, 105.0)).unwrap();
        assert_eq!((preview.cols, preview.rows, preview.copies), (2, 4, 8));
        assert!(preview.item_rotated);
        assert_close(preview.item_width_pt, mm(105.0));
        assert_close(preview.item_height_pt, mm(74.0));
        assert_close(preview.used_height_pt, mm(296.0));
    }

    #[test]
    fn safety_margins_inset_the_safe_box_and_image_keeps_its_ratio() {
        let mut opts = options(100.0, 100.0);
        opts.safety_top_mm = 10.0;
        opts.safety_right_mm = 20.0;
        opts.safety_bottom_mm = 30.0;
        opts.safety_left_mm = 10.0;
        let preview = resolve_imposition(200, 100, &opts).unwrap();
        let placement = &preview.placements[0];

        assert_close(placement.safe_rect.width(), mm(70.0));
        assert_close(placement.safe_rect.height(), mm(60.0));
        assert_close(placement.safe_rect.x0 - placement.cut_rect.x0, mm(10.0));
        assert_close(placement.safe_rect.y0 - placement.cut_rect.y0, mm(10.0));
        assert_close(placement.cut_rect.x1 - placement.safe_rect.x1, mm(20.0));
        assert_close(placement.cut_rect.y1 - placement.safe_rect.y1, mm(30.0));
        assert_close(
            placement.image_rect.width() / placement.image_rect.height(),
            2.0,
        );
        assert!(placement.image_rect.width() <= placement.safe_rect.width());
        assert!(placement.image_rect.height() <= placement.safe_rect.height());
        assert_close(
            placement.image_rect.y0 - placement.safe_rect.y0,
            placement.safe_rect.y1 - placement.image_rect.y1,
        );
    }

    #[test]
    fn rejects_invalid_safety_margins_and_items_that_do_not_fit() {
        let mut negative = options(100.0, 100.0);
        negative.safety_left_mm = -0.1;
        assert!(resolve_imposition(100, 100, &negative).is_err());

        let mut too_wide = options(100.0, 100.0);
        too_wide.safety_left_mm = 50.0;
        too_wide.safety_right_mm = 50.0;
        assert!(resolve_imposition(100, 100, &too_wide).is_err());

        assert!(resolve_imposition(100, 100, &options(400.0, 400.0)).is_err());
    }

    #[test]
    fn rejects_unreasonable_dimensions_and_copy_counts_before_allocating() {
        let mut huge = options(100.0, 100.0);
        huge.paper_width_mm = MAX_DIMENSION_MM + 0.1;
        assert!(resolve_imposition(100, 100, &huge).is_err());

        let too_many = options(0.1, 0.1);
        let error = resolve_imposition(100, 100, &too_many).unwrap_err();
        assert!(error.contains("At most"), "{error}");
    }

    #[test]
    fn rotated_safety_margins_move_with_the_item() {
        let mut opts = options(74.0, 105.0);
        opts.safety_top_mm = 1.0;
        opts.safety_right_mm = 2.0;
        opts.safety_bottom_mm = 3.0;
        opts.safety_left_mm = 4.0;
        let preview = resolve_imposition(100, 200, &opts).unwrap();
        assert!(preview.item_rotated);
        let item = &preview.placements[0];
        // Clockwise: physical top/ right/ bottom/ left = original left/top/right/bottom.
        assert_close(item.safe_rect.y0 - item.cut_rect.y0, mm(4.0));
        assert_close(item.cut_rect.x1 - item.safe_rect.x1, mm(1.0));
        assert_close(item.cut_rect.y1 - item.safe_rect.y1, mm(2.0));
        assert_close(item.safe_rect.x0 - item.cut_rect.x0, mm(3.0));
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "actual={actual}, expected={expected}"
        );
    }
}
