use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Cached WhisperContext — avoids reloading the model from disk on every transcription.
/// The cache is invalidated when the model path changes.
struct CachedContext {
    ctx: WhisperContext,
    model_path: String,
}

static WHISPER_CACHE: Mutex<Option<CachedContext>> = Mutex::new(None);

/// Resample audio from one sample rate to another using linear interpolation
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    (0..output_len)
        .map(|i| {
            let src_idx = i as f64 * ratio;
            let idx = src_idx as usize;
            let frac = src_idx - idx as f64;
            let s0 = samples.get(idx).copied().unwrap_or(0.0);
            let s1 = samples.get(idx + 1).copied().unwrap_or(s0);
            (s0 as f64 * (1.0 - frac) + s1 as f64 * frac) as f32
        })
        .collect()
}

/// Transcribe audio using whisper.cpp with cached model context.
/// `audio` must be 16kHz f32 mono
pub fn transcribe(
    model_path: &str,
    audio: &[f32],
    language: &str,
    translate: bool,
) -> Result<String, anyhow::Error> {
    if audio.is_empty() {
        return Ok(String::new());
    }

    let start = std::time::Instant::now();

    // Use cached context — hold the lock for the duration of transcription
    let mut cache = WHISPER_CACHE
        .lock()
        .map_err(|e| anyhow::anyhow!("Cache lock poisoned: {}", e))?;

    let needs_reload = match &*cache {
        Some(cached) => cached.model_path != model_path,
        None => true,
    };

    if needs_reload {
        log::info!("Loading whisper model: {} (Vulkan GPU enabled)", model_path);
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| anyhow::anyhow!("Failed to load whisper model: {}", e))?;
        *cache = Some(CachedContext {
            ctx,
            model_path: model_path.to_string(),
        });
    }

    let cached = cache.as_ref().unwrap(); // Safe: we just ensured it's Some

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    if language == "auto" {
        params.set_language(None);
    } else {
        params.set_language(Some(language));
    }
    params.set_translate(translate);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_suppress_nst(true);

    let mut state = cached
        .ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {}", e))?;

    state
        .full(params, audio)
        .map_err(|e| anyhow::anyhow!("Transcription failed: {}", e))?;

    let mut text = String::new();
    for segment in state.as_iter() {
        if let Ok(s) = segment.to_str() {
            text.push_str(s);
        }
    }

    let duration = start.elapsed();
    log::debug!(
        "Transcription: {:?} ({} samples, {:.1}s audio -> {:.1}s processing)",
        text.trim(),
        audio.len(),
        audio.len() as f32 / 16000.0,
        duration.as_secs_f32()
    );

    Ok(text.trim().to_string())
}

/// Clear the cached WhisperContext (e.g., when the model changes).
pub fn clear_cache() {
    if let Ok(mut cache) = WHISPER_CACHE.lock() {
        *cache = None;
    }
}
