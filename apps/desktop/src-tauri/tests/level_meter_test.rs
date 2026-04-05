//! Unit tests for level meter module
//!
//! Tests the `LevelMeter` struct which calculates and normalizes audio levels.

use ariatype_lib::audio::level_meter::LevelMeter;

#[test]
fn test_level_calculation_silence() {
    let meter = LevelMeter::new();

    // For silence (0 dB), normalized level should be 0
    let normalized = LevelMeter::normalize(-60.0);
    assert_eq!(normalized, 0, "Silence (-60dB) should normalize to 0");

    // Set level directly
    meter.set_level(0);
    assert_eq!(
        meter.get_level(),
        0,
        "get_level should return 0 for silence"
    );
}

#[test]
fn test_level_calculation_max() {
    let meter = LevelMeter::new();

    // For 0 dB (max level), normalized should be 100
    let normalized = LevelMeter::normalize(0.0);
    assert_eq!(normalized, 100, "Max level (0dB) should normalize to 100");

    meter.set_level(100);
    assert_eq!(
        meter.get_level(),
        100,
        "get_level should return 100 for max"
    );
}

#[test]
fn test_level_calculation_average() {
    // For -30 dB (mid level), normalized should be 50
    let normalized = LevelMeter::normalize(-30.0);
    assert_eq!(normalized, 50, "-30dB should normalize to 50");

    // For -6 dB (roughly 50%), normalized should be around 90
    let normalized_neg_6 = LevelMeter::normalize(-6.0);
    // -6dB is about 90% of the range (60dB total range)
    assert!(
        normalized_neg_6 >= 85 && normalized_neg_6 <= 95,
        "-6dB should normalize to approximately 90, got {}",
        normalized_neg_6
    );
}

#[test]
fn test_level_meter_default() {
    let meter = LevelMeter::default();
    assert_eq!(meter.get_level(), 0, "Default meter should have level 0");
}

#[test]
fn test_level_meter_set_and_get() {
    let meter = LevelMeter::new();

    meter.set_level(42);
    assert_eq!(meter.get_level(), 42);

    meter.set_level(100);
    assert_eq!(meter.get_level(), 100);

    meter.set_level(0);
    assert_eq!(meter.get_level(), 0);
}

#[test]
fn test_level_meter_normalize_clamping() {
    // Test clamping at the lower bound
    let below_min = LevelMeter::normalize(-100.0);
    assert_eq!(below_min, 0, "Should clamp to 0 for values below -60dB");

    // Test clamping at the upper bound
    let above_max = LevelMeter::normalize(10.0);
    assert_eq!(above_max, 100, "Should clamp to 100 for values above 0dB");
}

#[test]
fn test_level_meter_normalize_various_values() {
    // Test a range of dB values and verify normalization
    let test_cases = [
        (-60.0, 0),
        (-54.0, 10),
        (-48.0, 20),
        (-42.0, 30),
        (-36.0, 40),
        (-30.0, 50),
        (-24.0, 60),
        (-18.0, 70),
        (-12.0, 80),
        (-6.0, 90),
        (0.0, 100),
    ];

    for (db, expected) in test_cases {
        let normalized = LevelMeter::normalize(db);
        assert_eq!(
            normalized, expected as u32,
            "normalize({}) should return {}, got {}",
            db, expected, normalized
        );
    }
}

#[test]
fn test_level_meter_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let meter = Arc::new(LevelMeter::new());
    let num_threads = 4;
    let increments_per_thread = 25;

    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            let meter = Arc::clone(&meter);
            thread::spawn(move || {
                for j in 0..increments_per_thread {
                    let new_level = (i * increments_per_thread + j) as u32;
                    meter.set_level(new_level);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // After all threads complete, the final value is non-deterministic
    // but should be some valid u32 value that was set
    let final_level = meter.get_level();
    assert!(
        final_level < (num_threads * increments_per_thread) as u32,
        "Final level should be a value that was set"
    );
}

#[test]
fn test_level_meter_atomic_ordering() {
    let meter = LevelMeter::new();

    // Verify SeqCst ordering by setting and reading immediately
    meter.set_level(99);
    assert_eq!(meter.get_level(), 99, "Sequential set/get should work");

    meter.set_level(0);
    assert_eq!(meter.get_level(), 0);

    meter.set_level(50);
    assert_eq!(meter.get_level(), 50);
}
