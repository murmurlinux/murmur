use directories::ProjectDirs;
use serde::Serialize;
use std::path::PathBuf;
use tauri::Emitter;

const DEFAULT_MODEL: &str = "ggml-tiny.en.bin";
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin";

#[derive(Clone, Serialize)]
pub struct ModelDownloadProgress {
    pub model: String,
    pub percent: f32,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}

/// Get the models directory (~/.local/share/murmur/models/)
pub fn models_dir() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("com", "syncrotrade", "murmur") {
        let path = dirs.data_dir().join("models");
        std::fs::create_dir_all(&path).ok();
        path
    } else {
        // Fallback
        let path = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".local/share/murmur/models");
        std::fs::create_dir_all(&path).ok();
        path
    }
}

/// Get the path to the default model, or None if not downloaded
pub fn get_default_model_path() -> Option<PathBuf> {
    let path = models_dir().join(DEFAULT_MODEL);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Download the default model with progress events
pub async fn download_default_model(app: tauri::AppHandle) -> Result<PathBuf, anyhow::Error> {
    let dest = models_dir().join(DEFAULT_MODEL);

    if dest.exists() {
        println!("Model already downloaded: {}", dest.display());
        return Ok(dest);
    }

    println!("Downloading whisper model to: {}", dest.display());

    let client = reqwest::Client::new();
    let response = client.get(MODEL_URL).send().await?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = std::fs::File::create(&dest)?;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    use std::io::Write;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        let percent = if total > 0 {
            (downloaded as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let _ = app.emit(
            "model-download-progress",
            ModelDownloadProgress {
                model: DEFAULT_MODEL.to_string(),
                percent,
                bytes_downloaded: downloaded,
                total_bytes: total,
            },
        );
    }

    println!("Model download complete: {} bytes", downloaded);
    Ok(dest)
}
