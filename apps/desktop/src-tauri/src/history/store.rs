use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, Local, TimeZone};
use rusqlite::{params, Connection, Result as SqlResult};

use super::models::{
    DailyUsage, DashboardStats, EngineUsage, HistoryFilter, NewTranscriptionEntry,
    TranscriptionEntry,
};
use crate::utils::AppPaths;

const CREATE_TABLE_SQL: &str = "\
CREATE TABLE IF NOT EXISTS transcription_history (\
    id TEXT PRIMARY KEY,\
    created_at INTEGER NOT NULL,\
    raw_text TEXT NOT NULL,\
    final_text TEXT NOT NULL,\
    stt_engine TEXT NOT NULL,\
    stt_model TEXT,\
    language TEXT,\
    audio_duration_ms INTEGER,\
    stt_duration_ms INTEGER,\
    polish_duration_ms INTEGER,\
    total_duration_ms INTEGER,\
    polish_applied INTEGER NOT NULL DEFAULT 0,\
    polish_engine TEXT,\
    is_cloud INTEGER NOT NULL DEFAULT 0,\
    audio_path TEXT,\
    status TEXT NOT NULL DEFAULT 'success',\
    error TEXT\
)";

const CREATE_INDEX_SQL: &str = "\
CREATE INDEX IF NOT EXISTS idx_history_created_at ON transcription_history(created_at)";

pub struct HistoryStore {
    conn: parking_lot::Mutex<Connection>,
}

#[derive(Debug, Clone)]
struct DashboardEntry {
    created_at: i64,
    final_text: String,
    audio_duration_ms: Option<i64>,
    stt_duration_ms: Option<i64>,
    polish_applied: bool,
    is_cloud: bool,
    stt_engine: String,
}

/// Updates to apply to an entry after retry.
#[derive(Debug, Clone)]
pub struct EntryUpdates {
    pub raw_text: String,
    pub final_text: String,
    pub stt_engine: String,
    pub stt_model: Option<String>,
    pub language: Option<String>,
    pub stt_duration_ms: Option<i64>,
    pub polish_duration_ms: Option<i64>,
    pub polish_applied: bool,
    pub polish_engine: Option<String>,
    pub is_cloud: bool,
}

impl HistoryStore {
    pub fn new() -> Result<Self, String> {
        let db_path = AppPaths::data_dir().join("transcription_history.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn =
            Connection::open(&db_path).map_err(|e| format!("failed to open database: {e}"))?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, String> {
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| format!("failed to set pragmas: {e}"))?;

        Self::run_migrations(&conn)?;

        Ok(Self {
            conn: parking_lot::Mutex::new(conn),
        })
    }

    fn run_migrations(conn: &Connection) -> Result<(), String> {
        let current_version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap_or(0);

        if current_version < 1 {
            conn.execute_batch(
                format!(
                    "BEGIN;
                     {CREATE_TABLE_SQL};
                     {CREATE_INDEX_SQL};
                     PRAGMA user_version = 1;
                     COMMIT;"
                )
                .as_str(),
            )
            .map_err(|e| format!("migration v1 failed: {e}"))?;
        }

        // Migration v2: Add audio_path, status, error columns (only if not present)
        // These columns are now in CREATE_TABLE_SQL, but existing DBs may need migration
        if current_version < 2 {
            // Check if columns already exist (handles case where schema includes them)
            let has_audio_path: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('transcription_history') WHERE name='audio_path'",
                    [],
                    |row| row.get::<_, i32>(0),
                )
                .unwrap_or(0) > 0;

            if !has_audio_path {
                conn.execute_batch(
                    "BEGIN;
                     ALTER TABLE transcription_history ADD COLUMN audio_path TEXT;
                     ALTER TABLE transcription_history ADD COLUMN status TEXT NOT NULL DEFAULT 'success';
                     ALTER TABLE transcription_history ADD COLUMN error TEXT;
                     CREATE INDEX IF NOT EXISTS idx_history_status ON transcription_history(status);
                     COMMIT;",
                )
                .map_err(|e| format!("migration v2 failed: {e}"))?;
            }
            // Always set version to 2 after schema is complete
            conn.execute("PRAGMA user_version = 2", [])
                .map_err(|e| format!("failed to set user_version: {e}"))?;
        }

        Ok(())
    }

    pub fn insert(&self, entry: NewTranscriptionEntry) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp_millis();

        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO transcription_history \
             (id, created_at, raw_text, final_text, stt_engine, stt_model, language, \
              audio_duration_ms, stt_duration_ms, polish_duration_ms, total_duration_ms, \
              polish_applied, polish_engine, is_cloud, audio_path, status, error) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                id,
                created_at,
                entry.raw_text,
                entry.final_text,
                entry.stt_engine,
                entry.stt_model,
                entry.language,
                entry.audio_duration_ms,
                entry.stt_duration_ms,
                entry.polish_duration_ms,
                entry.total_duration_ms,
                entry.polish_applied as i32,
                entry.polish_engine,
                entry.is_cloud as i32,
                entry.audio_path,
                entry.status,
                entry.error,
            ],
        )
        .map_err(|e| format!("failed to insert history: {e}"))?;

        Ok(id)
    }

    pub fn get_history(&self, filter: &HistoryFilter) -> Result<Vec<TranscriptionEntry>, String> {
        let mut sql = String::from(
            "SELECT id, created_at, raw_text, final_text, stt_engine, \
             stt_model, language, audio_duration_ms, stt_duration_ms, polish_duration_ms, \
             total_duration_ms, polish_applied, polish_engine, is_cloud, audio_path, status, error \
             FROM transcription_history WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(ref search) = filter.search {
            sql.push_str(&format!(" AND final_text LIKE ?{param_idx}"));
            param_values.push(Box::new(format!("%{search}%")));
            param_idx += 1;
        }

        if let Some(ref engine) = filter.engine {
            if engine == "local" {
                sql.push_str(" AND is_cloud = 0");
            } else if engine == "cloud" {
                sql.push_str(" AND is_cloud = 1");
            } else {
                sql.push_str(&format!(" AND stt_engine = ?{param_idx}"));
                param_values.push(Box::new(engine.clone()));
                param_idx += 1;
            }
        }

        if let Some(ref status) = filter.status {
            sql.push_str(&format!(" AND status = ?{param_idx}"));
            param_values.push(Box::new(status.clone()));
            param_idx += 1;
        }

        if let Some(date_from) = filter.date_from {
            sql.push_str(&format!(" AND created_at >= ?{param_idx}"));
            param_values.push(Box::new(date_from));
            param_idx += 1;
        }

        if let Some(date_to) = filter.date_to {
            sql.push_str(&format!(" AND created_at <= ?{param_idx}"));
            param_values.push(Box::new(date_to));
            param_idx += 1;
        }

        sql.push_str(" ORDER BY created_at DESC");

        let limit = filter.limit.unwrap_or(50);
        sql.push_str(&format!(" LIMIT ?{param_idx}"));
        param_values.push(Box::new(limit));
        param_idx += 1;

        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET ?{param_idx}"));
            param_values.push(Box::new(offset));
        }

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("failed to prepare query: {e}"))?;

        let entries = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(TranscriptionEntry {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    raw_text: row.get(2)?,
                    final_text: row.get(3)?,
                    stt_engine: row.get(4)?,
                    stt_model: row.get(5)?,
                    language: row.get(6)?,
                    audio_duration_ms: row.get(7)?,
                    stt_duration_ms: row.get(8)?,
                    polish_duration_ms: row.get(9)?,
                    total_duration_ms: row.get(10)?,
                    polish_applied: row.get::<_, i32>(11)? != 0,
                    polish_engine: row.get(12)?,
                    is_cloud: row.get::<_, i32>(13)? != 0,
                    audio_path: row.get(14)?,
                    status: row.get::<_, String>(15)?,
                    error: row.get(16)?,
                })
            })
            .map_err(|e| format!("failed to query history: {e}"))?
            .collect::<SqlResult<Vec<_>>>()
            .map_err(|e| format!("failed to collect history: {e}"))?;

        Ok(entries)
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), String> {
        // First get the audio_path to delete the audio file
        let audio_path = self.get_audio_path(id)?;
        if let Some(path) = audio_path {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!(error = %e, path = %path, "audio_file_deletion_failed");
            }
        }

        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM transcription_history WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("failed to delete history entry: {e}"))?;
        Ok(())
    }

    /// Get a single entry by ID.
    pub fn get_entry(&self, id: &str) -> Result<Option<TranscriptionEntry>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, created_at, raw_text, final_text, stt_engine, \
                 stt_model, language, audio_duration_ms, stt_duration_ms, polish_duration_ms, \
                 total_duration_ms, polish_applied, polish_engine, is_cloud, audio_path, status, error \
                 FROM transcription_history WHERE id = ?1",
            )
            .map_err(|e| format!("failed to prepare query: {e}"))?;

        let result = stmt.query_row(params![id], |row| {
            Ok(TranscriptionEntry {
                id: row.get(0)?,
                created_at: row.get(1)?,
                raw_text: row.get(2)?,
                final_text: row.get(3)?,
                stt_engine: row.get(4)?,
                stt_model: row.get(5)?,
                language: row.get(6)?,
                audio_duration_ms: row.get(7)?,
                stt_duration_ms: row.get(8)?,
                polish_duration_ms: row.get(9)?,
                total_duration_ms: row.get(10)?,
                polish_applied: row.get::<_, i32>(11)? != 0,
                polish_engine: row.get(12)?,
                is_cloud: row.get::<_, i32>(13)? != 0,
                audio_path: row.get(14)?,
                status: row.get::<_, String>(15)?,
                error: row.get(16)?,
            })
        });

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("failed to get entry: {e}")),
        }
    }

    /// Get just the audio_path for an entry.
    pub fn get_audio_path(&self, id: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock();
        let result: Result<Option<String>, rusqlite::Error> = conn.query_row(
            "SELECT audio_path FROM transcription_history WHERE id = ?1",
            params![id],
            |row| row.get(0),
        );

        match result {
            Ok(path) => Ok(path),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(format!("failed to get audio path: {e}")),
        }
    }

    /// Update an entry after successful retry.
    pub fn update_entry(&self, id: &str, updates: EntryUpdates) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transcription_history SET \
             raw_text = ?1, final_text = ?2, stt_engine = ?3, stt_model = ?4, \
             language = ?5, stt_duration_ms = ?6, polish_duration_ms = ?7, polish_applied = ?8, \
             polish_engine = ?9, is_cloud = ?10, status = 'success', error = NULL \
             WHERE id = ?11",
            params![
                updates.raw_text,
                updates.final_text,
                updates.stt_engine,
                updates.stt_model,
                updates.language,
                updates.stt_duration_ms,
                updates.polish_duration_ms,
                updates.polish_applied as i32,
                updates.polish_engine,
                updates.is_cloud as i32,
                id,
            ],
        )
        .map_err(|e| format!("failed to update entry: {e}"))?;
        Ok(())
    }

    /// Mark an entry as failed.
    pub fn mark_error(&self, id: &str, error: &str) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE transcription_history SET status = 'error', error = ?1 WHERE id = ?2",
            params![error, id],
        )
        .map_err(|e| format!("failed to mark entry as error: {e}"))?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM transcription_history", [])
            .map_err(|e| format!("failed to clear history: {e}"))?;
        Ok(())
    }

    pub fn get_dashboard_stats(&self) -> Result<DashboardStats, String> {
        let entries = self.load_dashboard_entries(None)?;
        Ok(Self::build_dashboard_stats(&entries))
    }

    pub fn get_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>, String> {
        let cutoff = Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .single()
            .unwrap()
            - Duration::days(i64::from(days.saturating_sub(1)));
        let entries = self.load_dashboard_entries(Some(cutoff.timestamp_millis()))?;
        Self::build_daily_usage(&entries, days)
    }

    fn build_daily_usage(
        entries: &[DashboardEntry],
        range: u32,
    ) -> Result<Vec<DailyUsage>, String> {
        let mut grouped = BTreeMap::<chrono::NaiveDate, DailyUsage>::new();

        for entry in entries {
            let date = Self::local_date_from_timestamp(entry.created_at);
            let point = grouped.entry(date).or_insert_with(|| DailyUsage {
                date: date.format("%Y-%m-%d").to_string(),
                count: 0,
                audio_ms: 0,
                output_units: 0,
            });
            point.count += 1;
            point.audio_ms += entry.audio_duration_ms.unwrap_or(0);
            point.output_units += Self::approximate_output_units(&entry.final_text);
        }

        let mut filled = Vec::with_capacity(range as usize);
        let today = Local::now().date_naive();

        for i in (0..range).rev() {
            let date = today - Duration::days(i64::from(i));
            if let Some(point) = grouped.remove(&date) {
                filled.push(point);
            } else {
                filled.push(DailyUsage {
                    date: date.format("%Y-%m-%d").to_string(),
                    count: 0,
                    audio_ms: 0,
                    output_units: 0,
                });
            }
        }

        Ok(filled)
    }

    pub fn get_engine_usage(&self) -> Result<Vec<EngineUsage>, String> {
        let entries = self.load_dashboard_entries(None)?;
        let mut grouped = BTreeMap::<String, (i64, i64, i64)>::new();

        for entry in entries {
            let stats = grouped.entry(entry.stt_engine).or_insert((0, 0, 0));
            stats.0 += 1;
            if let Some(stt_ms) = entry.stt_duration_ms {
                stats.1 += stt_ms;
                stats.2 += 1;
            }
        }

        let mut result = grouped
            .into_iter()
            .map(|(engine, (count, stt_sum, stt_count))| EngineUsage {
                engine,
                count,
                avg_stt_ms: if stt_count > 0 {
                    Some(stt_sum / stt_count)
                } else {
                    None
                },
            })
            .collect::<Vec<_>>();

        result.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.engine.cmp(&b.engine)));
        Ok(result)
    }

    pub fn cleanup_old_entries(&self, max_age_days: u64) -> Result<u64, String> {
        let cutoff = chrono::Utc::now()
            - chrono::Duration::days(i64::try_from(max_age_days).unwrap_or(i64::MAX));
        let cutoff_ms = cutoff.timestamp_millis();

        let conn = self.conn.lock();
        let deleted = conn
            .execute(
                "DELETE FROM transcription_history WHERE created_at < ?1",
                params![cutoff_ms],
            )
            .map_err(|e| format!("failed to cleanup old entries: {e}"))?;

        Ok(deleted as u64)
    }

    fn load_dashboard_entries(&self, since_ms: Option<i64>) -> Result<Vec<DashboardEntry>, String> {
        let conn = self.conn.lock();
        match since_ms {
            Some(cutoff_ms) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT created_at, final_text, audio_duration_ms, stt_duration_ms, \
                         polish_applied, is_cloud, stt_engine FROM transcription_history \
                         WHERE created_at >= ?1 ORDER BY created_at ASC",
                    )
                    .map_err(|e| format!("failed to prepare dashboard query: {e}"))?;
                let rows = stmt
                    .query_map(params![cutoff_ms], Self::map_dashboard_entry)
                    .map_err(|e| format!("failed to query dashboard rows: {e}"))?;

                let mut result = Vec::new();
                for row in rows {
                    result.push(row.map_err(|e| format!("failed to read dashboard row: {e}"))?);
                }
                Ok(result)
            }
            None => {
                let mut stmt = conn
                    .prepare(
                        "SELECT created_at, final_text, audio_duration_ms, stt_duration_ms, \
                         polish_applied, is_cloud, stt_engine FROM transcription_history \
                         ORDER BY created_at ASC",
                    )
                    .map_err(|e| format!("failed to prepare dashboard query: {e}"))?;
                let rows = stmt
                    .query_map([], Self::map_dashboard_entry)
                    .map_err(|e| format!("failed to query dashboard rows: {e}"))?;

                let mut result = Vec::new();
                for row in rows {
                    result.push(row.map_err(|e| format!("failed to read dashboard row: {e}"))?);
                }
                Ok(result)
            }
        }
    }

    fn build_dashboard_stats(entries: &[DashboardEntry]) -> DashboardStats {
        let today = Local::now().date_naive();
        let last_7_cutoff = today - Duration::days(6);
        let mut total_chars = 0_i64;
        let mut total_output_units = 0_i64;
        let mut total_audio_ms = 0_i64;
        let mut audio_count = 0_i64;
        let mut total_stt_ms = 0_i64;
        let mut stt_count = 0_i64;
        let mut today_count = 0_i64;
        let mut local_count = 0_i64;
        let mut cloud_count = 0_i64;
        let mut polish_count = 0_i64;
        let mut last_7_days_count = 0_i64;
        let mut last_7_days_audio_ms = 0_i64;
        let mut last_7_days_output_units = 0_i64;
        let mut active_dates = BTreeSet::new();

        for entry in entries {
            let local_date = Self::local_date_from_timestamp(entry.created_at);
            let output_units = Self::approximate_output_units(&entry.final_text);
            let char_count = entry.final_text.chars().count() as i64;

            total_chars += char_count;
            total_output_units += output_units;

            if local_date == today {
                today_count += 1;
            }
            if local_date >= last_7_cutoff {
                last_7_days_count += 1;
                last_7_days_output_units += output_units;
            }

            if let Some(audio_ms) = entry.audio_duration_ms {
                total_audio_ms += audio_ms;
                audio_count += 1;
                if local_date >= last_7_cutoff {
                    last_7_days_audio_ms += audio_ms;
                }
            }

            if let Some(stt_ms) = entry.stt_duration_ms {
                total_stt_ms += stt_ms;
                stt_count += 1;
            }

            if entry.is_cloud {
                cloud_count += 1;
            } else {
                local_count += 1;
            }
            if entry.polish_applied {
                polish_count += 1;
            }

            active_dates.insert(local_date);
        }

        let total_count = entries.len() as i64;
        let (current_streak_days, longest_streak_days) =
            Self::calculate_streaks(&active_dates, today);

        DashboardStats {
            total_count,
            today_count,
            total_chars,
            total_output_units,
            total_audio_ms,
            avg_stt_ms: if stt_count > 0 {
                Some(total_stt_ms / stt_count)
            } else {
                None
            },
            avg_audio_ms: if audio_count > 0 {
                Some(total_audio_ms / audio_count)
            } else {
                None
            },
            avg_output_units: if total_count > 0 {
                Some(total_output_units as f64 / total_count as f64)
            } else {
                None
            },
            local_count,
            cloud_count,
            polish_count,
            active_days: active_dates.len() as i64,
            current_streak_days,
            longest_streak_days,
            last_7_days_count,
            last_7_days_audio_ms,
            last_7_days_output_units,
        }
    }

    fn map_dashboard_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<DashboardEntry> {
        Ok(DashboardEntry {
            created_at: row.get(0)?,
            final_text: row.get(1)?,
            audio_duration_ms: row.get(2)?,
            stt_duration_ms: row.get(3)?,
            polish_applied: row.get::<_, i32>(4)? != 0,
            is_cloud: row.get::<_, i32>(5)? != 0,
            stt_engine: row.get(6)?,
        })
    }

    fn calculate_streaks(
        active_dates: &BTreeSet<chrono::NaiveDate>,
        today: chrono::NaiveDate,
    ) -> (i64, i64) {
        if active_dates.is_empty() {
            return (0, 0);
        }

        let sorted_dates = active_dates.iter().copied().collect::<Vec<_>>();
        let mut longest_streak = 1_i64;
        let mut current_run = 1_i64;

        for window in sorted_dates.windows(2) {
            let previous = window[0];
            let current = window[1];
            if current == previous + Duration::days(1) {
                current_run += 1;
                longest_streak = longest_streak.max(current_run);
            } else {
                current_run = 1;
            }
        }

        let latest_date = *sorted_dates.last().unwrap();
        if latest_date < today - Duration::days(1) {
            return (0, longest_streak);
        }

        let mut current_streak = 0_i64;
        let mut cursor = latest_date;
        while active_dates.contains(&cursor) {
            current_streak += 1;
            cursor -= Duration::days(1);
        }

        (current_streak, longest_streak)
    }

    fn local_date_from_timestamp(timestamp_ms: i64) -> chrono::NaiveDate {
        Local
            .timestamp_millis_opt(timestamp_ms)
            .single()
            .unwrap()
            .date_naive()
    }

    fn approximate_output_units(text: &str) -> i64 {
        let mut units = 0_i64;
        let mut in_word = false;

        for ch in text.chars() {
            if ch.is_whitespace() {
                in_word = false;
                continue;
            }

            if Self::is_cjk_ideograph(ch) {
                units += 1;
                in_word = false;
                continue;
            }

            if ch.is_alphanumeric() {
                if !in_word {
                    units += 1;
                    in_word = true;
                }
                continue;
            }

            in_word = false;
        }

        units
    }

    fn is_cjk_ideograph(ch: char) -> bool {
        matches!(
            ch as u32,
            0x3400..=0x4DBF
                | 0x4E00..=0x9FFF
                | 0xF900..=0xFAFF
                | 0x20000..=0x2A6DF
                | 0x2A700..=0x2B73F
                | 0x2B740..=0x2B81F
                | 0x2B820..=0x2CEAF
                | 0x2CEB0..=0x2EBEF
                | 0x30000..=0x3134F
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> HistoryStore {
        HistoryStore::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn timestamp_for_day_offset(days_ago: i64, hour: u32) -> i64 {
        let date = Local::now().date_naive() - Duration::days(days_ago);
        date.and_hms_opt(hour, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .single()
            .unwrap()
            .timestamp_millis()
    }

    fn insert_entry(
        store: &HistoryStore,
        id: &str,
        created_at: i64,
        final_text: &str,
        audio_duration_ms: Option<i64>,
        stt_duration_ms: Option<i64>,
        polish_applied: bool,
        is_cloud: bool,
        stt_engine: &str,
    ) {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT INTO transcription_history \
             (id, created_at, raw_text, final_text, stt_engine, stt_model, language, \
              audio_duration_ms, stt_duration_ms, polish_duration_ms, total_duration_ms, \
              polish_applied, polish_engine, is_cloud) \
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?7, NULL, NULL, ?8, NULL, ?9)",
            params![
                id,
                created_at,
                final_text,
                final_text,
                stt_engine,
                audio_duration_ms,
                stt_duration_ms,
                polish_applied as i32,
                is_cloud as i32,
            ],
        )
        .unwrap();
    }

    #[test]
    fn dashboard_stats_aggregate_multilingual_usage_and_streaks() {
        let store = test_store();
        insert_entry(
            &store,
            "entry-1",
            timestamp_for_day_offset(2, 10),
            "draft release note",
            Some(12_000),
            Some(600),
            true,
            false,
            "Whisper",
        );
        insert_entry(
            &store,
            "entry-2",
            timestamp_for_day_offset(1, 11),
            "你好世界",
            Some(8_000),
            Some(500),
            false,
            true,
            "Volcengine",
        );
        insert_entry(
            &store,
            "entry-3",
            timestamp_for_day_offset(0, 9),
            "sprint planning notes",
            Some(6_000),
            Some(300),
            true,
            false,
            "Whisper",
        );

        let stats = store.get_dashboard_stats().unwrap();

        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.today_count, 1);
        assert_eq!(stats.total_chars, 43);
        assert_eq!(stats.total_output_units, 10);
        assert_eq!(stats.total_audio_ms, 26_000);
        assert_eq!(stats.avg_stt_ms, Some(466));
        assert_eq!(stats.avg_audio_ms, Some(8_666));
        assert_eq!(stats.avg_output_units, Some(10.0 / 3.0));
        assert_eq!(stats.local_count, 2);
        assert_eq!(stats.cloud_count, 1);
        assert_eq!(stats.polish_count, 2);
        assert_eq!(stats.active_days, 3);
        assert_eq!(stats.current_streak_days, 3);
        assert_eq!(stats.longest_streak_days, 3);
        assert_eq!(stats.last_7_days_count, 3);
        assert_eq!(stats.last_7_days_audio_ms, 26_000);
        assert_eq!(stats.last_7_days_output_units, 10);
    }

    #[test]
    fn daily_usage_includes_output_units_and_fills_missing_days() {
        let store = test_store();
        insert_entry(
            &store,
            "entry-1",
            timestamp_for_day_offset(2, 10),
            "alpha beta",
            Some(10_000),
            Some(400),
            false,
            false,
            "Whisper",
        );
        insert_entry(
            &store,
            "entry-2",
            timestamp_for_day_offset(0, 9),
            "你好",
            Some(4_000),
            Some(200),
            false,
            true,
            "Volcengine",
        );

        let usage = store.get_daily_usage(3).unwrap();

        assert_eq!(usage.len(), 3);
        assert_eq!(usage[0].count, 1);
        assert_eq!(usage[0].audio_ms, 10_000);
        assert_eq!(usage[0].output_units, 2);
        assert_eq!(usage[1].count, 0);
        assert_eq!(usage[1].audio_ms, 0);
        assert_eq!(usage[1].output_units, 0);
        assert_eq!(usage[2].count, 1);
        assert_eq!(usage[2].audio_ms, 4_000);
        assert_eq!(usage[2].output_units, 2);
    }

    #[test]
    fn engine_usage_reports_average_latency() {
        let store = test_store();
        insert_entry(
            &store,
            "entry-1",
            timestamp_for_day_offset(1, 10),
            "alpha beta",
            Some(10_000),
            Some(400),
            false,
            false,
            "Whisper",
        );
        insert_entry(
            &store,
            "entry-2",
            timestamp_for_day_offset(0, 9),
            "gamma delta",
            Some(8_000),
            Some(600),
            false,
            false,
            "Whisper",
        );
        insert_entry(
            &store,
            "entry-3",
            timestamp_for_day_offset(0, 11),
            "你好世界",
            Some(5_000),
            Some(300),
            false,
            true,
            "Volcengine",
        );

        let usage = store.get_engine_usage().unwrap();

        assert_eq!(usage.len(), 2);
        assert_eq!(usage[0].engine, "Whisper");
        assert_eq!(usage[0].count, 2);
        assert_eq!(usage[0].avg_stt_ms, Some(500));
        assert_eq!(usage[1].engine, "Volcengine");
        assert_eq!(usage[1].count, 1);
        assert_eq!(usage[1].avg_stt_ms, Some(300));
    }

    /// Helper to insert an entry with error state for retry tests
    fn insert_error_entry(
        store: &HistoryStore,
        id: &str,
        created_at: i64,
        audio_path: Option<&str>,
    ) {
        let conn = store.conn.lock();
        conn.execute(
            "INSERT INTO transcription_history \
             (id, created_at, raw_text, final_text, stt_engine, audio_path, status, error) \
             VALUES (?1, ?2, '', '', 'Whisper', ?3, 'error', 'Initial failure')",
            params![id, created_at, audio_path],
        )
        .unwrap();
    }

    #[test]
    fn mark_error_sets_status_and_error_message() {
        let store = test_store();
        insert_entry(
            &store,
            "entry-1",
            timestamp_for_day_offset(0, 10),
            "original text",
            Some(10_000),
            Some(500),
            false,
            false,
            "Whisper",
        );

        // Mark as error
        store
            .mark_error("entry-1", "Transcription failed: empty result")
            .unwrap();

        // Verify status changed to error
        let entry = store.get_entry("entry-1").unwrap().unwrap();
        assert_eq!(entry.status, "error");
        assert_eq!(
            entry.error,
            Some("Transcription failed: empty result".to_string())
        );
    }

    #[test]
    fn update_entry_clears_error_and_sets_success() {
        let store = test_store();
        insert_error_entry(
            &store,
            "entry-1",
            timestamp_for_day_offset(0, 10),
            Some("/tmp/audio.wav"),
        );

        // Verify initial state
        let before = store.get_entry("entry-1").unwrap().unwrap();
        assert_eq!(before.status, "error");
        assert_eq!(before.error, Some("Initial failure".to_string()));

        // Update after successful retry
        let updates = EntryUpdates {
            raw_text: "retry result".to_string(),
            final_text: "Retry Result".to_string(),
            stt_engine: "Whisper".to_string(),
            stt_model: Some("base".to_string()),
            language: Some("en-US".to_string()),
            stt_duration_ms: Some(450),
            polish_duration_ms: Some(100),
            polish_applied: true,
            polish_engine: Some("cloud".to_string()),
            is_cloud: false,
        };
        store.update_entry("entry-1", updates).unwrap();

        // Verify status changed to success
        let after = store.get_entry("entry-1").unwrap().unwrap();
        assert_eq!(after.status, "success");
        assert_eq!(after.error, None);
        assert_eq!(after.raw_text, "retry result");
        assert_eq!(after.final_text, "Retry Result");
        assert_eq!(after.stt_duration_ms, Some(450));
    }

    #[test]
    fn get_entry_returns_none_for_nonexistent_id() {
        let store = test_store();
        let result = store.get_entry("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_entry_includes_audio_path_for_retry() {
        let store = test_store();
        insert_error_entry(
            &store,
            "entry-1",
            timestamp_for_day_offset(0, 10),
            Some("/path/to/audio.wav"),
        );

        let entry = store.get_entry("entry-1").unwrap().unwrap();
        assert_eq!(entry.audio_path, Some("/path/to/audio.wav".to_string()));
        assert_eq!(entry.status, "error");
    }
}
