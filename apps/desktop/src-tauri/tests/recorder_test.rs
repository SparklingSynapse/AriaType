use std::path::PathBuf;

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ariatype_test_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[test]
fn test_recorder_new() {
    let recorder = ariatype_lib::audio::recorder::AudioRecorder::new();
    assert!(
        !recorder.is_recording(),
        "New recorder should not be recording"
    );
}

#[test]
fn test_recorder_default() {
    let recorder = ariatype_lib::audio::recorder::AudioRecorder::default();
    assert!(
        !recorder.is_recording(),
        "Default recorder should not be recording"
    );
}

#[test]
fn test_recorder_stop_not_recording() {
    let recorder = ariatype_lib::audio::recorder::AudioRecorder::new();
    let result = recorder.stop();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Not recording"),
        "Should error when stopping when not recording"
    );
}

#[test]
fn test_recorder_devices_returns_list() {
    let devices = ariatype_lib::audio::recorder::AudioRecorder::get_devices();
    assert!(
        devices.is_empty(),
        "Devices should return empty list to avoid crashes"
    );
}

#[test]
fn test_recorder_double_start() {
    let recorder = ariatype_lib::audio::recorder::AudioRecorder::new();
    let output_path = temp_dir().join("test.wav");

    let result1 = recorder.start(output_path.clone(), None, None::<fn(std::path::PathBuf)>);

    if result1.is_ok() {
        let result2 = recorder.start(output_path, None, None::<fn(std::path::PathBuf)>);
        assert!(result2.is_err(), "Should not be able to start twice");

        let _ = recorder.stop();
    }
}

#[test]
fn test_recorder_invalid_device() {
    let recorder = ariatype_lib::audio::recorder::AudioRecorder::new();
    let output_path = temp_dir().join("test.wav");

    let result = recorder.start(
        output_path,
        Some("nonexistent device name xyz".to_string()),
        None::<fn(std::path::PathBuf)>,
    );

    if result.is_err() {
        let err = result.unwrap_err();
        assert!(
            err.contains("No input device") || err.contains("device"),
            "Error should mention device issue"
        );
    }
}
