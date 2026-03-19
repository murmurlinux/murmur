use crate::audio::capture;
use crate::state::{AppState, RecordingState};
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::Emitter;

#[derive(Clone, Serialize)]
struct RecordingStatePayload {
    state: String,
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

    // Reset state
    inner.recording_state = RecordingState::Recording;
    inner.audio_buffer.clear();
    inner.stop_flag.store(false, Ordering::Relaxed);

    let stop_flag = Arc::clone(&inner.stop_flag);
    let audio_buffer = Arc::new(std::sync::Mutex::new(Vec::<f32>::new()));

    // Store a reference to the audio buffer in app state for later retrieval
    // For now, the capture thread accumulates internally
    let audio_buf_clone = Arc::clone(&audio_buffer);

    drop(inner); // Release the lock before spawning

    // Emit state change
    let _ = app.emit(
        "recording-state",
        RecordingStatePayload {
            state: "recording".to_string(),
        },
    );

    // Start audio capture on background thread
    capture::start_capture(app.clone(), audio_buf_clone, stop_flag)
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    println!("Recording started");
    Ok(())
}

#[tauri::command]
pub fn stop_recording(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut inner = state.lock().map_err(|e| e.to_string())?;

    if inner.recording_state != RecordingState::Recording {
        return Ok(()); // Not recording, nothing to stop
    }

    // Signal the audio thread to stop
    inner.stop_flag.store(true, Ordering::Relaxed);
    inner.recording_state = RecordingState::Idle;

    drop(inner);

    // Emit state change
    let _ = app.emit(
        "recording-state",
        RecordingStatePayload {
            state: "idle".to_string(),
        },
    );

    println!("Recording stopped");
    Ok(())
}
