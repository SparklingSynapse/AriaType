use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

use crate::events::EventName;
use crate::state::app_state::AppState;

const PILL_W_LOGICAL: f64 = 100.0;
const PILL_H_LOGICAL: f64 = 52.0;
const MARGIN_LOGICAL: f64 = 20.0;

/// Update pill window visibility based on indicator mode and recording state.
/// This is the single source of truth for pill visibility logic.
pub fn update_pill_visibility(app: &AppHandle) {
    use tracing::info;

    let state = app.state::<AppState>();
    let settings = state.settings.lock();
    let indicator_mode = settings.pill_indicator_mode.clone();
    drop(settings);

    let is_recording = state.is_recording.load(std::sync::atomic::Ordering::SeqCst);
    let is_transcribing = state.is_transcribing.load(std::sync::atomic::Ordering::SeqCst);

    if let Some(window) = app.get_webview_window("pill") {
        match indicator_mode.as_str() {
            "never" => {
                let _ = window.hide();
                info!("pill visibility: hidden (mode=never)");
            }
            "when_recording" => {
                // Only SHOW from backend; frontend handles hiding via exit animation
                if is_recording || is_transcribing {
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
                    info!("pill visibility: shown (mode=when_recording, is_recording={}, is_transcribing={})", is_recording, is_transcribing);
                }
            }
            _ => {
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
                info!("pill visibility: shown (mode=always)");
            }
        }
    }
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

        debug!(x = mouse_location.x, y = mouse_location.y, "mouse cursor position");

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
                debug!(screen = i, x = frame.origin.x, y = frame.origin.y, "found screen containing cursor");

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
                        info!(monitor = ?monitor.name(), logical_x, screen_x = frame.origin.x, "matched monitor by position+width");
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
                        info!(monitor = ?monitor.name(), "matched monitor by size (fallback)");
                        return Some(monitor);
                    }
                }
            }
        }
    }

    info!("no monitor found at cursor position");
    None
}

#[cfg(not(target_os = "macos"))]
fn get_monitor_at_cursor(_app: &AppHandle) -> Option<tauri::Monitor> {
    None
}

/// Converts a preset position string to physical screen coordinates and moves the pill window.
pub fn position_pill_window(app: &AppHandle, preset: &str) {
    use tracing::info;

    info!(preset = preset, "position_pill_window called");

    let Some(window) = app.get_webview_window("pill") else {
        info!("pill window not found");
        return;
    };

    // Try to get the monitor where the cursor is (where user is actively working)
    // Fall back to pill's current monitor, then primary monitor
    let monitor = get_monitor_at_cursor(app)
        .or_else(|| {
            info!("falling back to current monitor");
            window.current_monitor().ok().flatten()
        })
        .or_else(|| {
            info!("falling back to primary monitor");
            app.primary_monitor().ok().flatten()
        });

    let Some(monitor) = monitor else {
        info!("no monitor found");
        return
    };

    info!(monitor = ?monitor.name(), "using monitor for positioning");

    let scale = monitor.scale_factor();
    let screen = monitor.size();
    let origin = monitor.position();

    let pill_w = (PILL_W_LOGICAL * scale) as i32;
    let pill_h = (PILL_H_LOGICAL * scale) as i32;
    let margin = (MARGIN_LOGICAL * scale) as i32;

    let sw = screen.width as i32;
    let sh = screen.height as i32;

    let (rel_x, rel_y) = match preset {
        "top-left"      => (margin, margin),
        "top-center"    => ((sw - pill_w) / 2, margin),
        "top-right"     => (sw - pill_w - margin, margin),
        "bottom-left"   => (margin, sh - pill_h - margin),
        "bottom-right"  => (sw - pill_w - margin, sh - pill_h - margin),
        _               => ((sw - pill_w) / 2, sh - pill_h - margin), // bottom-center (default)
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
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
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
    use tracing::info;
    info!("show_pill_window called");
    if let Some(window) = app.get_webview_window("pill") {
        window.show().map_err(|e| e.to_string())?;
        info!("pill window shown");
    } else {
        info!("pill window not found");
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_pill_window(app: AppHandle) -> Result<(), String> {
    use tracing::info;
    info!("hide_pill_window called");
    if let Some(window) = app.get_webview_window("pill") {
        window.hide().map_err(|e| e.to_string())?;
        info!("pill window hidden");
    } else {
        info!("pill window not found");
    }
    Ok(())
}

#[tauri::command]
pub async fn show_toast(app: AppHandle, message: String) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("toast") {
        window.show().map_err(|e| e.to_string())?;
        window.emit(EventName::TOAST_MESSAGE, message).map_err(|e| e.to_string())?;
        
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let _ = window.hide();
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_toast(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("toast") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn update_pill_position(app: AppHandle, x: f64, y: f64) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("pill") {
        window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: x as i32, y: y as i32 }))
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
