use crate::state::AppState;
use crate::stt::{model_manager, whisper};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn list_models() -> Vec<model_manager::ModelInfo> {
    model_manager::list_available_models()
}

#[tauri::command]
pub async fn download_model(app: AppHandle, model_filename: String) -> Result<(), String> {
    use tauri::Emitter;
    let filename_for_progress = model_filename.clone();
    let on_progress = Box::new(move |percent: f32, downloaded: u64, total: u64| {
        let _ = app.emit(
            "model-download-progress",
            model_manager::ModelDownloadProgress {
                model: filename_for_progress.clone(),
                percent,
                bytes_downloaded: downloaded,
                total_bytes: total,
            },
        );
    });
    model_manager::download_model_by_name(&model_filename, Some(on_progress))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_model(app: AppHandle, model_filename: String) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;

    // Verify the model exists
    if model_manager::get_model_path(&model_filename).is_none() {
        return Err(format!("Model '{}' is not downloaded", model_filename));
    }

    // Update cached model in AppState
    if let Ok(mut inner) = app.state::<AppState>().lock() {
        inner.active_model = model_filename.clone();
    }

    // Clear cached whisper context so the new model is loaded on next transcription
    whisper::clear_cache();

    // Save to store for persistence across restarts
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("model", serde_json::json!(model_filename));
    store.save().map_err(|e| e.to_string())?;

    log::info!("Active model set to: {}", model_filename);
    Ok(())
}
