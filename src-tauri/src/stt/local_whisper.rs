use super::engine::{SttConfig, SttEngine};

/// Local whisper.cpp STT engine.
///
/// Delegates to the existing `whisper::transcribe()` function which
/// manages its own model cache internally.
pub struct LocalWhisperEngine {
    model_path: String,
}

impl LocalWhisperEngine {
    pub fn new(model_path: &str) -> Self {
        Self {
            model_path: model_path.to_string(),
        }
    }
}

impl SttEngine for LocalWhisperEngine {
    fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<String, anyhow::Error> {
        super::whisper::transcribe(&self.model_path, audio, &config.language, config.translate)
    }
}
