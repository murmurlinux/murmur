use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingState {
    Idle,
    Recording,
    Processing,
}

pub struct InnerState {
    pub recording_state: RecordingState,
    pub audio_buffer: Arc<Mutex<Vec<f32>>>,
    pub stop_flag: Arc<AtomicBool>,
    pub sample_rate: u32,
    pub previous_window_id: Option<String>,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            recording_state: RecordingState::Idle,
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            stop_flag: Arc::new(AtomicBool::new(false)),
            sample_rate: 44100,
            previous_window_id: None,
        }
    }
}

pub type AppState = Mutex<InnerState>;
