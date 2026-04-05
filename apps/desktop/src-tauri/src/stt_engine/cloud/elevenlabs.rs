//! Eleven Labs Scribe v2 Realtime STT Client
//!
//! Implements WebSocket-based streaming speech-to-text using Eleven Labs Scribe v2 API.
//! Protocol: wss://api.elevenlabs.io/v1/speech-to-text/realtime
//!
//! Audio format: PCM, 16kHz, 16-bit, mono, Base64-encoded in JSON messages
//!
//! Message protocol:
//! - Client sends: `input_audio_chunk` with `audio_base_64`, `commit` (bool), `sample_rate`
//! - Server responds: `partial_transcript` (interim) or `committed_transcript` (final)

use async_trait::async_trait;
use futures_util::{SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::tungstenite::Error as WsError;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

use crate::commands::settings::CloudSttConfig;
use crate::stt_engine::traits::{
    EngineType, PartialResult, PartialResultCallback, StreamingSttEngine, TranscriptionResult,
};

/// Eleven Labs Scribe v2 Realtime WebSocket endpoint
const ELEVENLABS_REALTIME_ENDPOINT: &str = "wss://api.elevenlabs.io/v1/speech-to-text/realtime";

/// Recommended chunk size: 1 second of audio at 16kHz = 16000 samples
/// This matches Eleven Labs' optimal streaming chunk size
pub const RECOMMENDED_CHUNK_SAMPLES: usize = 16000;

/// Type alias for boxed WebSocket stream
type BoxStream = Pin<Box<dyn Stream<Item = Result<Message, WsError>> + Send>>;

/// Type alias for boxed WebSocket sink
type BoxSink = Pin<Box<dyn futures_util::Sink<Message, Error = WsError> + Send>>;

/// Eleven Labs Scribe v2 Realtime WebSocket client
///
/// Manages a WebSocket connection to Eleven Labs' streaming STT API.
/// Audio is sent in chunks, and partial/final transcripts are received asynchronously.
pub struct ElevenLabsStreamingClient {
    tx: Arc<Mutex<Option<BoxSink>>>,
    rx: Arc<Mutex<Option<BoxStream>>>,
    config: CloudSttConfig,
    _language: String,
    audio_tx: Arc<Mutex<Option<mpsc::Sender<Vec<i16>>>>>,
    on_partial: Option<PartialResultCallback>,
    audio_sender_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    last_error: Arc<Mutex<Option<String>>>,
}

// SAFETY: WebSocket types are Send+Sync when used with tokio runtime
unsafe impl Send for ElevenLabsStreamingClient {}
unsafe impl Sync for ElevenLabsStreamingClient {}

impl ElevenLabsStreamingClient {
    /// Create a new Eleven Labs streaming client
    ///
    /// # Arguments
    /// * `config` - Cloud STT configuration with API key and settings
    /// * `language` - Optional language code (e.g., "en", "zh"). Empty string for auto-detect.
    ///
    /// # Returns
    /// A new client instance (not yet connected)
    pub fn new(config: CloudSttConfig, language: Option<&str>) -> Self {
        let lang = match language {
            Some(l) if l != "auto" => l,
            _ => "",
        };

        Self {
            tx: Arc::new(Mutex::new(None)),
            rx: Arc::new(Mutex::new(None)),
            config,
            _language: lang.to_string(),
            audio_tx: Arc::new(Mutex::new(None)),
            on_partial: None,
            audio_sender_task: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the callback for receiving partial transcription results
    ///
    /// The callback is invoked for each `partial_transcript` and `committed_transcript`
    /// message received from the server.
    pub fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.on_partial = Some(callback);
    }

    /// Get the audio sender channel
    ///
    /// Returns the mpsc channel for queuing audio chunks.
    /// Audio chunks are sent asynchronously via WebSocket.
    pub async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>> {
        self.audio_tx.lock().await.clone()
    }

    /// Establish WebSocket connection to Eleven Labs API
    ///
    /// # Process
    /// 1. Validate API key is present
    /// 2. Build WebSocket URL with optional custom base URL
    /// 3. Add authentication header (xi-api-key)
    /// 4. Connect and split WebSocket into sink/stream
    /// 5. Start background tasks for receiving results and sending audio
    ///
    /// # Errors
    /// Returns user-friendly error messages for common API errors:
    /// - 401: Invalid API key
    /// - 403: Access forbidden (insufficient permissions)
    /// - 429: Rate limit exceeded
    /// - 500: Server error
    pub async fn connect(&mut self) -> Result<(), String> {
        if self.config.api_key.is_empty() {
            return Err(
                "Eleven Labs API key is empty. Please configure your API key in Settings > Cloud STT.\n\
                \n\
                Get your API key from: https://elevenlabs.io/app/settings/api-keys"
                    .to_string(),
            );
        }

        let endpoint = if self.config.base_url.is_empty() {
            ELEVENLABS_REALTIME_ENDPOINT
        } else {
            &self.config.base_url
        };

        info!("[ElevenLabs] Connecting to {}", endpoint);

        let mut request =
            tokio_tungstenite::tungstenite::client::IntoClientRequest::into_client_request(
                endpoint,
            )
            .map_err(|e| format!("Invalid URL: {}", e))?;

        let headers = request.headers_mut();
        headers.insert("xi-api-key", self.config.api_key.parse().unwrap());

        let result = connect_async_tls_with_config(request, None, false, None).await;

        let (ws_stream, _response) = match result {
            Ok(stream) => stream,
            Err(e) => {
                let error_str = e.to_string();

                if error_str.contains("401") || error_str.contains("Unauthorized") {
                    return Err(format!(
                        "Eleven Labs API authentication failed (401 Unauthorized).\n\
                        \n\
                        Please verify your API key in Settings > Cloud STT.\n\
                        Get your API key from: https://elevenlabs.io/app/settings/api-keys\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                if error_str.contains("403") || error_str.contains("Forbidden") {
                    return Err(format!(
                        "Eleven Labs API access forbidden (403).\n\
                        \n\
                        Possible causes:\n\
                        1. Your API key doesn't have access to Scribe v2\n\
                        2. Scribe API is not enabled for your account\n\
                        3. You're using an organization API key without proper permissions\n\
                        \n\
                        Check your API key permissions at: https://elevenlabs.io/app/settings/api-keys\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                if error_str.contains("429") || error_str.contains("Too Many Requests") {
                    return Err(format!(
                        "Eleven Labs API rate limit exceeded (429).\n\
                        \n\
                        Your account has exceeded the request limit.\n\
                        Check your usage at: https://elevenlabs.io/app/settings/usage\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                if error_str.contains("500") || error_str.contains("Internal Server Error") {
                    return Err(format!(
                        "Eleven Labs server error (500).\n\
                        \n\
                        The server encountered an internal error. Please try again later.\n\
                        \n\
                        Technical details: {}",
                        error_str
                    ));
                }

                return Err(format!(
                    "Failed to connect to Eleven Labs API: {}",
                    error_str
                ));
            }
        };

        info!("[ElevenLabs] Connected successfully");

        let (sink, stream) = ws_stream.split();

        *self.tx.lock().await = Some(Box::pin(sink));
        *self.rx.lock().await = Some(Box::pin(stream));

        let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(50);
        *self.audio_tx.lock().await = Some(audio_tx.clone());

        self.start_result_receiver().await;

        self.start_audio_sender(audio_rx).await;

        Ok(())
    }

    /// Start background task to receive transcription results
    ///
    /// Listens for WebSocket messages and parses:
    /// - `partial_transcript`: Interim transcription (is_definite=false)
    /// - `committed_transcript`: Final transcription after commit (is_definite=true)
    /// - `error`: Error from server
    async fn start_result_receiver(&self) {
        let rx = self.rx.clone();
        let on_partial = self.on_partial.clone();
        let last_error = self.last_error.clone();

        tokio::spawn(async move {
            let mut rx_guard = rx.lock().await;
            let rx_stream = match rx_guard.take() {
                Some(s) => s,
                None => {
                    warn!("[ElevenLabs] No receiver available");
                    return;
                }
            };
            drop(rx_guard);

            let mut stream = rx_stream;
            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        debug!("[ElevenLabs] Received message: {:.200}...", text);

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(msg_type) =
                                parsed.get("message_type").and_then(|t| t.as_str())
                            {
                                match msg_type {
                                    "partial_transcript" => {
                                        if let Some(transcript) =
                                            parsed.get("text").and_then(|t| t.as_str())
                                        {
                                            debug!(
                                                "[ElevenLabs] Partial transcript: \"{}\"",
                                                transcript
                                            );

                                            if let Some(ref callback) = on_partial {
                                                callback(PartialResult {
                                                    text: transcript.to_string(),
                                                    is_definite: false,
                                                    is_final: false,
                                                });
                                            }
                                        }
                                    }
                                    "committed_transcript" => {
                                        if let Some(transcript) =
                                            parsed.get("text").and_then(|t| t.as_str())
                                        {
                                            info!(
                                                "[ElevenLabs] Committed transcript: \"{}\"",
                                                transcript
                                            );

                                            if let Some(ref callback) = on_partial {
                                                callback(PartialResult {
                                                    text: transcript.to_string(),
                                                    is_definite: true,
                                                    is_final: false,
                                                });
                                            }
                                        }
                                    }
                                    "error" => {
                                        if let Some(error_msg) =
                                            parsed.get("error").and_then(|e| e.as_str())
                                        {
                                            error!("[ElevenLabs] Server error: {}", error_msg);

                                            *last_error.lock().await = Some(error_msg.to_string());

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
                                    _ => {
                                        debug!("[ElevenLabs] Message type: {}", msg_type);
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("[ElevenLabs] Connection closed by server");
                        break;
                    }
                    Err(e) => {
                        error!("[ElevenLabs] WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    /// Start background task to send audio chunks
    ///
    /// Receives audio chunks from the channel and sends them via WebSocket.
    /// Chunks are encoded as Base64 and wrapped in JSON messages.
    async fn start_audio_sender(&self, mut audio_rx: mpsc::Receiver<Vec<i16>>) {
        let tx = self.tx.clone();
        let task_handle = tokio::spawn(async move {
            let mut chunk_count = 0u32;
            while let Some(pcm_data) = audio_rx.recv().await {
                let mut guard = tx.lock().await;
                if let Some(sender) = guard.as_mut() {
                    if let Err(e) = send_audio_chunk(sender, &pcm_data, false).await {
                        error!("[ElevenLabs] Failed to send audio chunk: {}", e);
                        break;
                    }
                    chunk_count += 1;
                } else {
                    debug!("[ElevenLabs] Sender closed, stopping audio sender");
                    break;
                }
            }
            info!(
                "[ElevenLabs] Audio sender finished, sent {} chunks",
                chunk_count
            );
        });

        *self.audio_sender_task.lock().await = Some(task_handle);
    }

    /// Send audio data synchronously via WebSocket
    ///
    /// Directly sends audio without queuing. Use `send_audio_async` for buffered sending.
    pub async fn send_audio(&self, pcm_data: &[i16]) -> Result<(), String> {
        let mut guard = self.tx.lock().await;
        let tx = guard.as_mut().ok_or("WebSocket not connected")?;
        send_audio_chunk(tx, pcm_data, false).await
    }

    /// Send audio data asynchronously via channel
    ///
    /// Queues audio chunk for background sending. More efficient for streaming.
    pub async fn send_audio_async(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        let guard = self.audio_tx.lock().await;
        let tx = guard
            .as_ref()
            .ok_or("Audio channel not initialized - call connect() first")?;
        tx.send(pcm_data)
            .await
            .map_err(|e| format!("Failed to queue audio: {}", e))
    }

    /// Finish transcription and get final result
    ///
    /// # Process
    /// 1. Close audio channel to stop sending
    /// 2. Wait for audio sender task to complete
    /// 3. Send commit message to get final transcript
    /// 4. Wait for server response
    /// 5. Close WebSocket connection
    ///
    /// # Returns
    /// Empty string on success (actual text comes via callback)
    /// Error message if commit fails
    pub async fn finish(&self) -> Result<String, String> {
        let start = Instant::now();

        drop(self.audio_tx.lock().await.take());

        let task_handle = self.audio_sender_task.lock().await.take();
        if let Some(handle) = task_handle {
            match handle.await {
                Ok(()) => debug!("[ElevenLabs] Audio sender task completed"),
                Err(e) => warn!("[ElevenLabs] Audio sender task error: {:?}", e),
            }
        }

        let mut guard = self.tx.lock().await;
        if let Some(tx) = guard.as_mut() {
            let message = serde_json::json!({
                "message_type": "input_audio_chunk",
                "audio_base_64": "",
                "commit": true,
                "sample_rate": 16000
            });

            tx.send(Message::Text(message.to_string().into()))
                .await
                .map_err(|e| format!("Failed to send commit message: {}", e))?;

            debug!("[ElevenLabs] Sent commit message");
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        if let Some(mut tx) = guard.take() {
            let _ = tx.close().await;
        }

        let total_ms = start.elapsed().as_millis() as u64;
        info!("[ElevenLabs] Finished in {}ms", total_ms);

        Ok(String::new())
    }

    /// Close WebSocket connection
    ///
    /// Gracefully closes the connection without waiting for final transcript.
    pub async fn close(&self) {
        let mut guard = self.tx.lock().await;
        if let Some(mut tx) = guard.take() {
            let _ = tx.close().await;
            info!("[ElevenLabs] Connection closed");
        }
        *self.rx.lock().await = None;
    }
}

#[async_trait]
impl StreamingSttEngine for ElevenLabsStreamingClient {
    async fn start(&mut self) -> Result<(), String> {
        self.connect().await
    }

    async fn send_chunk(&self, pcm_data: Vec<i16>) -> Result<(), String> {
        self.send_audio_async(pcm_data).await
    }

    async fn finish(&self) -> Result<String, String> {
        self.finish().await
    }

    fn set_partial_callback(&mut self, callback: PartialResultCallback) {
        self.on_partial = Some(callback);
    }

    async fn get_audio_sender(&self) -> Option<mpsc::Sender<Vec<i16>>> {
        self.audio_tx.lock().await.clone()
    }
}

/// Send an audio chunk via WebSocket
///
/// # Arguments
/// * `sender` - WebSocket sink
/// * `pcm_data` - PCM audio samples (16-bit, mono)
/// * `commit` - Whether this is the final chunk (triggers committed_transcript)
///
/// # Message format
/// ```json
/// {
///   "message_type": "input_audio_chunk",
///   "audio_base_64": "<base64 encoded PCM>",
///   "commit": false,
///   "sample_rate": 16000
/// }
/// ```
async fn send_audio_chunk(
    sender: &mut BoxSink,
    pcm_data: &[i16],
    commit: bool,
) -> Result<(), String> {
    let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();

    let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

    let message = serde_json::json!({
        "message_type": "input_audio_chunk",
        "audio_base_64": base64_data,
        "commit": commit,
        "sample_rate": 16000
    });

    sender
        .send(Message::Text(message.to_string().into()))
        .await
        .map_err(|e| format!("Failed to send audio: {}", e))?;

    debug!(
        "[ElevenLabs] Sent {} samples, commit={}",
        pcm_data.len(),
        commit
    );
    Ok(())
}

/// Transcribe audio file using Eleven Labs Scribe v2 Realtime API
///
/// # Process
/// 1. Convert audio to 16kHz mono PCM using ffmpeg
/// 2. Connect to Eleven Labs WebSocket
/// 3. Stream audio chunks (1 second each)
/// 4. Collect final transcript via callback
///
/// # Arguments
/// * `config` - Cloud STT configuration
/// * `audio_path` - Path to audio file (any format ffmpeg supports)
/// * `language` - Optional language code (empty for auto-detect)
///
/// # Returns
/// TranscriptionResult with text and timing metrics
pub async fn transcribe_elevenlabs(
    config: &CloudSttConfig,
    audio_path: &std::path::Path,
    language: Option<&str>,
) -> Result<TranscriptionResult, String> {
    let start = Instant::now();

    if !config.enabled {
        return Err("Eleven Labs STT is not enabled".to_string());
    }

    if config.api_key.is_empty() {
        return Err("Eleven Labs API key is empty".to_string());
    }

    let temp_wav_path = audio_path.with_extension("temp_16k.wav");

    let ffmpeg_result = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path.to_str().ok_or("Invalid audio path")?,
            "-ar",
            "16000",
            "-ac",
            "1",
            "-f",
            "wav",
            "-acodec",
            "pcm_s16le",
            temp_wav_path.to_str().ok_or("Invalid temp path")?,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if !ffmpeg_result.status.success() {
        return Err(format!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&ffmpeg_result.stderr)
        ));
    }

    let converted_bytes = tokio::fs::read(&temp_wav_path)
        .await
        .map_err(|e| format!("Failed to read converted audio: {}", e))?;

    let _ = tokio::fs::remove_file(&temp_wav_path).await;

    if converted_bytes.len() < 44 {
        return Err("Converted audio too short".to_string());
    }

    let pcm_bytes = &converted_bytes[44..];

    let samples_16khz_mono: Vec<i16> = pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    let mut client = ElevenLabsStreamingClient::new(config.clone(), language);

    let (result_tx, result_rx) = tokio::sync::oneshot::channel::<String>();
    let result_tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(result_tx)));

    let result_tx_clone = result_tx.clone();
    let final_text = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
    let final_text_clone = final_text.clone();

    client.set_partial_callback(Arc::new(move |result| {
        if result.is_definite {
            let text_clone = final_text_clone.clone();
            tokio::spawn(async move {
                let mut text = text_clone.lock().await;
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&result.text);
            });
        }
        if result.is_final {
            let tx_clone = result_tx_clone.clone();
            let text_clone = final_text_clone.clone();
            tokio::spawn(async move {
                if let Some(tx) = tx_clone.lock().await.take() {
                    let text = text_clone.lock().await.clone();
                    let _ = tx.send(text);
                }
            });
        }
    }));

    client.connect().await?;

    let audio_tx = client
        .get_audio_sender()
        .await
        .ok_or("Failed to get audio sender")?;

    for chunk in samples_16khz_mono.chunks(RECOMMENDED_CHUNK_SAMPLES) {
        audio_tx
            .send(chunk.to_vec())
            .await
            .map_err(|e| format!("Failed to send audio chunk: {}", e))?;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    drop(audio_tx);

    client.finish().await?;

    if let Some(err) = client.last_error.lock().await.take() {
        return Err(err);
    }

    drop(client);
    drop(result_tx);

    let final_text: String = result_rx.await.unwrap_or_default();

    let total_ms = start.elapsed().as_millis() as u64;

    info!(
        provider = "elevenlabs",
        chars = final_text.len(),
        total_ms = total_ms,
        "Eleven Labs transcription complete"
    );

    Ok(TranscriptionResult::with_metrics(
        final_text,
        EngineType::Cloud,
        total_ms,
        Some(0),
        Some(0),
        Some(total_ms),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevenlabs_endpoint_constant() {
        assert_eq!(
            ELEVENLABS_REALTIME_ENDPOINT,
            "wss://api.elevenlabs.io/v1/speech-to-text/realtime"
        );
    }

    #[test]
    fn test_recommended_chunk_samples() {
        // 1 second at 16kHz = 16000 samples
        assert_eq!(RECOMMENDED_CHUNK_SAMPLES, 16000);
        let chunk_duration_ms = (RECOMMENDED_CHUNK_SAMPLES as f64 / 16000.0) * 1000.0;
        assert!((chunk_duration_ms - 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_partial_result_fields() {
        let result = PartialResult {
            text: "hello".to_string(),
            is_definite: true,
            is_final: false,
        };
        assert_eq!(result.text, "hello");
        assert!(result.is_definite);
        assert!(!result.is_final);
    }

    #[test]
    fn test_partial_result_serialization() {
        let result = PartialResult {
            text: "test transcript".to_string(),
            is_definite: false,
            is_final: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test transcript"));
        assert!(json.contains("is_definite"));
        assert!(json.contains("is_final"));
    }

    #[test]
    fn test_audio_chunk_message_construction() {
        // Test that the JSON message format is correct
        let pcm_data: Vec<i16> = vec![100, -100, 500, -500];
        let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();
        let base64_data =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

        let message = serde_json::json!({
            "message_type": "input_audio_chunk",
            "audio_base_64": base64_data,
            "commit": false,
            "sample_rate": 16000
        });

        // Verify message structure
        assert_eq!(message["message_type"], "input_audio_chunk");
        assert_eq!(message["commit"], false);
        assert_eq!(message["sample_rate"], 16000);
        assert!(!message["audio_base_64"].as_str().unwrap().is_empty());
    }

    #[test]
    fn test_audio_chunk_commit_message() {
        // Test commit message (final chunk)
        let message = serde_json::json!({
            "message_type": "input_audio_chunk",
            "audio_base_64": "",
            "commit": true,
            "sample_rate": 16000
        });

        assert_eq!(message["message_type"], "input_audio_chunk");
        assert_eq!(message["commit"], true);
        assert_eq!(message["audio_base_64"], "");
        assert_eq!(message["sample_rate"], 16000);
    }

    #[test]
    fn test_base64_encoding_pcm() {
        // Test Base64 encoding of PCM data
        let pcm_data: Vec<i16> = vec![0, 1000, -1000, 32767, -32768];
        let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();

        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

        // Verify encoding is valid Base64
        assert!(encoded
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));

        // Verify decoding produces original bytes
        let decoded =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &encoded).unwrap();

        let decoded_samples: Vec<i16> = decoded
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        assert_eq!(decoded_samples, pcm_data);
    }

    #[test]
    fn test_partial_transcript_response_parsing() {
        // Test parsing of partial_transcript message
        let json = r#"{
            "message_type": "partial_transcript",
            "text": "hello wor..."
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(parsed["message_type"], "partial_transcript");
        assert_eq!(parsed["text"], "hello wor...");
    }

    #[test]
    fn test_committed_transcript_response_parsing() {
        // Test parsing of committed_transcript message
        let json = r#"{
            "message_type": "committed_transcript",
            "text": "hello world"
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(parsed["message_type"], "committed_transcript");
        assert_eq!(parsed["text"], "hello world");
    }

    #[test]
    fn test_error_response_parsing() {
        // Test parsing of error message
        let json = r#"{
            "message_type": "error",
            "error": "Invalid audio format"
        }"#;

        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(parsed["message_type"], "error");
        assert_eq!(parsed["error"], "Invalid audio format");
    }

    #[test]
    fn test_elevenlabs_client_new_with_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "elevenlabs".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = ElevenLabsStreamingClient::new(config, Some("en"));
        assert_eq!(client._language, "en");
    }

    #[test]
    fn test_elevenlabs_client_new_auto_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "elevenlabs".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = ElevenLabsStreamingClient::new(config, Some("auto"));
        assert_eq!(client._language, "");
    }

    #[test]
    fn test_elevenlabs_client_new_none_language() {
        let config = CloudSttConfig {
            enabled: true,
            provider_type: "elevenlabs".to_string(),
            api_key: "test-key".to_string(),
            app_id: "".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            language: "".to_string(),
        };
        let client = ElevenLabsStreamingClient::new(config, None);
        assert_eq!(client._language, "");
    }

    #[test]
    fn test_pcm_bytes_conversion() {
        // Test conversion of i16 samples to bytes
        let samples: Vec<i16> = vec![100, -100, 0, 255];
        let bytes: Vec<u8> = samples.iter().flat_map(|&s| s.to_le_bytes()).collect();

        // Each i16 produces 2 bytes
        assert_eq!(bytes.len(), samples.len() * 2);

        // Verify byte values (little-endian)
        assert_eq!(bytes[0], 100); // 100 = 0x0064, low byte = 100
        assert_eq!(bytes[1], 0); // high byte = 0

        // -100 in little-endian: 0xFF9C, low byte = 156 (0x9C), high byte = 255 (0xFF)
        let neg_bytes = (-100i16).to_le_bytes();
        assert_eq!(bytes[2], neg_bytes[0]);
        assert_eq!(bytes[3], neg_bytes[1]);
    }

    #[test]
    fn test_empty_pcm_chunk() {
        // Test handling of empty PCM data
        let pcm_data: Vec<i16> = vec![];
        let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();

        assert!(bytes.is_empty());

        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

        // Empty input produces empty base64
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_large_pcm_chunk() {
        // Test handling of large PCM chunk (1 second at 16kHz)
        let pcm_data: Vec<i16> = vec![0i16; RECOMMENDED_CHUNK_SAMPLES];
        let bytes: Vec<u8> = pcm_data.iter().flat_map(|&s| s.to_le_bytes()).collect();

        // 16000 samples * 2 bytes = 32000 bytes
        assert_eq!(bytes.len(), 32000);

        // Verify Base64 encoding works for large data
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);

        // Base64 encodes ~4/3 ratio, so 32000 bytes -> ~42667 chars
        assert!(encoded.len() > 40000);
        assert!(encoded.len() < 45000);
    }
}
