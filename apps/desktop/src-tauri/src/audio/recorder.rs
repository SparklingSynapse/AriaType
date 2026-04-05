use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

/// Streaming PCM callback type - receives i16 samples at system's buffer rate
/// Parameters: (samples, sample_rate, channels)
pub type PcmCallback = Box<dyn Fn(&[i16], u32, u16) + Send>;

struct RecordingHandle {
    stop_tx: mpsc::Sender<()>,
    thread_handle: thread::JoinHandle<Result<(), String>>,
}

struct ChunkedWavWriter {
    spec: hound::WavSpec,
    base_path: PathBuf,
    chunk_index: usize,
    samples_written: u64,
    samples_per_chunk: u64,
    writer: Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>,
    on_chunk: Option<Box<dyn Fn(PathBuf) + Send>>,
}

impl ChunkedWavWriter {
    fn new(
        base_path: PathBuf,
        spec: hound::WavSpec,
        on_chunk: Option<Box<dyn Fn(PathBuf) + Send>>,
    ) -> Result<Self, String> {
        let samples_per_chunk = spec.sample_rate as u64 * spec.channels as u64 * 5; // 5 seconds

        let mut writer = Self {
            spec,
            base_path,
            chunk_index: 0,
            samples_written: 0,
            samples_per_chunk,
            writer: None,
            on_chunk,
        };
        writer.open_next_chunk()?;
        Ok(writer)
    }

    fn get_chunk_path(&self) -> PathBuf {
        let mut path = self.base_path.clone();
        if let Some(stem) = path.file_stem() {
            let new_name = format!("{}_{}.wav", stem.to_string_lossy(), self.chunk_index);
            path.set_file_name(new_name);
        }
        path
    }

    fn open_next_chunk(&mut self) -> Result<(), String> {
        let path = self.get_chunk_path();
        self.writer = Some(
            hound::WavWriter::create(path, self.spec)
                .map_err(|e| format!("Failed to create chunk WAV: {}", e))?,
        );
        Ok(())
    }

    fn write_sample(&mut self, sample: i16) -> Result<(), String> {
        if let Some(writer) = &mut self.writer {
            let _ = writer.write_sample(sample);
            self.samples_written += 1;

            if self.samples_written >= self.samples_per_chunk {
                self.flush_chunk()?;
            }
        }
        Ok(())
    }

    fn flush_chunk(&mut self) -> Result<(), String> {
        if let Some(w) = self.writer.take() {
            let _ = w.finalize();
            let path = self.get_chunk_path();
            if let Some(cb) = &self.on_chunk {
                cb(path);
            }
            self.chunk_index += 1;
            self.samples_written = 0;
            self.open_next_chunk()?;
        }
        Ok(())
    }

    fn finalize(mut self) -> Result<Option<PathBuf>, String> {
        if let Some(w) = self.writer.take() {
            let _ = w.finalize();
            let path = self.get_chunk_path();
            if self.samples_written > 0 {
                if let Some(cb) = &self.on_chunk {
                    cb(path.clone());
                }
                Ok(Some(path))
            } else {
                // Remove the empty chunk file
                let _ = std::fs::remove_file(&path);
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

pub struct AudioRecorder {
    recording_handle: Arc<Mutex<Option<RecordingHandle>>>,
    /// Sample rate of the last recording (set after recording starts)
    last_sample_rate: Arc<Mutex<Option<u32>>>,
    /// Number of channels of the last recording
    last_channels: Arc<Mutex<Option<u16>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            recording_handle: Arc::new(Mutex::new(None)),
            last_sample_rate: Arc::new(Mutex::new(None)),
            last_channels: Arc::new(Mutex::new(None)),
        }
    }

    /// Start recording with streaming PCM callback.
    /// The callback receives raw i16 samples at system's buffer rate.
    /// Returns the sample rate and channels on success.
    pub fn start_streaming<F>(
        &self,
        device_name: Option<String>,
        on_pcm: F,
    ) -> Result<(u32, u16), String>
    where
        F: Fn(&[i16], u32, u16) + Send + Sync + 'static,
    {
        let mut handle_guard = self
            .recording_handle
            .lock()
            .map_err(|e| format!("Lock error: {}", e))?;

        if handle_guard.is_some() {
            return Err("Already recording".to_string());
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let sample_rate_arc = self.last_sample_rate.clone();
        let channels_arc = self.last_channels.clone();

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

            let sample_rate = config.sample_rate().0;
            let channels = config.channels();

            if let Ok(mut sr) = sample_rate_arc.lock() {
                *sr = Some(sample_rate);
            }
            if let Ok(mut ch) = channels_arc.lock() {
                *ch = Some(channels);
            }

            let on_pcm = Arc::new(on_pcm);
            let err_fn = |err| error!(error = %err, "audio_stream_error");

            let stream = match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    let cb = on_pcm.clone();
                    device
                        .build_input_stream(
                            &config.config(),
                            move |data: &[f32], _: &_| {
                                let samples: Vec<i16> = data
                                    .iter()
                                    .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
                                    .collect();
                                cb(&samples, sample_rate, channels);
                            },
                            err_fn,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                }
                cpal::SampleFormat::I16 => {
                    let cb = on_pcm.clone();
                    device
                        .build_input_stream(
                            &config.config(),
                            move |data: &[i16], _: &_| {
                                cb(data, sample_rate, channels);
                            },
                            err_fn,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                }
                cpal::SampleFormat::U16 => {
                    let cb = on_pcm.clone();
                    device
                        .build_input_stream(
                            &config.config(),
                            move |data: &[u16], _: &_| {
                                let samples: Vec<i16> =
                                    data.iter().map(|&s| (s as i32 - 32768) as i16).collect();
                                cb(&samples, sample_rate, channels);
                            },
                            err_fn,
                            None,
                        )
                        .map_err(|e| e.to_string())?
                }
                _ => {
                    return Err(format!(
                        "Unsupported sample format: {:?}",
                        config.sample_format()
                    ))
                }
            };

            stream.play().map_err(|e| e.to_string())?;
            info!(sample_rate, channels, "streaming_recording_started");

            let _ = stop_rx.recv();
            drop(stream);

            info!("streaming_recording_stopped");
            Ok(())
        });

        *handle_guard = Some(RecordingHandle {
            stop_tx: stop_tx.clone(),
            thread_handle,
        });

        const INIT_TIMEOUT_MS: u64 = 500;
        const INIT_CHECK_INTERVAL_MS: u64 = 25;

        let max_wait = Duration::from_millis(INIT_TIMEOUT_MS);
        let check_interval = Duration::from_millis(INIT_CHECK_INTERVAL_MS);
        let init_start = std::time::Instant::now();

        let (sr, ch) = loop {
            let sr = self.last_sample_rate.lock().ok().and_then(|g| *g);
            let ch = self.last_channels.lock().ok().and_then(|g| *g);

            match (sr, ch) {
                (Some(sr), Some(ch)) => break (sr, ch),
                _ => {
                    if init_start.elapsed() >= max_wait {
                        warn!(timeout_ms = INIT_TIMEOUT_MS, "audio_device_init_timeout");
                        *handle_guard = None;
                        let _ = stop_tx.send(());
                        thread::sleep(Duration::from_millis(50));
                        return Err("Failed to get recording parameters: audio device initialization timeout".to_string());
                    }
                    thread::sleep(check_interval);
                }
            }
        };

        Ok((sr, ch))
    }

    pub fn start<F>(
        &self,
        output_path: PathBuf,
        device_name: Option<String>,
        on_chunk: Option<F>,
    ) -> Result<(), String>
    where
        F: Fn(PathBuf) + Send + 'static,
    {
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

            let on_chunk_boxed: Option<Box<dyn Fn(PathBuf) + Send>> =
                on_chunk.map(|f| Box::new(f) as Box<dyn Fn(PathBuf) + Send>);

            let writer = Arc::new(Mutex::new(Some(
                ChunkedWavWriter::new(output_path, spec, on_chunk_boxed)
                    .map_err(|e| format!("Failed to create chunked WAV writer: {}", e))?,
            )));

            let writer_clone = writer.clone();
            let err_fn = |err| error!(error = %err, "audio_stream_error");

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
                _ => {
                    return Err(format!(
                        "Unsupported sample format: {:?}",
                        config.sample_format()
                    ))
                }
            };

            stream.play().map_err(|e| e.to_string())?;
            info!("audio_recording_started");

            // Wait for stop signal
            let _ = stop_rx.recv();

            // Stop stream
            drop(stream);

            // Finalize WAV
            if let Ok(mut guard) = writer.lock() {
                if let Some(w) = guard.take() {
                    let _ = w.finalize();
                }
            }

            info!("audio_recording_stopped");
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
        warn!("audio_device_enumeration_disabled");
        Vec::new()
    }

    /// Get the sample rate and channels of the current/last recording
    pub fn get_last_audio_params(&self) -> Option<(u32, u16)> {
        let sr = self.last_sample_rate.lock().ok().and_then(|g| *g);
        let ch = self.last_channels.lock().ok().and_then(|g| *g);
        sr.zip(ch)
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}
