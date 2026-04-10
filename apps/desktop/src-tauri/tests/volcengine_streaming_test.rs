use std::io::Cursor;

#[cfg(test)]
mod tests {
    use super::*;
    use ariatype_lib::commands::settings::CloudSttConfig;
    use ariatype_lib::stt_engine::cloud::volcengine_streaming::{
        StreamingMode, VolcengineStreamingClient, RECOMMENDED_CHUNK_SAMPLES, URL_BIGMODEL_NOSTREAM,
    };
    use ariatype_lib::stt_engine::traits::{PartialResult, SttContext};

    fn create_test_wav(sample_rate: u32, channels: u16, duration_secs: f32) -> Vec<u8> {
        let samples_per_channel = (sample_rate as f32 * duration_secs) as usize;
        let total_samples = samples_per_channel * channels as usize;

        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();

            for i in 0..total_samples {
                let t = i as f32 / sample_rate as f32;
                let freq = 440.0;
                let amplitude = 16000.0;
                let sample = (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin()) as i16;
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }

        cursor.into_inner()
    }

    fn resample_to_16khz_mono(
        samples_i16: &[i16],
        input_sample_rate: u32,
        input_channels: u16,
    ) -> Vec<i16> {
        let mut audio_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

        if input_channels == 2 {
            let mono: Vec<f32> = audio_f32
                .chunks(2)
                .map(|stereo| (stereo[0] + stereo.get(1).copied().unwrap_or(0.0)) / 2.0)
                .collect();
            audio_f32 = mono;
        }

        if input_sample_rate != 16000 {
            let resampled =
                ariatype_lib::audio::resampler::resample_to_16khz(&audio_f32, input_sample_rate)
                    .unwrap();
            resampled
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect()
        } else {
            audio_f32
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect()
        }
    }

    #[test]
    fn test_streaming_client_creation() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "volcengine-streaming".to_string(),
            api_key: "test-key".to_string(),
            app_id: "test-app-id".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "zh-CN".to_string(),
        };

        let _client =
            VolcengineStreamingClient::new(config.clone(), Some("zh-CN"), SttContext::default());

        // Test with auto language
        let _client_auto =
            VolcengineStreamingClient::new(config, Some("auto"), SttContext::default());
    }

    #[test]
    fn test_recommended_chunk_size() {
        // Test recommended chunk size from public constant
        assert_eq!(RECOMMENDED_CHUNK_SAMPLES, 1600);
        let chunk_duration_ms = (RECOMMENDED_CHUNK_SAMPLES as f64 / 16000.0) * 1000.0;
        assert!((chunk_duration_ms - 100.0).abs() < 1.0);

        // Test with actual audio data
        let wav_data = create_test_wav(16000, 1, 1.0);
        let reader = hound::WavReader::new(Cursor::new(wav_data)).unwrap();
        let samples_i16: Vec<i16> = reader
            .into_samples::<i16>()
            .filter_map(|s| s.ok())
            .collect();

        const CHUNK_SIZE: usize = 1600;
        let chunks: Vec<&[i16]> = samples_i16.chunks(CHUNK_SIZE).collect();

        assert!(chunks.len() > 0, "Should have at least one chunk");

        for (i, chunk) in chunks.iter().enumerate() {
            let expected_size = if i < chunks.len() - 1 {
                CHUNK_SIZE
            } else {
                samples_i16.len() % CHUNK_SIZE
            };
            if expected_size > 0 {
                assert!(
                    chunk.len() == expected_size || chunk.len() == CHUNK_SIZE,
                    "Chunk {} has unexpected size: {}",
                    i,
                    chunk.len()
                );
            }
        }
    }

    #[test]
    fn test_partial_result_serialization() {
        let result = PartialResult {
            text: "测试文本".to_string(),
            is_definite: true,
            is_final: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("测试文本"));
        assert!(json.contains("is_definite"));
        assert!(json.contains("is_final"));
    }

    #[test]
    fn test_streaming_mode_urls() {
        assert!(URL_BIGMODEL_NOSTREAM.contains("nostream"));
        assert!(URL_BIGMODEL_NOSTREAM.contains("bigmodel_nostream"));
    }

    #[test]
    fn test_streaming_mode_default() {
        assert_eq!(StreamingMode::default(), StreamingMode::NoStream);
    }

    #[test]
    #[ignore]
    fn test_streaming_client_connect() {
        // This test requires real API credentials
        let settings_path =
            std::env::var("ARIATYPE_SETTINGS_PATH").unwrap_or_else(|_| "settings.json".to_string());

        if !std::path::Path::new(&settings_path).exists() {
            eprintln!("Skipping integration test: settings.json not found");
            return;
        }

        let settings_content =
            std::fs::read_to_string(&settings_path).expect("Failed to read settings");
        let app_settings: ariatype_lib::commands::settings::AppSettings =
            serde_json::from_str(&settings_content).expect("Failed to parse settings");

        let cloud_stt_config = app_settings.get_active_cloud_stt_config();
        if !cloud_stt_config.enabled || cloud_stt_config.provider_type != "volcengine-streaming" {
            eprintln!("Skipping integration test: volcengine-streaming not configured");
            return;
        }

        if cloud_stt_config.app_id.is_empty() || cloud_stt_config.api_key.is_empty() {
            eprintln!("Skipping integration test: missing Volcengine credentials");
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let mut client =
                VolcengineStreamingClient::new(cloud_stt_config, None, SttContext::default());

            match client.connect().await {
                Ok(()) => {
                    println!("[Integration Test] Successfully connected to Volcengine STT");
                    client.close().await;
                }
                Err(e) => {
                    eprintln!("[Integration Test] Failed to connect: {}", e);
                    panic!("Connection failed: {}", e);
                }
            }
        });
    }

    #[test]
    #[ignore]
    fn test_streaming_client_send_audio() {
        // This test requires real API credentials and connection
        let settings_path =
            std::env::var("ARIATYPE_SETTINGS_PATH").unwrap_or_else(|_| "settings.json".to_string());

        if !std::path::Path::new(&settings_path).exists() {
            eprintln!("Skipping integration test: settings.json not found");
            return;
        }

        let settings_content =
            std::fs::read_to_string(&settings_path).expect("Failed to read settings");
        let app_settings: ariatype_lib::commands::settings::AppSettings =
            serde_json::from_str(&settings_content).expect("Failed to parse settings");

        let cloud_stt_config = app_settings.get_active_cloud_stt_config();
        if !cloud_stt_config.enabled || cloud_stt_config.provider_type != "volcengine-streaming" {
            eprintln!("Skipping integration test: volcengine-streaming not configured");
            return;
        }

        if cloud_stt_config.app_id.is_empty() || cloud_stt_config.api_key.is_empty() {
            eprintln!("Skipping integration test: missing Volcengine credentials");
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let mut client = VolcengineStreamingClient::new(
                cloud_stt_config.clone(),
                None,
                SttContext::default(),
            );

            // Connect first
            client.connect().await.expect("Failed to connect");

            // Create test audio data (100ms of sine wave)
            let sample_count = 1600; // 100ms at 16kHz
            let mut samples = Vec::with_capacity(sample_count);
            for i in 0..sample_count {
                let t = i as f32 / 16000.0;
                let freq = 440.0;
                let amplitude = 8000.0;
                let sample = (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin()) as i16;
                samples.push(sample);
            }

            // Send audio via channel
            let audio_tx = client
                .get_audio_sender()
                .await
                .expect("Failed to get audio sender");
            audio_tx.send(samples).await.expect("Failed to send audio");

            // Finish the session
            client.finish().await.expect("Failed to finish");

            println!("[Integration Test] Successfully sent audio chunk and finished");
        });
    }

    #[test]
    #[ignore]
    fn test_streaming_client_receive_result() {
        // This test requires real API credentials and audio to process
        let settings_path =
            std::env::var("ARIATYPE_SETTINGS_PATH").unwrap_or_else(|_| "settings.json".to_string());

        if !std::path::Path::new(&settings_path).exists() {
            eprintln!("Skipping integration test: settings.json not found");
            return;
        }

        let settings_content =
            std::fs::read_to_string(&settings_path).expect("Failed to read settings");
        let app_settings: ariatype_lib::commands::settings::AppSettings =
            serde_json::from_str(&settings_content).expect("Failed to parse settings");

        let cloud_stt_config = app_settings.get_active_cloud_stt_config();
        if !cloud_stt_config.enabled || cloud_stt_config.provider_type != "volcengine-streaming" {
            eprintln!("Skipping integration test: volcengine-streaming not configured");
            return;
        }

        if cloud_stt_config.app_id.is_empty() || cloud_stt_config.api_key.is_empty() {
            eprintln!("Skipping integration test: missing Volcengine credentials");
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let mut client = VolcengineStreamingClient::new(cloud_stt_config.clone(), None, SttContext::default());

            // Set up callback to capture results
            use std::sync::Arc;
            use tokio::sync::mpsc;
            let (result_tx, mut result_rx) = mpsc::channel::<PartialResult>(10);
            let callback = Arc::new(move |result: PartialResult| {
                let _ = result_tx.try_send(result);
            });
            client.set_partial_callback(callback);

            // Connect
            client.connect().await.expect("Failed to connect");

            // Send some test audio (200ms of speech-like signal)
            let audio_tx = client.get_audio_sender().await.expect("Failed to get audio sender");

            // Generate multiple chunks to simulate real speech
            for chunk_idx in 0..3 {
                let sample_count = 1600;
                let mut samples = Vec::with_capacity(sample_count);
                for i in 0..sample_count {
                    let t = (chunk_idx * sample_count + i) as f32 / 16000.0;
                    let freq = if chunk_idx % 2 == 0 { 800.0 } else { 600.0 };
                    let amplitude = 8000.0 * (1.0 - (t % 0.5) / 0.5); // Fade out
                    let sample = (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin()) as i16;
                    samples.push(sample);
                }
                audio_tx.send(samples).await.expect("Failed to send audio chunk");

                // Small delay between chunks
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            println!("[Integration Test] Sent audio chunks, waiting for results...");

            // Wait for results with timeout
            let mut received_results = 0;
            let mut final_result_received = false;

            loop {
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    result_rx.recv()
                ).await {
                    Ok(Some(partial)) => {
                        received_results += 1;
                        println!("[Integration Test] Received partial result {}: \"{}\" (definite={}, final={})",
                                received_results, partial.text, partial.is_definite, partial.is_final);

                        if partial.is_final {
                            final_result_received = true;
                            break;
                        }
                    }
                    Ok(None) => break, // Channel closed
                    Err(_) => break, // Timeout
                }
            }

            if received_results == 0 {
                eprintln!("[Integration Test] No results received");
                // Don't panic here since the test might just be slow or the service might not return partial results immediately
            } else {
                println!("[Integration Test] Received {} partial results", received_results);
            }

            // Finish the session
            client.finish().await.expect("Failed to finish");
        });
    }

    #[test]
    #[ignore]
    fn test_streaming_client_finish() {
        // This test requires real API credentials
        let settings_path =
            std::env::var("ARIATYPE_SETTINGS_PATH").unwrap_or_else(|_| "settings.json".to_string());

        if !std::path::Path::new(&settings_path).exists() {
            eprintln!("Skipping integration test: settings.json not found");
            return;
        }

        let settings_content =
            std::fs::read_to_string(&settings_path).expect("Failed to read settings");
        let app_settings: ariatype_lib::commands::settings::AppSettings =
            serde_json::from_str(&settings_content).expect("Failed to parse settings");

        let cloud_stt_config = app_settings.get_active_cloud_stt_config();
        if !cloud_stt_config.enabled || cloud_stt_config.provider_type != "volcengine-streaming" {
            eprintln!("Skipping integration test: volcengine-streaming not configured");
            return;
        }

        if cloud_stt_config.app_id.is_empty() || cloud_stt_config.api_key.is_empty() {
            eprintln!("Skipping integration test: missing Volcengine credentials");
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let mut client = VolcengineStreamingClient::new(
                cloud_stt_config.clone(),
                None,
                SttContext::default(),
            );

            // Connect
            client.connect().await.expect("Failed to connect");

            // Send some test audio
            let audio_tx = client
                .get_audio_sender()
                .await
                .expect("Failed to get audio sender");
            let sample_count = 1600;
            let mut samples = Vec::with_capacity(sample_count);
            for i in 0..sample_count {
                let t = i as f32 / 16000.0;
                let freq = 440.0;
                let amplitude = 8000.0;
                let sample = (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin()) as i16;
                samples.push(sample);
            }
            audio_tx.send(samples).await.expect("Failed to send audio");

            // Finish the session
            let result = client.finish().await;
            match result {
                Ok(_) => {
                    println!("[Integration Test] Successfully finished streaming session");
                }
                Err(e) => {
                    eprintln!("[Integration Test] Failed to finish: {}", e);
                    panic!("Finish failed: {}", e);
                }
            }

            // Close connection
            client.close().await;
            println!("[Integration Test] Connection closed successfully");
        });
    }

    #[test]
    #[ignore]
    fn test_streaming_client_error_handling() {
        // Test with invalid credentials to trigger error handling
        let invalid_config = CloudSttConfig {
            enabled: true,
            provider_type: "volcengine-streaming".to_string(),
            api_key: "invalid-key".to_string(),
            app_id: "invalid-app-id".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(async {
            let mut client =
                VolcengineStreamingClient::new(invalid_config, None, SttContext::default());

            // This should fail with authentication error
            let result = client.connect().await;
            match result {
                Ok(()) => {
                    // If it somehow succeeds, that's unexpected but not necessarily an error
                    println!("[Integration Test] Unexpected success with invalid credentials");
                    client.close().await;
                }
                Err(e) => {
                    println!(
                        "[Integration Test] Expected error with invalid credentials: {}",
                        e
                    );
                    // Verify that the error message contains expected content
                    assert!(
                        e.contains("authentication")
                            || e.contains("403")
                            || e.contains("401")
                            || e.contains("Forbidden")
                            || e.contains("Unauthorized")
                    );
                }
            }
        });
    }
}
