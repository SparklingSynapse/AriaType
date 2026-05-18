use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::thread;
use std::time::{Duration, Instant};

use tracing::{debug, warn};

use super::diff::{extract_correction_pair, is_word_level_correction_pair};
use super::types::{
    CorrectionApplyResult, CorrectionLearningFile, CorrectionMapping, CorrectionPair,
    CORRECTION_SOURCE_POST_DELIVERY_EDIT,
};
use crate::utils::AppPaths;

const MAX_STORED_CORRECTIONS: usize = 500;
const MAX_APPLIED_CORRECTIONS: usize = 50;
const AUTO_APPLY_MIN_FREQUENCY: u32 = 2;
const LOCK_WAIT_TIMEOUT: Duration = Duration::from_millis(750);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const LOCK_STALE_AFTER: Duration = Duration::from_secs(30);

static STORE_MUTEX: LazyLock<parking_lot::Mutex<()>> =
    LazyLock::new(|| parking_lot::Mutex::new(()));

#[derive(Debug, Clone)]
pub struct CorrectionStore {
    path: PathBuf,
}

impl CorrectionStore {
    pub fn shared() -> Self {
        Self {
            path: AppPaths::correction_learning_file(),
        }
    }

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn learn_from_edit(
        &self,
        before: &str,
        after: &str,
    ) -> Result<Option<CorrectionMapping>, String> {
        let Some(pair) = extract_correction_pair(before, after) else {
            return Ok(None);
        };

        self.upsert_pair(pair)
    }

    pub fn upsert_pair(&self, pair: CorrectionPair) -> Result<Option<CorrectionMapping>, String> {
        self.with_store_lock(|| {
            let now_ms = chrono::Utc::now().timestamp_millis();
            let mut file = self.load_or_empty_unlocked(now_ms)?;

            let learned =
                if let Some(existing) = file.corrections.iter_mut().find(|mapping| {
                    mapping.wrong == pair.wrong && mapping.corrected == pair.corrected
                }) {
                    existing.frequency = existing.frequency.saturating_add(1);
                    existing.last_seen_at_ms = now_ms;
                    existing.clone()
                } else {
                    let mapping = CorrectionMapping {
                        wrong: pair.wrong,
                        corrected: pair.corrected,
                        frequency: 1,
                        first_seen_at_ms: now_ms,
                        last_seen_at_ms: now_ms,
                        source: CORRECTION_SOURCE_POST_DELIVERY_EDIT.to_string(),
                    };
                    file.corrections.push(mapping.clone());
                    mapping
                };

            file.updated_at_ms = now_ms;
            file.corrections.sort_by(|left, right| {
                right
                    .frequency
                    .cmp(&left.frequency)
                    .then_with(|| right.last_seen_at_ms.cmp(&left.last_seen_at_ms))
            });
            file.corrections.truncate(MAX_STORED_CORRECTIONS);
            self.save_unlocked(&file)?;

            Ok(Some(learned))
        })
    }

    pub fn apply_to_text(&self, text: &str) -> Result<CorrectionApplyResult, String> {
        let file = self.with_store_lock(|| {
            self.load_or_empty_unlocked(chrono::Utc::now().timestamp_millis())
        })?;
        Ok(apply_corrections_to_text(text, &file.corrections))
    }

    pub fn load_or_empty(&self, now_ms: i64) -> Result<CorrectionLearningFile, String> {
        self.with_store_lock(|| self.load_or_empty_unlocked(now_ms))
    }

    pub fn clear(&self) -> Result<(), String> {
        self.with_store_lock(|| {
            if self.path.exists() {
                std::fs::remove_file(&self.path)
                    .map_err(|e| format!("failed to clear correction memory: {e}"))?;
            }
            Ok(())
        })
    }

    pub fn ensure_file(&self) -> Result<(), String> {
        self.with_store_lock(|| {
            if self.path.exists() {
                return Ok(());
            }

            let file = CorrectionLearningFile::empty(chrono::Utc::now().timestamp_millis());
            self.save_unlocked(&file)
        })
    }

    fn load_or_empty_unlocked(&self, now_ms: i64) -> Result<CorrectionLearningFile, String> {
        if !self.path.exists() {
            return Ok(CorrectionLearningFile::empty(now_ms));
        }

        let content = std::fs::read_to_string(&self.path)
            .map_err(|e| format!("failed to read correction learning file: {e}"))?;
        serde_json::from_str::<CorrectionLearningFile>(&content)
            .map_err(|e| format!("failed to parse correction learning file: {e}"))
    }

    fn save_unlocked(&self, file: &CorrectionLearningFile) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create correction learning dir: {e}"))?;
        }

        let tmp_path = self.path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(file)
            .map_err(|e| format!("failed to serialize correction learning file: {e}"))?;
        std::fs::write(&tmp_path, content)
            .map_err(|e| format!("failed to write correction learning temp file: {e}"))?;
        replace_file(&tmp_path, &self.path)?;
        Ok(())
    }

    fn with_store_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let _process_guard = STORE_MUTEX.lock();
        let _file_guard = StoreFileLock::acquire(&self.path)?;
        operation()
    }
}

struct StoreFileLock {
    path: PathBuf,
}

impl StoreFileLock {
    fn acquire(target_path: &Path) -> Result<Self, String> {
        let lock_path = target_path.with_extension("json.lock");
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create correction memory lock dir: {e}"))?;
        }

        let started_at = Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    let _ = writeln!(file, "pid={}", std::process::id());
                    return Ok(Self { path: lock_path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    remove_stale_lock_if_needed(&lock_path);
                    if started_at.elapsed() >= LOCK_WAIT_TIMEOUT {
                        return Err("timed out waiting for correction memory lock".to_string());
                    }
                    thread::sleep(LOCK_RETRY_INTERVAL);
                }
                Err(error) => {
                    return Err(format!("failed to create correction memory lock: {error}"));
                }
            }
        }
    }
}

impl Drop for StoreFileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn remove_stale_lock_if_needed(lock_path: &Path) {
    let Ok(metadata) = std::fs::metadata(lock_path) else {
        return;
    };
    let Ok(modified_at) = metadata.modified() else {
        return;
    };
    let Ok(age) = modified_at.elapsed() else {
        return;
    };
    if age > LOCK_STALE_AFTER {
        let _ = std::fs::remove_file(lock_path);
    }
}

fn replace_file(tmp_path: &Path, target_path: &Path) -> Result<(), String> {
    match std::fs::rename(tmp_path, target_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            std::fs::remove_file(target_path)
                .map_err(|e| format!("failed to remove old correction learning file: {e}"))?;
            std::fs::rename(tmp_path, target_path)
                .map_err(|e| format!("failed to replace correction learning file: {e}"))
        }
        Err(error) => Err(format!(
            "failed to replace correction learning file: {error}"
        )),
    }
}

pub fn apply_corrections_to_text(
    text: &str,
    mappings: &[CorrectionMapping],
) -> CorrectionApplyResult {
    let mut result = text.to_string();
    let mut applied = Vec::new();
    let mut mappings = mappings.to_vec();
    mappings.sort_by(|left, right| {
        right
            .wrong
            .chars()
            .count()
            .cmp(&left.wrong.chars().count())
            .then_with(|| right.frequency.cmp(&left.frequency))
    });

    for mapping in mappings
        .iter()
        .filter(|mapping| mapping.frequency >= AUTO_APPLY_MIN_FREQUENCY)
        .filter(|mapping| is_word_level_correction_pair(&mapping.wrong, &mapping.corrected))
        .take(MAX_APPLIED_CORRECTIONS)
    {
        let next = replace_mapping(&result, &mapping.wrong, &mapping.corrected);
        if next != result {
            applied.push(CorrectionPair::new(
                mapping.wrong.clone(),
                mapping.corrected.clone(),
            ));
            result = next;
        }
    }

    CorrectionApplyResult {
        text: result,
        applied,
    }
}

fn replace_mapping(text: &str, wrong: &str, corrected: &str) -> String {
    if wrong.is_empty() || wrong == corrected {
        return text.to_string();
    }

    let requires_boundary = wrong.chars().all(is_ascii_word_char);
    let mut result = String::with_capacity(text.len());
    let mut last = 0;

    for (index, _) in text.match_indices(wrong) {
        let end = index + wrong.len();
        if requires_boundary && !has_word_boundaries(text, index, end) {
            continue;
        }

        result.push_str(&text[last..index]);
        result.push_str(corrected);
        last = end;
    }

    if last == 0 {
        return text.to_string();
    }

    result.push_str(&text[last..]);
    result
}

fn has_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    let before_ok = text[..start]
        .chars()
        .next_back()
        .is_none_or(|c| !is_ascii_word_char(c));
    let after_ok = text[end..]
        .chars()
        .next()
        .is_none_or(|c| !is_ascii_word_char(c));
    before_ok && after_ok
}

fn is_ascii_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

pub fn apply_shared_corrections_best_effort(text: &str) -> String {
    match CorrectionStore::shared().apply_to_text(text) {
        Ok(result) => {
            if !result.applied.is_empty() {
                debug!(
                    applied = result.applied.len(),
                    "correction_learning_applied"
                );
            }
            result.text
        }
        Err(error) => {
            warn!(error = %error, "correction_learning_apply_failed");
            text.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_corrections_to_text, CorrectionStore, AUTO_APPLY_MIN_FREQUENCY};
    use crate::correction_learning::types::{
        CorrectionLearningFile, CorrectionMapping, CorrectionPair,
        CORRECTION_LEARNING_FILE_VERSION, CORRECTION_SOURCE_POST_DELIVERY_EDIT,
    };
    use tempfile::TempDir;

    fn mapping(wrong: &str, corrected: &str) -> CorrectionMapping {
        mapping_with_frequency(wrong, corrected, AUTO_APPLY_MIN_FREQUENCY)
    }

    fn mapping_with_frequency(wrong: &str, corrected: &str, frequency: u32) -> CorrectionMapping {
        CorrectionMapping {
            wrong: wrong.to_string(),
            corrected: corrected.to_string(),
            frequency,
            first_seen_at_ms: 1,
            last_seen_at_ms: 1,
            source: CORRECTION_SOURCE_POST_DELIVERY_EDIT.to_string(),
        }
    }

    #[test]
    fn upserts_and_increments_correction_mapping() {
        let dir = TempDir::new().unwrap();
        let store = CorrectionStore::new(dir.path().join("corrections.json"));

        let first = store
            .upsert_pair(CorrectionPair::new("分析", "分词"))
            .unwrap()
            .unwrap();
        let second = store
            .upsert_pair(CorrectionPair::new("分析", "分词"))
            .unwrap()
            .unwrap();

        assert_eq!(first.frequency, 1);
        assert_eq!(second.frequency, 2);

        let file = store.load_or_empty(0).unwrap();
        assert_eq!(file.corrections.len(), 1);
        assert_eq!(file.corrections[0].wrong, "分析");
        assert_eq!(file.corrections[0].corrected, "分词");
        assert_eq!(file.corrections[0].frequency, 2);
    }

    #[test]
    fn does_not_apply_single_observation_mapping() {
        let result = apply_corrections_to_text(
            "这个分析错误需要修复",
            &[mapping_with_frequency("分析", "分词", 1)],
        );

        assert_eq!(result.text, "这个分析错误需要修复");
        assert!(result.applied.is_empty());
    }

    #[test]
    fn applies_ascii_correction_only_on_word_boundaries() {
        let result =
            apply_corrections_to_text("right code, bright idea", &[mapping("right", "write")]);

        assert_eq!(result.text, "write code, bright idea");
        assert_eq!(result.applied.len(), 1);
    }

    #[test]
    fn applies_cjk_correction_mapping() {
        let result = apply_corrections_to_text("这个分析错误需要修复", &[mapping("分析", "分词")]);

        assert_eq!(result.text, "这个分词错误需要修复");
    }

    #[test]
    fn skips_non_word_level_mappings_when_applying() {
        let result = apply_corrections_to_text(
            "hello! 你好，",
            &[
                mapping("!", "?"),
                mapping("hello!", "hi!"),
                mapping("，", "。"),
            ],
        );

        assert_eq!(result.text, "hello! 你好，");
        assert!(result.applied.is_empty());
    }

    #[test]
    fn clear_removes_correction_memory_file() {
        let dir = TempDir::new().unwrap();
        let store = CorrectionStore::new(dir.path().join("corrections.json"));

        store
            .upsert_pair(CorrectionPair::new("分析", "分词"))
            .unwrap()
            .unwrap();
        assert!(store.path().exists());

        store.clear().unwrap();
        assert!(!store.path().exists());
    }

    #[test]
    fn ensure_file_creates_empty_correction_memory_file() {
        let dir = TempDir::new().unwrap();
        let store = CorrectionStore::new(dir.path().join("corrections.json"));

        store.ensure_file().unwrap();

        let content = std::fs::read_to_string(store.path()).unwrap();
        let file = serde_json::from_str::<CorrectionLearningFile>(&content).unwrap();
        assert_eq!(file.version, CORRECTION_LEARNING_FILE_VERSION);
        assert!(file.corrections.is_empty());
    }

    #[test]
    fn ensure_file_keeps_existing_file_without_parsing() {
        let dir = TempDir::new().unwrap();
        let store = CorrectionStore::new(dir.path().join("corrections.json"));
        std::fs::write(store.path(), "{not json").unwrap();

        store.ensure_file().unwrap();

        assert_eq!(std::fs::read_to_string(store.path()).unwrap(), "{not json");
    }

    #[test]
    fn learns_from_user_edits_then_applies_after_repeat_observation() {
        let dir = TempDir::new().unwrap();
        let store = CorrectionStore::new(dir.path().join("corrections.json"));

        let first = store
            .learn_from_edit("这个分析错误需要修复", "这个分词错误需要修复")
            .unwrap()
            .unwrap();
        assert_eq!(first.wrong, "分析");
        assert_eq!(first.corrected, "分词");
        assert_eq!(first.frequency, 1);

        let first_apply = store.apply_to_text("这个分析错误需要复查").unwrap();
        assert_eq!(first_apply.text, "这个分析错误需要复查");
        assert!(first_apply.applied.is_empty());

        let second = store
            .learn_from_edit("这个分析错误需要修复", "这个分词错误需要修复")
            .unwrap()
            .unwrap();
        assert_eq!(second.frequency, 2);

        let second_apply = store.apply_to_text("这个分析错误需要复查").unwrap();
        assert_eq!(second_apply.text, "这个分词错误需要复查");
        assert_eq!(second_apply.applied.len(), 1);
    }
}
