//! IPC commands for hotkey recording and profile management.
//!
//! Profile management uses map structure: { dictate, riff, custom? }
//! - dictate/riff: system profiles, cannot be deleted
//! - custom: optional user profile (max 1)

use tauri::{AppHandle, Emitter, Manager, State};

use crate::commands::settings::save_settings_internal;
use crate::events::EventName;
use crate::shortcut::{ShortcutManager, ShortcutProfile, ShortcutProfilesMap, ShortcutTriggerMode};
use crate::state::app_state::AppState;

/// Starts hotkey capture for a specific profile key.
///
/// The listener captures the next hotkey combination pressed by the user.
/// When captured, emits `hotkey-captured` event.
#[tauri::command]
pub fn start_hotkey_capture(app: AppHandle, profile_key: String) -> Result<(), String> {
    validate_profile_key(&profile_key)?;

    app.try_state::<ShortcutManager>()
        .ok_or_else(|| "shortcut manager not available".to_string())?
        .start_recording_capture()
}

/// Stops hotkey capture and binds to the specified profile key.
///
/// Returns the captured hotkey string, or None if no valid hotkey captured.
#[tauri::command]
pub fn stop_hotkey_capture(app: AppHandle, profile_key: String) -> Result<String, String> {
    if validate_profile_key(&profile_key).is_err() {
        return Err(format!("invalid_profile_key: {}", profile_key));
    }

    let shortcut_manager = app.try_state::<ShortcutManager>();
    let Some(shortcut_manager) = shortcut_manager else {
        return Err("shortcut_manager_not_available".to_string());
    };

    let captured_hotkey = shortcut_manager
        .stop_recording_capture()
        .map_err(|e| format!("stop_hotkey_capture_failed: {}", e))?;

    let hotkey = captured_hotkey.ok_or("no_hotkey_captured")?;

    let app_state = app.try_state::<crate::state::app_state::AppState>();
    let Some(app_state) = app_state else {
        return Err("app_state_unavailable".to_string());
    };

    // Validate hotkey uniqueness before registering
    {
        let settings = app_state.settings.lock();
        validate_hotkey_uniqueness(&settings, &profile_key, &hotkey)?;
    }

    let profile = crate::shortcut::ShortcutProfile {
        hotkey: hotkey.clone(),
        trigger_mode: get_current_trigger_mode(&app_state, &profile_key),
        action: crate::shortcut::ShortcutAction::Record {
            polish_template_id: get_current_template_id(&app_state, &profile_key),
        },
    };

    shortcut_manager
        .register_profile(&profile_key, &profile)
        .map_err(|e| format!("register_profile_failed: {}", e))?;

    {
        let mut settings = app_state.settings.lock();
        update_profile_in_map(
            &mut settings.shortcut_profiles,
            &profile_key,
            profile.clone(),
        );
    }

    crate::commands::settings::save_settings_internal(&app)
        .map_err(|e| format!("save_settings_failed: {}", e))?;

    let settings = app_state.settings.lock().clone();
    app.emit(crate::events::EventName::SETTINGS_CHANGED, settings)
        .map_err(|e| format!("emit_settings_changed_failed: {}", e))?;

    Ok(hotkey)
}

fn get_current_template_id(
    state: &crate::state::app_state::AppState,
    profile_key: &str,
) -> Option<String> {
    let settings = state.settings.lock();
    let profiles = &settings.shortcut_profiles;
    match profile_key {
        "dictate" => None,
        "riff" => match &profiles.riff.action {
            crate::shortcut::ShortcutAction::Record { polish_template_id } => {
                polish_template_id.clone()
            }
        },
        "custom" => profiles.custom.as_ref().and_then(|p| match &p.action {
            crate::shortcut::ShortcutAction::Record { polish_template_id } => {
                polish_template_id.clone()
            }
        }),
        _ => None,
    }
}

fn get_current_trigger_mode(
    state: &crate::state::app_state::AppState,
    profile_key: &str,
) -> ShortcutTriggerMode {
    let settings = state.settings.lock();
    let profiles = &settings.shortcut_profiles;
    match profile_key {
        "dictate" => profiles.dictate.trigger_mode,
        "riff" => profiles.riff.trigger_mode,
        "custom" => profiles
            .custom
            .as_ref()
            .map(|profile| profile.trigger_mode)
            .unwrap_or(ShortcutTriggerMode::Toggle),
        _ => ShortcutTriggerMode::Hold,
    }
}

/// Cancels hotkey capture without saving.
#[tauri::command]
pub fn cancel_hotkey_capture(app: AppHandle) {
    if let Some(shortcut_manager) = app.try_state::<ShortcutManager>() {
        shortcut_manager.cancel_recording_capture();
    } else {
        tracing::error!("shortcut_manager_not_available");
    }
}

/// Peeks at the currently captured hotkey without stopping.
#[tauri::command]
pub fn peek_hotkey_capture(app: AppHandle) -> Option<String> {
    app.try_state::<ShortcutManager>()
        .and_then(|shortcut_manager| shortcut_manager.peek_recording_capture())
}

/// Get all shortcut profiles (map structure).
#[tauri::command]
pub fn get_shortcut_profiles(state: State<'_, AppState>) -> Result<ShortcutProfilesMap, String> {
    let settings = state.settings.lock();
    Ok(settings.shortcut_profiles.clone())
}

/// Update a specific profile by key.
///
/// Validates:
/// - dictate: template_id must remain None
/// - riff: template_id cannot be None
/// - custom: no constraints
/// - hotkey uniqueness across all profiles
#[tauri::command]
pub fn update_shortcut_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    key: String,
    profile: ShortcutProfile,
) -> Result<(), String> {
    validate_profile_key(&key)?;
    validate_profile_constraints(&key, &profile)?;

    let shortcut_manager = app
        .try_state::<ShortcutManager>()
        .ok_or_else(|| "shortcut manager not available".to_string())?;

    {
        let settings = state.settings.lock();
        validate_hotkey_uniqueness(&settings, &key, &profile.hotkey)?;
    }

    // Register if hotkey is not empty, unregister if hotkey is empty
    if !profile.hotkey.is_empty() {
        shortcut_manager.register_profile(&key, &profile)?;
    } else {
        shortcut_manager.unregister_profile(&key)?;
    }

    {
        let mut settings = state.settings.lock();
        update_profile_in_map(&mut settings.shortcut_profiles, &key, profile.clone());
    }

    save_settings_internal(&app)?;

    let settings = state.settings.lock().clone();
    app.emit(EventName::SETTINGS_CHANGED, settings)
        .map_err(|e| format!("failed to emit settings changed: {}", e))?;

    tracing::info!(key = %key, hotkey = %profile.hotkey, "shortcut_profile_updated");
    Ok(())
}

/// Create custom profile (max 1).
///
/// Returns error if custom profile already exists.
#[tauri::command]
pub fn create_custom_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: ShortcutProfile,
) -> Result<(), String> {
    let shortcut_manager = app
        .try_state::<ShortcutManager>()
        .ok_or_else(|| "shortcut manager not available".to_string())?;

    {
        let settings = state.settings.lock();

        if settings.shortcut_profiles.custom.is_some() {
            return Err("custom_profile_already_exists".to_string());
        }

        validate_hotkey_uniqueness(&settings, "custom", &profile.hotkey)?;
    }

    // Only register if hotkey is not empty
    if !profile.hotkey.is_empty() {
        shortcut_manager.register_profile("custom", &profile)?;
    }

    {
        let mut settings = state.settings.lock();
        settings.shortcut_profiles.custom = Some(profile.clone());
    }

    save_settings_internal(&app)?;

    let settings = state.settings.lock().clone();
    app.emit(EventName::SETTINGS_CHANGED, settings)
        .map_err(|e| format!("failed to emit settings changed: {}", e))?;

    tracing::info!("custom_profile_created");
    Ok(())
}

/// Delete custom profile.
///
/// Cannot delete dictate or riff (system profiles).
#[tauri::command]
pub fn delete_custom_profile(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let shortcut_manager = app
        .try_state::<ShortcutManager>()
        .ok_or_else(|| "shortcut manager not available".to_string())?;

    {
        let mut settings = state.settings.lock();
        if settings.shortcut_profiles.custom.is_none() {
            return Err("custom_profile_not_found".to_string());
        }
        settings.shortcut_profiles.custom = None;
    }

    save_settings_internal(&app)?;

    shortcut_manager.unregister_profile("custom")?;

    let settings = state.settings.lock().clone();
    app.emit(EventName::SETTINGS_CHANGED, settings)
        .map_err(|e| format!("failed to emit settings changed: {}", e))?;

    tracing::info!("custom_profile_deleted");
    Ok(())
}

fn validate_profile_key(key: &str) -> Result<(), String> {
    match key {
        "dictate" | "riff" | "custom" => Ok(()),
        _ => Err(format!("unknown_profile_key: {}", key)),
    }
}

fn validate_profile_constraints(key: &str, profile: &ShortcutProfile) -> Result<(), String> {
    match &profile.action {
        crate::shortcut::ShortcutAction::Record { polish_template_id } => match key {
            "dictate" => {
                if polish_template_id.is_some() {
                    return Err("cannot_update_dictate_template".to_string());
                }
            }
            "riff" => {
                if polish_template_id.is_none() {
                    return Err("riff_template_cannot_be_null".to_string());
                }
            }
            "custom" => {}
            _ => return Err(format!("unknown_profile_key: {}", key)),
        },
    }
    Ok(())
}

fn validate_hotkey_uniqueness(
    settings: &crate::commands::settings::AppSettings,
    exclude_key: &str,
    hotkey: &str,
) -> Result<(), String> {
    if hotkey.is_empty() {
        return Ok(());
    }

    let profiles = &settings.shortcut_profiles;

    if exclude_key != "dictate" && profiles.dictate.hotkey == hotkey {
        return Err("hotkey_conflict:dictate".to_string());
    }

    if exclude_key != "riff" && profiles.riff.hotkey == hotkey {
        return Err("hotkey_conflict:riff".to_string());
    }

    if let Some(custom) = &profiles.custom {
        if exclude_key != "custom" && custom.hotkey == hotkey {
            return Err("hotkey_conflict:custom".to_string());
        }
    }

    Ok(())
}

fn update_profile_in_map(map: &mut ShortcutProfilesMap, key: &str, profile: ShortcutProfile) {
    match key {
        "dictate" => map.dictate = profile,
        "riff" => map.riff = profile,
        "custom" => map.custom = Some(profile),
        _ => {}
    }
}
