use crate::state::app_state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub(crate) const PILL_WINDOW_W_LOGICAL: f64 = 240.0;
pub(crate) const PILL_WINDOW_H_LOGICAL: f64 = 140.0;
const MARGIN_LOGICAL: f64 = 4.0;

#[derive(Debug, Clone, Serialize)]
pub struct MainWindowSnapshot {
    pub base64: String,
}

fn show_pill_without_focus(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use cocoa::base::id;
        use objc::{msg_send, sel, sel_impl};

        if let Ok(ns_window) = window.ns_window() {
            unsafe {
                let ns_window = ns_window as id;
                let _: () = msg_send![ns_window, orderFront: cocoa::base::nil];
            }
            Ok(())
        } else {
            window.show().map_err(|e| e.to_string())
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        window.show().map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "macos")]
fn activate_app_for_visible_window() {
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let ns_app: id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![ns_app, unhide: nil];
        let _: () = msg_send![ns_app, activateIgnoringOtherApps: true];
        let _: () = msg_send![ns_app, arrangeInFront: nil];
    }
}

#[cfg(target_os = "macos")]
fn order_window_front(window: &tauri::WebviewWindow) {
    use cocoa::base::{id, nil};
    use objc::{msg_send, sel, sel_impl};

    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_window = ns_window as id;
            let _: () = msg_send![ns_window, makeKeyAndOrderFront: nil];
            let _: () = msg_send![ns_window, orderFrontRegardless];
        }
    }
}

#[cfg(all(target_os = "macos", feature = "e2e-testing"))]
fn prepare_main_window_for_e2e_snapshot(window: &tauri::WebviewWindow) {
    let _ = window.set_visible_on_all_workspaces(true);
    let _ = window.set_size(tauri::LogicalSize::new(980.0, 740.0));
    let _ = window.center();

    use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
    use cocoa::base::{id, NO};

    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_window = ns_window as id;
            let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorManaged;
            ns_window.setCollectionBehavior_(behavior);
            ns_window.setCanHide_(NO);
            ns_window.setHidesOnDeactivate_(NO);
        }
    }
}

/// Update pill window visibility based on indicator mode and recording state.
/// This is the single source of truth for pill visibility logic.
pub fn update_pill_visibility(app: &AppHandle) {
    use tracing::info;

    tracing::info!("update_pill_visibility_entered");

    let state = app.try_state::<AppState>();
    if state.is_none() {
        tracing::error!("update_pill_visibility_app_state_not_available");
        return;
    }
    let state = state.unwrap();

    tracing::info!("update_pill_visibility_state_acquired");

    let settings = state.settings.lock();
    let indicator_mode = settings.pill_indicator_mode.clone();
    drop(settings);

    let is_recording = state.is_recording.load(std::sync::atomic::Ordering::SeqCst);
    let is_transcribing = state
        .is_transcribing
        .load(std::sync::atomic::Ordering::SeqCst);

    tracing::info!(
        indicator_mode = %indicator_mode,
        is_recording,
        is_transcribing,
        "update_pill_visibility_state"
    );

    // Clone necessary values before dispatching to main thread
    let indicator_mode_clone = indicator_mode.clone();
    let app_clone = app.clone();

    // Window operations on macOS must run on main thread.
    // Use run_on_main_thread to safely dispatch.
    let _ = app.run_on_main_thread(move || {
        tracing::info!("update_pill_visibility_main_thread");

        if let Some(window) = app_clone.get_webview_window("pill") {
            tracing::info!("update_pill_visibility_window_found");
            match indicator_mode_clone.as_str() {
                "never" => {
                    tracing::info!("update_pill_visibility_mode_never");
                    let _ = window.hide();
                    info!(mode = "never", "pill_visibility_hidden");
                }
                "when_recording" => {
                    tracing::info!("update_pill_visibility_mode_when_recording");
                    // Only SHOW from backend; frontend handles hiding via exit animation
                    if is_recording || is_transcribing {
                        tracing::info!("update_pill_visibility_showing_window");
                        #[cfg(target_os = "macos")]
                        {
                            tracing::info!("update_pill_visibility_macos_show");
                            use cocoa::base::id;
                            use objc::{msg_send, sel, sel_impl};
                            if let Ok(ns_window) = window.ns_window() {
                                tracing::info!("update_pill_visibility_ns_window_ok");
                                unsafe {
                                    tracing::info!("update_pill_visibility_before_orderfront");
                                    let ns_window = ns_window as id;
                                    let _: () = msg_send![ns_window, orderFront: cocoa::base::nil];
                                    tracing::info!("update_pill_visibility_after_orderfront");
                                }
                            } else {
                                tracing::error!("update_pill_visibility_ns_window_failed");
                            }
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            let _ = window.show();
                        }
                        info!(
                            mode = "when_recording",
                            is_recording, is_transcribing, "pill_visibility_shown"
                        );
                    }
                }
                _ => {
                    tracing::info!("update_pill_visibility_mode_always");
                    // "always" or any other value
                    #[cfg(target_os = "macos")]
                    {
                        use cocoa::base::id;
                        use objc::{msg_send, sel, sel_impl};
                        if let Ok(ns_window) = window.ns_window() {
                            unsafe {
                                let ns_window = ns_window as id;
                                let _: () = msg_send![ns_window, orderFront: cocoa::base::nil];
                            }
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = window.show();
                    }
                    info!(mode = "always", "pill_visibility_shown");
                }
            }
        } else {
            tracing::warn!("update_pill_visibility_window_not_found");
        }
        tracing::info!("update_pill_visibility_completed");
    });
}

/// Get the monitor where the user is currently working.
/// Uses mouse cursor location as the primary signal — the cursor is almost
/// always on the same display as the focused input when a hotkey is pressed.
#[cfg(target_os = "macos")]
fn get_monitor_at_cursor(app: &AppHandle) -> Option<tauri::Monitor> {
    use cocoa::foundation::NSPoint;
    use objc::{class, msg_send, sel, sel_impl};
    use tracing::{debug, info};

    unsafe {
        let event_class = class!(NSEvent);
        let mouse_location: NSPoint = msg_send![event_class, mouseLocation];

        debug!(
            x = mouse_location.x,
            y = mouse_location.y,
            "mouse_cursor_position"
        );

        let screens_class = class!(NSScreen);
        let screens: *mut objc::runtime::Object = msg_send![screens_class, screens];
        let count: usize = msg_send![screens, count];

        for i in 0..count {
            let screen: *mut objc::runtime::Object = msg_send![screens, objectAtIndex: i];
            let frame: cocoa::foundation::NSRect = msg_send![screen, frame];

            if mouse_location.x >= frame.origin.x
                && mouse_location.x < frame.origin.x + frame.size.width
                && mouse_location.y >= frame.origin.y
                && mouse_location.y < frame.origin.y + frame.size.height
            {
                debug!(
                    screen = i,
                    x = frame.origin.x,
                    y = frame.origin.y,
                    "screen_contains_cursor"
                );

                // Match by X origin + width — size-only matching fails when two
                // monitors share the same resolution.
                // NSScreen uses logical points; Tauri uses physical pixels.
                // X origin is consistent between the two coordinate systems
                // (both start at 0 for the primary display, increase rightward).
                let available_monitors = app.available_monitors().ok()?;
                for monitor in available_monitors {
                    let pos = monitor.position();
                    let size = monitor.size();
                    let scale = monitor.scale_factor();
                    let logical_x = pos.x as f64 / scale;
                    let logical_width = size.width as f64 / scale;

                    if (logical_x - frame.origin.x).abs() < 2.0
                        && (logical_width - frame.size.width).abs() < 2.0
                    {
                        info!(monitor = ?monitor.name(), logical_x, screen_x = frame.origin.x, "monitor_matched_position_width");
                        return Some(monitor);
                    }
                }

                // Position match failed — fall back to size-only as last resort
                let available_monitors = app.available_monitors().ok()?;
                for monitor in available_monitors {
                    let size = monitor.size();
                    let scale = monitor.scale_factor();
                    let logical_w = size.width as f64 / scale;
                    let logical_h = size.height as f64 / scale;
                    if (logical_w - frame.size.width).abs() < 10.0
                        && (logical_h - frame.size.height).abs() < 10.0
                    {
                        info!(monitor = ?monitor.name(), "monitor_matched_size_fallback");
                        return Some(monitor);
                    }
                }
            }
        }
    }

    info!("monitor_not_found_cursor");
    None
}

#[cfg(not(target_os = "macos"))]
fn get_monitor_at_cursor(_app: &AppHandle) -> Option<tauri::Monitor> {
    None
}

/// Converts a preset position string to physical screen coordinates and moves the pill window.
pub fn position_pill_window(app: &AppHandle, preset: &str) {
    use tracing::info;

    info!(preset = %preset, "pill_position_requested");

    let Some(window) = app.get_webview_window("pill") else {
        info!("pill_window_not_found");
        return;
    };

    // Try to get the monitor where the cursor is (where user is actively working)
    // Fall back to pill's current monitor, then primary monitor
    let monitor = get_monitor_at_cursor(app)
        .or_else(|| {
            info!("monitor_fallback_current");
            window.current_monitor().ok().flatten()
        })
        .or_else(|| {
            info!("monitor_fallback_primary");
            app.primary_monitor().ok().flatten()
        });

    let Some(monitor) = monitor else {
        info!("monitor_not_found");
        return;
    };

    info!(monitor = ?monitor.name(), "monitor_selected_positioning");

    let scale = monitor.scale_factor();
    let screen = monitor.size();
    let origin = monitor.position();

    let pill_w = (PILL_WINDOW_W_LOGICAL * scale) as i32;
    let pill_h = (PILL_WINDOW_H_LOGICAL * scale) as i32;
    let margin = (MARGIN_LOGICAL * scale) as i32;

    let sw = screen.width as i32;
    let sh = screen.height as i32;

    let (rel_x, rel_y) = match preset {
        "top-left" => (margin, margin),
        "top-center" => ((sw - pill_w) / 2, margin),
        "top-right" => (sw - pill_w - margin, margin),
        "bottom-left" => (margin, sh - pill_h - margin),
        "bottom-right" => (sw - pill_w - margin, sh - pill_h - margin),
        _ => ((sw - pill_w) / 2, sh - pill_h - margin), // bottom-center (default)
    };

    let x = origin.x + rel_x;
    let y = origin.y + rel_y;

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> Result<(), String> {
    #[cfg(all(target_os = "macos", feature = "e2e-testing"))]
    {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    }

    if let Some(window) = app.get_webview_window("main") {
        #[cfg(target_os = "macos")]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let window_for_main = window.clone();
            app.run_on_main_thread(move || {
                #[cfg(feature = "e2e-testing")]
                prepare_main_window_for_e2e_snapshot(&window_for_main);

                activate_app_for_visible_window();
                let result = window_for_main
                    .show()
                    .map_err(|e| e.to_string())
                    .and_then(|_| window_for_main.unminimize().map_err(|e| e.to_string()))
                    .map(|_| {
                        order_window_front(&window_for_main);
                        let _ = window_for_main.set_focus();
                    });
                let _ = tx.send(result);
            })
            .map_err(|e| e.to_string())?;

            return rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .map_err(|e| e.to_string())?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            window.show().map_err(|e| e.to_string())?;
            window.unminimize().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn capture_main_window_snapshot(app: AppHandle) -> Result<MainWindowSnapshot, String> {
    #[cfg(feature = "e2e-testing")]
    {
        show_main_window(app).await?;
        return tokio::task::spawn_blocking(capture_main_window_snapshot_blocking)
            .await
            .map_err(|e| e.to_string())?;
    }

    #[cfg(not(feature = "e2e-testing"))]
    {
        let _ = app;
        Err("main-window snapshot capture is only available in e2e builds".to_string())
    }
}

#[cfg(all(target_os = "macos", feature = "e2e-testing"))]
fn capture_main_window_snapshot_blocking() -> Result<MainWindowSnapshot, String> {
    use base64::Engine;
    use image::ImageFormat;
    use std::io::Cursor;

    let pid = std::process::id();
    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
    let mut same_pid_windows = Vec::new();
    let mut visible_window_summaries = Vec::new();
    let mut candidates = Vec::new();

    for window in windows {
        let window_pid = window.pid().unwrap_or_default();
        let id = window.id().unwrap_or_default();
        let width = window.width().unwrap_or_default();
        let height = window.height().unwrap_or_default();
        let title = window.title().unwrap_or_default();
        let app_name = window.app_name().unwrap_or_default();

        if visible_window_summaries.len() < 12 {
            visible_window_summaries.push(format!(
                "pid={window_pid} id={id} app={app_name:?} title={title:?} size={width}x{height}"
            ));
        }

        if window_pid != pid {
            continue;
        }

        same_pid_windows.push(format!(
            "id={id} app={app_name:?} title={title:?} size={width}x{height}"
        ));

        if width >= 600 && height >= 400 {
            candidates.push((u64::from(width) * u64::from(height), window));
        }
    }

    let Some((_, window)) = candidates.into_iter().max_by_key(|(area, _)| *area) else {
        return Err(format!(
            "no e2e main window found for pid {pid}; same-pid windows: {}; visible windows: {}",
            same_pid_windows.join(", "),
            visible_window_summaries.join(", ")
        ));
    };

    let image = window.capture_image().map_err(|e| e.to_string())?;
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let bytes = cursor.into_inner();
    let base64 = base64::engine::general_purpose::STANDARD.encode(bytes);

    Ok(MainWindowSnapshot { base64 })
}

#[cfg(all(not(target_os = "macos"), feature = "e2e-testing"))]
fn capture_main_window_snapshot_blocking() -> Result<MainWindowSnapshot, String> {
    Err("main-window snapshot capture is only implemented for macOS e2e".to_string())
}

#[tauri::command]
pub async fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn show_pill_window(app: AppHandle) -> Result<(), String> {
    use tracing::{error, info};

    info!("show_pill_window_requested");

    let app_for_main_thread = app.clone();
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

    app.run_on_main_thread(move || {
        info!("show_pill_window_main_thread");
        let result = if let Some(window) = app_for_main_thread.get_webview_window("pill") {
            match show_pill_without_focus(&window) {
                Ok(()) => {
                    info!("pill_window_shown");
                    Ok(())
                }
                Err(err) => {
                    error!(error = %err, "pill_window_show_failed");
                    Err(err)
                }
            }
        } else {
            info!("pill_window_not_found");
            Ok(())
        };

        let _ = result_tx.send(result);
    })
    .map_err(|err| err.to_string())?;

    result_rx
        .await
        .map_err(|_| "show_pill_window main thread task was canceled".to_string())?
}

#[tauri::command]
pub async fn hide_pill_window(app: AppHandle) -> Result<(), String> {
    use tracing::info;
    info!("hide_pill_window_requested");
    if let Some(window) = app.get_webview_window("pill") {
        window.hide().map_err(|e| e.to_string())?;
        info!("pill_window_hidden");
    } else {
        info!("pill_window_not_found");
    }
    Ok(())
}

#[tauri::command]
pub async fn update_pill_position(app: AppHandle, x: f64, y: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("pill") {
        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: x as i32,
                y: y as i32,
            }))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_pill_position(app: AppHandle) -> Result<Option<Position>, String> {
    if let Some(window) = app.get_webview_window("pill") {
        if let Ok(position) = window.outer_position() {
            return Ok(Some(Position {
                x: position.x as f64,
                y: position.y as f64,
            }));
        }
    }
    Ok(None)
}
