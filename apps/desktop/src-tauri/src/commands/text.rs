use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tracing::instrument;

/// Shared helper used by both the `insert_text` command and the audio pipeline.
#[instrument(fields(text_len = text.len()))]
pub fn do_insert_text(text: &str) {
    crate::text_injector::insert_text(text);
}

#[tauri::command]
#[instrument(skip(app), fields(text_len = text.len()), ret, err)]
pub async fn insert_text(app: AppHandle, text: String) -> Result<(), String> {
    let _ = app;
    do_insert_text(&text);
    Ok(())
}

#[tauri::command]
#[instrument(skip(app), fields(text_len = text.len()), ret, err)]
pub async fn copy_to_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(&text).map_err(|e| e.to_string())
}
