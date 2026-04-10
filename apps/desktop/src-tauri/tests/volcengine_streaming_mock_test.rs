use ariatype_lib::commands::settings::CloudSttConfig;
use ariatype_lib::stt_engine::cloud::volcengine_streaming::VolcengineStreamingClient;
use ariatype_lib::stt_engine::traits::{PartialResult, SttContext};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;

// Helper to build headers
fn build_header(
    message_type: u8,
    message_type_specific_flags: u8,
    serialization: u8,
    compression: u8,
) -> [u8; 4] {
    let mut header = [0u8; 4];
    header[0] = (0b0001 << 4) | 0b0001; // Version (4 bits) | Header Size (4 bits)
    header[1] = (message_type << 4) | message_type_specific_flags;
    header[2] = (serialization << 4) | compression;
    header[3] = 0x00; // Reserved
    header
}

#[tokio::test]
async fn test_volcengine_streaming_mock_flow() {
    // 1. Start mock server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mock_url = format!("ws://127.0.0.1:{}", port);

    let partial_results_received = Arc::new(Mutex::new(Vec::<PartialResult>::new()));
    let partials_clone = partial_results_received.clone();

    // Spawn server task
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws_stream = accept_async(stream).await.unwrap();

        // Expect full client request
        if let Some(Ok(Message::Binary(data))) = ws_stream.next().await {
            // Check it's a client request (type 0b0001)
            assert_eq!((data[1] >> 4) & 0x0F, 0b0001);
        } else {
            panic!("Expected full client request");
        }

        // Send a partial result back
        let response_json = serde_json::json!({
            "reqid": "mock-req-id",
            "code": 1000,
            "message": "Success",
            "sequence": 1,
            "result": [{
                "text": "测试",
                "utterances": [
                    {
                        "text": "测试",
                        "definite": false
                    }
                ]
            }]
        });
        let response_str = response_json.to_string();
        let response_bytes = response_str.as_bytes();
        let header = build_header(0b1001, 0b0000, 0b0001, 0b0000);
        let mut payload = Vec::with_capacity(12 + response_bytes.len());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&[0, 0, 0, 0]); // Message size, ignoring for mock
        payload.extend_from_slice(&(response_bytes.len() as u32).to_be_bytes()); // Payload size
        payload.extend_from_slice(response_bytes);

        ws_stream
            .send(Message::Binary(payload.into()))
            .await
            .unwrap();

        // Read audio packets
        let mut received_last = false;
        while let Some(Ok(Message::Binary(data))) = ws_stream.next().await {
            let message_type = (data[1] >> 4) & 0x0F;
            let specific_flags = data[1] & 0x0F;

            if message_type == 0b0010 {
                // Audio only request
                if specific_flags == 0b0010 {
                    // Last packet
                    received_last = true;
                    break;
                }
            }
        }
        assert!(received_last, "Should have received the last packet");

        // Send final result
        let final_json = serde_json::json!({
            "reqid": "mock-req-id",
            "code": 1000,
            "message": "Success",
            "sequence": 2,
            "result": [{
                "text": "测试完成",
                "utterances": [
                    {
                        "text": "测试完成",
                        "definite": true
                    }
                ]
            }],
            "payload_msg": {
                "is_last_package": true
            }
        });
        let final_str = final_json.to_string();
        let final_bytes = final_str.as_bytes();
        let header = build_header(0b1001, 0b0000, 0b0001, 0b0000);
        let mut payload = Vec::with_capacity(12 + final_bytes.len());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&[0, 0, 0, 0]);
        payload.extend_from_slice(&(final_bytes.len() as u32).to_be_bytes());
        payload.extend_from_slice(final_bytes);

        ws_stream
            .send(Message::Binary(payload.into()))
            .await
            .unwrap();
    });

    // 2. Client code
    let config = CloudSttConfig {
        enabled: true,
        provider_type: "volcengine-streaming".to_string(),
        api_key: "mock-key".to_string(),
        app_id: "mock-app".to_string(),
        base_url: mock_url,
        model: "mock-model".to_string(),
        language: "zh-CN".to_string(),
    };

    let mut client = VolcengineStreamingClient::new(config, Some("zh-CN"), SttContext::default());

    let (result_tx, result_rx) = tokio::sync::oneshot::channel::<String>();
    let result_tx = Arc::new(Mutex::new(Some(result_tx)));

    client.set_partial_callback(Arc::new(move |result: PartialResult| {
        let partials = partials_clone.clone();
        let result_tx_clone = result_tx.clone();
        tokio::spawn(async move {
            if result.is_final {
                if let Some(tx) = result_tx_clone.lock().await.take() {
                    let _ = tx.send(result.text.clone());
                }
            } else {
                partials.lock().await.push(result);
            }
        });
    }));

    // Connect
    client
        .connect()
        .await
        .expect("Failed to connect to mock server");

    // Get audio sender
    let audio_tx = client
        .get_audio_sender()
        .await
        .expect("Failed to get audio sender");

    // Send some audio chunks (mock continuous recorder)
    for _ in 0..3 {
        let chunk = vec![0i16; 1600]; // 100ms chunk at 16kHz
        audio_tx.send(chunk).await.expect("Failed to send chunk");
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Drop the sender to signal we're done (like stopping recording)
    drop(audio_tx);

    // Finish sending
    client.finish().await.expect("Failed to finish client");

    // Wait for final result
    let final_text = tokio::time::timeout(std::time::Duration::from_secs(5), result_rx)
        .await
        .expect("Timeout waiting for final result")
        .expect("Failed to get final result");

    // 3. Verify
    assert_eq!(final_text, "测试完成");

    // Give partial callback time to run
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    let partials = partial_results_received.lock().await;
    assert!(
        !partials.is_empty(),
        "Should have received at least one partial result"
    );
    assert_eq!(partials[0].text, "测试");
}

#[tokio::test]
async fn test_volcengine_streaming_mock_empty_audio() {
    // 1. Start mock server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mock_url = format!("ws://127.0.0.1:{}", port);

    // Spawn server task
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws_stream = accept_async(stream).await.unwrap();

        // Expect full client request
        if let Some(Ok(Message::Binary(data))) = ws_stream.next().await {
            assert_eq!((data[1] >> 4) & 0x0F, 0b0001);
        } else {
            panic!("Expected full client request");
        }

        // Read audio packets (should just be the last packet since we dropped audio_tx)
        let mut received_last = false;
        while let Some(Ok(Message::Binary(data))) = ws_stream.next().await {
            let message_type = (data[1] >> 4) & 0x0F;
            let specific_flags = data[1] & 0x0F;

            if message_type == 0b0010 {
                // Audio only request
                if specific_flags == 0b0010 {
                    // Last packet
                    received_last = true;
                    break;
                }
            }
        }
        assert!(received_last, "Should have received the last packet");

        // Send final result immediately without receiving audio
        let final_json = serde_json::json!({
            "reqid": "mock-req-id-empty",
            "code": 1000,
            "message": "Success",
            "sequence": 1,
            "result": [{
                "text": "",
                "utterances": []
            }],
            "payload_msg": {
                "is_last_package": true
            }
        });
        let final_str = final_json.to_string();
        let final_bytes = final_str.as_bytes();

        // This time, we need to correctly implement the build_header since we don't have access to the private function
        let mut header = vec![0u8; 4];
        header[0] = 0b00010001; // Version 1, Header size 1
        header[1] = 0b10010000; // Type: Full Server Response
        header[2] = 0b00010000; // Serialization: JSON
        header[3] = 0b00000000; // Compression: None

        let mut payload = Vec::with_capacity(12 + final_bytes.len());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&[0, 0, 0, 0]);
        payload.extend_from_slice(&(final_bytes.len() as u32).to_be_bytes());
        payload.extend_from_slice(final_bytes);

        ws_stream
            .send(Message::Binary(payload.into()))
            .await
            .unwrap();
    });

    // 2. Client code
    let config = CloudSttConfig {
        enabled: true,
        provider_type: "volcengine-streaming".to_string(),
        api_key: "mock-key".to_string(),
        app_id: "mock-app".to_string(),
        base_url: mock_url,
        model: "mock-model".to_string(),
        language: "zh-CN".to_string(),
    };

    let mut client = VolcengineStreamingClient::new(config, Some("zh-CN"), SttContext::default());
    let (result_tx, result_rx) = tokio::sync::oneshot::channel::<String>();
    let result_tx = Arc::new(Mutex::new(Some(result_tx)));

    client.set_partial_callback(Arc::new(move |result: PartialResult| {
        let result_tx_clone = result_tx.clone();
        tokio::spawn(async move {
            if result.is_final {
                if let Some(tx) = result_tx_clone.lock().await.take() {
                    let _ = tx.send(result.text.clone());
                }
            }
        });
    }));

    client.connect().await.expect("Failed to connect");
    let audio_tx = client
        .get_audio_sender()
        .await
        .expect("Failed to get audio sender");
    drop(audio_tx); // Send no audio
    client.finish().await.expect("Failed to finish");

    let final_text = tokio::time::timeout(std::time::Duration::from_secs(5), result_rx)
        .await
        .expect("Timeout waiting for final result")
        .expect("Failed to get final result");

    assert_eq!(final_text, "");
}

#[tokio::test]
async fn test_volcengine_streaming_mock_connection_failure() {
    let config = CloudSttConfig {
        enabled: true,
        provider_type: "volcengine-streaming".to_string(),
        api_key: "mock-key".to_string(),
        app_id: "mock-app".to_string(),
        base_url: "ws://127.0.0.1:1".to_string(), // Invalid port
        model: "mock-model".to_string(),
        language: "zh-CN".to_string(),
    };

    let mut client = VolcengineStreamingClient::new(config, Some("zh-CN"), SttContext::default());
    let result = client.connect().await;

    assert!(result.is_err(), "Connection should fail for invalid port");
}
