use futures_util::{SinkExt, Stream, StreamExt};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::Message};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use async_trait::async_trait;

use crate::commands::settings::CloudSttConfig;
use crate::stt_engine::traits::{PartialResult, PartialResultCallback, StreamingSttEngine};

// Protocol constants
const PROTOCOL_VERSION: u8 = 0b0001;
const HEADER_SIZE: u8 = 0b0001;

const MESSAGE_TYPE_FULL_CLIENT_REQUEST: u8 = 0b0001;
const MESSAGE_TYPE_AUDIO_ONLY_REQUEST: u8 = 0b0010;
const MESSAGE_TYPE_FULL_SERVER_RESPONSE: u8 = 0b1001;
const MESSAGE_TYPE_SERVER_ERROR_RESPONSE: u8 = 0b1111;

const SERIALIZATION_NONE: u8 = 0b0000;
const SERIALIZATION_JSON: u8 = 0b0001;

const COMPRESSION_NONE: u8 = 0b0000;

// Streaming mode URLs
// IMPORTANT: Per AGENTS.md Product Priority Order (accuracy > speed), we use nostream mode.
// Bidirectional streaming interfaces (bigmodel_async, bigmodel) have slightly lower accuracy.
// Only bigmodel_nostream (streaming input mode) meets our accuracy requirements.
//
/// Bidirectional streaming (optimized) - returns results only when changed
/// NOT RECOMMENDED: Lower accuracy than NoStream mode
pub const URL_BIGMODEL_ASYNC: &str = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async";
/// Bidirectional streaming (original) - returns results for every packet
/// NOT RECOMMENDED: Lower accuracy than NoStream mode
pub const URL_BIGMODEL: &str = "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel";
/// Streaming input mode - returns after 15s or last packet (not real-time)
/// RECOMMENDED: Highest accuracy, per product priority order
pub const URL_BIGMODEL_NOSTREAM: &str =
    "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_nostream";

/// Recommended chunk size: 100ms at 16kHz mono 16-bit = 1600 samples
/// Per Volcengine docs: "单包音频大小建议在 100~200ms 左右"
pub const RECOMMENDED_CHUNK_SAMPLES: usize = 1600;

fn build_header(
    message_type: u8,
    message_type_specific_flags: u8,
    serialization_method: u8,
    compression: u8,
) -> [u8; 4] {
    let byte0 = (PROTOCOL_VERSION << 4) | HEADER_SIZE;
    let byte1 = (message_type << 4) | message_type_specific_flags;
    let byte2 = (serialization_method << 4) | compression;
    let byte3 = 0x00;
    [byte0, byte1, byte2, byte3]
}

type BoxStream = Pin<Box<dyn Stream<Item = Result<Message, WsError>> + Send>>;
type BoxSink = Pin<Box<dyn futures_util::Sink<Message, Error = WsError> + Send>>;

struct ParsedRecognitionResult {
    text: String,
    is_definite: bool,
}

struct ParsedServerFrame<'a> {
    payload: &'a [u8],
    is_last_package: bool,
}

fn parse_recognition_result(parsed: &serde_json::Value) -> Option<ParsedRecognitionResult> {
    let result = parsed.get("result")?;

    let candidate = match result {
        serde_json::Value::Object(_) => result,
        serde_json::Value::Array(items) => items.first()?,
        _ => return None,
    };

    let text = candidate.get("text").and_then(|t| t.as_str())?.to_string();
    let is_definite = candidate
        .get("utterances")
        .and_then(|u| u.as_array())
        .and_then(|arr| arr.last())
        .and_then(|utt| utt.get("definite"))
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    Some(ParsedRecognitionResult { text, is_definite })
}

fn parse_server_frame(data: &[u8]) -> Option<ParsedServerFrame<'_>> {
    if data.len() < 8 {
        return None;
    }

    let header = &data[0..4];
    let header_size = usize::from(header[0] & 0x0F) * 4;
    if data.len() < header_size + 4 {
        return None;
    }

    let message_type_specific_flags = header[1] & 0x0F;
    let has_sequence = message_type_specific_flags & 0x01 != 0;
    let is_last_package = message_type_specific_flags & 0x02 != 0;

    let mut candidate_offsets = vec![header_size];
    if has_sequence || data.len() >= header_size + 8 {
        candidate_offsets.push(header_size + 4);
    }

    let mut best_payload_offset = None;
    let mut best_payload_size = 0usize;
    let mut best_score = 0u8;

    for size_offset in candidate_offsets {
        if data.len() < size_offset + 4 {
            continue;
        }

        let payload_size =
            u32::from_be_bytes(data[size_offset..size_offset + 4].try_into().ok()?) as usize;
        let payload_offset = size_offset + 4;
        let remaining = data.len().saturating_sub(payload_offset);
        let score = match (payload_size == remaining, payload_size > 0) {
            (true, true) => 3,
            (true, false) => 2,
            (false, true) if payload_size < remaining => 1,
            _ => 0,
        };

        if score > best_score {
            best_score = score;
            best_payload_offset = Some(payload_offset);
            best_payload_size = payload_size;
        }
    }

    let payload_offset = best_payload_offset?;
    let actual_payload_len = data.len().saturating_sub(payload_offset);
    let end_idx = payload_offset + std::cmp::min(best_payload_size, actual_payload_len);

    Some(ParsedServerFrame {
        payload: &data[payload_offset..end_idx],
        is_last_package,
    })
}

/// Streaming mode selection
///
/// Per product priority order (accuracy > speed), NoStream is the default and recommended mode.
/// Bidirectional streaming modes (Async, Standard) have slightly lower accuracy and are NOT recommended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamingMode {
    /// Optimized bidirectional - only returns when results change
    /// NOT RECOMMENDED: Lower accuracy than NoStream
    Async,
    /// Original bidirectional - returns for every packet
    /// NOT RECOMMENDED: Lower accuracy than NoStream
    Standard,
    /// Non-streaming - returns after 15s or last packet
    /// RECOMMENDED: Highest accuracy, default mode per product priority order
    #[default]
    NoStream,
}

/// Real-time streaming STT client for Volcengine
///
/// Supports bidirectional streaming where audio is sent in real-time
/// and partial results are returned as speech is recognized.
pub struct VolcengineStreamingClient {
    tx: Arc<Mutex<Option<BoxSink>>>,
    rx: Arc<Mutex<Option<BoxStream>>>,
    config: CloudSttConfig,
    language: String,
    connect_id: String,
    streaming_mode: StreamingMode,
    /// Channel for sending audio data from recording thread
    audio_tx: Arc<Mutex<Option<mpsc::Sender<Vec<i16>>>>>,
    /// Callback for partial results
    on_partial: Option<PartialResultCallback>,
    /// Handle for the audio sender task (to wait for completion)
    audio_sender_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Channel receiver for final result
    final_result_rx: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<String>>>>,
}

unsafe impl Send for VolcengineStreamingClient {}
unsafe impl Sync for VolcengineStreamingClient {}

impl VolcengineStreamingClient {
    /// Create a new streaming client (defaults to NoStream for highest accuracy)
    pub fn new(config: CloudSttConfig, language: Option<&str>) -> Self {
        Self::with_mode(config, language, StreamingMode::NoStream)
    }

    /// Create a new streaming client with specific mode
    pub fn with_mode(config: CloudSttConfig, language: Option<&str>, mode: StreamingMode) -> Self {
        let lang = match language {
            Some(l) if l != "auto" => l,
            _ => "",
        };

        Self {
            tx: Arc::new(Mutex::new(None)),
            rx: Arc::new(Mutex::new(None)),
            config,
            language: lang.to_string(),
            connect_id: Uuid::new_v4().to_string(),
            streaming_mode: mode,
            audio_tx: Arc::new(Mutex::new(None)),
            on_partial: None,
            audio_sender_task: Arc::new(Mutex::new(None)),
            final_result_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Set callback for receiving partial results
    pub fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.on_partial = Some(callback);
    }

    /// Get the audio sender channel for streaming PCM data
    /// Call this AFTER connect() to start streaming audio
    pub async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>> {
        self.audio_tx.lock().await.clone()
    }

    /// Connect to Volcengine STT WebSocket server
    #[instrument(
        skip(self),
        fields(
            mode = ?self.streaming_mode,
            language = %self.language,
        ),
        ret,
        err
    )]
    pub async fn connect(&mut self) -> Result<(), String> {
        if self.config.app_id.is_empty() {
            return Err("Volcengine App ID is empty. Please configure your Volcengine credentials in Settings > Cloud STT.".to_string());
        }
        if self.config.api_key.is_empty() {
            return Err("Volcengine Access Token is empty. Please configure your Volcengine credentials in Settings > Cloud STT.".to_string());
        }

        // Select URL based on streaming mode
        let base_url = if !self.config.base_url.is_empty() {
            self.config.base_url.clone()
        } else {
            match self.streaming_mode {
                StreamingMode::Async => URL_BIGMODEL_ASYNC.to_string(),
                StreamingMode::Standard => URL_BIGMODEL.to_string(),
                StreamingMode::NoStream => URL_BIGMODEL_NOSTREAM.to_string(),
            }
        };

        let resource_id = if self.config.model.is_empty() {
            "volc.bigasr.sauc.duration".to_string()
        } else {
            self.config.model.clone()
        };

        info!(
            provider = "volcengine",
            url = %base_url,
            mode = ?self.streaming_mode,
            "websocket_connecting"
        );
        info!(
            provider = "volcengine",
            app_id = %self.config.app_id,
            resource = %resource_id,
            "websocket_connect_headers"
        );

        let mut request =
            tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
                base_url.as_str(),
            )
            .map_err(|e| format!("Invalid URL: {}", e))?;

        let headers = request.headers_mut();
        headers.insert("X-Api-App-Key", self.config.app_id.parse().unwrap());
        headers.insert("X-Api-Access-Key", self.config.api_key.parse().unwrap());
        headers.insert("X-Api-Resource-Id", resource_id.parse().unwrap());
        headers.insert("X-Api-Connect-Id", self.connect_id.parse().unwrap());

        info!(provider = "volcengine", app_id = %self.config.app_id, resource = %resource_id, connect_id = %self.connect_id, "websocket_request_headers");
        info!(
            provider = "volcengine",
            token_len = self.config.api_key.len(),
            "websocket_auth_token_length"
        );

        let result = connect_async_tls_with_config(request, None, false, None).await;

        let (ws_stream, response) = match result {
            Ok(stream) => stream,
            Err(e) => {
                let error_str = e.to_string();

                // Provide helpful error messages for common issues
                if error_str.contains("403") || error_str.contains("Forbidden") {
                    error!(provider = "volcengine", http.status_code = 403, error = %error_str, "websocket_connect_failed");
                    return Err(format!(
                        "Volcengine STT authentication failed (403 Forbidden).\n\
                        \n\
                        Possible causes:\n\
                        1. Access Token has expired - get a fresh token from Volcengine Console\n\
                        2. Service not activated - enable STT at https://console.volcengine.com/sami\n\
                        3. Invalid App ID or Access Token - verify credentials in Console\n\
                        4. IP restriction - check if your IP is whitelisted\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                if error_str.contains("401") || error_str.contains("Unauthorized") {
                    error!(provider = "volcengine", http.status_code = 401, error = %error_str, "websocket_connect_failed");
                    return Err(format!(
                        "Volcengine STT unauthorized (401).\n\
                        Please verify your App ID and Access Token are correct.\n\
                        Get credentials from: https://console.volcengine.com/sami\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                error!(provider = "volcengine", error = %error_str, "websocket_connect_failed");
                return Err(format!(
                    "Failed to connect to Volcengine STT: {}",
                    error_str
                ));
            }
        };

        // Log the X-Tt-Logid for debugging
        if let Some(logid) = response.headers().get("X-Tt-Logid") {
            info!(provider = "volcengine", logid = ?logid, "websocket_server_logid");
        }

        info!(
            provider = "volcengine",
            http.status_code = 101,
            "websocket_connected"
        );

        let (sink, stream) = ws_stream.split();

        *self.tx.lock().await = Some(Box::pin(sink));
        *self.rx.lock().await = Some(Box::pin(stream));

        // Create audio channel (buffer for ~2 seconds of audio chunks)
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(20);
        *self.audio_tx.lock().await = Some(audio_tx.clone());

        // Create oneshot channel for final result
        let (final_tx, final_rx) = tokio::sync::oneshot::channel::<String>();
        *self.final_result_rx.lock().await = Some(final_rx);

        self.send_client_request().await?;

        // Start background task for receiving results
        self.start_result_receiver(final_tx).await;

        // Start background task for sending audio
        self.start_audio_sender(audio_rx).await;

        Ok(())
    }

    async fn send_client_request(&self) -> Result<(), String> {
        let mut req_json = json!({
            "user": {
                "uid": "ariatype_user"
            },
            "audio": {
                "format": "pcm",
                "rate": 16000,
                "bits": 16,
                "channel": 1,
                "codec": "raw"
            },
            "request": {
                "model_name": "bigmodel",
                "enable_itn": true,
                "enable_punc": true,
                "result_type": "full",
            }
        });

        // Add language for NoStream mode (supports language specification)
        if !self.language.is_empty() && self.streaming_mode == StreamingMode::NoStream {
            req_json["audio"]["language"] = json!(self.language);
        }

        let req_str = req_json.to_string();
        debug!(provider = "volcengine", request = %req_str, "client_request_payload");
        let req_bytes = req_str.as_bytes();

        let header = build_header(
            MESSAGE_TYPE_FULL_CLIENT_REQUEST,
            0b0000,
            SERIALIZATION_JSON,
            COMPRESSION_NONE,
        );

        let mut payload = Vec::with_capacity(4 + 4 + req_bytes.len());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&(req_bytes.len() as u32).to_be_bytes());
        payload.extend_from_slice(req_bytes);

        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or("WebSocket not connected")?;
        tx.send(Message::Binary(payload.into()))
            .await
            .map_err(|e| format!("Failed to send client request: {}", e))?;

        info!(provider = "volcengine", "client_request_sent");
        Ok(())
    }

    /// Start background task to receive and process results
    async fn start_result_receiver(&self, final_tx: tokio::sync::oneshot::Sender<String>) {
        let rx = self.rx.clone();
        let on_partial = self.on_partial.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut rx_guard = rx.lock().await;
            let rx_stream = match rx_guard.take() {
                Some(s) => s,
                None => {
                    warn!(provider = "volcengine", "websocket_no_receiver");
                    return;
                }
            };
            drop(rx_guard);

            // Process incoming messages
            let mut stream = rx_stream;
            let mut response_count = 0u32;
            let mut last_text = String::new();
            let mut final_text = String::new();
            let mut received_final_result = false;

            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Binary(data)) => {
                        let header = &data[0..4];
                        let message_type = (header[1] >> 4) & 0x0F;

                        if message_type == MESSAGE_TYPE_FULL_SERVER_RESPONSE {
                            let Some(frame) = parse_server_frame(&data) else {
                                warn!(provider = "volcengine", "server_response_parse_failed");
                                continue;
                            };

                            if let Ok(json_str) = std::str::from_utf8(frame.payload) {
                                response_count += 1;
                                if let Ok(parsed) =
                                    serde_json::from_str::<serde_json::Value>(json_str)
                                {
                                    if let Some(recognition) = parse_recognition_result(&parsed) {
                                        // Last-result marker may come from either the protocol header
                                        // flags or the JSON payload, depending on the server variant.
                                        let is_last = frame.is_last_package
                                            || parsed
                                                .get("payload_msg")
                                                .and_then(|m| m.get("is_last_package"))
                                                .and_then(|b| b.as_bool())
                                                .unwrap_or(false);

                                        if recognition.text != last_text || is_last {
                                            last_text = recognition.text.clone();

                                            debug!(
                                                provider = "volcengine",
                                                response_num = response_count,
                                                text = %recognition.text,
                                                definite = recognition.is_definite,
                                                last = is_last,
                                                "transcription_result"
                                            );

                                            // Emit partial result
                                            if let Some(ref callback) = on_partial {
                                                callback(PartialResult {
                                                    text: recognition.text.clone(),
                                                    is_definite: recognition.is_definite,
                                                    is_final: is_last,
                                                });
                                            }

                                            if is_last {
                                                received_final_result = true;
                                                final_text = recognition.text;
                                                info!(provider = "volcengine", text = %final_text, "final_result_received");
                                                break;
                                            }
                                        }
                                    } else {
                                        warn!(provider = "volcengine", payload = %json_str, "recognition_result_parse_failed");
                                    }
                                }
                            }
                        } else if message_type == MESSAGE_TYPE_SERVER_ERROR_RESPONSE {
                            let Some(frame) = parse_server_frame(&data) else {
                                warn!(provider = "volcengine", "server_error_frame_parse_failed");
                                continue;
                            };
                            let payload = frame.payload;
                            let err_msg = std::str::from_utf8(payload).unwrap_or("Unknown error");
                            error!(provider = "volcengine", error = %err_msg, "server_error");

                            if let Some(ref callback) = on_partial {
                                callback(PartialResult {
                                    text: String::new(),
                                    is_definite: false,
                                    is_final: true,
                                });
                            }
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!(provider = "volcengine", "connection_closed_by_server");
                        break;
                    }
                    Err(e) => {
                        error!(provider = "volcengine", error = %e, "websocket_error");
                        break;
                    }
                    _ => {}
                }
            }

            if received_final_result {
                let _ = final_tx.send(final_text.clone());
                debug!(provider = "volcengine", text = %final_text, "final_result_sent_to_finish");
            } else {
                warn!(provider = "volcengine", "stream_ended_no_final_result");
            }

            // Close the sender if still open
            let mut tx_guard = tx.lock().await;
            if let Some(mut sender) = tx_guard.take() {
                let _ = sender.close().await;
            }
        });
    }

    /// Start background task to send audio chunks from channel
    async fn start_audio_sender(&self, mut audio_rx: mpsc::Receiver<Vec<i16>>) {
        let tx = self.tx.clone();
        let task_handle = tokio::spawn(async move {
            let mut chunk_count = 0u32;
            while let Some(pcm_data) = audio_rx.recv().await {
                let mut guard = tx.lock().await;
                if let Some(sender) = guard.as_mut() {
                    if let Err(e) = send_audio_chunk(sender, &pcm_data, false).await {
                        error!(provider = "volcengine", error = %e, "audio_chunk_send_failed");
                        break;
                    }
                    chunk_count += 1;
                } else {
                    debug!(provider = "volcengine", "audio_sender_stopped");
                    break;
                }
            }
            info!(
                provider = "volcengine",
                chunks = chunk_count,
                "audio_sender_finished"
            );
        });

        *self.audio_sender_task.lock().await = Some(task_handle);
    }

    /// Send audio data directly (for non-channel based usage)
    pub async fn send_audio(&self, pcm_data: &[i16], is_last: bool) -> Result<(), String> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or("WebSocket not connected")?;
        send_audio_chunk(tx, pcm_data, is_last).await
    }

    /// Send audio via channel (returns immediately, processed in background)
    pub async fn send_audio_async(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        let guard = self.audio_tx.lock().await;
        let tx = guard
            .as_ref()
            .ok_or("Audio channel not initialized - call connect() first")?;
        tx.send(pcm_data)
            .await
            .map_err(|e| format!("Failed to queue audio: {}", e))
    }

    /// Close connection and wait for final result
    #[instrument(skip(self), ret, err)]
    pub async fn finish(&self) -> Result<String, String> {
        info!(provider = "volcengine", "finish_called");
        let start = Instant::now();

        // Close the audio channel to signal audio sender to stop
        drop(self.audio_tx.lock().await.take());

        // Wait for audio sender task to complete
        info!(provider = "volcengine", "waiting_for_audio_sender_task");
        let task_handle = self.audio_sender_task.lock().await.take();
        if let Some(handle) = task_handle {
            match handle.await {
                Ok(()) => info!(provider = "volcengine", "audio_sender_task_completed"),
                Err(e) => warn!(provider = "volcengine", error = %e, "audio_sender_task_error"),
            }
        } else {
            warn!(provider = "volcengine", "no_audio_sender_task");
        }

        // Send end packet to server
        let mut guard = self.tx.lock().await;
        if let Some(tx) = guard.as_mut() {
            let header = build_header(
                MESSAGE_TYPE_AUDIO_ONLY_REQUEST,
                0b0010,
                SERIALIZATION_NONE,
                COMPRESSION_NONE,
            );
            let mut payload = Vec::with_capacity(8);
            payload.extend_from_slice(&header);
            payload.extend_from_slice(&0u32.to_be_bytes());

            tx.send(Message::Binary(payload.into()))
                .await
                .map_err(|e| format!("Failed to send end packet: {}", e))?;

            info!(provider = "volcengine", "end_packet_sent");
        }

        // Wait for final result from result receiver task
        // IMPORTANT: Do this BEFORE closing the sink, otherwise the result receiver
        // task will exit prematurely when it sees the closed connection
        let final_rx = self.final_result_rx.lock().await.take();
        let result = if let Some(rx) = final_rx {
            info!(provider = "volcengine", "waiting_for_final_result");
            match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
                Ok(Ok(text)) => {
                    let total_ms = start.elapsed().as_millis() as u64;
                    info!(
                        provider = "volcengine",
                        duration_ms = total_ms,
                        text_len = text.len(),
                        "final_result_received_with_timing"
                    );
                    Ok(text)
                }
                Ok(Err(_)) => {
                    warn!(provider = "volcengine", "final_result_channel_closed");
                    Err("Final result channel closed".to_string())
                }
                Err(_) => {
                    warn!(provider = "volcengine", "final_result_timeout");
                    Err("Timeout waiting for final result".to_string())
                }
            }
        } else {
            warn!(provider = "volcengine", "no_final_result_receiver");
            Err("No final result receiver available".to_string())
        };

        // Close WebSocket sink AFTER receiving final result
        if let Some(mut tx) = guard.take() {
            let _ = tx.close().await;
        }

        result
    }

    /// Close connection immediately
    pub async fn close(&self) {
        let mut guard = self.tx.lock().await;
        if let Some(mut tx) = guard.take() {
            let _ = tx.close().await;
            info!(provider = "volcengine", "connection_closed");
        }
        *self.rx.lock().await = None;
    }
}

#[async_trait]
impl StreamingSttEngine for VolcengineStreamingClient {
    async fn start(&mut self) -> Result<(), String> {
        self.connect().await
    }

    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        self.send_audio_async(pcm_data).await
    }

    async fn finish(&self) -> Result<String, String> {
        VolcengineStreamingClient::finish(self).await
    }

    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.on_partial = Some(callback);
    }

    async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>> {
        self.audio_tx.lock().await.clone()
    }
}

/// Send an audio chunk over WebSocket
async fn send_audio_chunk(
    sender: &mut BoxSink,
    pcm_data: &[i16],
    is_last: bool,
) -> Result<(), String> {
    let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();

    let flags = if is_last { 0b0010 } else { 0b0000 };
    let header = build_header(
        MESSAGE_TYPE_AUDIO_ONLY_REQUEST,
        flags,
        SERIALIZATION_NONE,
        COMPRESSION_NONE,
    );

    let mut payload = Vec::with_capacity(4 + 4 + bytes.len());
    payload.extend_from_slice(&header);
    payload.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    payload.extend_from_slice(&bytes);

    sender
        .send(Message::Binary(payload.into()))
        .await
        .map_err(|e| format!("Failed to send audio: {}", e))?;

    debug!(
        provider = "volcengine",
        samples = pcm_data.len(),
        is_last = is_last,
        "audio_chunk_sent"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_header() {
        let header = build_header(
            MESSAGE_TYPE_FULL_CLIENT_REQUEST,
            0b0000,
            SERIALIZATION_JSON,
            COMPRESSION_NONE,
        );
        assert_eq!(header[0], 0b00010001);
        assert_eq!(header[1], 0b00010000);
        assert_eq!(header[2], 0b00010000);
        assert_eq!(header[3], 0x00);
    }

    #[test]
    fn test_streaming_mode_urls() {
        assert!(URL_BIGMODEL_ASYNC.contains("bigmodel_async"));
        assert!(URL_BIGMODEL.contains("bigmodel"));
        assert!(!URL_BIGMODEL.contains("async"));
        assert!(URL_BIGMODEL_NOSTREAM.contains("nostream"));
    }

    #[test]
    fn test_audio_only_request_header() {
        let header = build_header(
            MESSAGE_TYPE_AUDIO_ONLY_REQUEST,
            0b0000,
            SERIALIZATION_NONE,
            COMPRESSION_NONE,
        );
        assert_eq!(header[0], 0b00010001);
        assert_eq!(header[1], 0b00100000);
        assert_eq!(header[2], 0b00000000);
    }

    #[test]
    fn test_last_packet_flag() {
        let header = build_header(
            MESSAGE_TYPE_AUDIO_ONLY_REQUEST,
            0b0010,
            SERIALIZATION_NONE,
            COMPRESSION_NONE,
        );
        assert_eq!(header[1], 0b00100010);
    }

    #[test]
    fn test_server_response_header() {
        let header = build_header(
            MESSAGE_TYPE_FULL_SERVER_RESPONSE,
            0b0001,
            SERIALIZATION_JSON,
            COMPRESSION_NONE,
        );
        assert_eq!(header[1], 0b10010001);
    }

    #[test]
    fn test_server_error_header() {
        let header = build_header(
            MESSAGE_TYPE_SERVER_ERROR_RESPONSE,
            0b0000,
            SERIALIZATION_JSON,
            COMPRESSION_NONE,
        );
        assert_eq!(header[1], 0b11110000);
    }

    #[test]
    fn test_recommended_chunk_size() {
        assert_eq!(RECOMMENDED_CHUNK_SAMPLES, 1600);
        let chunk_duration_ms = (RECOMMENDED_CHUNK_SAMPLES as f64 / 16000.0) * 1000.0;
        assert!((chunk_duration_ms - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_streaming_mode_default() {
        assert_eq!(StreamingMode::default(), StreamingMode::NoStream);
    }

    #[test]
    fn test_parse_recognition_result_from_array_payload() {
        let parsed = serde_json::json!({
            "result": [{
                "text": "final result",
                "utterances": [{"text": "final result", "definite": true}]
            }]
        });

        let recognition = parse_recognition_result(&parsed).expect("result should parse");
        assert_eq!(recognition.text, "final result");
        assert!(recognition.is_definite);
    }

    #[test]
    fn test_parse_server_frame_marks_last_packet_from_header_flags() {
        let payload_bytes = br#"{"result":[{"text":"final result"}]}"#;
        let header = build_header(
            MESSAGE_TYPE_FULL_SERVER_RESPONSE,
            0b0011,
            SERIALIZATION_JSON,
            COMPRESSION_NONE,
        );

        let mut frame = Vec::new();
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&1i32.to_be_bytes());
        frame.extend_from_slice(&(payload_bytes.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload_bytes);

        let parsed = parse_server_frame(&frame).expect("frame should parse");
        assert!(parsed.is_last_package);
        assert_eq!(parsed.payload, payload_bytes);
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
    }
}
