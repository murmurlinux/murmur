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
    pub audio_buffer: Vec<f32>,
    pub stop_flag: Arc<AtomicBool>,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            recording_state: RecordingState::Idle,
            audio_buffer: Vec::new(),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub type AppState = Mutex<InnerState>;
