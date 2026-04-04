/// Configuration for STT transcription.
pub struct SttConfig {
    pub language: String,
    pub translate: bool,
}

/// Pluggable speech-to-text engine.
///
/// Implementations handle the specifics of transcription (local model,
/// cloud API, etc.). Audio is always 16kHz f32 mono.
pub trait SttEngine: Send + Sync {
    fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<String, anyhow::Error>;
}
