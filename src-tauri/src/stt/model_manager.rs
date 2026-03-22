use directories::ProjectDirs;
use serde::Serialize;
use std::path::PathBuf;
use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub url: String,
    pub size_mb: u32,
    pub description: String,
    pub downloaded: bool,
}

/// Model registry: (name, filename, url, size_mb, description, sha256)
const MODELS: &[(&str, &str, &str, u32, &str, &str)] = &[
    (
        "Tiny (English)",
        "ggml-tiny.en.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        75,
        "Fastest, lowest accuracy (~3-4s)",
        "921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f",
    ),
    (
        "Base (English)",
        "ggml-base.en.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        142,
        "Good balance of speed and accuracy (~8-10s)",
        "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c034d5",
    ),
    (
        "Small (English)",
        "ggml-small.en.bin",
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        466,
        "Best accuracy, slowest (~20-30s)",
        "6083e2549b2a66e4ba9a85b1a46833d7a8e43e4e065daca3b19e0d4e2b3304f2",
    ),
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
    if let Some(dirs) = ProjectDirs::from("com", "syncrotrade", "murmur") {
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

/// Validate a model filename — prevent path traversal.
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
        .map(|(name, filename, url, size_mb, desc, _sha)| {
            let path = models_dir().join(filename);
            ModelInfo {
                name: name.to_string(),
                filename: filename.to_string(),
                url: url.to_string(),
                size_mb: *size_mb,
                description: desc.to_string(),
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
        .find(|(_, f, _, _, _, _)| *f == filename)
        .map(|(_, _, url, _, _, sha)| (*url, *sha))
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

/// Simple SHA256 implementation using manual computation.
/// We avoid adding a new crate by using a minimal hasher.
struct Sha256Hasher {
    data: Vec<u8>,
}

fn sha2_hash_context() -> Sha256Hasher {
    Sha256Hasher { data: Vec::new() }
}

impl Sha256Hasher {
    fn update(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    fn finalize_hex(self) -> String {
        // Use the system sha256sum command for verification
        use std::io::Write;
        use std::process::{Command, Stdio};

        let mut child = Command::new("sha256sum")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("sha256sum not found");

        child.stdin.take().unwrap().write_all(&self.data).unwrap();
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.split_whitespace().next().unwrap_or("").to_string()
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
        println!("Model already downloaded: {}", dest.display());
        return Ok(dest);
    }

    let (url, expected_sha) = get_model_meta(filename)
        .ok_or_else(|| anyhow::anyhow!("Unknown model: {}", filename))?;

    println!("Downloading whisper model to: {}", dest.display());

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
            // Checksum matches — atomically rename to final destination
            std::fs::rename(&tmp_dest, &dest)?;
            println!("Model download complete and verified: {} bytes", downloaded);
            Ok(dest)
        }
        Ok(false) => {
            let _ = std::fs::remove_file(&tmp_dest);
            Err(anyhow::anyhow!(
                "Model checksum verification failed — file may be corrupted. Please try again."
            ))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_dest);
            Err(anyhow::anyhow!("Failed to verify model checksum: {}", e))
        }
    }
}
