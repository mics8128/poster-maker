mod layout;
mod pdf_output;

use layout::{resolve_layout, PosterOptions, PreviewInfo};
use serde::Serialize;
use std::path::Path;

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
fn generate_poster(input: String, output: String, options: PosterOptions) -> Result<GenerateResult, AppError> {
    if !Path::new(&input).exists() {
        return Err(AppError::Message("Input file does not exist".into()));
    }
    let image = image::open(&input)?;
    let preview = resolve_layout(image.width(), image.height(), &options).map_err(AppError::Message)?;
    pdf_output::generate(&image, &output, &options, &preview).map_err(AppError::Message)?;
    Ok(GenerateResult { pages: preview.cols * preview.rows, output })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![inspect_image, generate_poster])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
