use crate::audio::capture;
use crate::inject::paste;
use crate::stt::{model_manager, whisper};
use crate::state::{AppState, RecordingState};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;

const DEFAULT_MODEL: &str = "ggml-tiny.en.bin";

/// Read the active model filename from the settings store
fn get_active_model(app: &tauri::AppHandle) -> String {
    match app.store("settings.json") {
        Ok(store) => {
            let val: Option<serde_json::Value> = store.get("model");
            val.and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| DEFAULT_MODEL.to_string())
        }
        Err(_) => DEFAULT_MODEL.to_string(),
    }
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

#[tauri::command]
pub fn start_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut inner = state.lock().map_err(|e| e.to_string())?;

    if inner.recording_state != RecordingState::Idle {
        return Err("Already recording".to_string());
    }

    // Capture the currently focused window BEFORE we take focus
    inner.previous_window_id = paste::get_last_external_window();
    println!("Previous window: {:?}", inner.previous_window_id);

    // Reset state
    inner.recording_state = RecordingState::Recording;
    inner.stop_flag.store(false, Ordering::Relaxed);

    let stop_flag = inner.stop_flag.clone();
    let audio_buffer = inner.audio_buffer.clone();

    // Drop the outer lock BEFORE locking audio buffer (avoids nested lock)
    drop(inner);

    // Clear the audio buffer
    if let Ok(mut buf) = audio_buffer.lock() {
        buf.clear();
    }

    let _ = app.emit(
        "recording-state",
        RecordingStatePayload {
            state: "recording".to_string(),
        },
    );

    let actual_rate = capture::start_capture(app.clone(), audio_buffer, stop_flag)
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // Update sample rate from actual device config
    if let Ok(mut inner) = state.lock() {
        inner.sample_rate = actual_rate;
    }

    println!("Recording started ({}Hz)", actual_rate);
    Ok(())
}

#[tauri::command]
pub fn stop_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut inner = state.lock().map_err(|e| e.to_string())?;

    if inner.recording_state != RecordingState::Recording {
        return Ok(());
    }

    // Signal capture thread to stop
    inner.stop_flag.store(true, Ordering::Relaxed);
    inner.recording_state = RecordingState::Processing;

    let audio_buffer = inner.audio_buffer.clone();
    let sample_rate = inner.sample_rate;
    let previous_window = inner.previous_window_id.clone();

    // Drop outer lock BEFORE locking audio buffer (avoids nested lock)
    drop(inner);

    // Clone audio data from the buffer
    let audio_data: Vec<f32> = match audio_buffer.lock() {
        Ok(buf) => buf.clone(),
        Err(_) => Vec::new(),
    };

    let _ = app.emit(
        "recording-state",
        RecordingStatePayload {
            state: "processing".to_string(),
        },
    );

    println!(
        "Recording stopped. {} samples at {}Hz ({:.1}s)",
        audio_data.len(),
        sample_rate,
        audio_data.len() as f32 / sample_rate as f32
    );

    if audio_data.is_empty() {
        let _ = app.emit(
            "recording-state",
            RecordingStatePayload {
                state: "idle".to_string(),
            },
        );
        return Ok(());
    }

    // Spawn transcription on a background thread (whisper is blocking + heavy)
    let active_model = get_active_model(&app);
    let app_handle = app.clone();
    let app_for_state = app.clone();
    std::thread::spawn(move || {
        let start = std::time::Instant::now();

        // Check/download model
        let model_path = match model_manager::get_model_path(&active_model) {
            Some(p) => p,
            None => {
                println!("Model '{}' not found, downloading...", active_model);
                // Create a tokio runtime for the async download
                match tauri::async_runtime::block_on(model_manager::download_model_by_name(app_handle.clone(), &active_model)) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Model download failed: {}", e);
                        if let Ok(mut inner) = app_for_state.state::<AppState>().lock() {
                            inner.recording_state = RecordingState::Idle;
                        }
                        let _ = app_handle.emit(
                            "recording-state",
                            RecordingStatePayload {
                                state: "idle".to_string(),
                            },
                        );
                        return;
                    }
                }
            }
        };

        // Resample to 16kHz for whisper
        let audio_16k = whisper::resample(&audio_data, sample_rate, 16000);

        // Transcribe
        match whisper::transcribe(&model_path.to_string_lossy(), &audio_16k) {
            Ok(text) => {
                let duration_ms = start.elapsed().as_millis() as u64;

                if !text.is_empty() {
                    // Paste at cursor in the previously focused window
                    if let Err(e) = paste::paste_text(&text, previous_window.as_deref()) {
                        eprintln!("Paste failed: {}", e);
                    }

                    let _ = app_handle.emit(
                        "transcription-result",
                        TranscriptionResult {
                            text: text.clone(),
                            duration_ms,
                        },
                    );
                }
            }
            Err(e) => {
                eprintln!("Transcription failed: {}", e);
            }
        }

        // Back to idle — update BOTH the actual state AND emit the event
        if let Ok(mut inner) = app_for_state.state::<AppState>().lock() {
            inner.recording_state = RecordingState::Idle;
        }
        let _ = app_handle.emit(
            "recording-state",
            RecordingStatePayload {
                state: "idle".to_string(),
            },
        );
    });

    Ok(())
}

/// Internal start — called from global hotkey handler (no tauri::State available)
pub fn start_recording_internal(app: tauri::AppHandle) -> Result<(), String> {
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

    let _ = app.emit("recording-state", RecordingStatePayload { state: "recording".to_string() });

    let actual_rate = capture::start_capture(app.clone(), audio_buffer, stop_flag)
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // Update sample rate from actual device config
    if let Ok(mut inner) = app.state::<AppState>().lock() {
        inner.sample_rate = actual_rate;
    }

    Ok(())
}

/// Internal stop — called from global hotkey handler
pub fn stop_recording_internal(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut inner = state.lock().map_err(|e| e.to_string())?;

    if inner.recording_state != RecordingState::Recording {
        return Ok(());
    }

    inner.stop_flag.store(true, Ordering::Relaxed);
    inner.recording_state = RecordingState::Processing;

    let audio_buffer = inner.audio_buffer.clone();
    let sample_rate = inner.sample_rate;
    let previous_window = inner.previous_window_id.clone();

    // Drop outer lock before locking audio buffer
    drop(inner);

    let audio_data: Vec<f32> = match audio_buffer.lock() {
        Ok(buf) => buf.clone(),
        Err(_) => Vec::new(),
    };

    let _ = app.emit("recording-state", RecordingStatePayload { state: "processing".to_string() });

    if audio_data.is_empty() {
        if let Ok(mut inner) = app.state::<AppState>().lock() {
            inner.recording_state = RecordingState::Idle;
        }
        let _ = app.emit("recording-state", RecordingStatePayload { state: "idle".to_string() });
        return Ok(());
    }

    let active_model = get_active_model(&app);
    let app_handle = app.clone();
    let app_for_state2 = app.clone();
    std::thread::spawn(move || {
        // Helper to reset state to idle
        let reset_idle = || {
            if let Ok(mut inner) = app_for_state2.state::<AppState>().lock() {
                inner.recording_state = RecordingState::Idle;
            }
            let _ = app_handle.emit("recording-state", RecordingStatePayload { state: "idle".to_string() });
        };

        let start = std::time::Instant::now();

        let model_path = match model_manager::get_model_path(&active_model) {
            Some(p) => p,
            None => {
                match tauri::async_runtime::block_on(model_manager::download_model_by_name(app_handle.clone(), &active_model)) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Model download failed: {}", e);
                        reset_idle();
                        return;
                    }
                }
            }
        };

        let audio_16k = whisper::resample(&audio_data, sample_rate, 16000);

        match whisper::transcribe(&model_path.to_string_lossy(), &audio_16k) {
            Ok(text) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                if !text.is_empty() {
                    if let Err(e) = paste::paste_text(&text, previous_window.as_deref()) {
                        eprintln!("Paste failed: {}", e);
                    }
                    let _ = app_handle.emit("transcription-result", TranscriptionResult { text, duration_ms });
                }
            }
            Err(e) => eprintln!("Transcription failed: {}", e),
        }

        reset_idle();
    });

    Ok(())
}
