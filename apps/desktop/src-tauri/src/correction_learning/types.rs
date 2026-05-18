use serde::{Deserialize, Serialize};

pub const CORRECTION_LEARNING_FILE_VERSION: u32 = 1;
pub const CORRECTION_SOURCE_POST_DELIVERY_EDIT: &str = "post_delivery_edit";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrectionMapping {
    pub wrong: String,
    pub corrected: String,
    pub frequency: u32,
    pub first_seen_at_ms: i64,
    pub last_seen_at_ms: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrectionLearningFile {
    pub version: u32,
    pub updated_at_ms: i64,
    pub corrections: Vec<CorrectionMapping>,
}

impl CorrectionLearningFile {
    pub fn empty(now_ms: i64) -> Self {
        Self {
            version: CORRECTION_LEARNING_FILE_VERSION,
            updated_at_ms: now_ms,
            corrections: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorrectionLearnedEvent {
    pub wrong: String,
    pub corrected: String,
    pub frequency: u32,
}

impl From<&CorrectionMapping> for CorrectionLearnedEvent {
    fn from(mapping: &CorrectionMapping) -> Self {
        Self {
            wrong: mapping.wrong.clone(),
            corrected: mapping.corrected.clone(),
            frequency: mapping.frequency,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionPair {
    pub wrong: String,
    pub corrected: String,
}

impl CorrectionPair {
    pub fn new(wrong: impl Into<String>, corrected: impl Into<String>) -> Self {
        Self {
            wrong: wrong.into(),
            corrected: corrected.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionApplyResult {
    pub text: String,
    pub applied: Vec<CorrectionPair>,
}
