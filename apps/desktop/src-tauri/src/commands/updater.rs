use serde::Serialize;
use tauri::{ipc::Channel, AppHandle};
use tauri_plugin_updater::{Error as UpdaterError, UpdaterExt};
use tracing::{info, warn};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub version: String,
    pub current_version: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum UpdateInstallEvent {
    Started {
        content_length: Option<u64>,
    },
    Progress {
        downloaded: u64,
        content_length: Option<u64>,
    },
    Finished,
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<Option<AppUpdateInfo>, String> {
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(UpdaterError::EmptyEndpoints) => {
            info!("update_check_skipped-updater_not_configured");
            return Ok(None);
        }
        Err(error) => return Err(error.to_string()),
    };

    let update = updater.check().await.map_err(|error| error.to_string())?;

    Ok(update.map(|update| AppUpdateInfo {
        version: update.version.to_string(),
        current_version: update.current_version.to_string(),
    }))
}

#[tauri::command]
pub async fn install_update(
    app: AppHandle,
    on_event: Channel<UpdateInstallEvent>,
) -> Result<(), String> {
    let updater = app.updater().map_err(|error| match error {
        UpdaterError::EmptyEndpoints => "updater_not_configured".to_string(),
        error => error.to_string(),
    })?;

    let Some(update) = updater.check().await.map_err(|error| error.to_string())? else {
        return Err("no_update_available".to_string());
    };

    info!(
        version = %update.version,
        current_version = %update.current_version,
        "update_install_started"
    );

    let progress_channel = on_event.clone();
    let finished_channel = on_event;
    let mut downloaded = 0_u64;
    let mut started = false;

    update
        .download_and_install(
            move |chunk_length, content_length| {
                if !started {
                    started = true;
                    if let Err(error) =
                        progress_channel.send(UpdateInstallEvent::Started { content_length })
                    {
                        warn!(error = %error, "update_started_event_send_failed");
                    }
                }

                downloaded = downloaded.saturating_add(chunk_length as u64);
                if let Err(error) = progress_channel.send(UpdateInstallEvent::Progress {
                    downloaded,
                    content_length,
                }) {
                    warn!(error = %error, "update_progress_event_send_failed");
                }
            },
            move || {
                if let Err(error) = finished_channel.send(UpdateInstallEvent::Finished) {
                    warn!(error = %error, "update_finished_event_send_failed");
                }
            },
        )
        .await
        .map_err(|error| error.to_string())?;

    info!("update_install_finished");
    Ok(())
}
