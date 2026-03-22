use crate::stt::model_manager;
use tauri::AppHandle;

#[tauri::command]
pub fn list_models() -> Vec<model_manager::ModelInfo> {
    model_manager::list_available_models()
}

#[tauri::command]
pub async fn download_model(app: AppHandle, model_filename: String) -> Result<(), String> {
    model_manager::download_model_by_name(app, &model_filename)
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

    // Save to store
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store
        .set("model", serde_json::json!(model_filename));
    store.save().map_err(|e| e.to_string())?;

    println!("Active model set to: {}", model_filename);
    Ok(())
}
