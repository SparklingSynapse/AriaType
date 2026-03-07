// Suppress warnings from third-party macros (objc, cocoa) that we cannot control.
#![allow(unexpected_cfgs)]
#![allow(deprecated)]
use tracing::{error, info, warn};
use tauri::{Emitter, Manager};
use tauri_plugin_aptabase::EventTracker;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod audio;
pub mod commands;
pub mod events;
pub mod state;
pub mod text_injector;
pub mod stt_engine;
pub mod polish_engine;
pub mod utils;

use commands::audio::{
    start_audio_level_monitor, start_recording, stop_recording,
    get_audio_level, get_recording_state,
};
use commands::{model, model_cache, permissions, settings, system, text, window};
use events::EventName;
use state::app_state::AppState;
use stt_engine::EngineType;

fn cleanup_old_logs(log_dir: &std::path::Path, keep_days: u64) {
    let cutoff = std::time::SystemTime::now()
        - std::time::Duration::from_secs(keep_days * 24 * 3600);
    let Ok(entries) = std::fs::read_dir(log_dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("ariatype.log") { continue; }
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if modified < cutoff {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
}

fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = crate::utils::AppPaths::log_dir();
    
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("Failed to create log directory: {:?}", e);
    }

    // Clean up log files older than 7 days
    cleanup_old_logs(&log_dir, 7);

    let file_appender = tracing_appender::rolling::hourly(&log_dir, "ariatype.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_ansi(false)
            .with_target(false)
            .with_thread_ids(false)
            .with_line_number(true))
        .init();

    info!(log_dir = ?log_dir, "logging initialized");
    guard
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _log_guard = init_logging();

    info!("Starting AriaType application");

    // Initialize the global beep player for low-latency audio feedback
    crate::audio::beep::init_beep_player();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_aptabase::Builder::new("A-US-3957940978").build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init());

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            window::show_main_window,
            window::hide_main_window,
            window::show_pill_window,
            window::hide_pill_window,
            window::show_toast,
            window::hide_toast,
            window::update_pill_position,
            window::get_pill_position,
            start_recording,
            stop_recording,
            get_audio_level,
            get_recording_state,
            text::insert_text,
            text::copy_to_clipboard,
            text::restore_clipboard,
            settings::get_settings,
            settings::update_settings,
            settings::set_hotkey_capture_mode,
            settings::get_glossary_content,
            settings::get_available_subdomains,
            system::get_audio_devices,
            system::get_log_content,
            system::open_log_folder,
            permissions::check_permission,
            permissions::apply_permission,
            model::get_models,
            model::get_models_for_engine,
            model::is_model_downloaded,
            model::is_model_downloaded_for_engine,
            model::recommend_models_by_language,
            model::download_model,
            model::delete_model,
            model::cancel_download,
            model::get_polish_models,
            model::get_current_polish_model,
            model::is_polish_model_downloaded,
            model::is_polish_model_downloaded_for_model,
            model::download_polish_model,
            model::download_polish_model_by_id,
            model::cancel_polish_download,
            model::delete_polish_model,
            model::delete_polish_model_by_id,
            model::get_polish_templates,
            model::get_polish_template_prompt,
            model_cache::get_model_status,
            model_cache::preload_model,
            model_cache::unload_model,
            model_cache::get_polish_model_status,
            model_cache::preload_polish_model,
            model_cache::unload_polish_model,
        ])
        .setup(|app| {
            info!("Application setup complete");
            let analytics_opt_in = {
                let state = app.state::<AppState>();
                let settings = state.settings.lock();
                settings.analytics_opt_in
            };
            if analytics_opt_in {
                let _ = app.track_event("desktop_app_started", None);
            }

            // Intercept the main window's close button: hide instead of destroy.
            // Without this, clicking the red × on macOS destroys the WebviewWindow,
            // making it impossible to reopen via show_main_window.
            if let Some(main_win) = app.get_webview_window("main") {
                let win = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }

            let app_audio = app.handle().clone();
            std::thread::spawn(move || {
                if let Err(e) = start_audio_level_monitor(app_audio) {
                    error!(error = %e, "failed to start audio level monitor");
                }
            });

            let app_idle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    
                    let state = match app_idle.try_state::<AppState>() {
                        Some(s) => s,
                        None => continue,
                    };

                    let settings = state.settings.lock();
                    let model_resident = settings.model_resident;
                    let idle_minutes = settings.idle_unload_minutes;
                    drop(settings);

                    if !model_resident {
                        continue;
                    }

                    if idle_minutes == 0 {
                        continue;
                    }

                    // Model idle unloading is now handled by UnifiedEngineManager's engine cache
                    // TODO: Implement idle unload in UnifiedEngineManager if needed
                }
            });

            // Auto-download base model on first launch if no model is downloaded
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1000));

                let state = match app_handle.try_state::<AppState>() {
                    Some(s) => s,
                    None => return,
                };

                // Check for base model (default)
                if !state.engine_manager.is_model_downloaded(EngineType::Whisper, "base") {
                    info!("No model downloaded, auto-downloading base model...");
                    let app_clone = app_handle.clone();
                    let runtime = tokio::runtime::Runtime::new().unwrap();
                    runtime.block_on(async {
                        let result = state.engine_manager.download_model(
                            EngineType::Whisper,
                            "base",
                            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
                            |_, _| {}
                        ).await;
                        match result {
                            Ok(_) => {
                                info!(model = "base", "auto-download complete");
                                let _ = app_clone.emit(
                                    EventName::MODEL_DOWNLOAD_COMPLETE,
                                    serde_json::json!({ "model": "base" }),
                                );
                            }
                            Err(e) => {
                                error!(model = "base", error = %e, "auto-download failed");
                            }
                        }
                    });
                }
            });

            let app_warmup = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(2000));

                let state = match app_warmup.try_state::<AppState>() {
                    Some(s) => s,
                    None => return,
                };

                let settings = state.settings.lock();
                let model_resident = settings.model_resident;
                let model_name = settings.model.clone();
                let polish_enabled = settings.polish_enabled;
                let polish_model_id = settings.polish_model.clone();
                drop(settings);

                if model_resident {
                    // Preload STT model
                    if let Some(engine_type) = crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name) {
                        match state.engine_manager.load_model(engine_type, &model_name) {
                            Ok(_) => {
                                info!(
                                    engine = ?engine_type,
                                    model = %model_name,
                                    "startup warmup: STT model preloaded successfully"
                                );
                            }
                            Err(e) => {
                                warn!(
                                    engine = ?engine_type,
                                    model = %model_name,
                                    error = %e,
                                    "startup warmup: failed to preload STT model"
                                );
                            }
                        }
                    } else {
                        warn!(model = %model_name, "startup warmup: unknown STT model, cannot determine engine");
                    }

                    // Preload polish model if enabled
                    if polish_enabled && !polish_model_id.is_empty() {
                        if let Some(engine_type) = crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id) {
                            match state.polish_manager.load_model(engine_type, &polish_model_id) {
                                Ok(_) => {
                                    info!(
                                        engine = ?engine_type,
                                        model_id = %polish_model_id,
                                        "startup warmup: polish model preloaded successfully"
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        engine = ?engine_type,
                                        model_id = %polish_model_id,
                                        error = %e,
                                        "startup warmup: failed to preload polish model"
                                    );
                                }
                            }
                        } else {
                            warn!(model_id = %polish_model_id, "startup warmup: unknown polish model, cannot determine engine");
                        }
                    }
                }
            });

            // Create pill window programmatically so we can apply NSPanel on macOS
            let _pill_window = tauri::WebviewWindowBuilder::new(
                app,
                "pill",
                tauri::WebviewUrl::App("pill.html".into()),
            )
            .title("NoType Pill")
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible_on_all_workspaces(true)
            .inner_size(100.0, 52.0)
            .focused(false)
            .visible(false) // Start hidden; show after panel/collection-behavior setup
            .build()
            .expect("Failed to create pill window");

            // On macOS, convert to NSPanel — this is what actually makes the WKWebView
            // background transparent. `transparent: true` alone is not enough.
            // Also re-apply collection behavior after panel conversion so the pill
            // appears on all Spaces and in full-screen mode.
            #[cfg(target_os = "macos")]
            {
                use tauri_nspanel::WebviewWindowExt;
                match _pill_window.to_panel() {
                    Ok(panel) => {
                        use cocoa::appkit::NSWindowCollectionBehavior;
                        use objc::{msg_send, sel, sel_impl};

                        // Set as non-activating panel to avoid stealing focus from other apps.
                        // NSNonactivatingPanelMask = 1 << 7 = 128.
                        // RawNSPanel::set_style_mask takes i32; read current mask via msg_send first.
                        let current_mask: i32 = unsafe { msg_send![&*panel, styleMask] };
                        panel.set_style_mask(current_mask | 128);

                        // CanJoinAllSpaces: appear on every Space (including other apps' full-screen Spaces)
                        // FullScreenAuxiliary: appear alongside native full-screen apps (e.g. VSCode)
                        // NOTE: Transient is intentionally omitted. When combined with CanJoinAllSpaces,
                        // Transient causes the system to treat the window as Stationary (pinned to current
                        // Space), silently overriding CanJoinAllSpaces. The pill then disappears when the
                        // user switches to another Space.
                        let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary;
                        panel.set_collection_behaviour(behavior);

                        // Don't hide when the app loses focus — pill must always be visible
                        panel.set_hides_on_deactivate(false);

                        // Enable floating panel mode: ensures the panel floats above all other windows
                        // including full-screen apps. This is critical for CanJoinAllSpaces to work
                        // correctly in full-screen Spaces.
                        panel.set_floating_panel(true);

                        // Allow the panel to work even when modal dialogs are active
                        panel.set_works_when_modal(true);

                        // NSScreenSaverWindowLevel (1000): high enough to appear above full-screen app
                        // content. Combined with set_floating_panel(true) and NSNonactivatingPanelMask,
                        // the pill remains visible in all contexts without stealing focus.
                        panel.set_level(1000);
                        info!("Pill window converted to NSPanel with full-screen support");
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to convert pill window to NSPanel");
                    }
                }
            }

            // Position and show/hide the pill based on settings
            {
                let state = app.state::<AppState>();
                let settings = state.settings.lock();
                let preset = settings.pill_position.clone();
                drop(settings);

                // Position the pill
                commands::window::position_pill_window(app.handle(), &preset);

                // Update visibility based on indicator_mode and recording state
                commands::window::update_pill_visibility(app.handle());
            }

            // Trigger microphone permission request on first launch so the app
            // appears in System Settings > Privacy > Microphone.
            // Must be called in-process (not via a subprocess) so that macOS
            // registers the permission under com.ariatype.voicetotext, not swift.
            std::thread::spawn(move || {
                // Small delay to let the app fully initialize
                std::thread::sleep(std::time::Duration::from_millis(500));

                info!("Requesting microphone permission...");
                #[cfg(target_os = "macos")]
                {
                    use objc::{class, msg_send, sel, sel_impl};
                    use std::os::raw::c_char;
                    #[link(name = "AVFoundation", kind = "framework")]
                    extern "C" {}
                    // Check current status first; only request if not yet determined
                    let status: i64 = unsafe {
                        let media_type: *mut objc::runtime::Object = msg_send![
                            class!(NSString),
                            stringWithUTF8String: b"soun\0".as_ptr() as *const c_char
                        ];
                        msg_send![class!(AVCaptureDevice), authorizationStatusForMediaType: media_type]
                    };
                    if status == 0 {
                        // not_determined — delegate to the existing in-process implementation
                        let runtime = tokio::runtime::Runtime::new().unwrap();
                        let result = runtime.block_on(commands::permissions::apply_permission("microphone".to_string()));
                        info!("Microphone permission result: {:?}", result);
                    } else {
                        info!("Microphone permission already determined (status={})", status);
                    }
                }
            });

            // Call AXIsProcessTrusted() on startup so the app appears in the
            // Accessibility list in System Settings without prompting the user.
            #[cfg(target_os = "macos")]
            {
                let status = commands::permissions::check_permission("accessibility".to_string());
                info!("Accessibility permission status on startup: {}", status);
            }

            // Register global shortcut from settings
            let hotkey = {
                let state = app.state::<AppState>();
                let hotkey = state.settings.lock().hotkey.clone();
                hotkey
            };
            match commands::settings::register_global_shortcut(app.handle(), &hotkey) {
                Ok(_) => info!(hotkey = %hotkey, "global shortcut registered"),
                Err(e) => {
                    warn!(error = %e, "failed to register global shortcut; grant Accessibility permission");
                    let _ = app.emit(EventName::SHORTCUT_REGISTRATION_FAILED, e.to_string());
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
