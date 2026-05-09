mod layout;
mod pdf_output;

use layout::{resolve_layout, PosterOptions, PreviewInfo};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
enum AppError {
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
struct GenerateResult {
    pages: u32,
    output: String,
}

fn read_image_size(path: &str) -> Result<(u32, u32), AppError> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    Ok(reader.into_dimensions()?)
}

#[tauri::command]
fn inspect_image(path: String, options: PosterOptions) -> Result<PreviewInfo, AppError> {
    let (w, h) = read_image_size(&path)?;
    resolve_layout(w, h, &options).map_err(AppError::Message)
}

#[tauri::command]
fn output_exists(input: String, output_name: String) -> Result<bool, AppError> {
    Ok(default_output_path(&input, &output_name)?.exists())
}

#[tauri::command]
fn generate_poster(input: String, output_name: String, options: PosterOptions) -> Result<GenerateResult, AppError> {
    if !Path::new(&input).exists() {
        return Err(AppError::Message("Input file does not exist".into()));
    }
    let output = default_output_path(&input, &output_name)?;
    let output_string = output.to_string_lossy().to_string();
    let image = image::open(&input)?;
    let preview = resolve_layout(image.width(), image.height(), &options).map_err(AppError::Message)?;
    pdf_output::generate(&image, &output_string, &options, &preview).map_err(AppError::Message)?;
    Ok(GenerateResult { pages: preview.cols * preview.rows, output: output_string })
}

fn default_output_path(input: &str, output_name: &str) -> Result<PathBuf, AppError> {
    let input_path = Path::new(input);
    let dir = input_path.parent().ok_or_else(|| AppError::Message("Input file has no parent directory".into()))?;
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

fn default_output_name(input_path: &Path) -> String {
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("poster");
    format!("{}-poster.pdf", stem)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![inspect_image, output_exists, generate_poster])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
