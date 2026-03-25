use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavReader;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::{debug, error, info, warn};

use crate::events::{EventName, RecordingStateEvent, TranscriptionCompleteEvent};
use crate::state::app_state::AppState;
use crate::utils::AppPaths;

// Audio activity thresholds (0-100 normalized scale)
// ON: ~-36 dB, above typical office noise
// OFF: ~-45 dB, provides hysteresis
const AUDIO_ACTIVITY_ON_THRESHOLD: u32 = 40;
const AUDIO_ACTIVITY_OFF_THRESHOLD: u32 = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingState {
    pub is_recording: bool,
    pub is_transcribing: bool,
    pub audio_level: u32,
    pub output_path: Option<String>,
}

#[tauri::command]
pub async fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    start_recording_sync(app)?;
    let path = state.output_path.lock().clone().unwrap_or_default();
    Ok(path)
}

pub fn start_recording_sync(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();

    if state.is_recording.load(Ordering::SeqCst) {
        return Err("Already recording".to_string());
    }

    // Show pill immediately on hotkey press — before beep and recorder init.
    {
        let settings = state.settings.lock();
        let preset = settings.pill_position.clone();
        drop(settings);
        crate::commands::window::position_pill_window(&app, &preset);
    }
    state.is_recording.store(true, Ordering::SeqCst);
    state.is_transcribing.store(false, Ordering::SeqCst);
    crate::commands::window::update_pill_visibility(&app);

    // Play start beep if enabled (non-blocking, plays in background)
    {
        let settings = state.settings.lock();
        let beep_enabled = settings.beep_on_record;
        drop(settings);

        info!(beep_enabled = beep_enabled, "checking if start beep should play");
        if beep_enabled {
            info!("calling play_start_beep");
            crate::audio::beep::play_start_beep();
        }
    }

    let app_dir = AppPaths::recordings_dir();
    std::fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;

    let filename = format!("recording_{}.wav", uuid::Uuid::new_v4());
    let output_path = app_dir.join(&filename);

    let audio_device = {
        let settings = state.settings.lock();
        settings.audio_device.clone()
    };

    {
        let recorder = state.recorder.lock();
        let device_name = if audio_device == "default" { None } else { Some(audio_device) };
        recorder.start(output_path.clone(), device_name).map_err(|e| {
            error!(error = %e, "failed to start recorder");
            // Roll back the optimistic pill show
            state.is_recording.store(false, Ordering::SeqCst);
            crate::commands::window::update_pill_visibility(&app);
            e.to_string()
        })?;
    }

    *state.output_path.lock() = Some(output_path.to_string_lossy().to_string());

    // Record start time so the audio-level monitor can suppress the start-beep pickup
    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    state.recording_start_time.store(start_ms, Ordering::SeqCst);

    // Tell the level monitor thread to open the mic stream
    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(true);
    }

    // Assign a new task ID for this recording session
    let task_id = state.task_counter.fetch_add(1, Ordering::SeqCst) + 1;

    info!(task_id, "recording started");
    let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "recording".to_string(), task_id });

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle, _state: State<'_, AppState>) -> Result<Option<String>, String> {
    let output_path = stop_recording_sync(app.clone())?;
    Ok(output_path)
}

pub fn stop_recording_sync(app: AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<AppState>();

    if !state.is_recording.load(Ordering::SeqCst) {
        return Ok(None);
    }

    {
        let recorder = state.recorder.lock();
        recorder.stop().map_err(|e| e.to_string())?;
    }

    let output_path = state.output_path.lock().take();
    // task_counter always holds the current session's ID (only one recording active at a time)
    let task_id = state.task_counter.load(Ordering::SeqCst);

    // Allow immediate new recording
    state.is_recording.store(false, Ordering::SeqCst);

    // Tell the level monitor thread to close the mic stream
    if let Some(tx) = state.level_monitor_tx.lock().as_ref() {
        let _ = tx.send(false);
    }

    // Emit transcribing so the pill stays visible while transcription is pending.
    // The final "idle" is emitted by the transcription pipeline when it finishes.
    let status_after_stop = if output_path.is_some() { "transcribing" } else { "idle" };
    let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: status_after_stop.to_string(), task_id });

    // Play stop beep if enabled (after stopping recording)
    {
        let settings = state.settings.lock();
        let beep_enabled = settings.beep_on_record;
        drop(settings);

        info!(beep_enabled = beep_enabled, "checking if stop beep should play");
        if beep_enabled {
            info!("calling play_stop_beep");
            crate::audio::beep::play_stop_beep();
        }
    }

    // Add to transcription queue instead of immediate processing
    if let Some(ref path) = output_path {
        use crate::state::unified_state::TranscriptionJob;

        let job = TranscriptionJob {
            audio_path: path.clone(),
            timestamp: std::time::SystemTime::now(),
            task_id,
        };

        state.transcription_queue.lock().push_back(job);
        info!(task_id, "transcription job queued");

        // Trigger queue processor
        process_transcription_queue(app.clone());
    }

    Ok(output_path)
}

/// Process transcription queue in FIFO order
fn process_transcription_queue(app: AppHandle) {
    let state = app.state::<AppState>();

    // Check if already processing
    let current_count = state.processing_count.load(Ordering::SeqCst);
    if current_count > 0 {
        debug!(jobs = current_count, "queue processor already running");
        return;
    }

    // Get next job from queue
    let job = {
        let mut queue = state.transcription_queue.lock();
        queue.pop_front()
    };

    if let Some(job) = job {
        state.processing_count.fetch_add(1, Ordering::SeqCst);
        state.is_transcribing.store(true, Ordering::SeqCst);

        // Emit processing state; frontend will filter by task_id
        info!(task_id = job.task_id, "transcription processing started");
        let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "processing".to_string(), task_id: job.task_id });

        let app_clone = app.clone();
        let job_task_id = job.task_id;
        tauri::async_runtime::spawn(async move {
            run_transcription(app_clone.clone(), job.audio_path, job_task_id).await;

            // Decrement counter
            let state = app_clone.state::<AppState>();
            state.processing_count.fetch_sub(1, Ordering::SeqCst);
            state.is_transcribing.store(false, Ordering::SeqCst);

            // Process next job in queue
            process_transcription_queue(app_clone);
        });
    } else {
        debug!("transcription queue empty");
    }
}

async fn run_transcription(app: AppHandle, audio_path: String, task_id: u64) {
    let state = app.state::<AppState>();

    // Log wav file information for debugging
    log_wav_file_info(&audio_path, task_id);

    let (model_name, language, initial_prompt, domain, subdomain, glossary, _denoise_mode) = {
        let settings = state.settings.lock();
        debug!(task_id, model = %settings.model, language = %settings.stt_engine_language, "transcription settings");
        debug!(task_id, domain = %settings.stt_engine_work_domain, subdomain = %settings.stt_engine_work_subdomain, glossary_len = settings.stt_engine_user_glossary.len(), "domain glossary settings");
        (settings.model.clone(), settings.stt_engine_language.clone(), settings.stt_engine_initial_prompt.clone(), settings.stt_engine_work_domain.clone(), settings.stt_engine_work_subdomain.clone(), settings.stt_engine_user_glossary.clone(), settings.denoise_mode.clone())
    };

    // Auto-detect engine type from model name (more reliable than settings.stt_engine)
    let engine_type = match crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name) {
        Some(et) => et,
        None => {
            error!(task_id, model = %model_name, "unknown model, cannot determine engine");
            let _ = app.emit(EventName::TRANSCRIPTION_ERROR, &format!("Unknown model: {}", model_name));
            let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "error".to_string(), task_id });
            state.is_transcribing.store(false, Ordering::SeqCst);
            return;
        }
    };

    debug!(task_id, engine = ?engine_type, model = %model_name, "detected engine from model name");

    // Check if model is downloaded using engine manager
    if !state.engine_manager.is_model_downloaded(engine_type, &model_name) {
        let msg = format!(
            "Model '{}' not downloaded. Please download it in Settings > Model.",
            model_name
        );
        warn!(task_id, model = %model_name, engine = ?engine_type, "model not downloaded");
        let _ = app.emit(EventName::TRANSCRIPTION_ERROR, &msg);
        let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "error".to_string(), task_id });

        let app_error = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let _ = app_error.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "idle".to_string(), task_id });
        });
        state.is_transcribing.store(false, Ordering::SeqCst);
        return;
    }

    // Build transcription request
    let prompt = build_stt_engine_prompt(&initial_prompt, &language, &domain, &subdomain, &glossary);
    let mut request = crate::stt_engine::TranscriptionRequest::new(audio_path.clone())
        .with_model(model_name.clone())
        .with_language(language.clone());
    if let Some(p) = prompt {
        request = request.with_prompt(p);
    }

    // Execute transcription using engine manager
    let result = state.engine_manager.transcribe(engine_type, request).await;

    let (text, mut metrics) = match result {
        Ok(result) => (result.text, crate::events::TranscriptionMetrics {
            load_time_ms: result.model_load_ms.unwrap_or(0),
            preprocess_time_ms: result.preprocess_ms.unwrap_or(0),
            inference_time_ms: result.inference_ms.unwrap_or(0),
            polish_time_ms: 0,
            total_time_ms: result.total_ms,
        }),
        Err(e) => {
            error!(task_id, error = %e, "transcription failed");
            let _ = app.emit(EventName::TRANSCRIPTION_ERROR, &e);
            let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "error".to_string(), task_id });
            state.is_transcribing.store(false, Ordering::SeqCst);
            let _ = std::fs::remove_file(&audio_path);
            return;
        }
    };

    state.is_transcribing.store(false, Ordering::SeqCst);
    let _ = std::fs::remove_file(&audio_path);

    if !text.is_empty() {
        info!(task_id, chars = text.len(), "transcription complete");

        let polish_enabled = {
            let settings = state.settings.lock();
            settings.polish_enabled
        };

        let _polish_start_time = std::time::Instant::now();

        let final_text = if polish_enabled {
            let (polish_system_prompt, polish_language, polish_model_id, cloud_polish_config) = {
                let settings = state.settings.lock();
                let prompt = settings.polish_system_prompt.clone();
                (
                    if prompt.is_empty() { crate::polish_engine::DEFAULT_POLISH_PROMPT.to_string() } else { prompt },
                    settings.stt_engine_language.clone(),
                    settings.polish_model.clone(),
                    settings.cloud_polish.clone(),
                )
            };

            // Check if cloud polish is enabled
            if cloud_polish_config.enabled && !cloud_polish_config.api_key.is_empty() && !cloud_polish_config.model.is_empty() {
                debug!(task_id, provider = %cloud_polish_config.provider_type, model = %cloud_polish_config.model, "running cloud text polish");

                let request = crate::polish_engine::PolishRequest::new(
                    text.clone(),
                    polish_system_prompt,
                    polish_language,
                );

                match state.polish_manager.polish_cloud(
                    request,
                    &cloud_polish_config.provider_type,
                    &cloud_polish_config.api_key,
                    &cloud_polish_config.base_url,
                    &cloud_polish_config.model,
                    cloud_polish_config.enable_thinking,
                ).await {
                    Ok(result) if !result.text.is_empty() => {
                        info!(task_id, polish_ms = result.total_ms, "cloud polish complete");
                        metrics.polish_time_ms = result.total_ms;
                        metrics.total_time_ms += metrics.polish_time_ms;
                        result.text
                    }
                    Ok(_) => {
                        warn!(task_id, provider = %cloud_polish_config.provider_type, "cloud polish returned empty result, using raw transcription");
                        text
                    }
                    Err(e) => {
                        warn!(task_id, provider = %cloud_polish_config.provider_type, error = %e, "cloud polish failed, using raw transcription");
                        text
                    }
                }
            } else {
                // Use local polish engine
                match crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id) {
                    Some(engine_type) => {
                        let model_filename = state.polish_manager.get_model_filename(
                            engine_type,
                            &polish_model_id
                        );

                        if model_filename.is_some() && state.polish_manager.is_model_downloaded(
                            engine_type,
                            &polish_model_id
                        ) {
                            debug!(task_id, engine = ?engine_type, model_id = %polish_model_id, "running text polish");

                            let request = crate::polish_engine::PolishRequest::new(
                                text.clone(),
                                polish_system_prompt,
                                polish_language,
                            ).with_model(model_filename.unwrap());

                            match state.polish_manager.polish(
                                engine_type,
                                request
                            ).await {
                                Ok(result) if !result.text.is_empty() => {
                                    debug!(task_id, chars = result.text.len(), "polish complete");
                                    metrics.polish_time_ms = result.total_ms;
                                    metrics.total_time_ms += metrics.polish_time_ms;
                                    result.text
                                }
                                Ok(_) => {
                                    warn!(task_id, "polish returned empty result, using raw transcription");
                                    text
                                }
                                Err(e) => {
                                    warn!(task_id, error = %e, "polish failed, using raw transcription");
                                    text
                                }
                            }
                        } else {
                            warn!(task_id, "polish model not downloaded, using raw transcription");
                            text
                        }
                    }
                    None => {
                        warn!(task_id, model_id = %polish_model_id, "unknown polish model, cannot determine engine");
                        text
                    }
                }
            }
        } else {
            text
        };

        metrics.total_time_ms += metrics.polish_time_ms;
        let _ = app.emit(EventName::TRANSCRIPTION_METRICS, &metrics);
        info!(task_id, load_ms = metrics.load_time_ms, preprocess_ms = metrics.preprocess_time_ms, inference_ms = metrics.inference_time_ms, polish_ms = metrics.polish_time_ms, total_ms = metrics.total_time_ms, "transcription metrics");

        let _ = app.emit(EventName::TRANSCRIPTION_COMPLETE, TranscriptionCompleteEvent { text: final_text.clone(), task_id });

        let has_more = !state.transcription_queue.lock().is_empty();
        if has_more {
            debug!(task_id, "more jobs in queue");
            let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "processing".to_string(), task_id });
        } else {
            debug!(task_id, "queue empty, returning to idle");
            let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: "idle".to_string(), task_id });
        }

        let app_clone = app.clone();
        let text_clone = final_text.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            crate::commands::text::do_insert_text(app_clone, text_clone).await;
        });
    } else {
        let _ = app.emit(EventName::TRANSCRIPTION_METRICS, &metrics);
        info!(task_id, load_ms = metrics.load_time_ms, preprocess_ms = metrics.preprocess_time_ms, inference_ms = metrics.inference_time_ms, total_ms = metrics.total_time_ms, "transcription metrics");
        
        warn!(task_id, "transcription returned empty result");
        let has_more = !state.transcription_queue.lock().is_empty();
        let next = if has_more { "processing" } else { "idle" };
        let _ = app.emit(EventName::RECORDING_STATE_CHANGED, RecordingStateEvent { status: next.to_string(), task_id });
    }
}

#[tauri::command]
pub fn get_audio_level(state: State<'_, AppState>) -> u32 {
    state.audio_level.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn get_recording_state(state: State<'_, AppState>) -> RecordingState {
    RecordingState {
        is_recording: state.is_recording.load(Ordering::SeqCst),
        is_transcribing: state.is_transcribing.load(Ordering::SeqCst),
        audio_level: state.audio_level.load(Ordering::SeqCst),
        output_path: state.output_path.lock().clone(),
    }
}

pub fn start_audio_level_monitor(app: AppHandle) -> Result<(), String> {
    info!("Audio level monitor loop started");

    // cpal::Stream is !Send on macOS — it must live on this thread.
    // Commands (open/close) arrive via a channel sent from start/stop_recording_sync.
    let state = app.state::<AppState>();

    // Take the receiver out of AppState — it belongs to this thread from now on.
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

        // Detect new recording session and reset last_activity so the
        // diff-guard doesn't suppress the first audio-activity event.
        let current_start_ms = state.recording_start_time.load(Ordering::SeqCst);
        if current_start_ms != last_seen_start_ms {
            last_seen_start_ms = current_start_ms;
            last_activity = false;
        }

        // Drain all pending commands; only the last one matters.
        let mut cmd: Option<bool> = None;
        while let Ok(v) = rx.try_recv() {
            cmd = Some(v);
        }

        if let Some(should_open) = cmd {
            if should_open && stream.is_none() {
                // Open the mic stream
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
                        .and_then(|mut devs| devs.find(|d| d.name().ok().as_deref() == Some(&audio_device)))
                        .or_else(|| host.default_input_device())
                };

                if let Some(device) = device {
                    match device.default_input_config() {
                        Ok(config) => {
                            let level_clone = audio_level.clone();
                            let err_fn = |err| error!(error = %err, "audio stream error");
                            match device.build_input_stream(
                                &config.into(),
                                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                    let sum: f32 = data.iter().map(|&s| s * s).sum::<f32>() / data.len() as f32;
                                    let rms = sum.sqrt();
                                    let db = 20.0 * rms.log10();
                                    let normalized = ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0) as u32;
                                    level_clone.store(normalized, Ordering::SeqCst);
                                },
                                err_fn,
                                None,
                            ) {
                                Ok(s) => match s.play() {
                                    Ok(()) => {
                                        info!("Audio level monitor stream opened");
                                        stream = Some(s);
                                    }
                                    Err(e) => error!(error = %e, "failed to play audio level stream"),
                                },
                                Err(e) => error!(error = %e, "failed to build audio level stream"),
                            }
                        }
                        Err(e) => error!(error = %e, "failed to get input config for level monitor"),
                    }
                } else {
                    warn!("No input device found for audio level monitor");
                }
            } else if !should_open && stream.is_some() {
                // Close the mic stream
                drop(stream.take());
                audio_level.store(0, Ordering::SeqCst);
                info!("Audio level monitor stream closed");
            }
        }

        let level = if stream.is_some() {
            audio_level.load(Ordering::SeqCst)
        } else {
            0
        };

        state.audio_level.store(level, Ordering::SeqCst);
        let _ = app.emit(EventName::AUDIO_LEVEL, level);

        // Emit audio activity with hysteresis to avoid rapid flickering.
        let has_activity = if last_activity {
            level >= AUDIO_ACTIVITY_OFF_THRESHOLD
        } else {
            level > AUDIO_ACTIVITY_ON_THRESHOLD
        };
        // Suppress `true` for the first 400ms after recording starts so the
        // start-beep picked up by the mic doesn't flash the indicator green.
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

fn build_stt_engine_prompt(initial_prompt: &str, language: &str, domain: &str, subdomain: &str, glossary: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    
    // Language-specific base prompt (most important for output language)
    let lang_code = language.split('-').next().unwrap_or(language);
    let base_prompt = get_language_base_prompt(lang_code);
    if !base_prompt.is_empty() {
        parts.push(base_prompt.to_string());
    }
    
    // User's custom initial_prompt (if provided, takes precedence)
    if !initial_prompt.is_empty() {
        parts.push(initial_prompt.to_string());
    }
    
    // Domain-specific prompt in target language
    if domain != "general" {
        let domain_prompt = get_domain_prompt(lang_code, domain, subdomain);
        if !domain_prompt.is_empty() {
            parts.push(domain_prompt);
        }
    }
    
    // Glossary in target language format
    if !glossary.is_empty() {
        let glossary_formatted = format_glossary(lang_code, glossary);
        parts.push(glossary_formatted);
    }
    
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn get_language_base_prompt(lang: &str) -> &'static str {
    match lang {
        "zh" => "以下是普通话的句子，请用简体中文输出。",
        "en" => "The following is an English sentence.",
        "ja" => "以下は日本語の文章です。",
        "ko" => "다음은 한국어 문장입니다.",
        "fr" => "Voici une phrase en français.",
        "de" => "Das ist ein deutscher Satz.",
        "es" => "Esta es una oración en español.",
        "ru" => "Это предложение на русском языке.",
        "pt" => "Esta é uma frase em português.",
        "it" => "Questa è una frase in italiano.",
        _ => "",
    }
}

fn get_domain_prompt(lang: &str, domain: &str, subdomain: &str) -> String {
    if domain == "general" || domain.is_empty() {
        return String::new();
    }

    match lang {
        "zh" => {
            if subdomain == "general" || subdomain.is_empty() {
                format!("请优先识别{}领域的专业术语和技术词汇。", get_domain_name_zh(domain))
            } else {
                format!("请优先识别{}领域中{}相关的专业术语。", get_domain_name_zh(domain), subdomain)
            }
        }
        "en" | _ => {
            if subdomain == "general" || subdomain.is_empty() {
                format!("Prioritize recognition of {} terminology and technical terms.", get_domain_name_en(domain))
            } else {
                format!("Prioritize recognition of {} terminology, focusing on {}.", get_domain_name_en(domain), subdomain)
            }
        }
    }
}

fn get_domain_name_zh(domain: &str) -> String {
    match domain {
        "it" => "IT技术".to_string(),
        "legal" => "法律".to_string(),
        "medical" => "医学".to_string(),
        "finance" => "金融".to_string(),
        "education" => "教育".to_string(),
        _ => domain.to_string(),
    }
}

fn get_domain_name_en(domain: &str) -> String {
    match domain {
        "it" => "IT".to_string(),
        "legal" => "legal".to_string(),
        "medical" => "medical".to_string(),
        "finance" => "finance".to_string(),
        "education" => "education".to_string(),
        _ => domain.to_string(),
    }
}

fn format_glossary(lang: &str, glossary: &str) -> String {
    match lang {
        "zh" => format!("专业词汇：{}", glossary),
        "ja" => format!("専門用語：{}", glossary),
        "ko" => format!("전문 용어：{}", glossary),
        "en" | _ => format!("Terminology: {}", glossary),
    }
}

fn log_wav_file_info(audio_path: &str, task_id: u64) {
    let path = PathBuf::from(audio_path);
    
    let file_size = match std::fs::metadata(&path) {
        Ok(meta) => meta.len(),
        Err(e) => {
            warn!(task_id, error = %e, path = %audio_path, "failed to get file size");
            return;
        }
    };

    match WavReader::open(&path) {
        Ok(reader) => {
            let spec = reader.spec();
            let duration_secs = reader.duration() as f64 / spec.sample_rate as f64;
            
            info!(
                task_id,
                path = %audio_path,
                file_size_bytes = file_size,
                sample_rate = spec.sample_rate,
                channels = spec.channels,
                bits_per_sample = spec.bits_per_sample,
                duration_secs = duration_secs,
                "wav file info"
            );
        }
        Err(e) => {
            warn!(task_id, error = %e, path = %audio_path, "failed to read wav file");
        }
    }
}
