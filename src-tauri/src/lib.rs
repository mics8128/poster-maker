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
    let image = image::open(&input)?;
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
    let image = image::open(&input)?;
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
}
