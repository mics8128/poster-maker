pub mod imposition;
pub mod layout;
mod pdf_output;

use imposition::{resolve_imposition, ImpositionOptions, ImpositionPreview};
use layout::{resolve_layout, PosterOptions, PreviewInfo};
use pdf_output::PreviewGeometry;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Message(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct GenerateResult {
    pub pages: u32,
    pub output: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateImpositionResult {
    pub copies: u32,
    pub output: String,
}

pub fn read_image_size(path: &str) -> Result<(u32, u32), AppError> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.into_dimensions()?)
}

pub fn read_image(path: &str) -> Result<image::DynamicImage, AppError> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.decode()?)
}

#[tauri::command]
fn inspect_image(path: String, options: PosterOptions) -> Result<PreviewInfo, AppError> {
    let (w, h) = read_image_size(&path)?;
    resolve_layout(w, h, &options).map_err(AppError::Message)
}

#[tauri::command]
fn inspect_imposition(
    path: String,
    options: ImpositionOptions,
) -> Result<ImpositionPreview, AppError> {
    let (w, h) = read_image_size(&path)?;
    resolve_imposition(w, h, &options).map_err(AppError::Message)
}

#[tauri::command]
fn preview_geometry(path: String, options: PosterOptions) -> Result<PreviewGeometry, AppError> {
    let (w, h) = read_image_size(&path)?;
    let preview = resolve_layout(w, h, &options).map_err(AppError::Message)?;
    pdf_output::preview_geometry_for_image_size(w, h, &options, &preview).map_err(AppError::Message)
}

#[tauri::command]
fn output_exists(input: String, output_name: String) -> Result<bool, AppError> {
    Ok(default_output_path(&input, &output_name)?.exists())
}

pub fn generate_poster_file(
    input: String,
    output_name: String,
    overwrite: bool,
    options: PosterOptions,
) -> Result<GenerateResult, AppError> {
    if !Path::new(&input).exists() {
        return Err(AppError::Message("Input file does not exist".into()));
    }
    let output = default_output_path(&input, &output_name)?;
    if output.exists() && !overwrite {
        return Err(AppError::Message("Output file already exists".into()));
    }
    let output_string = output.to_string_lossy().to_string();
    let image = read_image(&input)?;
    let preview =
        resolve_layout(image.width(), image.height(), &options).map_err(AppError::Message)?;
    pdf_output::generate(&image, &output_string, &options, &preview).map_err(AppError::Message)?;
    Ok(GenerateResult {
        pages: preview.cols * preview.rows,
        output: output_string,
    })
}

pub fn generate_imposition_file(
    input: String,
    output_name: String,
    overwrite: bool,
    options: ImpositionOptions,
) -> Result<GenerateImpositionResult, AppError> {
    if !Path::new(&input).exists() {
        return Err(AppError::Message("Input file does not exist".into()));
    }
    let output = default_output_path(&input, &output_name)?;
    if output.exists() && !overwrite {
        return Err(AppError::Message("Output file already exists".into()));
    }
    let output_string = output.to_string_lossy().to_string();
    let image = read_image(&input)?;
    let preview =
        resolve_imposition(image.width(), image.height(), &options).map_err(AppError::Message)?;
    pdf_output::generate_imposition(&image, &output_string, &preview).map_err(AppError::Message)?;
    Ok(GenerateImpositionResult {
        copies: preview.copies,
        output: output_string,
    })
}

pub fn default_output_path(input: &str, output_name: &str) -> Result<PathBuf, AppError> {
    let input_path = Path::new(input);
    let dir = input_path
        .parent()
        .ok_or_else(|| AppError::Message("Input file has no parent directory".into()))?;
    let mut name = output_name.trim().to_string();
    if name.is_empty() {
        name = default_output_name(input_path);
    }
    if Path::new(&name).components().count() != 1 {
        return Err(AppError::Message("Output must be a file name only".into()));
    }
    if !name.to_lowercase().ends_with(".pdf") {
        name.push_str(".pdf");
    }
    Ok(dir.join(name))
}

pub fn default_output_name(input_path: &Path) -> String {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("poster");
    format!("{}-poster.pdf", stem)
}

#[tauri::command]
fn generate_poster(
    input: String,
    output_name: String,
    overwrite: bool,
    options: PosterOptions,
) -> Result<GenerateResult, AppError> {
    generate_poster_file(input, output_name, overwrite, options)
}

#[tauri::command]
fn generate_imposition(
    input: String,
    output_name: String,
    overwrite: bool,
    options: ImpositionOptions,
) -> Result<GenerateImpositionResult, AppError> {
    generate_imposition_file(input, output_name, overwrite, options)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            inspect_image,
            inspect_imposition,
            preview_geometry,
            output_exists,
            generate_poster,
            generate_imposition
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgb, RgbImage};

    #[test]
    fn generates_a_single_page_imposition_pdf_file() {
        let unique = format!(
            "poster-maker-imposition-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let temp = std::env::temp_dir();
        let input = temp.join(format!("{unique}.png"));
        let output_name = format!("{unique}.pdf");
        let output = temp.join(&output_name);
        DynamicImage::ImageRgb8(RgbImage::from_pixel(200, 100, Rgb([240, 120, 40])))
            .save(&input)
            .unwrap();

        let options = ImpositionOptions {
            paper_width_mm: 210.0,
            paper_height_mm: 297.0,
            item_width_mm: 105.0,
            item_height_mm: 148.0,
            safety_top_mm: 15.0,
            safety_right_mm: 15.0,
            safety_bottom_mm: 15.0,
            safety_left_mm: 15.0,
        };
        let result = generate_imposition_file(
            input.to_string_lossy().to_string(),
            output_name,
            false,
            options,
        )
        .unwrap();
        let pdf = std::fs::read(&output).unwrap();

        assert_eq!(result.copies, 4);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf, b"/Type /Page "), 1);
        assert_eq!(count_bytes(&pdf, b"/Subtype /Image"), 1);

        let _ = std::fs::remove_file(input);
        let _ = std::fs::remove_file(output);
    }

    fn count_bytes(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }

    #[test]
    fn generates_poster_and_imposition_when_extension_mismatches_content() {
        let unique = format!(
            "poster-maker-mismatch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let temp = std::env::temp_dir();
        // File 1: named .png, but actual JPEG bytes
        let jpeg_named_png = temp.join(format!("{unique}-jpeg.png"));
        // File 2: named .jpg, but actual PNG bytes
        let png_named_jpg = temp.join(format!("{unique}-png.jpg"));

        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(200, 150, Rgb([200, 100, 50])));
        {
            let mut f = std::fs::File::create(&jpeg_named_png).unwrap();
            img.write_to(&mut f, image::ImageFormat::Jpeg).unwrap();
        }
        {
            let mut f = std::fs::File::create(&png_named_jpg).unwrap();
            img.write_to(&mut f, image::ImageFormat::Png).unwrap();
        }

        let poster_opts = layout::default_options(2, 2);
        let imp_opts = ImpositionOptions {
            paper_width_mm: 210.0,
            paper_height_mm: 297.0,
            item_width_mm: 105.0,
            item_height_mm: 148.0,
            safety_top_mm: 15.0,
            safety_right_mm: 15.0,
            safety_bottom_mm: 15.0,
            safety_left_mm: 15.0,
        };

        // Check dimension identification matches decoded image dimensions
        let (w1, h1) = read_image_size(jpeg_named_png.to_str().unwrap()).unwrap();
        let decoded1 = read_image(jpeg_named_png.to_str().unwrap()).unwrap();
        assert_eq!((w1, h1), (decoded1.width(), decoded1.height()));

        let (w2, h2) = read_image_size(png_named_jpg.to_str().unwrap()).unwrap();
        let decoded2 = read_image(png_named_jpg.to_str().unwrap()).unwrap();
        assert_eq!((w2, h2), (decoded2.width(), decoded2.height()));

        // Test poster with jpeg-named-png
        let poster_out_name1 = format!("{unique}-p1.pdf");
        let poster_out1 = temp.join(&poster_out_name1);
        let res1 = generate_poster_file(
            jpeg_named_png.to_string_lossy().to_string(),
            poster_out_name1,
            true,
            poster_opts.clone(),
        );
        assert!(
            res1.is_ok(),
            "generate_poster_file failed for jpeg named .png: {:?}",
            res1.err()
        );
        let res1_val = res1.unwrap();
        assert_eq!(res1_val.pages, 4);
        assert!(poster_out1.exists());
        let pdf_p1 = std::fs::read(&poster_out1).unwrap();
        assert!(pdf_p1.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_p1, b"/Type /Page "), 4);

        // Test imposition with jpeg-named-png
        let imp_out_name1 = format!("{unique}-i1.pdf");
        let imp_out1 = temp.join(&imp_out_name1);
        let res2 = generate_imposition_file(
            jpeg_named_png.to_string_lossy().to_string(),
            imp_out_name1,
            true,
            imp_opts.clone(),
        );
        assert!(
            res2.is_ok(),
            "generate_imposition_file failed for jpeg named .png: {:?}",
            res2.err()
        );
        let res2_val = res2.unwrap();
        assert_eq!(res2_val.copies, 4);
        assert!(imp_out1.exists());
        let pdf_i1 = std::fs::read(&imp_out1).unwrap();
        assert!(pdf_i1.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_i1, b"/Type /Page "), 1);

        // Test poster with png-named-jpg
        let poster_out_name2 = format!("{unique}-p2.pdf");
        let poster_out2 = temp.join(&poster_out_name2);
        let res3 = generate_poster_file(
            png_named_jpg.to_string_lossy().to_string(),
            poster_out_name2,
            true,
            poster_opts,
        );
        assert!(
            res3.is_ok(),
            "generate_poster_file failed for png named .jpg: {:?}",
            res3.err()
        );
        let res3_val = res3.unwrap();
        assert_eq!(res3_val.pages, 4);
        assert!(poster_out2.exists());
        let pdf_p2 = std::fs::read(&poster_out2).unwrap();
        assert!(pdf_p2.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_p2, b"/Type /Page "), 4);

        // Test imposition with png-named-jpg
        let imp_out_name2 = format!("{unique}-i2.pdf");
        let imp_out2 = temp.join(&imp_out_name2);
        let res4 = generate_imposition_file(
            png_named_jpg.to_string_lossy().to_string(),
            imp_out_name2,
            true,
            imp_opts,
        );
        assert!(
            res4.is_ok(),
            "generate_imposition_file failed for png named .jpg: {:?}",
            res4.err()
        );
        let res4_val = res4.unwrap();
        assert_eq!(res4_val.copies, 4);
        assert!(imp_out2.exists());
        let pdf_i2 = std::fs::read(&imp_out2).unwrap();
        assert!(pdf_i2.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_i2, b"/Type /Page "), 1);

        // Clean up
        let _ = std::fs::remove_file(jpeg_named_png);
        let _ = std::fs::remove_file(png_named_jpg);
        let _ = std::fs::remove_file(poster_out1);
        let _ = std::fs::remove_file(imp_out1);
        let _ = std::fs::remove_file(poster_out2);
        let _ = std::fs::remove_file(imp_out2);
    }

    #[test]
    fn generates_poster_and_imposition_for_normal_png_and_jpeg() {
        let unique = format!(
            "poster-maker-normal-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let temp = std::env::temp_dir();
        let normal_png = temp.join(format!("{unique}.png"));
        let normal_jpg = temp.join(format!("{unique}.jpg"));

        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(160, 120, Rgb([30, 140, 200])));
        img.save(&normal_png).unwrap();
        img.save(&normal_jpg).unwrap();

        let poster_opts = layout::default_options(2, 2);
        let imp_opts = ImpositionOptions {
            paper_width_mm: 210.0,
            paper_height_mm: 297.0,
            item_width_mm: 105.0,
            item_height_mm: 148.0,
            safety_top_mm: 15.0,
            safety_right_mm: 15.0,
            safety_bottom_mm: 15.0,
            safety_left_mm: 15.0,
        };

        let p_png_name = format!("{unique}-p-png.pdf");
        let p_png_path = temp.join(&p_png_name);
        let p_png = generate_poster_file(
            normal_png.to_string_lossy().to_string(),
            p_png_name,
            true,
            poster_opts.clone(),
        );
        assert!(p_png.is_ok());
        assert_eq!(p_png.unwrap().pages, 4);
        assert!(p_png_path.exists());
        let pdf_p_png = std::fs::read(&p_png_path).unwrap();
        assert!(pdf_p_png.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_p_png, b"/Type /Page "), 4);

        let i_png_name = format!("{unique}-i-png.pdf");
        let i_png_path = temp.join(&i_png_name);
        let i_png = generate_imposition_file(
            normal_png.to_string_lossy().to_string(),
            i_png_name,
            true,
            imp_opts.clone(),
        );
        assert!(i_png.is_ok());
        assert_eq!(i_png.unwrap().copies, 4);
        assert!(i_png_path.exists());
        let pdf_i_png = std::fs::read(&i_png_path).unwrap();
        assert!(pdf_i_png.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_i_png, b"/Type /Page "), 1);

        let p_jpg_name = format!("{unique}-p-jpg.pdf");
        let p_jpg_path = temp.join(&p_jpg_name);
        let p_jpg = generate_poster_file(
            normal_jpg.to_string_lossy().to_string(),
            p_jpg_name,
            true,
            poster_opts,
        );
        assert!(p_jpg.is_ok());
        assert_eq!(p_jpg.unwrap().pages, 4);
        assert!(p_jpg_path.exists());
        let pdf_p_jpg = std::fs::read(&p_jpg_path).unwrap();
        assert!(pdf_p_jpg.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_p_jpg, b"/Type /Page "), 4);

        let i_jpg_name = format!("{unique}-i-jpg.pdf");
        let i_jpg_path = temp.join(&i_jpg_name);
        let i_jpg = generate_imposition_file(
            normal_jpg.to_string_lossy().to_string(),
            i_jpg_name,
            true,
            imp_opts,
        );
        assert!(i_jpg.is_ok());
        assert_eq!(i_jpg.unwrap().copies, 4);
        assert!(i_jpg_path.exists());
        let pdf_i_jpg = std::fs::read(&i_jpg_path).unwrap();
        assert!(pdf_i_jpg.starts_with(b"%PDF-1.4"));
        assert_eq!(count_bytes(&pdf_i_jpg, b"/Type /Page "), 1);

        let _ = std::fs::remove_file(normal_png);
        let _ = std::fs::remove_file(normal_jpg);
        let _ = std::fs::remove_file(p_png_path);
        let _ = std::fs::remove_file(i_png_path);
        let _ = std::fs::remove_file(p_jpg_path);
        let _ = std::fs::remove_file(i_jpg_path);
    }

    #[test]
    fn rejects_corrupt_files_in_read_and_generation() {
        let unique = format!(
            "poster-maker-corrupt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let temp = std::env::temp_dir();
        let corrupt_file = temp.join(format!("{unique}.png"));
        std::fs::write(
            &corrupt_file,
            b"not an image at all, just plain corrupted bytes",
        )
        .unwrap();

        let path_str = corrupt_file.to_string_lossy().to_string();

        assert!(read_image_size(&path_str).is_err());
        assert!(read_image(&path_str).is_err());

        let poster_opts = layout::default_options(2, 2);
        let poster_out = temp.join(format!("{unique}-p.pdf"));
        assert!(generate_poster_file(
            path_str.clone(),
            format!("{unique}-p.pdf"),
            true,
            poster_opts
        )
        .is_err());
        assert!(!poster_out.exists());

        let imp_opts = ImpositionOptions {
            paper_width_mm: 210.0,
            paper_height_mm: 297.0,
            item_width_mm: 105.0,
            item_height_mm: 148.0,
            safety_top_mm: 15.0,
            safety_right_mm: 15.0,
            safety_bottom_mm: 15.0,
            safety_left_mm: 15.0,
        };
        let imp_out = temp.join(format!("{unique}-i.pdf"));
        assert!(
            generate_imposition_file(path_str, format!("{unique}-i.pdf"), true, imp_opts).is_err()
        );
        assert!(!imp_out.exists());

        let _ = std::fs::remove_file(corrupt_file);
    }

    #[test]
    fn rejects_truncated_png_pixel_data_and_does_not_produce_pdf() {
        let unique = format!(
            "poster-maker-truncated-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let temp = std::env::temp_dir();
        let valid_png = temp.join(format!("{unique}-valid.png"));
        let truncated_png = temp.join(format!("{unique}-truncated.png"));

        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 80, Rgb([255, 0, 0])));
        img.save(&valid_png).unwrap();

        let full_bytes = std::fs::read(&valid_png).unwrap();
        // Valid PNG header (8 bytes) + IHDR chunk (25 bytes) + partial IDAT chunk.
        // Dimensions can be read from IHDR, but pixel data is truncated.
        assert!(full_bytes.len() > 60);
        std::fs::write(&truncated_png, &full_bytes[..60]).unwrap();

        let path_str = truncated_png.to_string_lossy().to_string();

        // Metadata-only read_image_size can read header dimensions
        let size_res = read_image_size(&path_str);
        assert_eq!(size_res.unwrap(), (100, 80));

        // Full decode read_image must reject truncated pixel data
        assert!(read_image(&path_str).is_err());

        // Poster generate must reject and not create PDF
        let poster_out_name = format!("{unique}-p.pdf");
        let poster_out = temp.join(&poster_out_name);
        let poster_opts = layout::default_options(2, 2);
        let poster_res = generate_poster_file(path_str.clone(), poster_out_name, true, poster_opts);
        assert!(poster_res.is_err());
        assert!(
            !poster_out.exists(),
            "PDF should not be generated on decode failure"
        );

        // Imposition generate must reject and not create PDF
        let imp_out_name = format!("{unique}-i.pdf");
        let imp_out = temp.join(&imp_out_name);
        let imp_opts = ImpositionOptions {
            paper_width_mm: 210.0,
            paper_height_mm: 297.0,
            item_width_mm: 105.0,
            item_height_mm: 148.0,
            safety_top_mm: 15.0,
            safety_right_mm: 15.0,
            safety_bottom_mm: 15.0,
            safety_left_mm: 15.0,
        };
        let imp_res = generate_imposition_file(path_str, imp_out_name, true, imp_opts);
        assert!(imp_res.is_err());
        assert!(
            !imp_out.exists(),
            "PDF should not be generated on decode failure"
        );

        let _ = std::fs::remove_file(valid_png);
        let _ = std::fs::remove_file(truncated_png);
    }
}
