use std::sync::atomic::{AtomicU32, Ordering};

pub struct LevelMeter {
    level: AtomicU32,
}

impl LevelMeter {
    pub fn new() -> Self {
        Self {
            level: AtomicU32::new(0),
        }
    }

    pub fn set_level(&self, level: u32) {
        self.level.store(level, Ordering::SeqCst);
    }

    pub fn get_level(&self) -> u32 {
        self.level.load(Ordering::SeqCst)
    }

    pub fn normalize(db: f32) -> u32 {
        let normalized = ((db + 60.0) / 60.0 * 100.0).clamp(0.0, 100.0);
        normalized as u32
    }
}

impl Default for LevelMeter {
    fn default() -> Self {
        Self::new()
    }
}
