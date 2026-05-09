use crate::layout::{mm, PosterOptions, PreviewInfo};
use image::DynamicImage;

pub fn generate(image: &DynamicImage, output: &str, options: &PosterOptions, preview: &PreviewInfo) -> Result<(), String> {
    // MVP placeholder: validates pipeline and writes a tiny PDF marker file.
    // Next step replaces this with real tiled image placement via printpdf.
    let page_count = preview.cols * preview.rows;
    let body = format!(
        "%PDF-1.4\n% Poster Maker Tauri MVP\n% pages={} image={}x{} overlap_pt={} margin_pt={}\n1 0 obj <</Type /Catalog>> endobj\n%%EOF\n",
        page_count,
        image.width(),
        image.height(),
        mm(options.overlap_mm),
        mm(options.margin_mm)
    );
    std::fs::write(output, body).map_err(|e| e.to_string())
}
