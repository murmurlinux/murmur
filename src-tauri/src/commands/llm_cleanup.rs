use serde::Serialize;
use std::time::Duration;

use crate::cleanup::factory::build_cleanup_service;

const CANNED_INPUT: &str =
    "um so like the thing is you know we should probably just you know ship it";

#[derive(Serialize)]
pub struct TestCleanupResult {
    pub success: bool,
    pub cleaned: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Tauri command invoked by the Settings "Test" button. Runs the LLM
/// cleanup against a canned noisy input so the user can confirm their
/// provider + key work before first real dictation.
#[tauri::command]
pub async fn test_cleanup(provider: String, api_key: String) -> TestCleanupResult {
    let handle = tauri::async_runtime::spawn_blocking(move || {
        let start = std::time::Instant::now();
        let svc = match build_cleanup_service(&provider, &api_key, Duration::from_secs(5)) {
            Ok(s) => s,
            Err(e) => {
                return TestCleanupResult {
                    success: false,
                    cleaned: None,
                    error: Some(e),
                    duration_ms: 0,
                };
            }
        };
        match svc.cleanup(CANNED_INPUT, "en") {
            Ok(cleaned) => TestCleanupResult {
                success: true,
                cleaned: Some(cleaned),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => TestCleanupResult {
                success: false,
                cleaned: None,
                error: Some(e.to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    });

    handle.await.unwrap_or_else(|_| TestCleanupResult {
        success: false,
        cleaned: None,
        error: Some("task panic".into()),
        duration_ms: 0,
    })
}
