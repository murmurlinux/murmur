use crate::audio::capture;
use crate::inject::paste;
use crate::state::{AppState, RecordingState};
use crate::stt::{model_manager, whisper};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;

/// Read the active model filename from AppState cache
fn get_active_model(app: &tauri::AppHandle) -> String {
    app.state::<AppState>()
        .lock()
        .map(|inner| inner.active_model.clone())
        .unwrap_or_else(|_| "ggml-tiny.en.bin".to_string())
}

#[derive(Clone, Serialize)]
struct RecordingStatePayload {
    state: String,
}

#[derive(Clone, Serialize)]
struct TranscriptionResult {
    text: String,
    duration_ms: u64,
}

fn emit_state(app: &tauri::AppHandle, state: &str) {
    let _ = app.emit(
        "recording-state",
        RecordingStatePayload {
            state: state.to_string(),
        },
    );
}

// --- Core recording logic (shared by command and internal variants) ---

fn start_recording_core(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut inner = state.lock().map_err(|e| e.to_string())?;

    if inner.recording_state != RecordingState::Idle {
        return Err("Already recording".to_string());
    }

    inner.previous_window_id = paste::get_last_external_window();
    inner.recording_state = RecordingState::Recording;
    inner.stop_flag.store(false, Ordering::Relaxed);

    let stop_flag = inner.stop_flag.clone();
    let audio_buffer = inner.audio_buffer.clone();
    drop(inner);

    // Clear buffer after dropping outer lock
    if let Ok(mut buf) = audio_buffer.lock() {
        buf.clear();
    }

    emit_state(app, "recording");
    super::popup::show_popup(app);

    // Read auto-stop silence setting (only active in tap mode)
    let auto_stop = app
        .store("settings.json")
        .ok()
        .map(|store| {
            let mode = store
                .get("recordMode")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "hold".to_string());
            let enabled = store
                .get("autoStopSilence")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            log::debug!(
                "VAD settings: recordMode={}, autoStopSilence={}, auto_stop={}",
                mode,
                enabled,
                mode == "tap" && enabled
            );
            mode == "tap" && enabled
        })
        .unwrap_or(false);

    // Pass Tauri event emission as closures to the decoupled capture function
    let app_for_level = app.clone();
    let on_level = Box::new(move |rms: f32, peak: f32, samples: Vec<f32>| {
        let _ = app_for_level.emit("audio-level", capture::AudioLevel { rms, peak, samples });
    });

    let app_for_stop = app.clone();
    let on_stopped = Box::new(move || {
        let _ = app_for_stop.emit("capture-auto-stopped", ());
    });

    let actual_rate = capture::start_capture(
        audio_buffer,
        stop_flag,
        auto_stop,
        Some(on_level),
        Some(on_stopped),
    )
    .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // Update sample rate from actual device config
    if let Ok(mut inner) = state.lock() {
        inner.sample_rate = actual_rate;
    }

    log::info!("Recording started ({}Hz)", actual_rate);
    Ok(())
}

fn stop_recording_core(app: &tauri::AppHandle) -> Result<(), String> {
    let state_handle = app.state::<AppState>();
    let mut inner = state_handle.lock().map_err(|e| e.to_string())?;

    if inner.recording_state != RecordingState::Recording {
        return Ok(());
    }

    inner.stop_flag.store(true, Ordering::Relaxed);
    inner.recording_state = RecordingState::Processing;

    let audio_buffer = inner.audio_buffer.clone();
    let sample_rate = inner.sample_rate;
    let previous_window = inner.previous_window_id.clone();
    drop(inner);

    let audio_data: Vec<f32> = match audio_buffer.lock() {
        Ok(buf) => buf.clone(),
        Err(_) => Vec::new(),
    };

    emit_state(app, "processing");
    super::popup::hide_popup(app);

    log::info!(
        "Recording stopped. {} samples at {}Hz ({:.1}s)",
        audio_data.len(),
        sample_rate,
        audio_data.len() as f32 / sample_rate as f32
    );

    if audio_data.is_empty() {
        emit_state(app, "idle");
        super::popup::hide_popup(app);
        if let Ok(mut inner) = state_handle.lock() {
            inner.recording_state = RecordingState::Idle;
        }
        return Ok(());
    }

    // Read language settings for transcription
    let (language, translate) = app
        .store("settings.json")
        .ok()
        .map(|store| {
            let lang = store
                .get("language")
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "en".to_string());
            let trans = store
                .get("translateToEnglish")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            (lang, trans)
        })
        .unwrap_or_else(|| ("en".to_string(), false));

    // Spawn transcription on a background thread
    let active_model = get_active_model(app);
    let app_handle = app.clone();
    let app_for_state = app.clone();

    std::thread::spawn(move || {
        let reset_idle = || {
            if let Ok(mut inner) = app_for_state.state::<AppState>().lock() {
                inner.recording_state = RecordingState::Idle;
            }
            emit_state(&app_handle, "idle");
        };

        let start = std::time::Instant::now();

        let model_path = match model_manager::get_model_path(&active_model) {
            Some(p) => p,
            None => {
                log::info!("Model '{}' not found, downloading...", active_model);
                match tauri::async_runtime::block_on(model_manager::download_model_by_name(
                    &active_model,
                    None,
                )) {
                    Ok(p) => p,
                    Err(e) => {
                        log::error!("Model download failed: {}", e);
                        reset_idle();
                        return;
                    }
                }
            }
        };

        // Trim trailing silence to prevent Whisper hallucinations.
        // When VAD auto-stops, the last ~2s are silence that Whisper fills
        // with phantom text ("Thank you", "♪♪", repeated words, etc.).
        let trimmed = trim_trailing_silence(&audio_data, sample_rate);
        let audio_16k = whisper::resample(trimmed, sample_rate, 16000);

        match whisper::transcribe(
            &model_path.to_string_lossy(),
            &audio_16k,
            &language,
            translate,
        ) {
            Ok(text) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                if !text.is_empty() {
                    if let Err(e) = paste::paste_text(&text, previous_window.as_deref()) {
                        log::error!("Paste failed: {}", e);
                    }
                    let _ = app_handle.emit(
                        "transcription-result",
                        TranscriptionResult { text, duration_ms },
                    );
                }
            }
            Err(e) => log::error!("Transcription failed: {}", e),
        }

        reset_idle();
    });

    Ok(())
}

// --- Tauri command wrappers (IPC from frontend) ---

#[tauri::command]
pub fn start_recording(app: tauri::AppHandle) -> Result<(), String> {
    start_recording_core(&app)
}

#[tauri::command]
pub fn stop_recording(app: tauri::AppHandle) -> Result<(), String> {
    stop_recording_core(&app)
}

/// Trim trailing silence from audio to prevent Whisper hallucinations.
/// Walks backwards from the end, finding the last sample above the threshold,
/// then keeps a small tail (~100ms) for natural trailing off.
pub fn trim_trailing_silence(audio: &[f32], sample_rate: u32) -> &[f32] {
    const RMS_THRESHOLD: f32 = 0.01;
    const CHUNK_MS: u32 = 30;
    let chunk_size = (sample_rate * CHUNK_MS / 1000) as usize;
    let tail_padding = (sample_rate as f32 * 0.1) as usize; // keep 100ms after last speech

    if audio.len() < chunk_size {
        return audio;
    }

    let mut last_speech: Option<usize> = None;
    let mut i = audio.len();
    while i >= chunk_size {
        let start = i - chunk_size;
        let chunk = &audio[start..i];
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        if rms >= RMS_THRESHOLD {
            last_speech = Some(i);
            break;
        }
        i -= chunk_size;
    }

    // Check the leading partial chunk that the main loop couldn't reach
    if last_speech.is_none() && i > 0 {
        let chunk = &audio[..i];
        let rms = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        if rms >= RMS_THRESHOLD {
            last_speech = Some(i);
        }
    }

    let end = match last_speech {
        Some(pos) => (pos + tail_padding).min(audio.len()),
        None => 0, // No speech found at all -- return empty
    };
    let trimmed_duration = audio.len() - end;
    if trimmed_duration > 0 {
        log::debug!(
            "Trimmed {:.1}s of trailing silence",
            trimmed_duration as f32 / sample_rate as f32
        );
    }
    &audio[..end]
}

// --- Internal wrappers (called from hotkey handler) ---

pub fn start_recording_internal(app: tauri::AppHandle) -> Result<(), String> {
    start_recording_core(&app)
}

pub fn stop_recording_internal(app: tauri::AppHandle) -> Result<(), String> {
    stop_recording_core(&app)
}
