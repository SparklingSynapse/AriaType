use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tracing::{error, info, warn};

use crate::events::EventName;
use crate::state::app_state::AppState;

use super::shared::{AUDIO_ACTIVITY_OFF_THRESHOLD, AUDIO_ACTIVITY_ON_THRESHOLD};

pub fn start_audio_level_monitor(app: AppHandle) -> Result<(), String> {
    info!("audio_level_monitor_started");

    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| "AppState not available".to_string())?;

    let rx = state
        .level_monitor_rx
        .lock()
        .take()
        .ok_or("level monitor receiver already taken")?;

    let audio_level = Arc::new(AtomicU32::new(0));
    let mut stream: Option<cpal::Stream> = None;
    let mut last_activity = false;
    let mut last_seen_start_ms: u64 = 0;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));

        let current_start_ms = state.recording_start_time.load(Ordering::SeqCst);
        if current_start_ms != last_seen_start_ms {
            last_seen_start_ms = current_start_ms;
            last_activity = false;
        }

        let mut cmd: Option<bool> = None;
        while let Ok(v) = rx.try_recv() {
            cmd = Some(v);
        }

        if let Some(should_open) = cmd {
            if should_open && stream.is_none() {
                let host = cpal::default_host();
                let audio_device = {
                    let settings = state.settings.lock();
                    settings.audio_device.clone()
                };
                let device = if audio_device == "default" {
                    host.default_input_device()
                } else {
                    host.input_devices()
                        .ok()
                        .and_then(|mut devs| {
                            devs.find(|d| d.name().ok().as_deref() == Some(&audio_device))
                        })
                        .or_else(|| host.default_input_device())
                };

                if let Some(device) = device {
                    match device.default_input_config() {
                        Ok(config) => {
                            let level_clone = audio_level.clone();
                            let err_fn = |err| error!(error = %err, "audio_stream_error");
                            match device.build_input_stream(
                                &config.into(),
                                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                    let sum: f32 = data.iter().map(|&s| s * s).sum::<f32>()
                                        / data.len() as f32;
                                    let rms = sum.sqrt();
                                    let db = 20.0 * rms.log10();
                                    let normalized =
                                        ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0) as u32;
                                    level_clone.store(normalized, Ordering::SeqCst);
                                },
                                err_fn,
                                None,
                            ) {
                                Ok(s) => match s.play() {
                                    Ok(()) => {
                                        info!("audio_level_stream_opened");
                                        stream = Some(s);
                                    }
                                    Err(e) => {
                                        error!(error = %e, "audio_level_stream_play_failed")
                                    }
                                },
                                Err(e) => error!(error = %e, "audio_level_stream_build_failed"),
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "audio_level_input_config_failed")
                        }
                    }
                } else {
                    warn!("audio_level_input_device_not_found");
                }
            } else if !should_open && stream.is_some() {
                drop(stream.take());
                audio_level.store(0, Ordering::SeqCst);
                info!("audio_level_stream_closed");
            }
        }

        let level = if stream.is_some() {
            audio_level.load(Ordering::SeqCst)
        } else {
            0
        };

        state.audio_level.store(level, Ordering::SeqCst);
        let _ = app.emit(EventName::AUDIO_LEVEL, level);

        let has_activity = if last_activity {
            level >= AUDIO_ACTIVITY_OFF_THRESHOLD
        } else {
            level > AUDIO_ACTIVITY_ON_THRESHOLD
        };
        let suppressed = if has_activity {
            let start_ms = state.recording_start_time.load(Ordering::SeqCst);
            if start_ms > 0 {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                now_ms.saturating_sub(start_ms) < 400
            } else {
                false
            }
        } else {
            false
        };
        let effective_activity = has_activity && !suppressed;
        if effective_activity != last_activity {
            let _ = app.emit(EventName::AUDIO_ACTIVITY, effective_activity);
            last_activity = effective_activity;
        }
    }
}
