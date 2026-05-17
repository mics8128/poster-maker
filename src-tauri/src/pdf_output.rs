use crate::layout::{mm, PosterOptions, PreviewInfo};
use image::{codecs::jpeg::JpegEncoder, DynamicImage};
use serde::Serialize;

const MARKER_SIZE_PT: f64 = 10.0;
const MARKER_GAP_PT: f64 = 2.0;
const CUT_GUIDE_SAFE_FRACTION: f64 = 0.35;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Rect {
    fn width(self) -> f64 {
        self.x1 - self.x0
    }
    fn height(self) -> f64 {
        self.y1 - self.y0
    }
}

#[derive(Debug, Clone, Copy)]
struct TileGeometry {
    base_canvas: Rect,
    guide_canvas: Rect,
    clip_canvas: Rect,
    guide_page: Rect,
    dest_page: Rect,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideGeometry {
    pub left_x: f64,
    pub right_x: f64,
    pub top_y: f64,
    pub bottom_y: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineGeometry {
    pub a: Point,
    pub b: Point,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkerGeometry {
    pub rect: Rect,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPageGeometry {
    pub row: u32,
    pub col: u32,
    pub clip_canvas: Rect,
    pub dest_page: Rect,
    pub guides: GuideGeometry,
    pub outer_lines: Vec<LineGeometry>,
    pub cut_lines: Vec<LineGeometry>,
    pub markers: Vec<MarkerGeometry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewGeometry {
    pub image_canvas: Rect,
    pub pages: Vec<PreviewPageGeometry>,
}

struct PageChunk {
    content: String,
    image: Vec<u8>,
    image_w: u32,
    image_h: u32,
}

pub fn generate(
    image: &DynamicImage,
    output: &str,
    options: &PosterOptions,
    preview: &PreviewInfo,
) -> Result<(), String> {
    let pages = build_pages(image, options, preview)?;
    let pdf = write_pdf(preview.page_width_pt, preview.page_height_pt, &pages);
    std::fs::write(output, pdf).map_err(|e| e.to_string())
}

fn build_pages(
    image: &DynamicImage,
    options: &PosterOptions,
    preview: &PreviewInfo,
) -> Result<Vec<PageChunk>, String> {
    let image_canvas = image_fit_canvas_size(image.width() as f64, image.height() as f64, preview);
    let mut pages = Vec::new();

    for row in 0..preview.rows {
        for col in 0..preview.cols {
            let tile = tile_geometry(row, col, preview, options, image_canvas)?;
            let page = page_geometry(tile, row, col, options, preview);
            let (sx, sy, sw, sh) = canvas_to_source_crop(
                tile.clip_canvas,
                image_canvas,
                image.width(),
                image.height(),
            );
            let jpeg = encode_tile_jpeg(image, sx, sy, sw, sh)?;
            let content = build_page_content(&page, preview);
            pages.push(PageChunk {
                content,
                image: jpeg,
                image_w: sw,
                image_h: sh,
            });
        }
    }

    Ok(pages)
}

// -----------------------------------------------------------------------------
// Shared tile/guide geometry. PDF output and GUI preview both use this data.
// -----------------------------------------------------------------------------

pub fn preview_geometry_for_image_size(
    image_w: u32,
    image_h: u32,
    options: &PosterOptions,
    preview: &PreviewInfo,
) -> Result<PreviewGeometry, String> {
    let image_canvas = image_fit_canvas_size(image_w as f64, image_h as f64, preview);
    let mut pages = Vec::new();
    for row in 0..preview.rows {
        for col in 0..preview.cols {
            let tile = tile_geometry(row, col, preview, options, image_canvas)?;
            pages.push(page_geometry(tile, row, col, options, preview));
        }
    }
    Ok(PreviewGeometry {
        image_canvas,
        pages,
    })
}

fn page_geometry(
    tile: TileGeometry,
    row: u32,
    col: u32,
    options: &PosterOptions,
    preview: &PreviewInfo,
) -> PreviewPageGeometry {
    let guides = guide_geometry(tile);
    let mut outer_lines = Vec::new();
    let mut cut_lines = Vec::new();
    let mut markers = Vec::new();

    if options.draw_outer_marks {
        outer_lines.extend(outer_mark_lines(
            tile.dest_page,
            preview.page_width_pt,
            preview.page_height_pt,
        ));
    }

    if options.draw_cut_guides {
        if col > 0 {
            let x = cut_guide_position(tile.guide_page.x0, guides.left_x);
            let line = LineGeometry {
                a: Point {
                    x,
                    y: tile.guide_page.y0,
                },
                b: Point {
                    x,
                    y: tile.guide_page.y1,
                },
            };
            let overlap = guides.left_x - tile.guide_page.x0;
            markers.extend(vertical_alignment_frames(
                tile.guide_page.x0,
                guides.left_x + overlap,
                tile.guide_page.y0,
                tile.guide_page.y1,
            ));
            cut_lines.push(line);
        }
        if row > 0 {
            let y = cut_guide_position(tile.guide_page.y0, guides.top_y);
            let line = LineGeometry {
                a: Point {
                    x: tile.guide_page.x0,
                    y,
                },
                b: Point {
                    x: tile.guide_page.x1,
                    y,
                },
            };
            let overlap = guides.top_y - tile.guide_page.y0;
            markers.extend(horizontal_alignment_frames(
                tile.guide_page.x0,
                tile.guide_page.x1,
                tile.guide_page.y0,
                guides.top_y + overlap,
            ));
            cut_lines.push(line);
        }
        if col < preview.cols - 1 {
            let overlap = tile.guide_page.x1 - guides.right_x;
            markers.extend(vertical_alignment_frames(
                guides.right_x - overlap,
                tile.guide_page.x1,
                tile.guide_page.y0,
                tile.guide_page.y1,
            ));
        }
        if row < preview.rows - 1 {
            let overlap = tile.guide_page.y1 - guides.bottom_y;
            markers.extend(horizontal_alignment_frames(
                tile.guide_page.x0,
                tile.guide_page.x1,
                guides.bottom_y - overlap,
                tile.guide_page.y1,
            ));
        }
    }

    PreviewPageGeometry {
        row,
        col,
        clip_canvas: tile.clip_canvas,
        dest_page: tile.dest_page,
        guides,
        outer_lines,
        cut_lines,
        markers,
    }
}

fn image_fit_canvas_size(image_w: f64, image_h: f64, preview: &PreviewInfo) -> Rect {
    fit_rect(
        image_w,
        image_h,
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: preview.canvas_width_pt,
            y1: preview.canvas_height_pt,
        },
    )
}

fn tile_geometry(
    row: u32,
    col: u32,
    preview: &PreviewInfo,
    options: &PosterOptions,
    image_canvas: Rect,
) -> Result<TileGeometry, String> {
    let overlap = mm(options.overlap_mm);
    let base = Rect {
        x0: col as f64 * preview.base_tile_width_pt,
        y0: row as f64 * preview.base_tile_height_pt,
        x1: (col + 1) as f64 * preview.base_tile_width_pt,
        y1: (row + 1) as f64 * preview.base_tile_height_pt,
    };
    let guide = Rect {
        x0: base.x0 - cutter_overlap_before(col, preview.cols, overlap),
        y0: base.y0 - cutter_overlap_before(row, preview.rows, overlap),
        x1: base.x1 + cutter_overlap_after(col, preview.cols, overlap),
        y1: base.y1 + cutter_overlap_after(row, preview.rows, overlap),
    };
    let clip = intersect_rect(guide, image_canvas)
        .ok_or_else(|| "Tile does not intersect image".to_string())?;
    if clip.width() <= 0.0 || clip.height() <= 0.0 {
        return Err("Tile does not intersect image".into());
    }

    let page_w = preview.page_width_pt;
    let page_h = preview.page_height_pt;
    let guide_dest = centered_rect(page_w, page_h, guide.width(), guide.height());
    let dest = map_rect_between(clip, guide, guide_dest);
    Ok(TileGeometry {
        base_canvas: base,
        guide_canvas: guide,
        clip_canvas: clip,
        guide_page: guide_dest,
        dest_page: dest,
    })
}

fn cutter_overlap_before(index: u32, count: u32, overlap: f64) -> f64 {
    match count {
        0 | 1 => 0.0,
        2 => overlap_if(index > 0, overlap),
        _ => overlap,
    }
}

fn cutter_overlap_after(index: u32, count: u32, overlap: f64) -> f64 {
    match count {
        0 | 1 => 0.0,
        2 => overlap_if(index + 1 < count, overlap),
        _ => overlap,
    }
}

fn overlap_if(condition: bool, overlap: f64) -> f64 {
    if condition {
        overlap
    } else {
        0.0
    }
}

fn intersect_rect(a: Rect, b: Rect) -> Option<Rect> {
    let rect = Rect {
        x0: a.x0.max(b.x0),
        y0: a.y0.max(b.y0),
        x1: a.x1.min(b.x1),
        y1: a.y1.min(b.y1),
    };
    (rect.width() > 0.0 && rect.height() > 0.0).then_some(rect)
}

fn centered_rect(page_w: f64, page_h: f64, width: f64, height: f64) -> Rect {
    Rect {
        x0: (page_w - width) / 2.0,
        y0: (page_h - height) / 2.0,
        x1: (page_w + width) / 2.0,
        y1: (page_h + height) / 2.0,
    }
}

fn map_rect_between(rect: Rect, source: Rect, dest: Rect) -> Rect {
    let sx = dest.width() / source.width();
    let sy = dest.height() / source.height();
    Rect {
        x0: dest.x0 + (rect.x0 - source.x0) * sx,
        y0: dest.y0 + (rect.y0 - source.y0) * sy,
        x1: dest.x0 + (rect.x1 - source.x0) * sx,
        y1: dest.y0 + (rect.y1 - source.y0) * sy,
    }
}

fn fit_rect(src_w: f64, src_h: f64, canvas: Rect) -> Rect {
    let scale = (canvas.width() / src_w).min(canvas.height() / src_h);
    let w = src_w * scale;
    let h = src_h * scale;
    let x0 = canvas.x0 + (canvas.width() - w) / 2.0;
    let y0 = canvas.y0 + (canvas.height() - h) / 2.0;
    Rect {
        x0,
        y0,
        x1: x0 + w,
        y1: y0 + h,
    }
}

fn guide_geometry(tile: TileGeometry) -> GuideGeometry {
    let sx = tile.guide_page.width() / tile.guide_canvas.width();
    let sy = tile.guide_page.height() / tile.guide_canvas.height();
    GuideGeometry {
        left_x: tile.guide_page.x0 + (tile.base_canvas.x0 - tile.guide_canvas.x0) * sx,
        right_x: tile.guide_page.x0 + (tile.base_canvas.x1 - tile.guide_canvas.x0) * sx,
        top_y: tile.guide_page.y0 + (tile.base_canvas.y0 - tile.guide_canvas.y0) * sy,
        bottom_y: tile.guide_page.y0 + (tile.base_canvas.y1 - tile.guide_canvas.y0) * sy,
    }
}

fn outer_mark_lines(dest: Rect, page_w: f64, page_h: f64) -> [LineGeometry; 4] {
    [
        LineGeometry {
            a: Point { x: 0.0, y: dest.y0 },
            b: Point {
                x: page_w,
                y: dest.y0,
            },
        },
        LineGeometry {
            a: Point { x: dest.x1, y: 0.0 },
            b: Point {
                x: dest.x1,
                y: page_h,
            },
        },
        LineGeometry {
            a: Point {
                x: page_w,
                y: dest.y1,
            },
            b: Point { x: 0.0, y: dest.y1 },
        },
        LineGeometry {
            a: Point {
                x: dest.x0,
                y: page_h,
            },
            b: Point { x: dest.x0, y: 0.0 },
        },
    ]
}

fn cut_guide_position(outer_edge: f64, inner_edge: f64) -> f64 {
    outer_edge + (inner_edge - outer_edge) * CUT_GUIDE_SAFE_FRACTION
}

fn vertical_alignment_frames(x0: f64, x1: f64, y0: f64, y1: f64) -> [MarkerGeometry; 2] {
    let (x0, x1) = ordered_pair(x0, x1);
    let top_y1 = y0 - MARKER_GAP_PT;
    let bottom_y0 = y1 + MARKER_GAP_PT;
    [
        MarkerGeometry {
            rect: Rect {
                x0,
                y0: top_y1 - MARKER_SIZE_PT,
                x1,
                y1: top_y1,
            },
        },
        MarkerGeometry {
            rect: Rect {
                x0,
                y0: bottom_y0,
                x1,
                y1: bottom_y0 + MARKER_SIZE_PT,
            },
        },
    ]
}

fn horizontal_alignment_frames(x0: f64, x1: f64, y0: f64, y1: f64) -> [MarkerGeometry; 2] {
    let (y0, y1) = ordered_pair(y0, y1);
    let left_x1 = x0 - MARKER_GAP_PT;
    let right_x0 = x1 + MARKER_GAP_PT;
    [
        MarkerGeometry {
            rect: Rect {
                x0: left_x1 - MARKER_SIZE_PT,
                y0,
                x1: left_x1,
                y1,
            },
        },
        MarkerGeometry {
            rect: Rect {
                x0: right_x0,
                y0,
                x1: right_x0 + MARKER_SIZE_PT,
                y1,
            },
        },
    ]
}

fn ordered_pair(a: f64, b: f64) -> (f64, f64) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

fn canvas_to_source_crop(r: Rect, fitted: Rect, img_w: u32, img_h: u32) -> (u32, u32, u32, u32) {
    let sx0 = ((r.x0 - fitted.x0) / fitted.width() * img_w as f64)
        .floor()
        .clamp(0.0, img_w as f64 - 1.0) as u32;
    let sy0 = ((r.y0 - fitted.y0) / fitted.height() * img_h as f64)
        .floor()
        .clamp(0.0, img_h as f64 - 1.0) as u32;
    let sx1 = ((r.x1 - fitted.x0) / fitted.width() * img_w as f64)
        .ceil()
        .clamp(sx0 as f64 + 1.0, img_w as f64) as u32;
    let sy1 = ((r.y1 - fitted.y0) / fitted.height() * img_h as f64)
        .ceil()
        .clamp(sy0 as f64 + 1.0, img_h as f64) as u32;
    (sx0, sy0, sx1 - sx0, sy1 - sy0)
}

fn encode_tile_jpeg(
    image: &DynamicImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<Vec<u8>, String> {
    let cropped = image.crop_imm(x, y, w, h).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 92)
        .encode_image(&cropped)
        .map_err(|e| e.to_string())?;
    Ok(jpeg)
}

// -----------------------------------------------------------------------------
// Page content assembly. Draws from shared geometry only.
// -----------------------------------------------------------------------------

fn build_page_content(page: &PreviewPageGeometry, preview: &PreviewInfo) -> String {
    let mut out = String::new();
    draw_image(&mut out, page.dest_page, preview.page_height_pt);
    for line in &page.outer_lines {
        draw_outer_line(&mut out, line, preview.page_height_pt);
    }
    for line in &page.cut_lines {
        draw_contrast_line(&mut out, line, preview.page_height_pt);
    }
    for marker in &page.markers {
        draw_alignment_frame(&mut out, marker.rect, preview.page_height_pt);
    }
    out
}

// -----------------------------------------------------------------------------
// Guide/line drawing only. These functions must not change tile/grid geometry.
// -----------------------------------------------------------------------------

fn draw_contrast_line(out: &mut String, line: &LineGeometry, page_h: f64) {
    out.push_str("q\n/GS60 gs\n");
    set_stroke(out, 0.45, 0.45, 0.45, 0.9, Some("[7 3] 0"));
    draw_line(out, line.a, line.b, page_h);
    out.push_str("Q\n");
}

fn draw_outer_line(out: &mut String, line: &LineGeometry, page_h: f64) {
    out.push_str("q\n/GS50 gs\n");
    set_stroke(out, 0.55, 0.55, 0.55, 0.5, Some("[3 3] 0"));
    draw_line(out, line.a, line.b, page_h);
    out.push_str("Q\n");
}

fn set_stroke(out: &mut String, r: f64, g: f64, b: f64, width: f64, dash: Option<&str>) {
    out.push_str(&format!("{:.3} {:.3} {:.3} RG\n{:.3} w\n", r, g, b, width));
    if let Some(d) = dash {
        out.push_str(&format!("{} d\n", d));
    } else {
        out.push_str("[] 0 d\n");
    }
}

fn draw_line(out: &mut String, a: Point, b: Point, page_h: f64) {
    out.push_str(&format!(
        "{:.3} {:.3} m {:.3} {:.3} l S\n",
        a.x,
        page_h - a.y,
        b.x,
        page_h - b.y
    ));
}

fn draw_image(out: &mut String, dest: Rect, page_h: f64) {
    let x = dest.x0;
    let y = pdf_y(dest.y0, dest.height(), page_h);
    out.push_str(&format!(
        "q\n{:.3} 0 0 {:.3} {:.3} {:.3} cm\n/Im0 Do\nQ\n",
        dest.width(),
        dest.height(),
        x,
        y
    ));
}

fn pdf_y(y_top: f64, h: f64, page_h: f64) -> f64 {
    page_h - y_top - h
}

fn draw_alignment_frame(out: &mut String, r: Rect, page_h: f64) {
    out.push_str("q\n/GS60 gs\n");
    draw_alignment_frame_path(out, r, page_h, 0.45, 0.45, 0.45, 0.8);
    out.push_str("Q\n");
}

fn draw_alignment_frame_path(
    out: &mut String,
    r: Rect,
    page_h: f64,
    red: f64,
    green: f64,
    blue: f64,
    width: f64,
) {
    out.push_str(&format!(
        "{:.3} {:.3} {:.3} RG\n{:.3} w\n[] 0 d\n1 J 1 j\n",
        red, green, blue, width
    ));
    out.push_str(&format!(
        "{:.3} {:.3} m {:.3} {:.3} l {:.3} {:.3} l {:.3} {:.3} l h {:.3} {:.3} m {:.3} {:.3} l {:.3} {:.3} m {:.3} {:.3} l S\n",
        r.x0,
        page_h - r.y0,
        r.x1,
        page_h - r.y0,
        r.x1,
        page_h - r.y1,
        r.x0,
        page_h - r.y1,
        r.x0,
        page_h - r.y0,
        r.x1,
        page_h - r.y1,
        r.x0,
        page_h - r.y1,
        r.x1,
        page_h - r.y0,
    ));
}

// -----------------------------------------------------------------------------
// Minimal PDF writer.
// -----------------------------------------------------------------------------

fn write_pdf(page_w: f64, page_h: f64, pages: &[PageChunk]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let page_count = pages.len();
    let pages_obj = 2usize;
    let first_page_obj = 3usize;

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: String = (0..page_count)
        .map(|i| format!("{} 0 R ", first_page_obj + i * 3))
        .collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids, page_count).into_bytes());

    for (i, p) in pages.iter().enumerate() {
        let page_obj = first_page_obj + i * 3;
        let content_obj = page_obj + 1;
        let image_obj = page_obj + 2;
        objects.push(format!(
            "<< /Type /Page /Parent {} 0 R /MediaBox [0 0 {:.3} {:.3}] /Resources << /XObject << /Im0 {} 0 R >> /ExtGState << /GS50 << /Type /ExtGState /CA 0.5 /ca 0.5 >> /GS60 << /Type /ExtGState /CA 0.6 /ca 0.6 >> >> >> /Contents {} 0 R >>",
            pages_obj, page_w, page_h, image_obj, content_obj
        ).into_bytes());
        objects.push(stream_object(p.content.as_bytes()));
        let mut img_dict = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            p.image_w, p.image_h, p.image.len()
        ).into_bytes();
        img_dict.extend_from_slice(&p.image);
        img_dict.extend_from_slice(b"\nendstream");
        objects.push(img_dict);
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0usize];
    for (idx, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref
        )
        .as_bytes(),
    );
    out
}

fn stream_object(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{default_options, resolve_layout};

    #[test]
    fn cut_guides_reuse_same_page_position_for_batch_trimming() {
        let options = default_options(4, 3);
        let preview = resolve_layout(1600, 1200, &options).unwrap();
        let geometry = preview_geometry_for_image_size(1600, 1200, &options, &preview).unwrap();
        let mut vertical_x = Vec::new();
        let mut horizontal_y = Vec::new();

        for page in geometry.pages {
            for line in page.cut_lines {
                if (line.a.x - line.b.x).abs() < 0.001 {
                    vertical_x.push(round_milli(line.a.x));
                }
                if (line.a.y - line.b.y).abs() < 0.001 {
                    horizontal_y.push(round_milli(line.a.y));
                }
            }
        }

        assert!(!vertical_x.is_empty());
        assert!(!horizontal_y.is_empty());
        assert!(
            vertical_x.iter().all(|x| *x == vertical_x[0]),
            "{vertical_x:?}"
        );
        assert!(
            horizontal_y.iter().all(|y| *y == horizontal_y[0]),
            "{horizontal_y:?}"
        );
    }

    #[test]
    fn alignment_frames_span_the_full_shared_overlap() {
        let options = default_options(4, 3);
        let preview = resolve_layout(1600, 1200, &options).unwrap();
        let geometry = preview_geometry_for_image_size(1600, 1200, &options, &preview).unwrap();
        let expected_overlap_span = mm(options.overlap_mm) * 2.0;
        let mut vertical_frames = 0;
        let mut horizontal_frames = 0;

        for page in geometry.pages {
            for marker in page.markers {
                let width = marker.rect.width();
                let height = marker.rect.height();
                if width > height {
                    vertical_frames += 1;
                    assert_close(width, expected_overlap_span);
                    assert_close(height, MARKER_SIZE_PT);
                } else {
                    horizontal_frames += 1;
                    assert_close(width, MARKER_SIZE_PT);
                    assert_close(height, expected_overlap_span);
                }
            }
        }

        assert!(vertical_frames > 0);
        assert!(horizontal_frames > 0);
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 0.001,
            "actual={actual}, expected={expected}"
        );
    }

    fn round_milli(value: f64) -> i64 {
        (value * 1000.0).round() as i64
    }
}
