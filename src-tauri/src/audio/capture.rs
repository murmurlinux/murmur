use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

#[derive(Clone, Serialize)]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
    pub samples: Vec<f32>,
}

/// Starts audio capture on a background thread.
/// The cpal stream is created INSIDE the thread (Stream is !Send).
pub fn start_capture(
    app: tauri::AppHandle,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    stop_flag: Arc<AtomicBool>,
) -> Result<(), anyhow::Error> {
    // Verify we can access an input device before spawning the thread
    let host = cpal::default_host();
    let _device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input audio device available"))?;

    let stop = Arc::clone(&stop_flag);
    let app_handle = app.clone();

    std::thread::spawn(move || {
        // Create the cpal stream inside this thread (Stream is !Send)
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                eprintln!("No input device available");
                return;
            }
        };

        let default_config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get input config: {}", e);
                return;
            }
        };

        let channels = default_config.channels() as usize;
        println!(
            "Audio: {} ({}Hz, {}ch)",
            device.name().unwrap_or_default(),
            default_config.sample_rate().0,
            channels
        );

        // Shared buffer: cpal callback → this thread's processing loop
        let capture_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let capture_buf_clone = Arc::clone(&capture_buffer);

        let stream = match device.build_input_stream(
            &default_config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Convert multi-channel to mono
                let mono: Vec<f32> = if channels > 1 {
                    data.chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect()
                } else {
                    data.to_vec()
                };
                if let Ok(mut buf) = capture_buf_clone.lock() {
                    buf.extend_from_slice(&mono);
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to build input stream: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("Failed to play stream: {}", e);
            return;
        }

        // Processing loop — runs on this thread alongside the stream
        const WAVEFORM_BARS: usize = 48;
        let emit_interval = std::time::Duration::from_millis(16); // ~60fps
        let mut last_emit = std::time::Instant::now();
        let start_time = std::time::Instant::now();

        // Auto-gain: track recent peak to normalise waveform dynamically
        // Start at a reasonable voice level so quiet room doesn't overreact
        let mut recent_peak: f32 = 0.05;

        loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(4));

            let now = std::time::Instant::now();
            if now.duration_since(last_emit) < emit_interval {
                continue;
            }
            last_emit = now;

            // Drain capture buffer
            let samples: Vec<f32> = {
                match capture_buffer.lock() {
                    Ok(mut buf) => buf.drain(..).collect(),
                    Err(_) => continue,
                }
            };

            if samples.is_empty() {
                continue;
            }

            // Skip first 150ms to avoid startup spike
            if now.duration_since(start_time) < std::time::Duration::from_millis(150) {
                continue;
            }

            // Compute RMS and peak
            let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
            let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

            // Update auto-gain: slowly track the recent peak level
            // Ramp up quickly (to catch loud speech), decay slowly (to stay sensitive)
            if peak > recent_peak {
                recent_peak = recent_peak * 0.5 + peak * 0.5; // fast attack
            } else {
                recent_peak = recent_peak * 0.995 + peak * 0.005; // slow decay
            }
            // Clamp gain so quiet room noise doesn't get blown up
            let gain = 1.0 / recent_peak.max(0.02);

            // Downsample to bar heights for waveform display (with auto-gain applied)
            let waveform: Vec<f32> = if samples.len() >= WAVEFORM_BARS {
                let chunk_size = samples.len() / WAVEFORM_BARS;
                (0..WAVEFORM_BARS)
                    .map(|i| {
                        let start = i * chunk_size;
                        let end = (start + chunk_size).min(samples.len());
                        let bar_peak = samples[start..end]
                            .iter()
                            .map(|s| s.abs())
                            .fold(0.0f32, f32::max);
                        (bar_peak * gain).min(1.0) // normalised 0-1
                    })
                    .collect()
            } else {
                samples.iter().map(|s| (s.abs() * gain).min(1.0)).collect()
            };

            // Emit to frontend
            let _ = app_handle.emit(
                "audio-level",
                AudioLevel {
                    rms,
                    peak,
                    samples: waveform,
                },
            );

            // Accumulate for STT (Slice 3)
            if let Ok(mut buf) = audio_buffer.lock() {
                buf.extend_from_slice(&samples);
            }
        }

        // Stream drops here, stopping audio capture
        drop(stream);
        println!("Audio capture thread stopped");
    });

    Ok(())
}
