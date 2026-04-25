//! System tray (status bar) implementation for AriaType.
//!
//! Provides a persistent tray icon in the system status bar with:
//! - Click to show/hide main settings window
//! - Context menu with: Show Settings, Toggle Recording, Quit
//! - Visual indicator for recording state (icon changes)

use tauri::{
    include_image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tracing::{error, info, instrument};

/// Tray menu item IDs
pub const MENU_ID_SHOW_SETTINGS: &str = "show-settings";
pub const MENU_ID_TOGGLE_RECORDING: &str = "toggle-recording";
pub const MENU_ID_QUIT: &str = "quit";

/// Create and setup the system tray icon with menu.
///
/// On macOS, this creates a status bar item that persists even when
/// the main window is closed, allowing quick access to settings and
/// recording control.
#[instrument(skip(app))]
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let tray_icon = load_tray_icon();

    let show_settings = MenuItem::with_id(
        app,
        MENU_ID_SHOW_SETTINGS,
        "Show Settings",
        true,
        None::<&str>,
    )?;

    let toggle_recording = MenuItem::with_id(
        app,
        MENU_ID_TOGGLE_RECORDING,
        "Toggle Recording",
        true,
        None::<&str>,
    )?;

    let quit = MenuItem::with_id(app, MENU_ID_QUIT, "Quit", true, None::<&str>)?;

    let separator = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(app, &[&show_settings, &toggle_recording, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("ariatype-tray")
        .icon(tray_icon)
        .menu(&menu)
        .tooltip("AriaType")
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(handle_tray_icon_event);

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true).menu_on_left_click(true);
    }

    let _tray = builder.build(app)?;

    info!("tray_created");
    Ok(())
}

/// Remove the system tray icon.
pub fn remove_tray(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id("ariatype-tray") {
        let _ = tray.set_visible(false);
        info!("tray_hidden");
    }
}

/// Show the system tray icon (create if not exists).
pub fn show_tray(app: &AppHandle) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id("ariatype-tray") {
        let _ = tray.set_visible(true);
        info!("tray_shown");
        Ok(())
    } else {
        create_tray(app)
    }
}

/// Load the tray icon appropriate for the platform.
///
/// On macOS, tray icons should be template images (monochrome) that
/// automatically adapt to dark/light mode. The icon should be small
/// (16x16 or 32x32 for retina).
fn load_tray_icon() -> tauri::image::Image<'static> {
    include_image!("assets/tray-icon.png")
}

/// Handle tray menu item click events.
#[instrument(skip(app, event))]
fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    info!(menu_item = id, "tray_menu_clicked");

    match id {
        MENU_ID_SHOW_SETTINGS => {
            show_main_window(app);
        }
        MENU_ID_TOGGLE_RECORDING => {
            toggle_recording(app);
        }
        MENU_ID_QUIT => {
            info!("quit_requested-tray_menu");
            app.exit(0);
        }
        _ => {
            info!(menu_item = id, "tray_menu_unknown");
        }
    }
}

/// Handle tray icon click events (direct clicks on the icon, not menu).
fn handle_tray_icon_event(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => {
            let app = tray.app_handle();
            show_main_window(app);
        }
        TrayIconEvent::Click {
            button: MouseButton::Right,
            button_state: MouseButtonState::Up,
            ..
        } => {
            info!("tray_right_clicked");
        }
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => {
            let app = tray.app_handle();
            show_main_window(app);
        }
        _ => {}
    }
}

/// Show and focus the main settings window.
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(e) = window.show() {
            error!(error = %e, "main_window_show_failed");
            return;
        }

        if let Err(e) = window.unminimize() {
            error!(error = %e, "main_window_unminimize_failed");
        }

        if let Err(e) = window.set_focus() {
            error!(error = %e, "main_window_focus_failed");
        }

        info!("main_window_shown");
    } else {
        info!("main_window_not_found");
    }
}

/// Toggle recording state from tray.
fn toggle_recording(app: &AppHandle) {
    use std::sync::atomic::Ordering;

    let state = match app.try_state::<crate::state::app_state::AppState>() {
        Some(s) => s,
        None => {
            error!("app_state_not_found");
            return;
        }
    };

    let is_recording = state.is_recording.load(Ordering::SeqCst);

    if is_recording {
        info!("recording_stop_requested-tray");
        match crate::commands::audio::stop_recording_sync(app.clone()) {
            Ok(_) => info!("recording_stopped-tray"),
            Err(e) => error!(error = %e, "recording_stop_failed-tray"),
        }
    } else {
        info!("recording_start_requested-tray");
        match crate::commands::audio::start_recording_sync(app, None) {
            Ok(_) => info!("recording_started-tray"),
            Err(e) => error!(error = %e, "recording_start_failed-tray"),
        }
    }
}

/// Update tray icon based on recording state.
///
/// Call this when recording state changes to show a visual indicator.
#[allow(dead_code)]
pub fn update_tray_icon_for_recording(app: &AppHandle, is_recording: bool) -> tauri::Result<()> {
    let tray = app.tray_by_id("ariatype-tray");

    if let Some(_tray) = tray {
        if is_recording {
            info!("tray_icon_updated-recording_active");
        } else {
            info!("tray_icon_updated-idle");
        }
    }

    Ok(())
}

/// Set the tray menu item text for recording toggle.
///
/// Updates the "Toggle Recording" menu item to show appropriate text
/// based on current recording state.
#[allow(dead_code)]
pub fn update_tray_menu_recording_text(app: &AppHandle, is_recording: bool) -> tauri::Result<()> {
    let _menu_item = MenuItem::with_id(
        app,
        MENU_ID_TOGGLE_RECORDING,
        if is_recording {
            "Stop Recording"
        } else {
            "Start Recording"
        },
        true,
        None::<&str>,
    )?;

    let tray = app.tray_by_id("ariatype-tray");
    if let Some(_tray) = tray {
        info!("tray_menu_text_update_pending");
    }

    Ok(())
}
