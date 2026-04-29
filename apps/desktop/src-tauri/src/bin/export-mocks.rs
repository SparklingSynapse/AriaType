use ariatype_lib::history::models::{HistoryFilter, TranscriptionEntry};
use ariatype_lib::state::app_state::AppState;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY_ENTRIES: usize = 20;
const FAKE_TEXT: &str = "hello";

fn sanitize_history_entry(mut entry: TranscriptionEntry) -> TranscriptionEntry {
    entry.raw_text = FAKE_TEXT.to_string();
    entry.final_text = FAKE_TEXT.to_string();
    entry.error = None;
    entry.audio_path = None;
    entry
}

fn main() {
    let output_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/e2e/mocks".to_string());

    let mock_dir = PathBuf::from(&output_dir);
    fs::create_dir_all(&mock_dir).expect("Failed to create mock directory");

    let state = AppState::new();

    let models = state.engine_manager.get_all_models();
    let models_json = serde_json::to_string_pretty(&models).expect("Failed to serialize models");
    fs::write(mock_dir.join("models.json"), &models_json).expect("Failed to write models");
    println!("Exported: models.json");

    let history_store = state.history_store.lock();
    let history_entries = history_store
        .get_history(&HistoryFilter {
            search: None,
            engine: None,
            status: None,
            date_from: None,
            date_to: None,
            limit: None,
            offset: None,
        })
        .expect("Failed to get history");

    let sanitized_entries: Vec<TranscriptionEntry> = history_entries
        .into_iter()
        .take(MAX_HISTORY_ENTRIES)
        .map(sanitize_history_entry)
        .collect();

    let history_json =
        serde_json::to_string_pretty(&sanitized_entries).expect("Failed to serialize history");
    fs::write(mock_dir.join("history.json"), &history_json).expect("Failed to write history");
    println!(
        "Exported: history.json ({} entries, sanitized)",
        sanitized_entries.len()
    );

    let stats = history_store
        .get_dashboard_stats()
        .expect("Failed to get stats");
    let stats_json = serde_json::to_string_pretty(&stats).expect("Failed to serialize stats");
    fs::write(mock_dir.join("dashboard-stats.json"), &stats_json).expect("Failed to write stats");
    println!("Exported: dashboard-stats.json");

    println!("\nMock data exported to: {}", output_dir);
}
