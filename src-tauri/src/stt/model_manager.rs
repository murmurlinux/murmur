use directories::ProjectDirs;
use serde::Serialize;
use std::path::PathBuf;
use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub url: String,
    pub size_mb: u64,
    pub description: String,
    pub downloaded: bool,
}

/// A Whisper model available for download.
#[derive(Debug)]
pub struct ModelEntry {
    pub name: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    pub size_mb: u64,
    pub description: &'static str,
    pub sha256: &'static str,
}

/// Model registry with download URLs and checksums.
pub const MODELS: &[ModelEntry] = &[
    ModelEntry {
        name: "Tiny (English)",
        filename: "ggml-tiny.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        size_mb: 75,
        description: "Fastest, lowest accuracy (~3-4s)",
        sha256: "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
    },
    ModelEntry {
        name: "Base (English)",
        filename: "ggml-base.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        size_mb: 142,
        description: "Good balance of speed and accuracy (~8-10s)",
        sha256: "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c034d5",
    },
    ModelEntry {
        name: "Small (English)",
        filename: "ggml-small.en.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        size_mb: 466,
        description: "Best accuracy, slowest (~20-30s)",
        sha256: "6083e2549b2a66e4ba9a85b1a46833d7a8e43e4e065daca3b19e0d4e2b3304f2",
    },
    ModelEntry {
        name: "Tiny (Multilingual)",
        filename: "ggml-tiny.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        size_mb: 75,
        description: "Fastest, 99+ languages (~3-4s)",
        sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
    },
    ModelEntry {
        name: "Base (Multilingual)",
        filename: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        size_mb: 142,
        description: "Good balance, 99+ languages (~8-10s)",
        sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
    },
    ModelEntry {
        name: "Small (Multilingual)",
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        size_mb: 466,
        description: "Best accuracy, 99+ languages (~20-30s)",
        sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
    },
];

#[derive(Clone, Serialize)]
pub struct ModelDownloadProgress {
    pub model: String,
    pub percent: f32,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}

/// Get the models directory (~/.local/share/murmur/models/)
pub fn models_dir() -> PathBuf {
    if let Some(dirs) = ProjectDirs::from("com", "murmurlinux", "murmur") {
        let path = dirs.data_dir().join("models");
        std::fs::create_dir_all(&path).ok();
        path
    } else {
        let path = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".local/share/murmur/models");
        std::fs::create_dir_all(&path).ok();
        path
    }
}

/// Validate a model filename -- prevent path traversal.
fn validate_filename(filename: &str) -> Result<(), anyhow::Error> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err(anyhow::anyhow!("Invalid model filename: {}", filename));
    }
    Ok(())
}

/// List all available models with their download status
pub fn list_available_models() -> Vec<ModelInfo> {
    MODELS
        .iter()
        .map(|m| {
            let path = models_dir().join(m.filename);
            ModelInfo {
                name: m.name.to_string(),
                filename: m.filename.to_string(),
                url: m.url.to_string(),
                size_mb: m.size_mb,
                description: m.description.to_string(),
                downloaded: path.exists(),
            }
        })
        .collect()
}

/// Get the path to a specific model, or None if not downloaded
pub fn get_model_path(filename: &str) -> Option<PathBuf> {
    if validate_filename(filename).is_err() {
        return None;
    }
    let path = models_dir().join(filename);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Look up model metadata by filename
fn get_model_meta(filename: &str) -> Option<(&'static str, &'static str)> {
    MODELS
        .iter()
        .find(|m| m.filename == filename)
        .map(|m| (m.url, m.sha256))
}

/// Verify SHA256 checksum of a file
fn verify_checksum(path: &PathBuf, expected_sha256: &str) -> Result<bool, anyhow::Error> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha2_hash_context();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let hash = hasher.finalize_hex();
    Ok(hash == expected_sha256)
}

/// SHA256 hasher using the sha2 crate -- no shell-out, no memory doubling.
struct Sha256Hasher {
    hasher: sha2::Sha256,
}

fn sha2_hash_context() -> Sha256Hasher {
    use sha2::Digest;
    Sha256Hasher {
        hasher: sha2::Sha256::new(),
    }
}

impl Sha256Hasher {
    fn update(&mut self, bytes: &[u8]) {
        use sha2::Digest;
        self.hasher.update(bytes);
    }

    fn finalize_hex(self) -> String {
        use sha2::Digest;
        let result = self.hasher.finalize();
        hex::encode(result)
    }
}

/// Download a model by filename with progress events, atomic writes, and checksum verification
pub async fn download_model_by_name(
    app: tauri::AppHandle,
    filename: &str,
) -> Result<PathBuf, anyhow::Error> {
    validate_filename(filename)?;

    let dest = models_dir().join(filename);

    if dest.exists() {
        log::info!("Model already downloaded: {}", dest.display());
        return Ok(dest);
    }

    let (url, expected_sha) =
        get_model_meta(filename).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", filename))?;

    log::info!("Downloading whisper model to: {}", dest.display());

    // Download to a temporary file first (atomic write pattern)
    let tmp_dest = models_dir().join(format!("{}.tmp", filename));

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = std::fs::File::create(&tmp_dest)?;
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    use std::io::Write;

    let result: Result<(), anyhow::Error> = async {
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
                    model: filename.to_string(),
                    percent,
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                },
            );
        }
        Ok(())
    }
    .await;

    // If download failed, clean up the temp file
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp_dest);
        return Err(e);
    }

    // Flush and close the file before verification
    drop(file);

    // Verify checksum
    match verify_checksum(&tmp_dest, expected_sha) {
        Ok(true) => {
            // Checksum matches -- atomically rename to final destination
            std::fs::rename(&tmp_dest, &dest)?;
            log::info!("Model download complete and verified: {} bytes", downloaded);
            Ok(dest)
        }
        Ok(false) => {
            let _ = std::fs::remove_file(&tmp_dest);
            Err(anyhow::anyhow!(
                "Model checksum verification failed -- file may be corrupted. Please try again."
            ))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_dest);
            Err(anyhow::anyhow!("Failed to verify model checksum: {}", e))
        }
    }
}
