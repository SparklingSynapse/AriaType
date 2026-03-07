use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct RecordingHandle {
    stop_tx: mpsc::Sender<()>,
    thread_handle: thread::JoinHandle<Result<(), String>>,
}

pub struct AudioRecorder {
    recording_handle: Arc<Mutex<Option<RecordingHandle>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            recording_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&self, output_path: PathBuf, device_name: Option<String>) -> Result<(), String> {
        let mut handle_guard = self
            .recording_handle
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        if handle_guard.is_some() {
            return Err("Already recording".to_string());
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let thread_handle = thread::spawn(move || -> Result<(), String> {
            let host = cpal::default_host();
            let device = match device_name.as_deref() {
                None | Some("default") => host
                    .default_input_device()
                    .ok_or("No input device available")?,
                Some(name) => host
                    .input_devices()
                    .map_err(|e| e.to_string())?
                    .find(|d| d.name().ok().as_deref() == Some(name))
                    .or_else(|| host.default_input_device())
                    .ok_or("No input device available")?,
            };

            let config = device
                .default_input_config()
                .map_err(|e| format!("Failed to get input config: {}", e))?;

            let spec = hound::WavSpec {
                channels: config.channels(),
                sample_rate: config.sample_rate().0,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let writer = Arc::new(Mutex::new(Some(
                hound::WavWriter::create(&output_path, spec)
                    .map_err(|e| format!("Failed to create WAV file: {}", e))?,
            )));

            let writer_clone = writer.clone();
            let err_fn = |err| tracing::error!("Audio stream error: {}", err);

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    let w = writer_clone.clone();
                    device
                        .build_input_stream(
                            &config.config(),
                            move |data: &[f32], _: &_| {
                                if let Ok(mut guard) = w.try_lock() {
                                    if let Some(writer) = guard.as_mut() {
                                        for &sample in data {
                                            let s = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                                            let _ = writer.write_sample(s);
                                        }
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                }
                cpal::SampleFormat::I16 => {
                    let w = writer_clone.clone();
                    device
                        .build_input_stream(
                            &config.config(),
                            move |data: &[i16], _: &_| {
                                if let Ok(mut guard) = w.try_lock() {
                                    if let Some(writer) = guard.as_mut() {
                                        for &sample in data {
                                            let _ = writer.write_sample(sample);
                                        }
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                }
                cpal::SampleFormat::U16 => {
                    let w = writer_clone.clone();
                    device
                        .build_input_stream(
                            &config.config(),
                            move |data: &[u16], _: &_| {
                                if let Ok(mut guard) = w.try_lock() {
                                    if let Some(writer) = guard.as_mut() {
                                        for &sample in data {
                                            let s = (sample as i32 - 32768) as i16;
                                            let _ = writer.write_sample(s);
                                        }
                                    }
                                }
                            },
                            err_fn,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                }
                _ => return Err(format!("Unsupported sample format: {:?}", config.sample_format())),
            };

            stream.play().map_err(|e| e.to_string())?;
            tracing::info!("Audio recording started");

            // Wait for stop signal
            let _ = stop_rx.recv();

            // Stop stream
            drop(stream);

            // Finalize WAV
            if let Ok(mut guard) = writer.lock() {
                if let Some(w) = guard.take() {
                    w.finalize().map_err(|e| format!("Failed to finalize WAV: {}", e))?;
                }
            }

            tracing::info!("Audio recording stopped and finalized");
            Ok(())
        });

        *handle_guard = Some(RecordingHandle {
            stop_tx,
            thread_handle,
        });

        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        let handle = self
            .recording_handle
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?
            .take();

        if let Some(handle) = handle {
            let _ = handle.stop_tx.send(());

            // Wait for thread with timeout
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                if handle.thread_handle.is_finished() {
                    return handle
                        .thread_handle
                        .join()
                        .map_err(|_| "Recording thread panicked".to_string())?;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err("Recording thread timed out".to_string())
        } else {
            Err("Not recording".to_string())
        }
    }

    pub fn is_recording(&self) -> bool {
        self.recording_handle
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    pub fn get_devices() -> Vec<String> {
        // Return empty list to avoid crashes during enumeration
        // Devices will be enumerated when actually needed for recording
        tracing::warn!("audio device enumeration disabled to prevent crashes");
        Vec::new()
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}
