use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Shared helper used by both the `insert_text` command and the audio pipeline.
pub async fn do_insert_text(_app: AppHandle, text: String) {
    let injector = crate::text_injector::create_injector();
    // The write_clipboard callback is a no-op: clipboard handling is done
    // natively inside MacosInjector (NSPasteboard save/restore).
    injector.insert(&text, &|| {});
}

#[tauri::command]
pub async fn insert_text(app: AppHandle, text: String) -> Result<(), String> {
    do_insert_text(app, text).await;
    Ok(())
}

#[tauri::command]
pub async fn copy_to_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(&text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(&text)
        .map_err(|e| e.to_string())
}
