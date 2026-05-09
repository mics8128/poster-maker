use crate::layout::{mm, PosterOptions, PreviewInfo};
use image::{codecs::jpeg::JpegEncoder, DynamicImage};

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
    let overlap = mm(options.overlap_mm);
    let margin = mm(options.margin_mm);
    let page_w = preview.page_width_pt;
    let page_h = preview.page_height_pt;
    let img_w = image.width() as f64;
    let img_h = image.height() as f64;

    let canvas = Rect { x0: 0.0, y0: 0.0, x1: preview.canvas_width_pt, y1: preview.canvas_height_pt };
    let fitted = fit_rect(img_w, img_h, canvas);

    let mut pages = Vec::new();
    for row in 0..preview.rows {
        for col in 0..preview.cols {
            let base = Rect {
                x0: col as f64 * preview.base_tile_width_pt,
                y0: row as f64 * preview.base_tile_height_pt,
                x1: (col + 1) as f64 * preview.base_tile_width_pt,
                y1: (row + 1) as f64 * preview.base_tile_height_pt,
            };
            let clip = Rect {
                x0: (base.x0 - if col > 0 { overlap } else { 0.0 }).max(fitted.x0),
                y0: (base.y0 - if row > 0 { overlap } else { 0.0 }).max(fitted.y0),
                x1: (base.x1 + if col < preview.cols - 1 { overlap } else { 0.0 }).min(fitted.x1),
                y1: (base.y1 + if row < preview.rows - 1 { overlap } else { 0.0 }).min(fitted.y1),
            };
            if clip.width() <= 0.0 || clip.height() <= 0.0 {
                return Err("Tile does not intersect image".into());
            }

            let src_crop = canvas_to_source_crop(clip, fitted, image.width(), image.height());
            let cropped = image.crop_imm(src_crop.0, src_crop.1, src_crop.2, src_crop.3).to_rgb8();
            let mut jpg = Vec::new();
            JpegEncoder::new_with_quality(&mut jpg, 92)
                .encode_image(&cropped)
                .map_err(|e| e.to_string())?;

            let dest = Rect {
                x0: (page_w - clip.width()) / 2.0,
                y0: (page_h - clip.height()) / 2.0,
                x1: (page_w + clip.width()) / 2.0,
                y1: (page_h + clip.height()) / 2.0,
            };
            let mut content = String::new();
            draw_image(&mut content, dest, page_h);
            if options.draw_outer_marks {
                draw_outer_marks(&mut content, dest, page_w, page_h, margin);
            }
            if options.draw_cut_guides {
                draw_guides(&mut content, dest, clip, base, col, row, preview.cols, preview.rows, page_h);
            }
            pages.push(PageChunk { content, image: jpg, image_w: src_crop.2, image_h: src_crop.3 });
        }
    }
    Ok(pages)
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

fn pdf_y(y_top: f64, h: f64, page_h: f64) -> f64 {
    page_h - y_top - h
}

fn draw_image(out: &mut String, dest: Rect, page_h: f64) {
    let x = dest.x0;
    let y = pdf_y(dest.y0, dest.height(), page_h);
    out.push_str(&format!("q\n{:.3} 0 0 {:.3} {:.3} {:.3} cm\n/Im0 Do\nQ\n", dest.width(), dest.height(), x, y));
}

fn draw_outer_marks(out: &mut String, dest: Rect, _page_w: f64, page_h: f64, _margin: f64) {
    set_stroke(out, 0.65, 0.65, 0.65, 0.35, Some("[3 3] 0"));
    draw_rect(out, dest, page_h);
    let l = 10.0;
    let gap = 2.0;
    let lines = [
        ((dest.x0 - l, dest.y0), (dest.x0 - gap, dest.y0)),
        ((dest.x0, dest.y0 - l), (dest.x0, dest.y0 - gap)),
        ((dest.x1 + gap, dest.y0), (dest.x1 + l, dest.y0)),
        ((dest.x1, dest.y0 - l), (dest.x1, dest.y0 - gap)),
        ((dest.x0 - l, dest.y1), (dest.x0 - gap, dest.y1)),
        ((dest.x0, dest.y1 + gap), (dest.x0, dest.y1 + l)),
        ((dest.x1 + gap, dest.y1), (dest.x1 + l, dest.y1)),
        ((dest.x1, dest.y1 + gap), (dest.x1, dest.y1 + l)),
    ];
    for (a, b) in lines { draw_line(out, a, b, page_h); }
}

#[allow(clippy::too_many_arguments)]
fn draw_guides(out: &mut String, dest: Rect, clip: Rect, base: Rect, col: u32, row: u32, cols: u32, rows: u32, page_h: f64) {
    let left_x = dest.x0 + (base.x0 - clip.x0) * dest.width() / clip.width();
    let right_x = dest.x0 + (base.x1 - clip.x0) * dest.width() / clip.width();
    let top_y = dest.y0 + (base.y0 - clip.y0) * dest.height() / clip.height();
    let bottom_y = dest.y0 + (base.y1 - clip.y0) * dest.height() / clip.height();

    if col > 0 { draw_crop_line_with_boxes(out, (left_x, dest.y0), (left_x, dest.y1), page_h); }
    if row > 0 { draw_crop_line_with_boxes(out, (dest.x0, top_y), (dest.x1, top_y), page_h); }
    if col < cols - 1 { draw_end_boxes(out, (right_x, dest.y0), (right_x, dest.y1), page_h); }
    if row < rows - 1 { draw_end_boxes(out, (dest.x0, bottom_y), (dest.x1, bottom_y), page_h); }
}

fn line_outer_points(a: (f64, f64), b: (f64, f64), offset: f64) -> ((f64, f64), (f64, f64)) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    ((a.0 - dx / len * offset, a.1 - dy / len * offset), (b.0 + dx / len * offset, b.1 + dy / len * offset))
}

fn draw_crop_line_with_boxes(out: &mut String, a: (f64, f64), b: (f64, f64), page_h: f64) {
    // Crop line extends slightly beyond marker centers, so the marked line can be cut off.
    let (line_a, line_b) = line_outer_points(a, b, 17.0);
    // White underlay keeps crop mark visible on red/dark image areas.
    set_stroke(out, 1.0, 1.0, 1.0, 2.4, Some("[7 3] 0"));
    draw_line(out, line_a, line_b, page_h);
    // Cyan + black is more robust than pure red when source artwork contains red.
    set_stroke(out, 0.0, 0.75, 0.95, 1.05, Some("[7 3] 0"));
    draw_line(out, line_a, line_b, page_h);
    set_stroke(out, 0.0, 0.0, 0.0, 0.35, Some("[1 8] 0"));
    draw_line(out, line_a, line_b, page_h);
    draw_end_boxes(out, a, b, page_h);
}

fn draw_end_boxes(out: &mut String, a: (f64, f64), b: (f64, f64), page_h: f64) {
    let (s, e) = line_outer_points(a, b, 12.0);
    draw_x_box(out, s, 9.0, page_h);
    draw_x_box(out, e, 9.0, page_h);
}

fn set_stroke(out: &mut String, r: f64, g: f64, b: f64, width: f64, dash: Option<&str>) {
    out.push_str(&format!("{:.3} {:.3} {:.3} RG\n{:.3} w\n", r, g, b, width));
    if let Some(d) = dash { out.push_str(&format!("{} d\n", d)); } else { out.push_str("[] 0 d\n"); }
}

fn draw_line(out: &mut String, a: (f64, f64), b: (f64, f64), page_h: f64) {
    out.push_str(&format!("{:.3} {:.3} m {:.3} {:.3} l S\n", a.0, page_h - a.1, b.0, page_h - b.1));
}

fn draw_rect(out: &mut String, r: Rect, page_h: f64) {
    out.push_str(&format!("{:.3} {:.3} {:.3} {:.3} re S\n", r.x0, pdf_y(r.y0, r.height(), page_h), r.width(), r.height()));
}

fn draw_x_box(out: &mut String, center: (f64, f64), size: f64, page_h: f64) {
    let r = Rect { x0: center.0 - size / 2.0, y0: center.1 - size / 2.0, x1: center.0 + size / 2.0, y1: center.1 + size / 2.0 };
    // White halo + cyan stroke keeps markers visible on colored artwork.
    draw_x_box_path(out, r, page_h, 1.0, 1.0, 1.0, 2.2);
    draw_x_box_path(out, r, page_h, 0.0, 0.75, 0.95, 0.8);
    // Tiny black center cross gives contrast when cyan hits light artwork.
    draw_x_box_path(out, r, page_h, 0.0, 0.0, 0.0, 0.25);
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
