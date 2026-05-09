use crate::layout::{mm, PosterOptions, PreviewInfo};
use image::{codecs::jpeg::JpegEncoder, DynamicImage};

const MARKER_SIZE_PT: f64 = 12.0;
const MARKER_GAP_PT: f64 = 2.0;

#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    fn width(self) -> f64 { self.x1 - self.x0 }
    fn height(self) -> f64 { self.y1 - self.y0 }
}

#[derive(Debug, Clone, Copy)]
struct TileGeometry {
    base_canvas: Rect,
    clip_canvas: Rect,
    dest_page: Rect,
}

#[derive(Debug, Clone, Copy)]
struct GuideGeometry {
    left_x: f64,
    right_x: f64,
    top_y: f64,
    bottom_y: f64,
}

struct PageChunk {
    content: String,
    image: Vec<u8>,
    image_w: u32,
    image_h: u32,
}

pub fn generate(image: &DynamicImage, output: &str, options: &PosterOptions, preview: &PreviewInfo) -> Result<(), String> {
    let pages = build_pages(image, options, preview)?;
    let pdf = write_pdf(preview.page_width_pt, preview.page_height_pt, &pages);
    std::fs::write(output, pdf).map_err(|e| e.to_string())
}

fn build_pages(image: &DynamicImage, options: &PosterOptions, preview: &PreviewInfo) -> Result<Vec<PageChunk>, String> {
    let image_canvas = image_fit_canvas(image, preview);
    let mut pages = Vec::new();

    for row in 0..preview.rows {
        for col in 0..preview.cols {
            let tile = tile_geometry(row, col, preview, options, image_canvas)?;
            let (sx, sy, sw, sh) = canvas_to_source_crop(tile.clip_canvas, image_canvas, image.width(), image.height());
            let jpeg = encode_tile_jpeg(image, sx, sy, sw, sh)?;
            let content = build_page_content(tile, row, col, options, preview);
            pages.push(PageChunk { content, image: jpeg, image_w: sw, image_h: sh });
        }
    }

    Ok(pages)
}

// -----------------------------------------------------------------------------
// Tile/grid geometry. These functions decide where each image tile goes.
// They do not draw guide lines or marker boxes.
// -----------------------------------------------------------------------------

fn image_fit_canvas(image: &DynamicImage, preview: &PreviewInfo) -> Rect {
    fit_rect(
        image.width() as f64,
        image.height() as f64,
        Rect { x0: 0.0, y0: 0.0, x1: preview.canvas_width_pt, y1: preview.canvas_height_pt },
    )
}

fn tile_geometry(row: u32, col: u32, preview: &PreviewInfo, options: &PosterOptions, image_canvas: Rect) -> Result<TileGeometry, String> {
    let overlap = mm(options.overlap_mm);
    let base = Rect {
        x0: col as f64 * preview.base_tile_width_pt,
        y0: row as f64 * preview.base_tile_height_pt,
        x1: (col + 1) as f64 * preview.base_tile_width_pt,
        y1: (row + 1) as f64 * preview.base_tile_height_pt,
    };
    let clip = Rect {
        x0: (base.x0 - overlap_if(col > 0, overlap)).max(image_canvas.x0),
        y0: (base.y0 - overlap_if(row > 0, overlap)).max(image_canvas.y0),
        x1: (base.x1 + overlap_if(col < preview.cols - 1, overlap)).min(image_canvas.x1),
        y1: (base.y1 + overlap_if(row < preview.rows - 1, overlap)).min(image_canvas.y1),
    };
    if clip.width() <= 0.0 || clip.height() <= 0.0 {
        return Err("Tile does not intersect image".into());
    }

    let page_w = preview.page_width_pt;
    let page_h = preview.page_height_pt;
    let dest = Rect {
        x0: (page_w - clip.width()) / 2.0,
        y0: (page_h - clip.height()) / 2.0,
        x1: (page_w + clip.width()) / 2.0,
        y1: (page_h + clip.height()) / 2.0,
    };
    Ok(TileGeometry { base_canvas: base, clip_canvas: clip, dest_page: dest })
}

fn overlap_if(condition: bool, overlap: f64) -> f64 {
    if condition { overlap } else { 0.0 }
}

fn fit_rect(src_w: f64, src_h: f64, canvas: Rect) -> Rect {
    let scale = (canvas.width() / src_w).min(canvas.height() / src_h);
    let w = src_w * scale;
    let h = src_h * scale;
    let x0 = canvas.x0 + (canvas.width() - w) / 2.0;
    let y0 = canvas.y0 + (canvas.height() - h) / 2.0;
    Rect { x0, y0, x1: x0 + w, y1: y0 + h }
}

fn canvas_to_source_crop(r: Rect, fitted: Rect, img_w: u32, img_h: u32) -> (u32, u32, u32, u32) {
    let sx0 = ((r.x0 - fitted.x0) / fitted.width() * img_w as f64).floor().clamp(0.0, img_w as f64 - 1.0) as u32;
    let sy0 = ((r.y0 - fitted.y0) / fitted.height() * img_h as f64).floor().clamp(0.0, img_h as f64 - 1.0) as u32;
    let sx1 = ((r.x1 - fitted.x0) / fitted.width() * img_w as f64).ceil().clamp(sx0 as f64 + 1.0, img_w as f64) as u32;
    let sy1 = ((r.y1 - fitted.y0) / fitted.height() * img_h as f64).ceil().clamp(sy0 as f64 + 1.0, img_h as f64) as u32;
    (sx0, sy0, sx1 - sx0, sy1 - sy0)
}

fn encode_tile_jpeg(image: &DynamicImage, x: u32, y: u32, w: u32, h: u32) -> Result<Vec<u8>, String> {
    let cropped = image.crop_imm(x, y, w, h).to_rgb8();
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 92)
        .encode_image(&cropped)
        .map_err(|e| e.to_string())?;
    Ok(jpeg)
}

// -----------------------------------------------------------------------------
// Page content assembly. Calls image drawing and guide drawing separately.
// -----------------------------------------------------------------------------

fn build_page_content(tile: TileGeometry, row: u32, col: u32, options: &PosterOptions, preview: &PreviewInfo) -> String {
    let mut out = String::new();
    draw_image(&mut out, tile.dest_page, preview.page_height_pt);
    if options.draw_outer_marks {
        draw_outer_marks(&mut out, tile.dest_page, preview.page_height_pt);
    }
    if options.draw_cut_guides {
        let guides = guide_geometry(tile);
        draw_cut_and_alignment_guides(&mut out, tile.dest_page, guides, row, col, preview, preview.page_height_pt);
    }
    out
}

fn guide_geometry(tile: TileGeometry) -> GuideGeometry {
    let sx = tile.dest_page.width() / tile.clip_canvas.width();
    let sy = tile.dest_page.height() / tile.clip_canvas.height();
    GuideGeometry {
        left_x: tile.dest_page.x0 + (tile.base_canvas.x0 - tile.clip_canvas.x0) * sx,
        right_x: tile.dest_page.x0 + (tile.base_canvas.x1 - tile.clip_canvas.x0) * sx,
        top_y: tile.dest_page.y0 + (tile.base_canvas.y0 - tile.clip_canvas.y0) * sy,
        bottom_y: tile.dest_page.y0 + (tile.base_canvas.y1 - tile.clip_canvas.y0) * sy,
    }
}

// -----------------------------------------------------------------------------
// Guide/line drawing only. These functions must not change tile/grid geometry.
// -----------------------------------------------------------------------------

fn draw_cut_and_alignment_guides(out: &mut String, dest: Rect, guides: GuideGeometry, row: u32, col: u32, preview: &PreviewInfo, page_h: f64) {
    if col > 0 {
        draw_crop_line_with_boxes(out, Point { x: guides.left_x, y: dest.y0 }, Point { x: guides.left_x, y: dest.y1 }, page_h);
    }
    if row > 0 {
        draw_crop_line_with_boxes(out, Point { x: dest.x0, y: guides.top_y }, Point { x: dest.x1, y: guides.top_y }, page_h);
    }
    if col < preview.cols - 1 {
        draw_marker_boxes(out, Point { x: guides.right_x, y: dest.y0 }, Point { x: guides.right_x, y: dest.y1 }, page_h);
    }
    if row < preview.rows - 1 {
        draw_marker_boxes(out, Point { x: dest.x0, y: guides.bottom_y }, Point { x: dest.x1, y: guides.bottom_y }, page_h);
    }
}

fn draw_crop_line_with_boxes(out: &mut String, a: Point, b: Point, page_h: f64) {
    draw_contrast_line(out, a, b, page_h);
    draw_marker_boxes(out, a, b, page_h);
}

fn draw_marker_boxes(out: &mut String, a: Point, b: Point, page_h: f64) {
    // Markers sit outside the image at the line ends. For a vertical left trim
    // line this means top marker moves up and bottom marker moves down, not left/right.
    let offset = MARKER_SIZE_PT / 2.0 + MARKER_GAP_PT;
    let (a_dir, b_dir) = endpoint_dirs(a, b);
    draw_x_box(out, Point { x: a.x + a_dir.x * offset, y: a.y + a_dir.y * offset }, MARKER_SIZE_PT, page_h);
    draw_x_box(out, Point { x: b.x + b_dir.x * offset, y: b.y + b_dir.y * offset }, MARKER_SIZE_PT, page_h);
}

fn endpoint_dirs(a: Point, b: Point) -> (Point, Point) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    (Point { x: -ux, y: -uy }, Point { x: ux, y: uy })
}

fn draw_contrast_line(out: &mut String, a: Point, b: Point, page_h: f64) {
    set_stroke(out, 1.0, 1.0, 1.0, 2.4, Some("[7 3] 0"));
    draw_line(out, a, b, page_h);
    set_stroke(out, 0.0, 0.75, 0.95, 1.05, Some("[7 3] 0"));
    draw_line(out, a, b, page_h);
}

fn draw_outer_marks(out: &mut String, dest: Rect, page_h: f64) {
    set_stroke(out, 0.65, 0.65, 0.65, 0.35, Some("[3 3] 0"));
    draw_rect(out, dest, page_h);
    let mark = 10.0;
    let gap = 2.0;
    for (a, b) in [
        (Point { x: dest.x0 - mark, y: dest.y0 }, Point { x: dest.x0 - gap, y: dest.y0 }),
        (Point { x: dest.x0, y: dest.y0 - mark }, Point { x: dest.x0, y: dest.y0 - gap }),
        (Point { x: dest.x1 + gap, y: dest.y0 }, Point { x: dest.x1 + mark, y: dest.y0 }),
        (Point { x: dest.x1, y: dest.y0 - mark }, Point { x: dest.x1, y: dest.y0 - gap }),
        (Point { x: dest.x0 - mark, y: dest.y1 }, Point { x: dest.x0 - gap, y: dest.y1 }),
        (Point { x: dest.x0, y: dest.y1 + gap }, Point { x: dest.x0, y: dest.y1 + mark }),
        (Point { x: dest.x1 + gap, y: dest.y1 }, Point { x: dest.x1 + mark, y: dest.y1 }),
        (Point { x: dest.x1, y: dest.y1 + gap }, Point { x: dest.x1, y: dest.y1 + mark }),
    ] {
        draw_line(out, a, b, page_h);
    }
}

fn set_stroke(out: &mut String, r: f64, g: f64, b: f64, width: f64, dash: Option<&str>) {
    out.push_str(&format!("{:.3} {:.3} {:.3} RG\n{:.3} w\n", r, g, b, width));
    if let Some(d) = dash { out.push_str(&format!("{} d\n", d)); } else { out.push_str("[] 0 d\n"); }
}

fn draw_line(out: &mut String, a: Point, b: Point, page_h: f64) {
    out.push_str(&format!("{:.3} {:.3} m {:.3} {:.3} l S\n", a.x, page_h - a.y, b.x, page_h - b.y));
}

fn draw_rect(out: &mut String, r: Rect, page_h: f64) {
    out.push_str(&format!("{:.3} {:.3} {:.3} {:.3} re S\n", r.x0, pdf_y(r.y0, r.height(), page_h), r.width(), r.height()));
}

fn draw_image(out: &mut String, dest: Rect, page_h: f64) {
    let x = dest.x0;
    let y = pdf_y(dest.y0, dest.height(), page_h);
    out.push_str(&format!("q\n{:.3} 0 0 {:.3} {:.3} {:.3} cm\n/Im0 Do\nQ\n", dest.width(), dest.height(), x, y));
}

fn pdf_y(y_top: f64, h: f64, page_h: f64) -> f64 {
    page_h - y_top - h
}

fn draw_x_box(out: &mut String, center: Point, size: f64, page_h: f64) {
    let r = Rect { x0: center.x - size / 2.0, y0: center.y - size / 2.0, x1: center.x + size / 2.0, y1: center.y + size / 2.0 };
    draw_x_box_path(out, r, page_h, 0.0, 0.0, 0.0, 1.1);
}

fn draw_x_box_path(out: &mut String, r: Rect, page_h: f64, red: f64, green: f64, blue: f64, width: f64) {
    out.push_str(&format!("{:.3} {:.3} {:.3} RG\n{:.3} w\n[] 0 d\n1 J 1 j\n", red, green, blue, width));
    out.push_str(&format!(
        "{:.3} {:.3} m {:.3} {:.3} l {:.3} {:.3} l {:.3} {:.3} l h {:.3} {:.3} m {:.3} {:.3} l {:.3} {:.3} m {:.3} {:.3} l S\n",
        r.x0, page_h - r.y0,
        r.x1, page_h - r.y0,
        r.x1, page_h - r.y1,
        r.x0, page_h - r.y1,
        r.x0, page_h - r.y0,
        r.x1, page_h - r.y1,
        r.x0, page_h - r.y1,
        r.x1, page_h - r.y0,
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
    let kids: String = (0..page_count).map(|i| format!("{} 0 R ", first_page_obj + i * 3)).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids, page_count).into_bytes());

    for (i, p) in pages.iter().enumerate() {
        let page_obj = first_page_obj + i * 3;
        let content_obj = page_obj + 1;
        let image_obj = page_obj + 2;
        objects.push(format!(
            "<< /Type /Page /Parent {} 0 R /MediaBox [0 0 {:.3} {:.3}] /Resources << /XObject << /Im0 {} 0 R >> >> /Contents {} 0 R >>",
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
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", objects.len() + 1, xref).as_bytes());
    out
}

fn stream_object(data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}
