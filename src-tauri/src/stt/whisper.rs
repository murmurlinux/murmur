use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

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

/// Transcribe audio using whisper.cpp
/// `audio` must be 16kHz f32 mono
pub fn transcribe(model_path: &str, audio: &[f32]) -> Result<String, anyhow::Error> {
    if audio.is_empty() {
        return Ok(String::new());
    }

    let start = std::time::Instant::now();

    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| anyhow::anyhow!("Failed to load whisper model: {}", e))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_suppress_blank(true);
    params.set_suppress_non_speech_tokens(true);

    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create whisper state: {}", e))?;

    state
        .full(params, audio)
        .map_err(|e| anyhow::anyhow!("Transcription failed: {}", e))?;

    let num_segments = state
        .full_n_segments()
        .map_err(|e| anyhow::anyhow!("Failed to get segments: {}", e))?;

    let mut text = String::new();
    for i in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(i) {
            text.push_str(&segment);
        }
    }

    let duration = start.elapsed();
    println!(
        "Transcription: {:?} ({} samples, {:.1}s audio → {:.1}s processing)",
        text.trim(),
        audio.len(),
        audio.len() as f32 / 16000.0,
        duration.as_secs_f32()
    );

    Ok(text.trim().to_string())
}
