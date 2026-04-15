// Suppress warnings from third-party macros (objc, cocoa) that we cannot control.
#![allow(unexpected_cfgs)]
#![allow(deprecated)]
use tauri::{Emitter, Manager};
use tauri_plugin_aptabase::EventTracker;
use std::fmt;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{
    fmt::{
        format::{Compact, FormatEvent, FormatFields, Writer},
        FmtContext,
    },
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
    EnvFilter,
};

pub mod audio;
pub mod commands;
pub mod events;
pub mod history;
pub mod permissions;
pub mod polish_engine;
pub mod provider_schema;
pub mod services;
pub mod shortcut;
pub mod state;
pub mod stt_engine;
pub mod text_injector;
pub mod tray;
pub mod utils;

use commands::audio::{
    cancel_recording, get_audio_level, get_recording_state, start_audio_level_monitor,
    start_recording, stop_recording,
};
use commands::{hotkey, model, model_cache, settings, system, text, window};
use events::EventName;
use state::app_state::AppState;

fn cleanup_old_logs(log_dir: &std::path::Path, keep_days: u64) {
    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(keep_days * 24 * 3600);
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("ariatype.log") {
            continue;
        }
        if let Ok(meta) = std::fs::metadata(&path) {
            if let Ok(modified) = meta.modified() {
                if modified < cutoff {
                    if let Err(e) = std::fs::remove_file(&path) {
                        tracing::debug!(error = %e, path = ?path, "Failed to remove old log file during cleanup");
                    }
                }
            }
        }
    }
}

struct EnvPrefixFormat<'a> {
    prefix: &'a str,
    inner: tracing_subscriber::fmt::format::Format<Compact>,
}

impl<'a, S, N> FormatEvent<S, N> for EnvPrefixFormat<'a>
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        write!(writer, "[{}] ", self.prefix)?;
        self.inner.format_event(ctx, writer, event)
    }
}

fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = crate::utils::AppPaths::log_dir();

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("failed to create log directory {:?}: {}", log_dir, e);
    }

    cleanup_old_logs(&log_dir, 7);

    let file_appender = tracing_appender::rolling::hourly(&log_dir, "ariatype.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    #[cfg(debug_assertions)]
    let env_prefix = "DEV";
    #[cfg(not(debug_assertions))]
    let env_prefix = "PROD";

    let base_fmt = tracing_subscriber::fmt::format::Format::default().compact();
    let stderr_format = EnvPrefixFormat {
        prefix: env_prefix,
        inner: base_fmt.clone(),
    };
    let file_format = EnvPrefixFormat {
        prefix: env_prefix,
        inner: base_fmt,
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .event_format(stderr_format),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .event_format(file_format),
        )
        .init();

    tracing::info!(log_dir = ?log_dir, env = env_prefix, "logging_initialized");
    guard
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Install panic hook BEFORE any other initialization
    // This ensures panics are logged even if logging isn't fully initialized
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown_location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic payload".to_string()
        };

        // Capture backtrace for debugging
        let backtrace = std::backtrace::Backtrace::capture();
        let backtrace_str = match backtrace.status() {
            std::backtrace::BacktraceStatus::Captured => format!("\nBacktrace:\n{}", backtrace),
            _ => String::new(),
        };

        // Log to tracing (may work if logging is initialized)
        tracing::error!(
            location = %location,
            message = %message,
            "application_panic{}", backtrace_str
        );

        // Also write to stderr as a fallback
        eprintln!("PANIC at {}: {}{}", location, message, backtrace_str);
    }));

    let _log_guard = init_logging();

    info!("app_started");

    // Ensure all application directories exist (models, recordings, cache, etc.)
    crate::utils::AppPaths::ensure_dirs();

    // Initialize the global beep player with settings
    let beep_enabled = crate::commands::settings::load_settings_from_disk().beep_on_record;
    crate::audio::beep::init_beep_player();
    crate::audio::beep::initialize_beep_player(beep_enabled);

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_aptabase::Builder::new("A-US-3957940978").build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = app.emit("single-instance", ());
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }));

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            window::show_main_window,
            window::hide_main_window,
            window::show_pill_window,
            window::hide_pill_window,
            window::update_pill_position,
            window::get_pill_position,
            start_recording,
            stop_recording,
            cancel_recording,
            get_audio_level,
            get_recording_state,
            text::insert_text,
            text::copy_to_clipboard,
            text::restore_clipboard,
            settings::get_settings,
            settings::update_settings,
            settings::get_glossary_content,
            settings::get_available_subdomains,
            settings::get_cloud_provider_schemas,
            system::get_audio_devices,
            system::get_log_content,
            system::open_log_folder,
            system::get_platform,
            commands::permissions::check_permission,
            commands::permissions::apply_permission,
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
            model::create_polish_custom_template,
            model::update_polish_custom_template,
            model::delete_polish_custom_template,
            model::select_polish_template,
            model::get_polish_selected_template,
            model::get_polish_custom_templates,
            model_cache::get_model_status,
            model_cache::preload_model,
            model_cache::unload_model,
            model_cache::get_polish_model_status,
            model_cache::preload_polish_model,
            model_cache::unload_polish_model,
            history::get_transcription_history,
            history::get_transcription_entry,
            history::get_dashboard_stats,
            history::get_daily_usage,
            history::get_engine_usage,
            history::delete_transcription_entry,
            history::clear_transcription_history,
            history::retry_transcription,
            hotkey::start_hotkey_recording,
            hotkey::stop_hotkey_recording,
            hotkey::cancel_hotkey_recording,
            hotkey::peek_hotkey_recording,
        ])
        .setup(|app| {
            let _ = crate::permissions::report_startup_permission_snapshot();
            info!("setup_completed");

            #[cfg(target_os = "macos")]
            {
                use tauri::Manager;
                // Use Tauri's PathResolver to locate the resource dynamically
                // This correctly maps to the physical path whether in dev or build mode
                if let Ok(metal_path) = app
                    .path()
                    .resolve("bin/apple-silicon", tauri::path::BaseDirectory::Resource)
                {
                    if metal_path.exists() {
                        std::env::set_var("GGML_METAL_PATH_RESOURCES", &metal_path);
                        tracing::info!(path = ?metal_path, "ggml_metal_path_resources_set");
                    } else {
                        tracing::warn!(path = ?metal_path, "ggml_metal_path_resources_not_found");
                    }
                } else {
                    tracing::warn!("ggml_metal_path_resolve_failed");
                }
            }

            {
                let state = app.state::<AppState>();
                let store = state.history_store.lock();
                if let Err(e) = store.cleanup_old_entries(90) {
                    tracing::warn!(error = %e, "history_cleanup_failed");
                }
            }

            // Auto-ensure default model at startup
            let app_ensure = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(1000));

                let state = match app_ensure.try_state::<AppState>() {
                    Some(s) => s,
                    None => return,
                };

                let language = state.settings.lock().stt_engine_language.clone();
                let engine_manager = state.engine_manager.clone();

                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    if let Err(e) = engine_manager.ensure_default_model(&language).await {
                        tracing::error!(error = %e, language = %language, "startup_model_ensure_failed");
                    }
                });
            });

            let analytics_opt_in = {
                let state = app.state::<AppState>();
                let settings = state.settings.lock();
                settings.analytics_opt_in
            };
            if analytics_opt_in {
                if let Err(e) = app.track_event("desktop_app_started", None) {
                    tracing::debug!(error = %e, "Analytics tracking failed for app startup event");
                }
            }

            // Intercept the main window's close button: hide instead of destroy.
            // Without this, clicking the red × on macOS destroys the WebviewWindow,
            // making it impossible to reopen via show_main_window.
            if let Some(main_win) = app.get_webview_window("main") {
                let win = main_win.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Err(e) = win.hide() {
                            tracing::warn!(error = %e, "Failed to hide main window on close request");
                        }
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
                    if let Some(engine_type) = crate::stt_engine::UnifiedEngineManager::get_engine_by_model_name(&model_name) {
                        if state.engine_manager.is_model_downloaded(engine_type, &model_name) {
                            match state.engine_manager.load_model(engine_type, &model_name) {
                                Ok(_) => {
                                    info!(
                                        engine = ?engine_type,
                                        model = %model_name,
                                        "model_loaded-startup_warmup"
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        engine = ?engine_type,
                                        model = %model_name,
                                        error = %e,
                                        "model_load_failed-startup_warmup"
                                    );
                                }
                            }
                        } else {
                            debug!(
                                model = %model_name,
                                "model_not_downloaded-skipping_warmup"
                            );
                        }
                    } else {
                        warn!(model = %model_name, "model_unknown-cannot_determine_engine");
                    }

                    // Preload polish model if enabled
                    if polish_enabled && !polish_model_id.is_empty() {
                        if let Some(engine_type) = crate::polish_engine::UnifiedPolishManager::get_engine_by_model_id(&polish_model_id) {
                            match state.polish_manager.load_model(engine_type, &polish_model_id) {
                                Ok(_) => {
                                    info!(
                                        engine = ?engine_type,
                                        model_id = %polish_model_id,
                                        "polish_model_loaded-startup_warmup"
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        engine = ?engine_type,
                                        model_id = %polish_model_id,
                                        error = %e,
                                        "polish_model_load_failed-startup_warmup"
                                    );
                                }
                            }
                        } else {
                            warn!(model_id = %polish_model_id, "polish_model_unknown-cannot_determine_engine");
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
            .inner_size(140.0, 80.0)
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
                        info!("pill_window_nspanel_converted");
                    }
                    Err(e) => {
                        warn!(error = %e, "pill_window_nspanel_conversion_failed");
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

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));

                let permission = crate::permissions::PermissionKind::Microphone;
                let status = crate::permissions::check_permission(permission);

                if status == crate::permissions::PermissionStatus::NotDetermined {
                    info!(
                        permission = permission.as_str(),
                        status = status.as_str(),
                        "app_permission_requesting"
                    );
                    if let Err(error) = crate::permissions::apply_permission(permission) {
                        warn!(
                            permission = permission.as_str(),
                            error = %error,
                            "app_permission_request_failed"
                        );
                    }
                    let _ = crate::permissions::report_permission_snapshot_if_changed(
                        "microphone_permission_request",
                    );
                } else {
                    info!(
                        permission = permission.as_str(),
                        status = status.as_str(),
                        "app_permission_request_skipped"
                    );
                }
            });

            // Initialize ShortcutManager and register global shortcut from settings
            let hotkey = {
                let state = app.state::<AppState>();
                let hotkey = state.settings.lock().hotkey.clone();
                hotkey
            };

            // Create and start shortcut manager
            let mut shortcut_manager = crate::shortcut::ShortcutManager::new()
                .expect("shortcut manager creation should succeed");
            match shortcut_manager.start(app.handle().clone()) {
                Ok(_) => {
                    // Register the initial hotkey
                    match shortcut_manager.register(&hotkey) {
                        Ok(_) => info!(hotkey = %hotkey, "shortcut_registered"),
                        Err(e) => {
                            warn!(error = %e, "shortcut_registration_failed");
                            if let Err(emit_err) = app.emit(EventName::SHORTCUT_REGISTRATION_FAILED, e) {
                                tracing::warn!(error = %emit_err, "event_emit_failed-shortcut_registration");
                            }
                        }
                    }
                    // Store manager in app state for later access
                    app.manage(shortcut_manager);
                }
                Err(e) => {
                    warn!(error = %e, "shortcut_manager_start_failed");
                }
            }

            // Create tray only if stay_in_tray is enabled
            let stay_in_tray = app.state::<AppState>().settings.lock().stay_in_tray;
            if stay_in_tray {
                match tray::create_tray(app.handle()) {
                    Ok(_) => info!("tray_created"),
                    Err(e) => warn!(error = %e, "tray_creation_failed"),
                }
            } else {
                info!("tray_creation_skipped-stay_in_tray_disabled");
            }

            #[cfg(target_os = "macos")]
            {
                if let Err(e) = commands::settings::set_activation_policy_for_app(app.handle(), stay_in_tray) {
                    warn!(error = %e, "activation_policy_set_failed");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
