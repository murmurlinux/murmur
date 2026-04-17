use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Serialize)]
pub struct AudioLevel {
    pub rms: f32,
    pub peak: f32,
    pub samples: Vec<f32>,
}

/// Callback type for real-time audio level updates: (rms, peak, waveform_bars)
pub type AudioLevelCallback = Box<dyn Fn(f32, f32, Vec<f32>) + Send + 'static>;

/// Callback type for capture auto-stop notifications (VAD or max duration)
pub type AutoStopCallback = Box<dyn Fn() + Send + 'static>;

/// Downmix a multi-channel slice of any cpal sample type into mono f32.
fn convert_to_mono_f32<T>(data: &[T], channels: usize) -> Vec<f32>
where
    T: Sample + Copy,
    f32: FromSample<T>,
{
    if channels > 1 {
        data.chunks(channels)
            .map(|frame| {
                let sum: f32 = frame.iter().map(|&s| f32::from_sample(s)).sum();
                sum / channels as f32
            })
            .collect()
    } else {
        data.iter().map(|&s| f32::from_sample(s)).collect()
    }
}

/// Starts audio capture on a background thread.
/// The cpal stream is created INSIDE the thread (Stream is !Send).
/// Returns the actual device sample rate once the stream is confirmed running.
///
/// Supports F32, I16, and U16 device sample formats; other formats produce
/// an error. Failures during stream build or play are now propagated back
/// to the caller via a sync channel instead of being silently logged.
///
/// Callbacks are optional:
/// - `on_audio_level`: called ~60fps with (rms, peak, waveform_bars)
/// - `on_auto_stopped`: called when capture ends due to VAD or max duration
pub fn start_capture(
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    stop_flag: Arc<AtomicBool>,
    auto_stop_silence: bool,
    on_audio_level: Option<AudioLevelCallback>,
    on_auto_stopped: Option<AutoStopCallback>,
) -> Result<u32, anyhow::Error> {
    // Verify we can access an input device and get actual sample rate.
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input audio device available"))?;
    let actual_sample_rate = device
        .default_input_config()
        .map(|c| c.sample_rate().0)
        .unwrap_or(44100);

    let stop = Arc::clone(&stop_flag);

    // Signal back to this function whether the stream successfully started.
    // The stream itself lives on the spawned thread because cpal::Stream is !Send.
    let (init_tx, init_rx) = mpsc::channel::<Result<(), String>>();

    std::thread::spawn(move || {
        // Create the cpal stream inside this thread (Stream is !Send)
        let host = cpal::default_host();
        let device = match host.default_input_device() {
            Some(d) => d,
            None => {
                let _ = init_tx.send(Err("No input device available".to_string()));
                return;
            }
        };

        let supported_config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                let _ = init_tx.send(Err(format!("Failed to get input config: {}", e)));
                return;
            }
        };

        let sample_format = supported_config.sample_format();
        let channels = supported_config.channels() as usize;
        let stream_config = supported_config.config();

        log::info!(
            "Audio: {} ({}Hz, {}ch, {:?})",
            device.name().unwrap_or_default(),
            stream_config.sample_rate.0,
            channels,
            sample_format,
        );

        // Shared buffer: cpal callback → this thread's processing loop
        let capture_buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

        let err_cb = |e| log::error!("Audio stream error: {}", e);

        let build_res = match sample_format {
            SampleFormat::F32 => {
                let buf = Arc::clone(&capture_buffer);
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let mono = convert_to_mono_f32(data, channels);
                        if let Ok(mut b) = buf.lock() {
                            b.extend_from_slice(&mono);
                        }
                    },
                    err_cb,
                    None,
                )
            }
            SampleFormat::I16 => {
                let buf = Arc::clone(&capture_buffer);
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let mono = convert_to_mono_f32(data, channels);
                        if let Ok(mut b) = buf.lock() {
                            b.extend_from_slice(&mono);
                        }
                    },
                    err_cb,
                    None,
                )
            }
            SampleFormat::U16 => {
                let buf = Arc::clone(&capture_buffer);
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        let mono = convert_to_mono_f32(data, channels);
                        if let Ok(mut b) = buf.lock() {
                            b.extend_from_slice(&mono);
                        }
                    },
                    err_cb,
                    None,
                )
            }
            format => {
                let _ = init_tx.send(Err(format!("Unsupported sample format: {:?}", format)));
                return;
            }
        };

        let stream = match build_res {
            Ok(s) => s,
            Err(e) => {
                let _ = init_tx.send(Err(format!("Failed to build input stream: {}", e)));
                return;
            }
        };

        if let Err(e) = stream.play() {
            let _ = init_tx.send(Err(format!("Failed to play input stream: {}", e)));
            return;
        }

        // Stream is live. Signal success to the outer function.
        let _ = init_tx.send(Ok(()));

        // Processing loop -- runs on this thread alongside the stream
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

            // Notify listener (UI or CLI)
            if let Some(ref cb) = on_audio_level {
                cb(rms, peak, waveform);
            }

            // Auto-stop on silence (VAD) -- only when enabled
            if auto_stop_silence {
                const SILENCE_RMS_THRESHOLD: f32 = 0.008;
                const SILENCE_TIMEOUT_FRAMES: u32 = 125; // ~2s at 16ms per frame

                // Use a thread-local counter (this closure runs on one thread)
                use std::cell::Cell;
                thread_local! {
                    static SILENT_FRAMES: Cell<u32> = const { Cell::new(0) };
                    static HAS_SPOKEN: Cell<bool> = const { Cell::new(false) };
                }

                SILENT_FRAMES.with(|counter| {
                    HAS_SPOKEN.with(|spoken| {
                        if rms < SILENCE_RMS_THRESHOLD {
                            if spoken.get() {
                                let count = counter.get() + 1;
                                counter.set(count);
                                if count >= SILENCE_TIMEOUT_FRAMES {
                                    log::debug!(
                                        "VAD: silence detected ({:.1}s) -- auto-stopping",
                                        count as f32 * 0.016
                                    );
                                    stop_flag.store(true, Ordering::Relaxed);
                                }
                            }
                        } else {
                            counter.set(0);
                            spoken.set(true);
                        }
                    });
                });
            }

            // Accumulate for STT -- cap at 60 seconds (~2.6M samples at 44.1kHz)
            const MAX_SAMPLES: usize = 44100 * 60;
            if let Ok(mut buf) = audio_buffer.lock() {
                if buf.len() < MAX_SAMPLES {
                    let remaining = MAX_SAMPLES - buf.len();
                    let to_add = samples.len().min(remaining);
                    buf.extend_from_slice(&samples[..to_add]);
                    if buf.len() >= MAX_SAMPLES {
                        log::info!("Max recording duration (60s) reached -- auto-stopping");
                        stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }

        // Stream drops here, stopping audio capture
        drop(stream);

        // If capture ended due to VAD or max duration (not user-initiated stop),
        // notify the system to run the full stop flow (transcribe, inject, hide popup)
        if let Some(ref cb) = on_auto_stopped {
            cb();
        }
    });

    // Wait for the spawned thread to report init success or a build/play error.
    match init_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok(actual_sample_rate),
        Ok(Err(e)) => Err(anyhow::anyhow!(e)),
        Err(_) => Err(anyhow::anyhow!("Audio stream init timed out after 3s")),
    }
}
