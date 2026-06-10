use super::hotwords::{
    detect_csv_header, hotword_entry_from_csv_row, parse_custom_hotword_entries,
    serialize_custom_hotword_entries, CsvHeaderKind, HotwordEntry, CUSTOM_DICTIONARY_SOURCE,
};
use super::storage::CorrectionStore;
use super::types::CorrectionMapping;
use crate::commands::settings;
use crate::events::EventName;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictionaryEntry {
    pub term: String,
    pub aliases: Vec<String>,
    pub frequency: u32,
    pub first_seen_at_ms: i64,
    pub last_seen_at_ms: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictionaryImportResult {
    pub imported: usize,
    pub skipped: usize,
}

impl DictionaryEntry {
    fn manual(entry: HotwordEntry, now_ms: i64) -> Self {
        Self {
            term: entry.term,
            aliases: entry.aliases,
            frequency: entry.frequency,
            first_seen_at_ms: now_ms,
            last_seen_at_ms: now_ms,
            source: CUSTOM_DICTIONARY_SOURCE.to_string(),
        }
    }
}

#[tauri::command]
pub fn clear_correction_memory() -> Result<(), String> {
    CorrectionStore::shared().clear()
}

#[tauri::command]
pub fn get_auto_dictionary_entries() -> Result<Vec<DictionaryEntry>, String> {
    let file = CorrectionStore::shared().load_or_empty(chrono::Utc::now().timestamp_millis())?;
    Ok(auto_dictionary_entries_from_mappings(file.corrections))
}

#[tauri::command]
pub fn get_custom_dictionary_entries(
    state: State<'_, AppState>,
) -> Result<Vec<DictionaryEntry>, String> {
    let dictionary = state.settings.lock().custom_dictionary.clone();
    Ok(parse_custom_dictionary_entries(&dictionary))
}

#[tauri::command]
pub fn add_custom_dictionary_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    term: String,
) -> Result<DictionaryEntry, String> {
    let hotword = HotwordEntry::new(term, Vec::new(), 1, CUSTOM_DICTIONARY_SOURCE)?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut entries = load_custom_entries_from_state(&state);
    let (entry, changed) = upsert_custom_entry(&mut entries, hotword, now_ms);
    if changed {
        persist_custom_dictionary_entries(&app, &state, &entries)?;
    }
    Ok(entry)
}

#[tauri::command]
pub fn import_custom_dictionary_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    csv_content: String,
) -> Result<DictionaryImportResult, String> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut entries = load_custom_entries_from_state(&state);
    let mut imported = 0;
    let mut skipped = 0;
    let mut header_kind = CsvHeaderKind::None;

    for (index, row) in parse_csv_rows(&csv_content).into_iter().enumerate() {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }

        if index == 0 {
            header_kind = detect_csv_header(&row);
        }

        if index == 0 && header_kind != CsvHeaderKind::None {
            continue;
        }

        match hotword_entry_from_csv_row(&row, header_kind) {
            Ok(entry) => {
                let (_, changed) = upsert_custom_entry(&mut entries, entry, now_ms);
                if changed {
                    imported += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    if imported > 0 {
        persist_custom_dictionary_entries(&app, &state, &entries)?;
    }

    Ok(DictionaryImportResult { imported, skipped })
}

#[tauri::command]
pub fn delete_custom_dictionary_entry(
    app: AppHandle,
    state: State<'_, AppState>,
    term: String,
) -> Result<(), String> {
    let mut entries = load_custom_entries_from_state(&state);
    let before = entries.len();
    entries.retain(|entry| !entry.term.eq_ignore_ascii_case(&term));
    if entries.len() != before {
        persist_custom_dictionary_entries(&app, &state, &entries)?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_correction_memory_directory() -> Result<(), String> {
    let store = CorrectionStore::shared();
    store.ensure_file()?;
    let directory = store
        .path()
        .parent()
        .ok_or_else(|| "correction memory directory is unavailable".to_string())?;
    open_directory(directory)
}

fn open_directory(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open correction memory directory: {error}"))
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open correction memory directory: {error}"))
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open correction memory directory: {error}"))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = path;
        Err("opening correction memory directory is not supported on this platform".to_string())
    }
}

fn load_custom_entries_from_state(state: &AppState) -> Vec<DictionaryEntry> {
    let dictionary = state.settings.lock().custom_dictionary.clone();
    parse_custom_dictionary_entries(&dictionary)
}

fn persist_custom_dictionary_entries(
    app: &AppHandle,
    state: &AppState,
    entries: &[DictionaryEntry],
) -> Result<(), String> {
    let updated_settings = {
        let mut settings = state.settings.lock();
        settings.custom_dictionary = serialize_custom_dictionary_entries(entries);
        settings.clone()
    };

    settings::save_settings_internal(app)?;
    app.emit(EventName::SETTINGS_CHANGED, updated_settings)
        .map_err(|error| format!("failed to emit settings change: {error}"))?;
    Ok(())
}

fn parse_custom_dictionary_entries(glossary: &str) -> Vec<DictionaryEntry> {
    parse_custom_hotword_entries(glossary)
        .into_iter()
        .map(|entry| DictionaryEntry::manual(entry, 0))
        .collect()
}

fn upsert_custom_entry(
    entries: &mut Vec<DictionaryEntry>,
    hotword: HotwordEntry,
    now_ms: i64,
) -> (DictionaryEntry, bool) {
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.term.eq_ignore_ascii_case(&hotword.term))
    {
        let previous_aliases = existing.aliases.clone();
        let previous_frequency = existing.frequency;
        for alias in hotword.aliases {
            if !existing
                .aliases
                .iter()
                .any(|existing_alias| existing_alias.eq_ignore_ascii_case(&alias))
            {
                existing.aliases.push(alias);
            }
        }
        existing.frequency = existing.frequency.max(hotword.frequency);

        if existing.aliases == previous_aliases && existing.frequency == previous_frequency {
            return (existing.clone(), false);
        }
        existing.last_seen_at_ms = now_ms;
        return (existing.clone(), true);
    }

    let entry = DictionaryEntry::manual(hotword, now_ms);
    entries.push(entry.clone());
    entries.sort_by(|left, right| left.term.cmp(&right.term));
    (entry, true)
}

fn serialize_custom_dictionary_entries(entries: &[DictionaryEntry]) -> String {
    let hotwords = entries
        .iter()
        .filter_map(|entry| {
            HotwordEntry::new(
                entry.term.clone(),
                entry.aliases.clone(),
                entry.frequency,
                entry.source.clone(),
            )
            .ok()
        })
        .collect::<Vec<_>>();
    serialize_custom_hotword_entries(&hotwords)
}

fn auto_dictionary_entries_from_mappings(mappings: Vec<CorrectionMapping>) -> Vec<DictionaryEntry> {
    let mut entries: Vec<DictionaryEntry> = Vec::new();

    for mapping in mappings {
        let Some(existing) = entries
            .iter_mut()
            .find(|entry| entry.term.eq_ignore_ascii_case(&mapping.corrected))
        else {
            entries.push(DictionaryEntry {
                term: mapping.corrected,
                aliases: vec![mapping.wrong],
                frequency: mapping.frequency,
                first_seen_at_ms: mapping.first_seen_at_ms,
                last_seen_at_ms: mapping.last_seen_at_ms,
                source: mapping.source,
            });
            continue;
        };

        if !existing
            .aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(&mapping.wrong))
        {
            existing.aliases.push(mapping.wrong);
        }
        existing.frequency = existing.frequency.saturating_add(mapping.frequency);
        existing.first_seen_at_ms = existing.first_seen_at_ms.min(mapping.first_seen_at_ms);
        existing.last_seen_at_ms = existing.last_seen_at_ms.max(mapping.last_seen_at_ms);
    }

    entries.sort_by(|left, right| {
        right
            .frequency
            .cmp(&left.frequency)
            .then_with(|| right.last_seen_at_ms.cmp(&left.last_seen_at_ms))
    });
    entries
}

fn parse_csv_rows(content: &str) -> Vec<Vec<String>> {
    content.lines().map(parse_csv_row).collect()
}

fn parse_csv_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                cells.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    cells.push(current.trim().to_string());
    cells
}

#[cfg(test)]
mod tests {
    use super::{parse_csv_rows, parse_custom_dictionary_entries};

    #[test]
    fn parses_hotword_and_legacy_custom_dictionary_entries() {
        let entries = parse_custom_dictionary_entries("Claude | Cloud\n搜题 -> sootie\nAriaType");

        assert_eq!(entries.len(), 3);
        assert!(entries
            .iter()
            .any(|entry| entry.term == "Claude" && entry.aliases == vec!["Cloud"]));
        assert!(entries
            .iter()
            .any(|entry| entry.term == "sootie" && entry.aliases == vec!["搜题"]));
        assert!(entries.iter().any(|entry| entry.term == "AriaType"));
    }

    #[test]
    fn parses_quoted_csv_rows() {
        let rows =
            parse_csv_rows("wrong,corrected\n\"node, js\",Node.js\n\"say \"\"hi\"\"\",hello");

        assert_eq!(rows[0], vec!["wrong", "corrected"]);
        assert_eq!(rows[1], vec!["node, js", "Node.js"]);
        assert_eq!(rows[2], vec!["say \"hi\"", "hello"]);
    }
}
